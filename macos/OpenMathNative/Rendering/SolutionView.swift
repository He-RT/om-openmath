import AppKit
import SwiftUI

/// Each displayed entry is an original solver record and ordinal. Conditions, verification,
/// roots and numeric strings come from the retained producer; layout does not simplify them.
struct NativeSolutionView:View {
  @ObservedObject var presenter:ResultPresenter
  let page:NativeSolutionPage
  var body:some View {
    VStack(alignment:.leading,spacing:10) {
      switch page.solution_kind {
      case .none:Text("无解").font(Theme.Font.paperH2)
      case .all:Text("所有取值均成立").font(Theme.Font.paperH2)
      case .finite:
        if page.total_solutions<=4 {
          if !page.lineLatex.isEmpty {
            NativeMathView(request:.init(identity:presenter.original.result_ref+"-solution-line",originalLatex:page.lineLatex,wolfram:page.groups.map{group in "{"+group.entry.bindings.map{$0.variable+" = "+$0.input_form}.joined(separator:", ")+"}"}.joined(separator:", "),modern:nil),showsControls:false)
          } else {Text("满足下列条件的所有值").font(Theme.Font.paperBody)}
          ForEach(page.groups) {group in
            if group.multiplicity>1 {Text("重数 \(group.multiplicity)").font(.caption).foregroundStyle(.secondary)}
            ForEach(Array(group.entry.bindings.enumerated()),id:\.offset) {index,value in
              if value.root_index != nil || projected(group,index) != nil {
                Text(value.variable+" ≈ "+approximation(group,index)).font(.system(size:14,design:.monospaced)).textSelection(.enabled)
              }
              if let radicals=value.radicals {
                DisclosureGroup("内核提供的根式") {
                  NativeMathView(request:.init(identity:identity(group.index,index)+"-radicals",originalLatex:radicals.latex,wolfram:radicals.input_form,modern:radicals.modern_form.value),showsControls:false)
                }.font(.caption)
              }
            }
          }
        } else {
          Table(page.groups) {
            TableColumn("序号") {group in Text(String(group.firstIndex+1)).monospacedDigit()}.width(min:32,ideal:44,max:64)
            TableColumn("精确值") {group in
              VStack(alignment:.leading,spacing:4) {
                ForEach(Array(group.entry.bindings.enumerated()),id:\.offset) {index,value in
                  NativeMathView(request:.init(identity:identity(group.index,index),originalLatex:(value.var_latex ?? value.variable)+" = "+value.latex,wolfram:value.input_form,modern:value.modern_form,fontSize:18),showsControls:false)
                }
                if group.multiplicity>1 {Text("重数 \(group.multiplicity)").font(.caption).foregroundStyle(.secondary)}
              }
            }.width(min:130,ideal:240)
            TableColumn("近似值") {group in
              VStack(alignment:.leading,spacing:4) {
                ForEach(Array(group.entry.bindings.enumerated()),id:\.offset) {index,_ in
                  ScrollView(.horizontal) {
                    Text(approximation(group,index)).font(.system(size:12,design:.monospaced))
                      .textSelection(.enabled).fixedSize(horizontal:true,vertical:false)
                  }.scrollIndicators(.automatic)
                }
              }
            }.width(min:100,ideal:220)
          }.tableStyle(.inset).frame(height:min(380,max(92,CGFloat(page.groups.count)*40+32)))
        }
      case .region:
        if let region=page.region_latex.value {
          NativeMathView(request:.init(identity:presenter.original.result_ref+"-region",originalLatex:region,wolfram:"",modern:nil),showsControls:false)
        }
      }
      ForEach(page.groups) {group in
        if let condition=group.entry.condition_latex.value {
          ScrollView(.horizontal) {HStack(spacing:6) {
            Image(systemName:Theme.Symbol.conditional).font(.caption)
            Text(page.groups.count>1 ? "解 \(group.firstIndex+1) 成立条件":"成立条件").font(.system(size:12))
            NativeMathView(request:.init(identity:identity(group.index,-1)+"-condition",originalLatex:group.entry.condition_display_latex ?? condition,wolfram:"",modern:nil,fontSize:15),showsControls:false)
          }.fixedSize(horizontal:true,vertical:false).foregroundStyle(Theme.Color.conditional).padding(.horizontal,10).padding(.vertical,5)
            .background(Theme.Color.conditional.opacity(0.1),in:Capsule())
            .contextMenu {Button("复制原条件 LaTeX") {copy(condition)}}
          }.scrollIndicators(.hidden)
        }
      }
      if page.total>ResultPresenter.pageSize {
        HStack {
          Button("上一页") {Task {await presenter.load(offset:page.offset>=ResultPresenter.pageSize ? page.offset-ResultPresenter.pageSize:0)}}.disabled(page.offset==0 || presenter.busy)
          Text("第 \(page.offset/ResultPresenter.pageSize+1) 页 · 按重数计 \(page.total_solutions) 个解").font(.caption)
          Button("下一页") {Task {await presenter.load(offset:page.offset+ResultPresenter.pageSize)}}.disabled(UInt64(page.offset)+UInt64(ResultPresenter.pageSize)>=UInt64(page.total) || presenter.busy)
        }.buttonStyle(.borderless)
      }
    }.accessibilityIdentifier("native-solutions-"+presenter.original.result_id)
  }
  private func identity(_ row:Int,_ binding:Int)->String {presenter.original.result_ref+"/\(page.offset+UInt32(row))/\(binding)"}
  private func projected(_ group:NativeSolutionPage.Group,_ index:Int)->NativeExpressionProjection? {
    let value=group.entry.bindings[index]
    return presenter.numeric[ResultPresenter.solutionNumericKey(index:page.offset+UInt32(group.index),binding:index,source:value.input_form,digits:presenter.solutionDigits)]?.value
  }
  private func approximation(_ group:NativeSolutionPage.Group,_ index:Int)->String {
    let value=projected(group,index)?.input_form ?? group.entry.bindings[index].numeric.value
    guard let value else {return "—"}
    return NumericDisplay.text(value)
  }
  private func copy(_ text:String) {NSPasteboard.general.clearContents();NSPasteboard.general.setString(text,forType:.string)}
}

