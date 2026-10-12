import AppKit
import SwiftUI

/// The notebook host remains the same AppKit view through all breakpoints and divider drags.
/// Collapsed destinations retain their owners; compact access uses native popovers.
struct WorkspaceView:View {
  @ObservedObject var workspace:NativeFileWorkspace
  let session:NativeDocumentSession
  @ObservedObject private var runtime:NotebookRuntime
  @ObservedObject private var navigation:WorkbenchNavigation
  @State private var outlinePopover=false
  @State private var inspectorPopover=false
  init(workspace:NativeFileWorkspace,session:NativeDocumentSession) {self.workspace=workspace;self.session=session;_runtime=ObservedObject(wrappedValue:session.runtime);_navigation=ObservedObject(wrappedValue:session.runtime.navigation)}
  var body:some View {
    VStack(spacing:0) {
      HStack(spacing:12) {
        Button("目录",systemImage:"sidebar.left") {
          if navigation.layout == .three {navigation.outlineVisible.toggle()} else {outlinePopover.toggle()}
        }.labelStyle(.iconOnly).help("笔记本目录").accessibilityIdentifier("workspace-outline-toggle")
          .popover(isPresented:$outlinePopover) {outline.frame(width:240,height:360)}
        Text(session.drafts.confirmed.file.title).font(.headline).lineLimit(1)
        Spacer()
        Button("运行",systemImage:"play.fill") {runSelected()}.disabled(!canRun)
          .keyboardShortcut(.return,modifiers:[.command]).accessibilityIdentifier("workspace-run")
        Button("停止",systemImage:"stop.fill") {Task {await runtime.stop()}}.disabled(!runtime.canStop)
          .accessibilityIdentifier("workspace-stop")
        Button("检查器",systemImage:"sidebar.right") {
          if navigation.layout == .compact {inspectorPopover.toggle()} else {navigation.inspectorVisible.toggle()}
        }.labelStyle(.iconOnly).help("查看文档与计算状态").accessibilityIdentifier("workspace-inspector-toggle")
          .popover(isPresented:$inspectorPopover) {inspector.frame(width:300,height:420)}
      }.padding(.horizontal,16).padding(.vertical,10)
      Divider()
      NativeWorkbenchSplit(outline:AnyView(outline),notebook:AnyView(NativeNotebookFileView(workspace:workspace,session:session)),inspector:AnyView(inspector),navigation:navigation)
      Divider()
      HStack {
        if runtime.operation != nil {ProgressView().controlSize(.small)}
        Text(status).lineLimit(1).accessibilityIdentifier("workspace-operation-status")
        if runtime.unknownOperation != nil {Button("核对原运行") {Task {await runtime.reconcile()}}.disabled(runtime.operation != nil)}
        Spacer();Text("源码版本 \(session.drafts.confirmed.revision.value)").foregroundStyle(.secondary)
      }.font(.caption).padding(.horizontal,16).padding(.vertical,8)
    }
    .task {await runtime.refreshProjection()}
  }
  private var canRun:Bool {
    runtime.canRun && !workspace.busy && session.drafts.confirmed.file.cells.contains{$0.id==runtime.selectedCell && $0.kind == .math}
  }
  private func runSelected() {if let cell=runtime.selectedCell,canRun {runtime.run(cell)}}
  private var status:String {
    guard runtime.operation != nil,let phase=runtime.phase else {return runtime.message}
    switch phase {
    case .queued:return "已进入计算队列"
    case .running:return runtime.cancelPending ? "已请求停止，正在核对结果" : "正在计算"
    case .waiting_source:return "正在等待源码确认"
    case .candidate:return "正在校验计算结果"
    case .committing:return "正在保存计算结果"
    case .unknown:return "计算结果待核对"
    default:return runtime.message
    }
  }
  private var outline:some View {
    VStack(alignment:.leading,spacing:8) {
      Text("笔记本目录").font(.headline).padding([.top,.horizontal],14)
      List(session.drafts.confirmed.file.cells,id:\.id,selection:Binding(get:{runtime.selectedCell},set:{runtime.navigate(to:$0)})) {cell in
        HStack(alignment:.top,spacing:8) {
          Image(systemName:cell.kind == .math ? "function" : "text.alignleft").foregroundStyle(.secondary)
          Text(cell.source.split(whereSeparator:\.isNewline).first.map(String.init) ?? (cell.kind == .math ? "空白数学单元格" : "空白文字单元格")).lineLimit(2).font(.callout)
        }.tag(cell.id)
      }.listStyle(.sidebar).accessibilityIdentifier("workspace-outline")
    }.frame(maxWidth:.infinity,maxHeight:.infinity,alignment:.topLeading)
  }
  private var inspector:some View {
    VStack(alignment:.leading,spacing:16) {
      Text("检查器").font(.headline)
      Picker("检查内容",selection:$navigation.inspector) {
        ForEach(WorkbenchDestination.allCases) {destination in Text(destination.title).tag(destination)}
      }.pickerStyle(.menu).accessibilityIdentifier("workspace-inspector-destination")
      switch navigation.inspector {
      case .document:
        LabeledContent("单元格",value:String(session.drafts.confirmed.file.cells.count))
        LabeledContent("保存状态",value:session.document.statusText)
        Text(session.backup.status).font(.caption).foregroundStyle(.secondary)
      case .result:
        if let cell=runtime.selectedCell,let result=runtime.results[cell] {
          Text(result.receipt.producer.terminal_status == .done ? "已接纳计算结果" : "已接纳计算记录，有错误")
          LabeledContent("完成语句",value:String(result.receipt.producer.successful_statements.value))
          ForEach(Array(result.sources.enumerated()),id:\.offset) {_,text in
            Text(String(text.prefix(2000))).font(.system(.callout,design:.monospaced)).textSelection(.enabled)
          }
        } else {Text("运行一个数学单元格后，在这里查看已接纳的结果。").foregroundStyle(.secondary)}
      case .assistant:
        Image(systemName:"bubble.left.and.bubble.right").font(.title2).foregroundStyle(.secondary)
        Text("助手尚未启用").font(.headline)
        Text("你可以继续编辑和计算笔记本。启用助手后，可在这里提问并查看工具操作。").foregroundStyle(.secondary)
      case .steps:
        if let cell=runtime.selectedCell,let result=runtime.results[cell],let binding=result.bindings.last(where:{$0.out_index.value>0 && $0.view_id.value != nil}) {
          WorkbenchStepsPreview(host:session.host,binding:binding).id(binding.result_ref)
        } else {Text("选择已运行的数学单元格，查看实际记录的步骤。").foregroundStyle(.secondary)}
      case .variables:
        if let error=runtime.projectionError {Text(error).foregroundStyle(.secondary)}
        else if let actual=runtime.projection,actual.active_checkpoint_id.value != nil {
          let current=actual.definitions_current && actual.execution_epoch.value==session.drafts.confirmed.execution_epoch.value && !session.drafts.confirmed.file.cells.contains {cell in
            guard cell.kind == .math,let draft=session.drafts.overlay(cell.id) else {return false};return draft.composing || !draft.source.utf8.elementsEqual(cell.source.utf8)
          }
          Text(current ? "已接纳当前计算定义" : "源码已修改，计算定义需要重新确认")
          LabeledContent("计算版本",value:String(actual.kernel_state_revision.value))
          Text("变量值列表尚不可用。").foregroundStyle(.secondary)
        } else {Text("尚未建立计算定义。运行数学单元格后可确认定义状态。").foregroundStyle(.secondary)}
        Button("核对定义状态") {Task {await runtime.refreshProjection()}}
          .accessibilityIdentifier("workspace-refresh-definitions")
        }
      Spacer()
    }.padding(16).frame(maxWidth:.infinity,maxHeight:.infinity,alignment:.topLeading)
  }
}

