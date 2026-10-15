import AppKit
import SwiftUI

/// Each displayed entry is an original solver record and ordinal. Conditions, verification,
/// roots and numeric strings come from the retained producer; layout does not simplify them.
struct NativeSolutionView:View {
  @ObservedObject var presenter:ResultPresenter
  let page:NativeSolutionPage
  var body:some View {
    VStack(alignment:.leading,spacing:12) {
      Text(heading).font(.headline)
      if page.solution_kind == .finite,page.solutions.contains(where:{$0.condition_latex.value != nil}) {Text("以下条件须同时成立。").font(.caption).foregroundStyle(.secondary)}
      ForEach(page.groups,id:\.index) {group in
        VStack(alignment:.leading,spacing:8) {
          HStack {
            Text("解 \(group.firstIndex+1)").font(.caption).foregroundStyle(.secondary)
            Spacer()
            Menu("解的操作",systemImage:"number") {
              ForEach(Array(group.entry.bindings.enumerated()),id:\.offset) {index,value in
                Button("\(value.variable) · 数值投影（20 位）") {Task {await presenter.projectSolution(index:page.offset+UInt32(group.index),binding:index,source:value.input_form)}}
              }
            }.labelStyle(.iconOnly).menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize().disabled(presenter.busy).help("查看解的数值近似")
          }
          if group.firstIndex<page.offset {Text("此页包含该解的重复记录。").font(.caption).foregroundStyle(.secondary)}
          ForEach(Array(group.entry.bindings.enumerated()),id:\.offset) {index,value in
            VStack(alignment:.leading,spacing:6) {
              HStack(alignment:.center,spacing:12) {
                Text(value.variable+" =").font(.system(size:20,design:.serif)).textSelection(.enabled)
                NativeMathView(request:.init(identity:identity(group.index,index),originalLatex:value.latex,wolfram:value.input_form,modern:value.modern_form))
              }
              if let root=value.root_index {Text("精确代数根 · 根序 \(root)").font(.caption).foregroundStyle(.secondary)}
              if let radicals=value.radicals {
                DisclosureGroup("内核提供的根式") {NativeMathView(request:.init(identity:identity(group.index,index)+"-radicals",originalLatex:radicals.latex,wolfram:radicals.input_form,modern:radicals.modern_form.value))}
              }
              let key=ResultPresenter.solutionNumericKey(index:page.offset+UInt32(group.index),binding:index,source:value.input_form)
              if let numeric=presenter.numeric[key] {
                Text("数值投影 · 原精确式保持").font(.caption).foregroundStyle(.secondary)
                NativeMathView(request:.init(identity:identity(group.index,index)+"-numeric",originalLatex:numeric.value.latex,wolfram:numeric.value.input_form,modern:numeric.value.modern_form.value))
              }
            }
          }
          if group.multiplicity>1 {Text("重数 \(group.multiplicity)").font(.caption)}
          if let condition=group.entry.condition_latex.value {
            Text("条件").font(.caption)
            NativeMathView(request:.init(identity:identity(group.index,-1)+"-condition",originalLatex:group.entry.condition_display_latex ?? condition,wolfram:"",modern:nil))
            if let display=group.entry.condition_display_latex,!display.utf8.elementsEqual(condition.utf8) {
              Button("复制原条件 LaTeX") {copy(condition)}.font(.caption)
            }
          }
          Text(group.entry.verified.label).font(.caption).foregroundStyle(.secondary)
        }.padding(.vertical,8)
        Divider()
      }
      if let region=page.region_latex.value {
        Text("实际区域条件").font(.caption)
        NativeMathView(request:.init(identity:presenter.original.result_ref+"-region",originalLatex:region,wolfram:"",modern:nil))
      }
      ForEach(Array(page.intervals.enumerated()),id:\.offset) {index,interval in
        VStack(alignment:.leading,spacing:4) {
          Text("区间 \(page.offset+UInt32(index)+1)").font(.caption)
          HStack(alignment:.firstTextBaseline,spacing:10) {
            Text(interval.lo_closed ? "[" : "(")
            if let lo=interval.lo.value {NativeMathView(request:.init(identity:presenter.original.result_ref+"-lo-\(page.offset+UInt32(index))",originalLatex:lo,wolfram:"",modern:nil))} else {Text("−∞")}
            Text(",")
            if let hi=interval.hi.value {NativeMathView(request:.init(identity:presenter.original.result_ref+"-hi-\(page.offset+UInt32(index))",originalLatex:hi,wolfram:"",modern:nil))} else {Text("+∞")}
            Text(interval.hi_closed ? "]" : ")")
          }
        }
      }
      if page.total>ResultPresenter.pageSize {
        HStack {
          Button("上一页") {Task {await presenter.load(offset:page.offset>=ResultPresenter.pageSize ? page.offset-ResultPresenter.pageSize : 0)}}.disabled(page.offset==0 || presenter.busy)
          Text("第 \(page.offset/ResultPresenter.pageSize+1) 页 · 按重数计 \(page.total_solutions) 个解"+(page.total_intervals>0 ? " / \(page.total_intervals) 个区间" : "")).font(.caption)
          Button("下一页") {Task {await presenter.load(offset:page.offset+ResultPresenter.pageSize)}}.disabled(UInt64(page.offset)+UInt64(ResultPresenter.pageSize)>=UInt64(page.total) || presenter.busy)
        }
        Text("重数按整个解集计算，分页保留原始序号。").font(.caption).foregroundStyle(.secondary)
      }
    }.accessibilityIdentifier("native-solutions-"+presenter.original.result_id)
  }
  private var heading:String {
    switch page.solution_kind {
    case .none:return "无解"
    case .all:return "所有取值均成立"
    case .region:return "解区域"
    case .finite:return "\(page.total_solutions) 个解（按重数计）"
    }
  }
  private func identity(_ row:Int,_ binding:Int)->String {presenter.original.result_ref+"/\(page.offset+UInt32(row))/\(binding)"}
  private func copy(_ text:String) {NSPasteboard.general.clearContents();NSPasteboard.general.setString(text,forType:.string)}
}

