import Foundation
import Darwin

/// Byte framing only. Complete original frames go to Rust for closed schema and duplicate-key checks.
struct AgentFrameBytes {
  static let maximum=1024*1024
  private var pending=Data(),failed=false
  mutating func push(_ bytes:Data)throws->[Data] {
    guard !failed,bytes.count<=128*1024 else {failed=true;throw AgentSessionError.invalidFrame}
    var frames:[Data]=[]
    for byte in bytes {
      if byte==10 {
        if pending.last==13 {pending.removeLast()}
        guard !pending.isEmpty,frames.count<32 else {failed=true;throw AgentSessionError.invalidFrame}
        frames.append(pending);pending=Data()
      } else {
        guard pending.count<Self.maximum else {failed=true;throw AgentSessionError.invalidFrame}
        pending.append(byte)
      }
    }
    return frames
  }
  mutating func finish()throws {guard !failed,pending.isEmpty else {failed=true;throw AgentSessionError.invalidFrame}}
}

/// Exit is registered before launch, so observing EOF/exit before close cannot miss completion.
private final class AgentChildExit:@unchecked Sendable {
  private let lock=NSLock()
  private var finished=false,waiters:[CheckedContinuation<Void,Never>]=[]
  func finish() {
    let pending=lock.withLock {()->[CheckedContinuation<Void,Never>] in
      guard !finished else {return []};finished=true
      let pending=waiters;waiters.removeAll();return pending
    }
    for waiter in pending {waiter.resume()}
  }
  func wait()async {
    await withCheckedContinuation {continuation in
      let ready=lock.withLock {if finished {return true};waiters.append(continuation);return false}
      if ready {continuation.resume()}
    }
  }
}