private struct RecordedStepsPreview:Decodable,Sendable {
  struct Entry:Decodable,Sendable {struct Step:Decodable,Sendable {let id:String;let rule_id:String;let before_latex:[String];let after_latex:[String]};let step:Step;let child_count:UInt32}
  let recorded:Bool,total:UInt32,entries:[Entry]
}
/// Foundation-level destination uses real immutable steps. The paged tree/typesetter belongs to
/// the later steps presentation task; an absent record never produces a synthetic explanation.
private struct WorkbenchStepsPreview:View {
  let host:NativeHostClient,binding:ResultBinding
  @State private var page:RecordedStepsPreview?
  @State private var error:String?
  var body:some View {
    Group {
      if let page {
        if !page.recorded {Text("本次计算没有记录步骤。").foregroundStyle(.secondary)}
        else if page.total==0 {Text("本次计算记录中没有步骤。").foregroundStyle(.secondary)}
        else {ScrollView {VStack(alignment:.leading,spacing:12) {
          ForEach(page.entries,id:\.step.id) {entry in
            VStack(alignment:.leading,spacing:4) {
              Text(entry.step.rule_id).font(.headline)
              ForEach(Array((entry.step.before_latex+entry.step.after_latex).enumerated()),id:\.offset) {_,source in Text(source).font(.system(.caption,design:.monospaced)).textSelection(.enabled)}
              if entry.child_count>0 {Text("包含 \(entry.child_count) 个子步骤").font(.caption).foregroundStyle(.secondary)}
            }
          }
          if page.total>UInt32(page.entries.count) {Text("显示前 \(page.entries.count) 项，共 \(page.total) 项。").font(.caption).foregroundStyle(.secondary)}
        }}}
      } else if let error {Text(error).foregroundStyle(.secondary)}
      else {ProgressView("正在读取记录")}
    }.task {
      let reader=ResultClient(host:host)
      do {
        try reader.select(binding)
        guard let reply=try await reader.inspect(.inspect_result_steps(.init(kind:.steps,parent_path:[],offset:0,limit:16))) else {throw ResultClientError.staleSelection}
        let actual=try await Task.detached {try JSONDecoder().decode(RecordedStepsPreview.self,from:JSONEncoder().encode(reply.payload))}.value
        if !Task.isCancelled {page=actual}
      } catch {if !Task.isCancelled {self.error="步骤暂无法读取，可重新选择结果后核对。"}}
      await reader.close()
    }
  }
}

