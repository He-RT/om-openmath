import CryptoKit
import Darwin
import Foundation
import SQLite3
@main struct BlobFixtures {
  static func main() async throws {
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    if CommandLine.arguments.count>3 {
      let stage=CommandLine.arguments[3]
      if stage=="inspect-crash" {
        let (store,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
        let bytes=Data("owned crash payload".utf8)
        let hash=SHA256.hash(data:bytes).map{String(format:"%02x",$0)}.joined()
        let state=try await store.inspectBlob(.init(hash:hash,byteLength:UInt64(bytes.count)))
        precondition(state.referenceCount==0 && state.pinCount==0)
        print(state.state == .missing ? "missing-no-ready-reference" : "complete-unreferenced-bytes")
        try await store.close();return
      }
      let (store,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
      let faults=StorageFaults { point in if point==stage {FileHandle.standardOutput.write(Data((point+"\n").utf8));raise(SIGSTOP)} }
      _ = try await store.publishBlob(data:Data("owned crash payload".utf8),faults:faults)
      try await store.close();return
    }
    let fixtures=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true)
    let (store,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let original=try Data(contentsOf:fixtures.appendingPathComponent("Media/math-text.pdf"))
    let publication=try await store.publishBlob(data:original)
    precondition(publication.descriptor.hash==SHA256.hash(data:original).map{String(format:"%02x",$0)}.joined())
    let reader=try await store.openBlob(publication)
    let copied=try await store.readBlob(reader,offset:0,count:1024*1024)
    precondition(copied==original)
    try await store.releaseBlob(publication)
    let active=try await store.inspectBlob(publication.descriptor)
    precondition(active.state == .pinned && active.pinCount==1)
    try await store.closeBlob(reader)
    let orphan=try await store.inspectBlob(publication.descriptor)
    precondition(orphan.state == .orphanCandidate && orphan.allStoresChecked)
    let again=try await store.publishBlob(file:fixtures.appendingPathComponent("Media/math-text.pdf"),expectedHash:publication.descriptor.hash)
    precondition(again.descriptor==publication.descriptor)
    let owner=BlobOwner(kind:.attachment,id:"attachment-one")
    let reference=try await store.attachBlob(again,owner:owner,mediaType:"application/pdf",codec:"raw-v1")
    let duplicate=try await store.attachBlob(again,owner:owner,mediaType:"application/pdf",codec:"raw-v1")
    precondition(reference==duplicate)
    do { _ = try await store.attachBlob(again,owner:owner,mediaType:"image/png",codec:"raw-v1");fatalError("same owner metadata overwritten") }
    catch StorageError.idempotencyConflict { }
    try await store.releaseBlob(again)
    let referenced=try await store.inspectBlob(again.descriptor)
    precondition(referenced.state == .referenced && referenced.referenceCount==1)
    // Publication and ownership are separate facts. A lost post-COMMIT ref acknowledgement
    // can be reconciled without re-copying or inventing a new owner identity.
    let lost=try await store.publishBlob(data:Data("lost reference ACK".utf8))
    let lostOwner=BlobOwner(kind:.context,id:"context-lost")
    let lostAck=StorageFaults { point in if point=="blob_reference_committed" {throw StorageError.system(EIO,"fixture-lost-ref-ack")} }
    do {_ = try await store.attachBlob(lost,owner:lostOwner,mediaType:"application/json",codec:"json-v1",faults:lostAck);fatalError("lost ref ack reported definite success")}
    catch StorageError.unknownCommit { }
    let resolved=try await store.blobReference(owner:lostOwner,hash:lost.descriptor.hash)
    precondition(resolved?.descriptor==lost.descriptor);try await store.releaseBlob(lost)
    let failed=try await store.publishBlob(data:Data("released orphan".utf8))
    let fail=StorageFaults { point in if point=="blob_reference_inserted" {throw StorageError.system(ENOSPC,"fixture-ref")}}
    do {_ = try await store.attachBlob(failed,owner:.init(kind:.result,id:"failed-result"),mediaType:"application/json",codec:"test-v1",faults:fail);fatalError("failed DB reference reported success")}
    catch StorageError.system { }
    let absent=try await store.blobReference(owner:.init(kind:.result,id:"failed-result"),hash:failed.descriptor.hash)
    precondition(absent==nil)
    try await store.releaseBlob(failed)
    let failedState=try await store.inspectBlob(failed.descriptor)
    precondition(failedState.state == .orphanCandidate)
    let forged=BlobPublication(descriptor:failed.descriptor,pinID:"forged")
    do {_ = try await store.attachBlob(forged,owner:.init(kind:.result,id:"fake"),mediaType:"application/json",codec:"v1");fatalError("forged pin became database reference")}
    catch BlobError.invalidPin { }
    do {_ = try await store.publishBlob(data:Data([1,2]),expectedHash:String(repeating:"0",count:64));fatalError("wrong SHA reported ready")}
    catch BlobError.corrupt { }
    do {_ = try await store.publishBlob(data:Data([1]),expectedHash:"../../escape");fatalError("hash chose path")}
    catch BlobError.invalidHash { }
    let damaged=try await store.publishBlob(data:Data("original immutable bytes".utf8))
    let damagedPath=root.appendingPathComponent("Blobs/sha256/"+String(damaged.descriptor.hash.prefix(2))+"/"+damaged.descriptor.hash)
    let badBytes=Data(repeating:0x7a,count:Int(damaged.descriptor.byteLength))
    try badBytes.write(to:damagedPath)
    do {_ = try await store.publishBlob(data:Data("original immutable bytes".utf8));fatalError("same-name damaged blob silently replaced")}
    catch BlobError.corrupt { }
    let stillDamaged=try Data(contentsOf:damagedPath);precondition(stillDamaged==badBytes)
    let damagedState=try await store.inspectBlob(damaged.descriptor);precondition(damagedState.state == .corrupt)
    try await store.releaseBlob(damaged)
    let missing=try await store.publishBlob(data:Data("missing later".utf8))
    let missingPath=root.appendingPathComponent("Blobs/sha256/"+String(missing.descriptor.hash.prefix(2))+"/"+missing.descriptor.hash)
    try FileManager.default.removeItem(at:missingPath)
    let missingState=try await store.inspectBlob(missing.descriptor);precondition(missingState.state == .missing)
    do {_ = try await store.openBlob(missing);fatalError("missing blob read as empty success")}
    catch BlobError.missing { }
    try await store.releaseBlob(missing)
    let empty=try await store.publishBlob(data:Data())
    let emptyReader=try await store.openBlob(empty)
    let emptyBytes=try await store.readBlob(emptyReader,offset:0,count:1024)
    precondition(emptyBytes.isEmpty && empty.descriptor.byteLength==0)
    try await store.closeBlob(emptyReader);try await store.releaseBlob(empty)
    let changing=root.appendingPathComponent("changing-input")
    try Data("original input".utf8).write(to:changing)
    let change=StorageFaults { point in
      if point=="blob_stage_opened" {
        let fd=Darwin.open(changing.path,O_WRONLY|O_TRUNC)
        guard fd>=0 else {throw StorageError.system(errno,"fixture-input")}
        defer {Darwin.close(fd)}
        let bytes=Data("changed input".utf8)
        _ = bytes.withUnsafeBytes{Darwin.write(fd,$0.baseAddress,$0.count)}
      }
    }
    do {_ = try await store.publishBlob(file:changing,faults:change);fatalError("changed input presented as original")}
    catch BlobError.sourceChanged { }
    let fifo=root.appendingPathComponent("not-regular-input")
    precondition(mkfifo(fifo.path,0o600)==0)
    do {_ = try await store.publishBlob(file:fifo);fatalError("FIFO treated as regular media input")}
    catch StorageError.unsafePath { }
    let hugePath=root.appendingPathComponent("sparse-input")
    let sparse=Darwin.open(hugePath.path,O_WRONLY|O_CREAT|O_EXCL,0o600)
    precondition(sparse>=0 && ftruncate(sparse,128*1024*1024+1)==0);Darwin.close(sparse)
    do {_ = try await store.publishBlob(file:hugePath);fatalError("over-limit source copied")}
    catch BlobError.budgetExceeded { }
    // A >1MiB self-authored value is reconstructed through bounded reads, including the final byte.
    let large=Data((0..<(3*1024*1024+17)).map{UInt8($0%251)})
    let big=try await store.publishBlob(data:large),largeReader=try await store.openBlob(big)
    var output=Data(),offset:UInt64=0
    while offset<big.descriptor.byteLength {let next=try await store.readBlob(largeReader,offset:offset,count:1024*1024);output.append(next);offset+=UInt64(next.count)}
    precondition(output==large)
    do {_ = try await store.readBlob(largeReader,offset:0,count:2*1024*1024);fatalError("unbounded read admitted")}
    catch BlobError.budgetExceeded { }
    try await store.closeBlob(largeReader);try await store.releaseBlob(big)
    // Cancellation is an actual per-job signal; the blocking test boundary only controls timing.
    let entered=DispatchSemaphore(value:0),release=DispatchSemaphore(value:0)
    let blocked=StorageFaults { point in if point=="blob_copied" {entered.signal();release.wait()} }
    let cancelled=Task {try await store.publishBlob(data:Data("cancel fixture".utf8),faults:blocked)}
    let reached:Bool=await withCheckedContinuation { continuation in
      DispatchQueue.global(qos:.utility).async {continuation.resume(returning:entered.wait(timeout:.now()+5) == .success)}
    }
    precondition(reached)
    cancelled.cancel();release.signal()
    do {_ = try await cancelled.value;fatalError("cancelled publication reported ready")}
    catch BlobError.cancelled { }
    catch is CancellationError { }
    let cancelBytes=Data("cancel fixture".utf8),cancelHash=SHA256.hash(data:cancelBytes).map{String(format:"%02x",$0)}.joined()
    let cancelledState=try await store.inspectBlob(.init(hash:cancelHash,byteLength:UInt64(cancelBytes.count)))
    precondition(cancelledState.state == .missing)
    try await store.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let persisted=try await reopened.blobReference(owner:owner,hash:again.descriptor.hash)
    precondition(persisted==reference)
    let oldPin=BlobPublication(descriptor:again.descriptor,pinID:again.pinID)
    do {_ = try await reopened.openBlob(oldPin);fatalError("old pin resurrected after restart")}
    catch BlobError.invalidPin { }
    let verified=try await reopened.openReferencedBlob(owner:owner,hash:again.descriptor.hash)
    let recovered=try await reopened.readBlob(verified,offset:0,count:1024*1024)
    precondition(recovered==original);try await reopened.closeBlob(verified)
    let restored=try await reopened.inspectBlob(again.descriptor)
    precondition(restored.state == .referenced && restored.pinCount==0)
    try await reopened.close()
    print("Actual immutable blobs: exact original PDF and 3MiB byte readback, non-overwrite dedup, pin/reader retention, atomic DB refs/rollback, corruption/missing/hash rejection and persisted refs with no resurrected pins passed")
  }
}
