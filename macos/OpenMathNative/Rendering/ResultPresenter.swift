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
enum NativeValueKind:String,Codable,Sendable {
  case scalar,list,matrix,record,table,model,interpolation,series,quantity
  var label:String {switch self {
    case .scalar:"表达式";case .list:"列表";case .matrix:"矩阵";case .record:"记录";case .table:"表格"
    case .model:"拟合模型";case .interpolation:"插值数据";case .series:"有限级数";case .quantity:"量与单位"
  }}
}
enum NativeValueNature:String,Codable,Sendable {
  case exact,machine,high_precision,symbolic,text,boolean,null
  var label:String {switch self {
    case .exact:"精确数";case .machine:"机器近似值";case .high_precision:"高精度近似值"
    case .symbolic:"符号值";case .text:"文本";case .boolean:"布尔值";case .null:"Null 值"
  }}
}
struct NativeValueEntry:Codable,Sendable {
  let id:String,path:[UInt32],kind:NativeValueKind,nature:NativeValueNature,count:UInt32
  let source:NativeExpressionProjection?
  /// A display excerpt is never a complete copy route or mathematical input.
  var excerpt:String {
    guard let source else {return kind.label+" · \(count) 项"}
    let text=source.input_form
    return String(text.prefix(128))+(text.count>128 ? "…〔摘录〕" : "")
  }
}
struct NativeValueRow:Codable,Sendable {let id:String,label:String,cells:[NativeValueEntry]}
struct NativeScientificOrigin:Codable,Sendable {let function_id:String,name:String}
struct NativeValuePage:Codable,Sendable {
  let view_id:String,path:[UInt32],kind:NativeValueKind,row_count:UInt32,column_count:UInt32,offset:UInt32,column_offset:UInt32
  let columns:[String],rows:[NativeValueRow],source:NativeExpressionProjection?,origin:NativeScientificOrigin?
  var authoritative:Bool {path.isEmpty && origin != nil}
  var rowEnd:UInt32 {offset+UInt32(rows.count)}
  var columnEnd:UInt32 {column_offset+UInt32(columns.count)}
  static func decode(_ payload:HostJSONValue,binding:ResultBinding,query:InspectResultPage)throws->Self {
    let page=try JSONDecoder().decode(Self.self,from:JSONEncoder().encode(payload))
    guard query.path.count<=32,(1...100).contains(query.limit),(1...32).contains(query.column_limit),
      page.view_id.utf8.elementsEqual((binding.view_id.value ?? "").utf8),page.path==query.path,
      page.offset==query.offset,page.column_offset==query.column_offset,
      page.offset<=page.row_count,page.column_offset<=page.column_count,
      page.rows.count==Int(min(query.limit,page.row_count-page.offset)),
      page.columns.count==Int(min(query.column_limit,page.column_count-page.column_offset)),
      Set(page.rows.map{Data($0.id.utf8)}).count==page.rows.count,
      page.rows.allSatisfy({!$0.id.isEmpty && $0.cells.count==page.columns.count &&
        Set($0.cells.map{Data($0.id.utf8)}).count==$0.cells.count && $0.cells.allSatisfy({entry in
          !entry.id.isEmpty && entry.path.count>page.path.count && entry.path.count<=page.path.count+4 &&
          entry.path.starts(with:page.path) && (entry.kind != .scalar || entry.source != nil)
        })}) else {throw ResultClientError.invalidReply}
    return page
  }
  /// Root callback provenance is mandatory; nested/cached/user records retain their own names.
  func fieldLabel(_ key:String)->String {
    guard authoritative else {return key}
    return ["converged":"计算状态","guarantee":"数学保证","termination":"停止原因","method":"方法",
      "error_estimate":"误差估计（非证书）","residual_norm":"残差范数","iterations":"实际迭代",
      "evaluations":"实际调用","point":"参数点","value":"结果值","parameters":"拟合参数",
      "data":"样本与预测","certificate":"证书数据","precision":"精度模式","domain":"定义域",
      "accepted_steps":"接受步","rejected_steps":"拒绝步","numerical_rank":"数值秩",
      "model":"模型","solution":"连续解","sum_squares":"残差平方和","rms":"均方根残差"] [key] ?? key
  }
  func statusLabel(_ entry:NativeValueEntry)->String? {
    guard authoritative,let source=entry.source,
      let text=try? JSONDecoder().decode(String.self,from:Data(source.input_form.utf8)) else {return nil}
    return ["certified_global":"精确全局认证","numerical_stationary_candidate":"数值驻点候选",
      "numerical_bounded_candidate":"数值有界候选","numerical_local_fit":"数值局部拟合",
      "numerical_linear_least_squares":"数值线性最小二乘","residual_tolerance":"达到残差容差",
      "gradient_tolerance":"达到梯度容差","overflow":"超出机器表示范围","underflow":"低于机器表示范围",
      "finite":"有限可表示","machine":"机器精度","local":"局部","global":"全局",
      "end":"到达终点","event":"事件终止","exact_ldlt":"精确 LDLᵀ","linear_qr":"线性 QR"] [text]
  }
  var excerpt:String {
    let header="OpenMath 页面摘录 · \(kind.label) · 行 \(row_count==0 ? 0 : UInt64(offset)+1)–\(rowEnd)/\(row_count) · 列 \(column_count==0 ? 0 : UInt64(column_offset)+1)–\(columnEnd)/\(column_count)"
    return ([header,(["字段/序号"]+columns).joined(separator:"\t")]+rows.map{([$0.label]+$0.cells.map(\.excerpt)).joined(separator:"\t")}).joined(separator:"\n")
  }
}
struct NativeValueRoute:Sendable {
  let path:[UInt32],offset:UInt32,columnOffset:UInt32,label:String,nature:NativeValueNature?
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
  @Published private(set) var valuePage:NativeValuePage?
  @Published private(set) var valueTrail:[NativeValueRoute]=[]
  @Published private(set) var valueRoute=NativeValueRoute(path:[],offset:0,columnOffset:0,label:"根结果",nature:nil)
  private let client:ResultClient
  private let host:NativeHostClient
  private var geometryOwner:NativeGeometryModel?
  private var stepsOwner:NativeStepsModel?
  private var closing=false
  init(host:NativeHostClient,binding:ResultBinding,probes:ResultTransportProbes = .init()) {
    original=binding;self.binding=binding;self.host=host;client=ResultClient(host:host,probes:probes)
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
      var structured:NativeValuePage?
      var structuredError:String?
      if let statement=actualSummary.statement.value,statement.value_kind != "scalar",!statement.suppressed,value.kind=="expression" {
        let query=InspectResultPage(kind:.page,path:[],offset:0,limit:32,column_offset:0,column_limit:8)
        do {
          guard let reply=try await client.inspect(.inspect_result_page(query)),let scoped=reply.binding.value else {throw ResultClientError.staleSelection}
          structured=try await Task.detached {try NativeValuePage.decode(reply.payload,binding:scoped,query:query)}.value
        } catch ResultClientError.failed("RESULT_PAGE_TOO_LARGE") {
          // Preserve accepted metadata/full original source when the existing host byte budget
          // refuses a page. Recheck the current scope before publishing the earlier metadata.
          guard try await client.inspect(.inspect_result_summary(.init(kind:.summary))) != nil else {throw ResultClientError.staleSelection}
          structuredError="结构化页面超过读取大小上限，完整原结果仍可查看或复制。"
        }
      }
      guard !closing,!Task.isCancelled else {return}
      binding=client.selection ?? actual;summary=actualSummary;presentation=value
      valuePage=structured;valueTrail=[];valueRoute = .init(path:[],offset:0,columnOffset:0,label:"根结果",nature:nil)
      message=structuredError
    } catch {if !closing,!Task.isCancelled {message="结果暂无法核对："+String(describing:error)}}
  }
  func loadValue(path:[UInt32],offset:UInt32=0,columnOffset:UInt32=0,limit:UInt32=32,columnLimit:UInt32=8,
    descend:Bool=false,label:String?=nil,nature:NativeValueNature?=nil)async {
    guard !closing,!busy else {return}
    guard path.count<=32,(1...100).contains(limit),(1...32).contains(columnLimit) else {message="结果路径或分页范围超出读取边界。";return}
    busy=true;message=nil;defer {busy=false}
    do {
      try client.select(original)
      let query=InspectResultPage(kind:.page,path:path,offset:offset,limit:limit,column_offset:columnOffset,column_limit:columnLimit)
      guard let reply=try await client.inspect(.inspect_result_page(query)),let actual=reply.binding.value else {throw ResultClientError.staleSelection}
      let page=try await Task.detached {try NativeValuePage.decode(reply.payload,binding:actual,query:query)}.value
      guard !closing,!Task.isCancelled else {return}
      if descend {valueTrail.append(valueRoute)}
      valuePage=page;binding=actual;valueRoute = .init(path:path,offset:offset,columnOffset:columnOffset,label:label ?? valueRoute.label,nature:nature ?? valueRoute.nature)
    } catch {if !closing,!Task.isCancelled {message="结构化结果暂无法读取："+String(describing:error)}}
  }
  func openValue(_ entry:NativeValueEntry,label:String)async {
    guard let page=valuePage,page.rows.contains(where:{$0.cells.contains(where:{$0.id.utf8.elementsEqual(entry.id.utf8) && $0.path==entry.path})}) else {message="所选值不属于当前结果页。";return}
    await loadValue(path:entry.path,descend:true,label:label,nature:entry.nature)
  }
  func backValue()async {
    guard !busy,let route=valueTrail.last else {return}
    await loadValue(path:route.path,offset:route.offset,columnOffset:route.columnOffset,label:route.label,nature:route.nature)
    if valuePage?.path==route.path {valueTrail.removeLast()}
  }
  func readValueSource(format:InspectResultSourceFormat)async->String? {
    guard !closing,!busy,let page=valuePage else {return nil}
    busy=true;message=nil;defer {busy=false}
    do {
      try client.select(original)
      guard let source=try await client.completeSource(format:format,path:page.path),!closing,!Task.isCancelled else {throw ResultClientError.staleSelection}
      return source
    } catch {if !closing,!Task.isCancelled {message="完整值源码暂无法读取："+String(describing:error)};return nil}
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
  func geometryModel()->NativeGeometryModel {
    if let geometryOwner {return geometryOwner}
    let owner=NativeGeometryModel(host:host,binding:original);geometryOwner=owner;return owner
  }
  func stepsModel()->NativeStepsModel {
    if let stepsOwner {return stepsOwner}
    let owner=NativeStepsModel(host:host,binding:original);stepsOwner=owner;return owner
  }
  func close()async {guard !closing else {return};closing=true;await geometryOwner?.close();await stepsOwner?.close();await client.close()}
}