enum NativeResultDisplayContext {case current,refreshing,sourceChanged,history}
struct NativeResultView:View {
  @ObservedObject var presenter:ResultPresenter
  let value:NotebookValueResult?
  let displayContext:NativeResultDisplayContext
  @State private var digits:UInt32=20
  @State private var showingSteps=false
  var body:some View {
    Group {if presenter.summary?.statement.value?.suppressed != true {content}}
      .task {if presenter.summary==nil {await presenter.load()}}
      .accessibilityIdentifier("native-result-"+presenter.original.result_id)
      .sheet(isPresented:$showingSteps) {
        VStack(spacing:0) {
          HStack {
            Text("推导步骤").font(.headline)
            Spacer()
            Button("完成") {showingSteps=false}.keyboardShortcut(.cancelAction)
          }.padding(20)
          Divider()
          ScrollView {NativeStepsView(model:presenter.stepsModel()).padding(24)}
        }.frame(minWidth:600,idealWidth:760,minHeight:560)
      }
  }
  private var content:some View {
    VStack(alignment:.leading,spacing:10) {
      HStack {
        Text(status).font(.caption).foregroundStyle(.secondary)
        if let statement=presenter.summary?.statement.value {Text(nature(statement.nature)).font(.caption).foregroundStyle(.secondary)}
        if presenter.busy {ProgressView().controlSize(.small)}
        Spacer()
        if presenter.summary?.statement.value?.steps_recorded==true {
          Button("步骤",systemImage:"list.number") {showingSteps=true}.buttonStyle(.borderless).help("查看实际记录的推导步骤")
        }
        Menu("结果操作",systemImage:"ellipsis") {
          Button("核对结果") {Task {await presenter.load()}}
          Button("查看原输入") {Task {await presenter.readOriginalInput()}}
          if presenter.original.out_index.value>0,presenter.presentation?.kind != "geometry" {
            Divider()
            ForEach([UInt32(20),50,80],id:\.self) {precision in
              Button("数值投影 · \(precision) 位") {digits=precision;Task {await presenter.projectValue(digits:precision)}}
            }
          }
        }.labelStyle(.iconOnly).menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize().disabled(presenter.busy)
      }
      if let summary=presenter.summary {
        ForEach(Array(summary.messages.enumerated()),id:\.offset) {_,message in
          Text(message.symbol+"::"+message.tag+"："+message.text).font(.callout).textSelection(.enabled)
        }
      }
      if let page=presenter.valuePage {NativeStructuredValueView(presenter:presenter,page:page)}
      if presenter.presentation?.kind=="geometry" {NativeGeometryView(model:presenter.geometryModel())}
      if let page=presenter.presentation?.solutions {NativeSolutionView(presenter:presenter,page:page)}
      else if presenter.presentation?.kind=="error" {
        Text(presenter.summary?.statement_count ?? 0>0 ? "后续计算有错误，已接纳的结果仍保留。" : "本次计算有错误，未产生成功结果。").font(.callout)
        if let error=presenter.presentation?.error {Text(error).textSelection(.enabled)}
      }
      if presenter.presentation?.kind != "error",presenter.presentation?.kind != "suppressed" {
        if let actual=originalValue {
          if presenter.presentation?.solutions != nil || presenter.valuePage != nil || presenter.presentation?.kind=="geometry" {DisclosureGroup("完整原结果") {expression(actual)}}
          else {expression(actual)}
          if presenter.original.out_index.value>0 {
            if let projected=presenter.numeric[Data("value/\(digits)".utf8)] {
              Text("只读数值投影 · 不是新的精确结果或误差证书").font(.caption).foregroundStyle(.secondary)
              expression(projected.value,suffix:"-numeric-\(digits)")
            }
          }
        } else if let value {
          Text(String(value.source.prefix(4096))).font(.system(.callout,design:.monospaced)).textSelection(.enabled)
          Text("公式源暂不可读取，已保留实际结果源式。").font(.caption).foregroundStyle(.secondary)
          Button("复制完整 Wolfram 源式") {copy(value.source)}
          if let modern=value.modern {Button("复制完整 Modern 源式") {copy(modern)}}
          if value.source.count>4096 {Text("此处显示预览，可复制完整结果。").font(.caption).foregroundStyle(.secondary)}
        } else if presenter.original.out_index.value>0 {
          Button("读取完整历史结果") {Task {await presenter.readOriginalValue()}}.disabled(presenter.busy)
        }
      }
      if let input=presenter.originalInput {
        Text(String(input.prefix(4096))).font(.system(.callout,design:.monospaced)).textSelection(.enabled)
        if input.count>4096 {Text("此处为输入预览，可复制完整原输入。").font(.caption).foregroundStyle(.secondary)}
        Button("复制完整原输入") {copy(input)}
      }
      if let message=presenter.message {Text(message).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)}
    }.padding(.vertical,6)
  }
  private var originalValue:NativeExpressionProjection? {
    if let value,let latex=value.latex {return .init(input_form:value.source,modern_form:value.modern,latex:latex)}
    return presenter.originalValue
  }
  private var status:String {
    if displayContext == .refreshing {return "上次结果 · 正在重算"}
    if displayContext == .sourceChanged {return "上次结果 · 源码已修改，需要核对"}
    if displayContext == .history || presenter.binding.acceptance == .history_only {return "历史结果 · 计算版本 \(presenter.original.kernel_state_revision.value) / 源码版本 \(presenter.original.source_revision.value)"}
    switch presenter.binding.freshness {
    case .current:return "结果"
    case .stale:return "历史结果 · 来源已过期"
    case .refreshing:return "上次结果 · 正在重算"
    case .partial:return presenter.summary?.statement_count ?? 0>0 ? "部分结果 · 查看实际消息" : "计算有错误 · 未产生成功结果"
    }
  }
  private func nature(_ value:String)->String {
    switch value {
    case "exact":return "精确数值"
    case "machine":return "机器近似值"
    case "high_precision":return "高精度近似值 · 完整源式保留精度标记"
    case "symbolic":return "符号结果"
    case "text":return "文字值"
    case "boolean":return "布尔值"
    case "null":return "Null 值"
    default:return "数值表示暂无法识别，已保留原式。"
    }
  }
  private func expression(_ source:NativeExpressionProjection,suffix:String="")->some View {
    NativeMathView(request:.init(identity:presenter.original.result_ref+suffix,originalLatex:source.latex,wolfram:source.input_form,modern:source.modern_form.value))
  }
  private func copy(_ text:String) {NSPasteboard.general.clearContents();NSPasteboard.general.setString(text,forType:.string)}
}
