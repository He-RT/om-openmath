import Foundation
import Combine

/// These are projections of the existing kernel payload, never a second solver/value owner.
struct NativeExpressionProjection:Codable,Sendable {
  let input_form:String,modern_form:HostNullable<String>,latex:String
  init(input_form:String,modern_form:String?,latex:String) {self.input_form=input_form;self.modern_form = .init(modern_form);self.latex=latex}
}
enum NativeVerification:Codable,Sendable {
  case exact,construction,numeric(UInt32),unverified
  private enum Key:String,CodingKey {case Numeric,digits}
  init(from decoder:Decoder)throws {
    let single=try decoder.singleValueContainer()
    if let text=try? single.decode(String.self) {
      switch text {
      case "Exact":self = .exact
      case "ByConstruction":self = .construction
      case "Unverified":self = .unverified
      default:throw ResultClientError.invalidReply
      }
    } else {
      let all=try decoder.container(keyedBy:HostCodingKey.self)
      guard all.allKeys.map(\.stringValue)==["Numeric"] else {throw ResultClientError.invalidReply}
      let outer=try decoder.container(keyedBy:Key.self)
      guard outer.allKeys == [.Numeric] else {throw ResultClientError.invalidReply}
      let inner=try outer.nestedContainer(keyedBy:Key.self,forKey:.Numeric)
      let raw=try all.nestedContainer(keyedBy:HostCodingKey.self,forKey:HostCodingKey(stringValue:"Numeric")!)
      guard raw.allKeys.map(\.stringValue)==["digits"] else {throw ResultClientError.invalidReply}
      let digits=try inner.decode(UInt32.self,forKey:.digits)
      guard inner.allKeys == [.digits],digits>0 else {throw ResultClientError.invalidReply}
      self = .numeric(digits)
    }
  }
  func encode(to encoder:Encoder)throws {
    switch self {
    case .exact:var c=encoder.singleValueContainer();try c.encode("Exact")
    case .construction:var c=encoder.singleValueContainer();try c.encode("ByConstruction")
    case .unverified:var c=encoder.singleValueContainer();try c.encode("Unverified")
    case .numeric(let digits):var c=encoder.container(keyedBy:Key.self);var n=c.nestedContainer(keyedBy:Key.self,forKey:.Numeric);try n.encode(digits,forKey:.digits)
    }
  }
  var label:String {switch self {case .exact:"精确验证";case .construction:"按构造成立";case .unverified:"尚未验证";case .numeric(let digits):"数值验证（\(digits) 位）"}}
}
struct NativeSolutionBinding:Codable,Sendable {
  let variable:String,latex:String,input_form:String,modern_form:String
  let numeric:HostNullable<String>
  let var_latex:String?
  let root_index:UInt32?
  let radicals:NativeExpressionProjection?
  private enum CodingKeys:String,CodingKey {case variable="var",latex,input_form,modern_form,numeric,var_latex,root_index,radicals}
}
struct NativeSolutionEntry:Codable,Sendable {
  let bindings:[NativeSolutionBinding]
  let condition_latex:HostNullable<String>
  let condition_display_latex:String?
  let verified:NativeVerification
  /// Raw byte identity includes conditions and verification. Canonically equivalent Swift
  /// strings and identical pretty labels cannot merge distinct mathematical records.
  var identity:Data? {let encoder=JSONEncoder();encoder.outputFormatting = .sortedKeys;return try? encoder.encode(self)}
}
enum NativeSolutionKind:String,Codable,Sendable {case finite,none,all,region}
struct NativeSolutionInterval:Decodable,Sendable {
  let lo:HostNullable<String>,hi:HostNullable<String>
  let lo_closed:Bool,hi_closed:Bool
  let lo_value:HostNullable<Double>,hi_value:HostNullable<Double>
}
struct NativeSolutionPage:Decodable,Sendable {
  let kind:String,solution_kind:NativeSolutionKind,vars:[String]
  let region_latex:HostNullable<String>
  let total_solutions:UInt32,total_intervals:UInt32,offset:UInt32
  let solutions:[NativeSolutionEntry],intervals:[NativeSolutionInterval]
  let solution_multiplicities:[UInt32],solution_first_indices:[UInt32]
  var total:UInt32 {max(total_solutions,total_intervals)}
  var complete:Bool {offset==0 && solutions.count==Int(total_solutions) && intervals.count==Int(total_intervals)}
  struct Group:Sendable {let index:Int,firstIndex:UInt32,entry:NativeSolutionEntry,multiplicity:UInt32}
  var groups:[Group] {
    // Global multiplicity/first ordinal are provided by the kernel from all real records.
    // Only display grouping within this page is local, with the same complete byte identity.
    var result:[Group]=[],indices:[Data:Int]=[:]
    for (index,entry) in solutions.enumerated() {
      if let key=entry.identity {
        if indices[key] != nil {continue}
        indices[key]=result.count
      }
      result.append(.init(index:index,firstIndex:solution_first_indices[index],entry:entry,multiplicity:solution_multiplicities[index]))
    }
    return result
  }
}
struct NativeResultMessage:Decodable,Sendable {let symbol:String,tag:String,text:String,level:String}
struct NativeStatementSummary:Codable,Sendable {
  let out_index:UInt32,view_id:String,suppressed:Bool,value_kind:String,nature:String,output_kind:String,steps_recorded:Bool
}
struct NativeResultSummary:Decodable,Sendable {
  let cell_id:String,cell_kind:String,status:String
  let source_byte_length:UInt64,statement_count:UInt32
  let statement:HostNullable<NativeStatementSummary>
  let messages:[NativeResultMessage]
}
struct NativeResultPresentation:Sendable {
  let kind:String,viewID:String?,solutions:NativeSolutionPage?,error:String?
  static func decode(_ payload:HostJSONValue,binding:ResultBinding,offset:UInt32,limit:UInt32)throws->Self {
    guard case .object(let root)=payload else {throw ResultClientError.invalidReply}
    if binding.out_index.value==0 {
      guard root["kind"] == .string("error") else {throw ResultClientError.invalidReply}
      return .init(kind:"error",viewID:nil,solutions:nil,error:nil)
    }
    guard root["view_id"] == .string(binding.view_id.value ?? ""),case .object(let view)=root["presentation"],case .string(let kind)=view["kind"] else {throw ResultClientError.invalidReply}
    var solutions:NativeSolutionPage?
    if kind=="solutions" {
      let page=try JSONDecoder().decode(NativeSolutionPage.self,from:JSONEncoder().encode(HostJSONValue.object(view)))
      guard page.offset==offset,page.solutions.count<=limit,page.intervals.count<=limit,page.offset<=page.total,
        page.kind=="solutions",
        page.solution_multiplicities.count==page.solutions.count,page.solution_first_indices.count==page.solutions.count,
        page.solutions.count<=Int(page.total_solutions>offset ? page.total_solutions-offset : 0),
        page.intervals.count<=Int(page.total_intervals>offset ? page.total_intervals-offset : 0),
        zip(page.solution_multiplicities,page.solution_first_indices).enumerated().allSatisfy({$0.element.0>0 && $0.element.0<=page.total_solutions && $0.element.1<=offset+UInt32($0.offset)}) else {throw ResultClientError.invalidReply}
      solutions=page
    }
    let error:String?;if case .string(let message)=view["message"] {error=message} else {error=nil}
    return .init(kind:kind,viewID:binding.view_id.value,solutions:solutions,error:error)
  }
}
struct NativeNumericProjection:Sendable {
  let binding:ResultBinding,target:Data,value:NativeExpressionProjection
}
/// One immutable result-ref owns its readonly projections. The document keeps this owner alive
/// through pane hiding; no request changes the active notebook, Out, definitions or random state.
@MainActor final class ResultPresenter:ObservableObject {
  nonisolated static let pageSize:UInt32=16
  let original:ResultBinding
  @Published private(set) var binding:ResultBinding
  @Published private(set) var summary:NativeResultSummary?
  @Published private(set) var presentation:NativeResultPresentation?
  @Published private(set) var busy=false
  @Published private(set) var message:String?
  @Published private(set) var numeric:[Data:NativeNumericProjection]=[:]
  @Published private(set) var originalInput:String?
  @Published private(set) var originalValue:NativeExpressionProjection?
  private let client:ResultClient
  private var closing=false
  init(host:NativeHostClient,binding:ResultBinding,probes:ResultTransportProbes = .init()) {
    original=binding;self.binding=binding;client=ResultClient(host:host,probes:probes)
  }
  private func decode<T:Decodable & Sendable>(_ payload:HostJSONValue,as type:T.Type)async throws->T {
    try await Task.detached {try JSONDecoder().decode(type,from:JSONEncoder().encode(payload))}.value
  }
  func load(offset:UInt32=0)async {
    guard !closing,!busy else {return};busy=true;message=nil;defer {busy=false}
    do {
      try client.select(original)
      guard let overview=try await client.inspect(.inspect_result_summary(.init(kind:.summary))) else {throw ResultClientError.staleSelection}
      let actualSummary:NativeResultSummary=try await decode(overview.payload,as:NativeResultSummary.self)
      guard actualSummary.cell_id==original.cell_id,
        actualSummary.statement.value.map({UInt64($0.out_index)==original.out_index.value && $0.view_id==original.view_id.value}) ?? (original.out_index.value==0) else {throw ResultClientError.invalidReply}
      guard let reply=try await client.inspect(.inspect_result_presentation(.init(kind:.presentation,offset:offset,limit:Self.pageSize))),let actual=reply.binding.value else {throw ResultClientError.staleSelection}
      let value=try await Task.detached {try NativeResultPresentation.decode(reply.payload,binding:actual,offset:offset,limit:Self.pageSize)}.value
      guard !closing,!Task.isCancelled else {return}
      binding=actual;summary=actualSummary;presentation=value
    } catch {if !closing,!Task.isCancelled {message="结果暂无法核对："+String(describing:error)}}
  }
  static func solutionNumericKey(index:UInt32,binding:Int,source:String)->Data {
    var data=Data("solution/\(index)/\(binding)/".utf8);data.append(contentsOf:source.utf8);return data
  }
  func projectSolution(index:UInt32,binding indexInRow:Int,source:String)async {
    guard let page=presentation?.solutions,index>=page.offset,Int(index-page.offset)<page.solutions.count,indexInRow>=0,
      indexInRow<page.solutions[Int(index-page.offset)].bindings.count,
      page.solutions[Int(index-page.offset)].bindings[indexInRow].input_form.utf8.elementsEqual(source.utf8) else {message="解的来源已改变，请重新核对。";return}
    let key=Self.solutionNumericKey(index:index,binding:indexInRow,source:source)
    await project(key:key,query:.inspect_result_expression(.init(kind:.readonly_expression,source:source,numeric:true)))
  }
  func projectValue(digits:UInt32=20)async {
    await project(key:Data("value/\(digits)".utf8),query:.inspect_result_numeric(.init(kind:.numeric,path:[],digits:digits)))
  }
  private func project(key:Data,query:NativeResultQuery)async {
    guard !closing,!busy else {return};busy=true;message=nil;defer {busy=false}
    do {
      try client.select(original)
      guard let reply=try await client.inspect(query),let actual=reply.binding.value else {throw ResultClientError.staleSelection}
      let value:NativeExpressionProjection=try await decode(reply.payload,as:NativeExpressionProjection.self)
      guard !closing,!Task.isCancelled else {return};binding=actual;numeric[key] = .init(binding:actual,target:key,value:value)
    } catch {if !closing,!Task.isCancelled {message="数值投影暂不可用："+String(describing:error)}}
  }
  func readOriginalInput()async {
    guard !closing,!busy else {return};busy=true;message=nil;defer {busy=false}
    do {
      try client.select(original)
      guard let source=try await client.completeSource(format:original.out_index.value==0 ? .cell_source : .statement_input) else {throw ResultClientError.staleSelection}
      guard !closing,!Task.isCancelled else {return};originalInput=source
    } catch {if !closing,!Task.isCancelled {message="原输入暂无法读取："+String(describing:error)}}
  }
  func readOriginalValue()async {
    guard !closing,!busy,original.out_index.value>0 else {return};busy=true;message=nil;defer {busy=false}
    do {
      try client.select(original)
      guard let source=try await client.completeSource(),let modern=try await client.completeSource(format:.modern),let latex=try await client.completeSource(format:.latex) else {throw ResultClientError.staleSelection}
      guard !closing,!Task.isCancelled else {return};originalValue = .init(input_form:source,modern_form:modern,latex:latex)
    } catch {if !closing,!Task.isCancelled {message="完整历史结果暂无法读取："+String(describing:error)}}
  }
  func close()async {guard !closing else {return};closing=true;await client.close()}
}