/// The native caller supplies the trusted executable. The child receives a private minimal
/// environment, not app credentials, shell profiles or inherited provider settings.
actor AgentPrivatePipe {
  private let process:Process,input:FileHandle,output:FileHandle,errors:FileHandle
  private let exit:AgentChildExit
  private let ownedDirectory:URL?
  private var capture:(@Sendable (Data,Bool)async->Void)?
  private var closed=false,writing=false
  private var nextWrite:UInt64=1
  private var pending:[UInt64:Data]=[:],replays:[Data]=[]
  private var written:[UInt64:Data]=[:]
  private var inflight:(UInt64,Data)?
  private var pendingBytes=0,writtenBytes=0
  private init(process:Process,input:FileHandle,output:FileHandle,errors:FileHandle,exit:AgentChildExit,directory:URL?) {self.exit=exit;self.process=process;self.input=input;self.output=output;self.errors=errors;self.ownedDirectory=directory}
  static func launch(executable:URL,arguments:[String],directory:URL,environment:[String:String]=["PATH":"/usr/bin:/bin"],removeDirectoryOnClose:Bool=false) async throws->AgentPrivatePipe {
    try await Task.detached {
      let process=Process(),stdin=Pipe(),stdout=Pipe(),stderr=Pipe(),exit=AgentChildExit()
      process.terminationHandler={_ in exit.finish()}
      process.executableURL=executable;process.arguments=arguments;process.currentDirectoryURL=directory
      process.environment=environment
      process.standardInput=stdin;process.standardOutput=stdout;process.standardError=stderr
      try process.run()
      let pipe=AgentPrivatePipe(process:process,input:stdin.fileHandleForWriting,output:stdout.fileHandleForReading,errors:stderr.fileHandleForReading,exit:exit,directory:removeDirectoryOnClose ? directory:nil)
      Task {await pipe.drainErrors()}
      return pipe
    }.value
  }
  /// Bounded drain; stderr content is untrusted and never presented, persisted or included in errors.
  private func drainErrors() async {
    while !closed {
      let fd=Darwin.dup(errors.fileDescriptor);guard fd>=0 else {return}
      let bytes=try? await Task.detached {defer {_ = Darwin.close(fd)};return try Self.readChunk(fd,maximum:16*1024)}.value
      if bytes == nil || bytes?.isEmpty==true {return}
    }
  }
  func read() async throws->Data? {
    guard !closed else {return nil};let fd=Darwin.dup(output.fileDescriptor)
    guard fd>=0 else {throw AgentSessionError.invalidFrame}
    let bytes=try await Task.detached {defer {_ = Darwin.close(fd)};return try Self.readChunk(fd,maximum:128*1024)}.value
    if let bytes,let capture {await capture(bytes,false)}
    return bytes
  }
  /// Native command completions can resume out of order. Sequence buffering preserves complete
  /// frames; exact old ACK replays remain byte-identical. No write waits on the actor's read loop.
  func write(_ bytes:Data) async throws {
    guard !closed,bytes.count<=AgentFrameBytes.maximum+1,bytes.last==10,
      let object=try JSONSerialization.jsonObject(with:bytes) as? [String:Any],let sequence=object["sequence"] as? UInt64,sequence>0 else {throw AgentSessionError.invalidFrame}
    if let inflight,sequence==inflight.0 {
      guard inflight.1==bytes else {throw AgentSessionError.invalidFrame}
      replays.append(bytes)
    } else if sequence<nextWrite {
      guard written[sequence]==bytes else {throw AgentSessionError.invalidFrame}
      replays.append(bytes)
    } else {
      if let original=pending[sequence] {guard original==bytes else {throw AgentSessionError.invalidFrame};return}
      pending[sequence]=bytes
    }
    pendingBytes+=bytes.count
    guard pending.count+replays.count<=32,pendingBytes<=2*AgentFrameBytes.maximum else {throw AgentSessionError.invalidFrame}
    guard !writing else {return};writing=true;defer {writing=false}
    while !closed {
      let item:Data,isReplay:Bool
      if let next=pending.removeValue(forKey:nextWrite) {item=next;isReplay=false}
      else if !replays.isEmpty {item=replays.removeFirst();isReplay=true}
      else {return}
      pendingBytes-=item.count
      if !isReplay {inflight=(nextWrite,item)}
      let handle=input
      try await Task.detached {try handle.write(contentsOf:item)}.value
      if let capture {await capture(item,true)}
      if !isReplay {
        guard written.count<4096,writtenBytes+item.count<=8*AgentFrameBytes.maximum else {throw AgentSessionError.invalidFrame}
        written[nextWrite]=item;writtenBytes+=item.count;nextWrite+=1;inflight=nil
      }
    }
  }
  /// One POSIX read returns available bytes; Foundation read(upToCount:) can wait to fill
  /// its requested length and deadlock a peer that waits for a small-frame ACK.
  private nonisolated static func readChunk(_ fd:Int32,maximum:Int)throws->Data? {
    var bytes=[UInt8](repeating:0,count:maximum)
    while true {
      let count=bytes.withUnsafeMutableBytes {Darwin.read(fd,$0.baseAddress,maximum)}
      if count==0 {return nil}
      if count>0 {return Data(bytes.prefix(count))}
      if errno != EINTR {throw AgentSessionError.invalidFrame}
    }
  }
  /// Development fixtures can retain their own authorized exact wire inputs/outputs.
  func setCapture(_ capture:@escaping @Sendable (Data,Bool)async->Void) {self.capture=capture}
  /// Supervisor fault recovery can force termination of its own private child.
  func terminateImmediately() {if process.isRunning {_ = Darwin.kill(process.processIdentifier,SIGKILL)}}
  func close() async {
    guard !closed else {return};closed=true
    if process.isRunning {process.terminate()}
    for _ in 0..<20 {
      if !process.isRunning {break}
      try? await Task.sleep(for:.milliseconds(10))
    }
    if process.isRunning {_ = Darwin.kill(process.processIdentifier,SIGKILL)}
    await exit.wait()
    try? input.close();try? output.close();try? errors.close()
    if let directory=ownedDirectory {try? FileManager.default.removeItem(at:directory)}
  }
}