private struct NativeWorkbenchSplit:NSViewRepresentable {
  let outline:AnyView,notebook:AnyView,inspector:AnyView
  @ObservedObject var navigation:WorkbenchNavigation
  func makeNSView(context:Context)->WorkbenchSplitView {
    let view=WorkbenchSplitView(navigation:navigation)
    view.outline.rootView=outline;view.notebook.rootView=notebook;view.inspector.rootView=inspector
    return view
  }
  func updateNSView(_ view:WorkbenchSplitView,context:Context) {
    view.outline.rootView=outline;view.notebook.rootView=notebook;view.inspector.rootView=inspector
    view.showOutline=navigation.outlineVisible;view.showInspector=navigation.inspectorVisible;view.needsLayout=true
  }
}
@MainActor final class WorkbenchSplitView:NSSplitView,NSSplitViewDelegate {
  let outline=NSHostingView(rootView:AnyView(EmptyView())),notebook=NSHostingView(rootView:AnyView(EmptyView())),inspector=NSHostingView(rootView:AnyView(EmptyView()))
  var showOutline=true,showInspector=true
  private weak var navigation:WorkbenchNavigation?
  private var lastLayout:WorkbenchLayout?
  private var lastWidth:CGFloat = -1
  private var arranging=false
  private(set) var outlineWidth:CGFloat=208,inspectorWidth:CGFloat=288
  init(navigation:WorkbenchNavigation) {
    self.navigation=navigation;super.init(frame:.zero)
    isVertical=true;dividerStyle = .thin;delegate=self
    addArrangedSubview(outline);addArrangedSubview(notebook);addArrangedSubview(inspector)
    setAccessibilityIdentifier("workspace-split")
  }
  required init?(coder:NSCoder) {fatalError("init(coder:) has not been implemented")}
  override func layout() {
    super.layout();guard bounds.width>0,!arranging else {return}
    let mode=WorkbenchLayout.forWidth(bounds.width)
    if lastLayout != mode || lastWidth != bounds.width || outline.isHidden != !(showOutline && mode == .three) || inspector.isHidden != !(showInspector && mode != .compact) {
      arranging=true;defer {arranging=false}
      outline.isHidden = !(showOutline && mode == .three);inspector.isHidden = !(showInspector && mode != .compact)
      let left=outline.isHidden ? 0 : min(outlineWidth,240),right=inspector.isHidden ? 0 : min(inspectorWidth,320)
      outline.frame=NSRect(x:0,y:0,width:left,height:bounds.height)
      notebook.frame=NSRect(x:left+(left>0 ? dividerThickness : 0),y:0,width:max(1,bounds.width-left-right-(left>0 ? dividerThickness : 0)-(right>0 ? dividerThickness : 0)),height:bounds.height)
      inspector.frame=NSRect(x:bounds.width-right,y:0,width:right,height:bounds.height)
      lastLayout=mode;lastWidth=bounds.width
      if navigation?.layout != mode {DispatchQueue.main.async { [weak self] in self?.navigation?.layout=mode}}
    }
  }
  func splitView(_ splitView:NSSplitView,constrainMinCoordinate proposedMinimumPosition:CGFloat,ofSubviewAt dividerIndex:Int)->CGFloat {
    dividerIndex==0 ? (outline.isHidden ? 0 : 160) : notebook.frame.minX+400
  }
  func splitView(_ splitView:NSSplitView,constrainMaxCoordinate proposedMaximumPosition:CGFloat,ofSubviewAt dividerIndex:Int)->CGFloat {
    dividerIndex==0 ? min(280,bounds.width-(inspector.isHidden ? 0 : inspector.frame.width)-400) : bounds.width-240
  }
  func splitViewDidResizeSubviews(_ notification:Notification) {
    guard !arranging,lastWidth==bounds.width else {return}
    if !outline.isHidden {outlineWidth=max(160,outline.frame.width)}
    if !inspector.isHidden {inspectorWidth=max(240,inspector.frame.width)}
  }
}