enum NativeResultDisplayContext {case current,refreshing,sourceChanged,history}
struct NativeResultView:View {
  @ObservedObject var presenter:ResultPresenter
  let value:NotebookValueResult?
  let displayContext:NativeResultDisplayContext
  var cellState:CellPresentationState?=nil
  var durationMs:Double?=nil
  var hasSourceDiagnostics=false
  var onShowSteps:(()->Void)?=nil
  @State private var showingSteps=false
  @State private var showingOriginal=false
  @State private var showingInput=false
  @State private var showingPlot=false
  var body:some View {
    Group {if presenter.summary?.statement.value?.suppressed != true {content}}
      .task {
        if presenter.summary==nil {await presenter.load()}
        if displayContext == .history,value==nil,presenter.original.out_index.value>0,presenter.originalValue==nil {await presenter.readOriginalValue()}
      }
      .accessibilityIdentifier("native-result-"+presenter.original.result_id)
      .sheet(isPresented:$showingSteps) {
        VStack(spacing:0) {
          HStack {Text("推导步骤").font(.headline);Spacer();Button("完成") {showingSteps=false}.keyboardShortcut(.cancelAction)}.padding(20)
          Divider()
          ScrollView {NativeStepsView(model:presenter.stepsModel()).padding(24)}
        }.frame(minWidth:600,idealWidth:760,minHeight:560)
      }
      .sheet(isPresented:$showingPlot) {
        VStack(spacing:0) {
          HStack {Text("绘图").font(.headline);Spacer();Button("完成") {showingPlot=false}.keyboardShortcut(.cancelAction)}.padding(20)
          Divider()
          NativeGeometryView(model:presenter.geometryModel()).padding(20)
        }.frame(minWidth:640,minHeight:500)
      }
      .popover(isPresented:$showingOriginal) {originalPanel}
      .popover(isPresented:$showingInput) {
        VStack(alignment:.leading,spacing:12) {
          Text("原输入").font(.headline)
          ScrollView {Text(presenter.originalInput ?? "正在读取…").font(Theme.Font.source).textSelection(.enabled)}
          Button("复制完整原输入") {if let input=presenter.originalInput {copy(input)}}.disabled(presenter.originalInput==nil)
        }.padding(20).frame(width:520,height:280)
      }
  }
  private var content:some View {
    VStack(alignment:.leading,spacing:10) {
      if let summary=presenter.summary {
        ForEach(Array(summary.messages.enumerated()),id:\.offset) {_,message in
          if !(hasSourceDiagnostics && message.symbol=="Parse" && message.level.lowercased()=="error") {
          if message.level.lowercased()=="error" {
            Label(message.text,systemImage:Theme.Symbol.error).font(.callout).textSelection(.enabled)
              .padding(12).frame(maxWidth:.infinity,alignment:.leading)
              .background(Theme.Color.red.opacity(0.07),in:RoundedRectangle(cornerRadius:6))
              .overlay(RoundedRectangle(cornerRadius:6).stroke(Theme.Color.red.opacity(0.35)))
          } else {
          Text(message.symbol+"::"+message.tag+"："+message.text).font(.callout).textSelection(.enabled)
          }
          }
        }
      }
      if let page=presenter.valuePage {NativeStructuredValueView(presenter:presenter,page:page)}
      if presenter.presentation?.kind=="geometry" {NativeGeometryView(model:presenter.geometryModel())}
      if let page=presenter.presentation?.solutions {NativeSolutionView(presenter:presenter,page:page)}
      else if presenter.presentation?.kind=="error" {
        if !hasSourceDiagnostics {
          Text(presenter.summary?.statement_count ?? 0>0 ? "后续计算有错误，已接纳的结果仍保留。":"本次计算有错误，未产生成功结果。").font(.callout)
        }
        if let error=presenter.presentation?.error {Text(error).textSelection(.enabled)}
      }
      if presenter.presentation?.kind != "error",presenter.presentation?.kind != "suppressed",presenter.presentation?.solutions==nil,presenter.valuePage==nil,presenter.presentation?.kind != "geometry" {
        if let actual=originalValue {expression(actual)}
        else if let value {
          Text(String(value.source.prefix(4096))).font(Theme.Font.source).textSelection(.enabled)
          Text("公式源暂不可读取，已保留实际结果源式。").font(.caption).foregroundStyle(.secondary)
        } else if presenter.original.out_index.value>0 {
          Button("读取完整历史结果") {Task {await presenter.readOriginalValue()}}.disabled(presenter.busy)
        }
      }
      if let projected=presenter.numeric[Data("value/\(presenter.valueDigits)".utf8)] {
        HStack(alignment:.top,spacing:6) {
          Text("≈").font(Theme.Font.paperH2)
          expression(projected.value,suffix:"-numeric-\(presenter.valueDigits)")
        }
      }
      statusLine
      if let message=presenter.message {Text(message).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)}
    }.padding(.vertical,4)
  }
  private var statusLine:some View {
    ViewThatFits(in:.horizontal) {
      HStack(spacing:8) {stateLabel;summaryLabel;Spacer(minLength:12);actions}
      VStack(alignment:.leading,spacing:6) {
        HStack(spacing:8) {stateLabel;summaryLabel}
        HStack {Spacer();actions}
      }
    }.font(.system(size:12)).foregroundStyle(Theme.Color.ink2).buttonStyle(.borderless)
  }
  private var stateLabel:some View {
    Label(status,systemImage:statusSymbol).foregroundStyle(cellState?.color ?? Theme.Color.ink2)
  }
  @ViewBuilder private var summaryLabel:some View {
    if let page=presenter.presentation?.solutions {
      Text("\(page.total_solutions) 个解").foregroundStyle(Theme.Color.ink2)
    }
    if let durationMs {Text(durationMs<1000 ? "\(Int(durationMs.rounded())) ms":String(format:"%.2f 秒",durationMs/1000))}
    if cellState?.byAgent==true {Text("助手修改").foregroundStyle(Theme.Color.agent)}
  }
  private var actions:some View {
    HStack(spacing:12) {
      if presenter.summary?.statement.value?.steps_recorded==true {Button("步骤") {
        if let onShowSteps {onShowSteps()} else {showingSteps=true}
      }.help("查看实际记录的推导步骤")}
      if presenter.hasGeometry {Button("绘图") {showingPlot=true}.help("打开当前结果的图形视图")}
      Menu("复制") {
        Button("源码（Modern）") {copySource(.modern)}
        Button("源码（Wolfram）") {copySource(.input_form)}
        Button("LaTeX") {copySource(.latex)}
        Button("数值") {copyNumeric()}.disabled(presenter.original.out_index.value==0)
      }.fixedSize().disabled(presenter.busy)
      Menu("结果操作",systemImage:Theme.Symbol.more) {
        Button("查看完整结果") {showingOriginal=true;Task {if originalValue==nil {await presenter.readOriginalValue()}}}
          .disabled(presenter.original.out_index.value==0)
        Button("查看原输入") {showingInput=true;Task {await presenter.readOriginalInput()}}
        Button("核对结果") {Task {await presenter.load()}}
        if presenter.original.out_index.value>0,presenter.presentation?.kind != "geometry" {
          Divider()
          ForEach([UInt32(20),50,80],id:\.self) {precision in
            Button("数值精度 · \(precision) 位") {project(precision)}
          }
        }
      }.labelStyle(.iconOnly).menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize().disabled(presenter.busy)
    }
  }
  private var originalPanel:some View {
    VStack(alignment:.leading,spacing:12) {
      Text("完整原结果").font(.headline)
      if let value=originalValue {
        ScrollView {expression(value,suffix:"-original");Text(value.input_form).font(Theme.Font.source).textSelection(.enabled)}
        HStack {Button("复制源码") {copy(value.input_form)};Button("复制 LaTeX") {copy(value.latex)}}
      } else {ProgressView("正在读取完整结果…")}
    }.padding(20).frame(width:560,height:340)
  }
  private var originalValue:NativeExpressionProjection? {
    if let value,let latex=value.latex {return .init(input_form:value.source,modern_form:value.modern,latex:latex)}
    return presenter.originalValue
  }
  private var status:String {
    if displayContext == .history || presenter.binding.acceptance == .history_only {return "历史结果"}
    if let cellState {return cellState.label}
    if displayContext == .refreshing {return "正在计算…"}
    if displayContext == .sourceChanged {return "结果过期"}
    if presenter.presentation?.kind=="error" {return "计算出错"}
    if let page=presenter.presentation?.solutions {
      if page.solutions.contains(where:{$0.condition_latex.value != nil}) {return "有条件成立"}
      if page.complete && !page.solutions.isEmpty && page.solutions.allSatisfy({if case .exact=$0.verified {return true};return false}) {return "精确解，已代入验证"}
    }
    if ["machine","high_precision"].contains(presenter.summary?.statement.value?.nature ?? "") {return "数值近似"}
    return "精确值"
  }
  private var statusSymbol:String {
    if let state=cellState {
      switch state.status {
      case .verified:return Theme.Symbol.verified
      case .conditional:return Theme.Symbol.conditional
      case .error:return Theme.Symbol.error
      case .stale:return "clock"
      case .running:return "arrow.trianglehead.2.clockwise"
      case .stopped:return Theme.Symbol.stop
      case .pending,.unconfirmed:return "circle"
      case .unevaluated:return "equal"
      case .numeric:return "equal"
      default:return Theme.Symbol.success
      }
    }
    return Theme.Symbol.success
  }
  private func expression(_ source:NativeExpressionProjection,suffix:String="")->some View {
    NativeMathView(request:.init(identity:presenter.original.result_ref+suffix,originalLatex:source.latex,wolfram:source.input_form,modern:source.modern_form.value),showsControls:false)
  }
  private func copySource(_ format:InspectResultSourceFormat) {
    if let source=originalValue {
      if format == .latex {copy(source.latex);return}
      if format == .input_form {copy(source.input_form);return}
      if format == .modern,let modern=source.modern_form.value {copy(modern);return}
    }
    Task {await presenter.readOriginalValue();if let source=presenter.originalValue {
      copy(format == .latex ? source.latex:format == .modern ? source.modern_form.value ?? source.input_form:source.input_form)
    }}
  }
  private func project(_ digits:UInt32) {
    Task {
      if let page=presenter.presentation?.solutions {
        for group in page.groups {for (index,value) in group.entry.bindings.enumerated() {
          await presenter.projectSolution(index:page.offset+UInt32(group.index),binding:index,source:value.input_form,digits:digits)
        }}
      } else {await presenter.projectValue(digits:digits)}
    }
  }
  private func copyNumeric() {
    if let page=presenter.presentation?.solutions {
      let numbers=page.groups.flatMap{group in group.entry.bindings.enumerated().compactMap{index,value->String? in
        presenter.numeric[ResultPresenter.solutionNumericKey(index:page.offset+UInt32(group.index),binding:index,source:value.input_form,digits:presenter.solutionDigits)]?.value.input_form ?? value.numeric.value
      }}
      if !numbers.isEmpty {copy(numbers.joined(separator:", "))}
    } else {
      Task {await presenter.projectValue(digits:presenter.valueDigits);if let numeric=presenter.numeric[Data("value/\(presenter.valueDigits)".utf8)] {copy(numeric.value.input_form)}}
    }
  }
  private func copy(_ text:String) {NSPasteboard.general.clearContents();NSPasteboard.general.setString(text,forType:.string)}
}
