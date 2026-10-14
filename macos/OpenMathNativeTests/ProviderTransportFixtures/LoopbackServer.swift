import Foundation
import Darwin

enum LoopbackFixtureError:Error {case socket}
/// One real HTTP connection, owned entirely by this fixture. No system proxy/server changes.
final class DelayedLoopbackServer:@unchecked Sendable {
  let port:UInt16
  private let descriptor:Int32,lock=NSLock()
  private var received=false
  private var request=Data()
  var receivedBody:Bool {lock.withLock {received}}
  var requestBytes:Data {lock.withLock {request}}
  init(response:Data,delay:Double)throws {
    let descriptor=Darwin.socket(AF_INET,SOCK_STREAM,0)
    guard descriptor>=0 else {throw LoopbackFixtureError.socket}
    var address=sockaddr_in();address.sin_len=UInt8(MemoryLayout<sockaddr_in>.size);address.sin_family=sa_family_t(AF_INET);address.sin_port=0;address.sin_addr.s_addr=inet_addr("127.0.0.1")
    let bound=withUnsafePointer(to:&address){pointer in pointer.withMemoryRebound(to:sockaddr.self,capacity:1){Darwin.bind(descriptor,$0,socklen_t(MemoryLayout<sockaddr_in>.size))}}
    guard bound==0,Darwin.listen(descriptor,1)==0 else {Darwin.close(descriptor);throw LoopbackFixtureError.socket}
    var size=socklen_t(MemoryLayout<sockaddr_in>.size)
    let named=withUnsafeMutablePointer(to:&address){pointer in pointer.withMemoryRebound(to:sockaddr.self,capacity:1){Darwin.getsockname(descriptor,$0,&size)}}
    guard named==0 else {Darwin.close(descriptor);throw LoopbackFixtureError.socket}
    self.descriptor=descriptor;port=UInt16(bigEndian:address.sin_port)
    DispatchQueue.global(qos:.userInitiated).async {[self] in serve(response,delay)}
  }
  private func serve(_ response:Data,_ delay:Double) {
    defer {Darwin.close(descriptor)}
    let peer=Darwin.accept(descriptor,nil,nil);guard peer>=0 else {return};defer {Darwin.close(peer)}
    var noSignal:Int32=1;_ = Darwin.setsockopt(peer,SOL_SOCKET,SO_NOSIGPIPE,&noSignal,socklen_t(MemoryLayout<Int32>.size))
    var timeout=timeval(tv_sec:5,tv_usec:0);_ = Darwin.setsockopt(peer,SOL_SOCKET,SO_RCVTIMEO,&timeout,socklen_t(MemoryLayout<timeval>.size))
    var data=Data(),buffer=[UInt8](repeating:0,count:4096)
    while data.count<2*1024*1024 {
      let count=Darwin.recv(peer,&buffer,buffer.count,0);guard count>0 else {return};data.append(contentsOf:buffer.prefix(count))
      if let boundary=data.range(of:Data("\r\n\r\n".utf8)),let header=String(data:data[..<boundary.lowerBound],encoding:.utf8) {
        let length=header.components(separatedBy:"\r\n").first(where:{$0.lowercased().hasPrefix("content-length:")}).flatMap{Int($0.split(separator:":",maxSplits:1)[1].trimmingCharacters(in:.whitespaces))}
        if let length,length>0,data.count-boundary.upperBound>=length {break}
      }
    }
    lock.withLock {received=true;request=data}
    Thread.sleep(forTimeInterval:delay)
    let head=Data("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: \(response.count)\r\nConnection: close\r\n\r\n".utf8),wire=head+response
    wire.withUnsafeBytes {raw in
      var offset=0
      while offset<raw.count {let count=Darwin.send(peer,raw.baseAddress!.advanced(by:offset),raw.count-offset,0);if count<=0{return};offset+=count}
    }
  }
}
