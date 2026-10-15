import Foundation
import Combine

struct NativeRecordedStep:Decodable,Sendable {
  let id:String,rule_id:String,level:String,title_key:String,params:[String:String]
  let before_latex:[String],after_latex:[String]
}
struct NativeStepEntry:Decodable,Sendable {
  let path:[UInt32],step:NativeRecordedStep,child_count:UInt32
}
struct NativeStepPage:Decodable,Sendable {
  let view_id:String,parent_path:[UInt32],recorded:Bool,total:UInt32,offset:UInt32,entries:[NativeStepEntry]
}
struct NativeStepRoute:Sendable {
  let path:[UInt32],offset:UInt32,label:String
}
/// A page is an immutable original solver record. Navigation never evaluates an expression.
@MainActor final class NativeStepsModel:ObservableObject {
  let original:ResultBinding
  @Published private(set) var page:NativeStepPage?
  @Published private(set) var trail:[NativeStepRoute]=[]
  @Published private(set) var route=NativeStepRoute(path:[],offset:0,label:"全部步骤")
  @Published private(set) var busy=false
  @Published private(set) var error:String?
  private let client:ResultClient
  private var closed=false
  nonisolated static let pageSize:UInt32=16
  init(host:NativeHostClient,binding:ResultBinding) {original=binding;client=ResultClient(host:host)}
  func load(offset:UInt32?=nil)async {await read(.init(path:route.path,offset:offset ?? route.offset,label:route.label),trail:trail)}
  func children(_ entry:NativeStepEntry)async {
    guard entry.child_count>0,entry.path.count<=128 else {return}
    await read(.init(path:entry.path,offset:0,label:entry.step.id),trail:trail+[route])
  }
  func back(to index:Int)async {
    guard trail.indices.contains(index) else {return}
    await read(trail[index],trail:Array(trail.prefix(index)))
  }
  private func read(_ next:NativeStepRoute,trail:[NativeStepRoute])async {
    guard !closed,!busy else {return};busy=true;error=nil;defer {busy=false}
    do {
      try client.select(original)
      guard let reply=try await client.inspect(.inspect_result_steps(.init(kind:.steps,parent_path:next.path,offset:next.offset,limit:Self.pageSize))) else {throw ResultClientError.staleSelection}
      let actual=try await Task.detached {try JSONDecoder().decode(NativeStepPage.self,from:JSONEncoder().encode(reply.payload))}.value
      guard actual.view_id==original.view_id.value,actual.parent_path==next.path,actual.offset==next.offset,actual.offset<=actual.total,
        actual.entries.count==Int(min(Self.pageSize,actual.total-actual.offset)),
        actual.entries.enumerated().allSatisfy({i,entry in entry.path==next.path+[actual.offset+UInt32(i)] && !entry.step.id.isEmpty && !entry.step.rule_id.isEmpty}),
        actual.recorded || (next.path.isEmpty && actual.total==0 && actual.offset==0) else {throw ResultClientError.invalidReply}
      guard !closed,!Task.isCancelled else {return}
      page=actual;route=next;self.trail=trail
    } catch {if !closed,!Task.isCancelled {self.error="步骤暂无法读取："+String(describing:error)}}
  }
  func close()async {closed=true;await client.close()}
}
