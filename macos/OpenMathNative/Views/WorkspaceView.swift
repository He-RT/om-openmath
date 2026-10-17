import AppKit
import SwiftUI

/// The notebook host remains the same AppKit view through all breakpoints and divider drags.
/// Collapsed destinations retain their owners; compact access uses native popovers.
struct WorkspaceView:View {
  @ObservedObject var workspace:NativeFileWorkspace
  let session:NativeDocumentSession
  @ObservedObject private var runtime:NotebookRuntime
  @ObservedObject private var navigation:WorkbenchNavigation
  @ObservedObject private var assistant:NativeAssistant
  init(workspace:NativeFileWorkspace,session:NativeDocumentSession) {self.workspace=workspace;self.session=session;_runtime=ObservedObject(wrappedValue:session.runtime);_navigation=ObservedObject(wrappedValue:session.runtime.navigation);_assistant=ObservedObject(wrappedValue:session.assistant)}
  var body:some View {
    NativeWorkbenchSplit(outline:AnyView(outline),notebook:AnyView(NativeNotebookFileView(workspace:workspace,session:session)),inspector:AnyView(inspector),navigation:navigation,notebookIdentity:ObjectIdentifier(session))
    .navigationTitle(documentTitle)
    .navigationSubtitle(subtitle)
    .toolbar {
      ToolbarItem(placement:.navigation) {
        Button("目录",systemImage:Theme.Symbol.outline) {navigation.toggleOutline()}
          .help("显示或隐藏目录（⌘1）").accessibilityIdentifier("workspace-outline-toggle")
          .popover(isPresented:$navigation.outlinePopover) {outline.frame(width:240,height:480)}
      }
      ToolbarItemGroup(placement:.primaryAction) {
        if runtime.operation != nil || assistant.busy {
          Button("停止",systemImage:Theme.Symbol.stop) {Task {if assistant.busy {await assistant.stop()} else {await runtime.stop()}}}
            .labelStyle(.titleAndIcon).disabled(assistant.stopping || (!assistant.busy && !runtime.canStop))
            .accessibilityIdentifier("workspace-stop")
        } else {
          Button("运行",systemImage:Theme.Symbol.run) {runSelected()}
            .labelStyle(.titleAndIcon).foregroundStyle(Theme.Color.verified).disabled(!canRun)
            .keyboardShortcut(.return,modifiers:[.command]).help("运行当前单元格（⌘↩）")
            .accessibilityIdentifier("workspace-run")
        }
        Menu("添加单元格",systemImage:Theme.Symbol.add) {
          Button("数学单元格",systemImage:"function") {Task {await workspace.insertCell(.math,after:runtime.selectedCell)}}
          Button("文字单元格",systemImage:"text.alignleft") {Task {await workspace.insertCell(.text,after:runtime.selectedCell)}}
        }.disabled(!canChangeStructure).help("在当前单元格下方添加内容")
        Button("检查器",systemImage:Theme.Symbol.inspector) {navigation.toggleInspector()}
          .help("显示或隐藏检查器（⌥⌘0）").accessibilityIdentifier("workspace-inspector-toggle")
          .popover(isPresented:$navigation.inspectorPopover) {inspector.frame(width:380,height:600)}
      }
    }
    .tint(Theme.Color.verified)
    .task {await runtime.refreshProjection()}
  }
  private var canChangeStructure:Bool {!workspace.busy && runtime.operation==nil && !assistant.busy}
  private var canRun:Bool {
    runtime.canRun && !workspace.busy && !assistant.busy && session.drafts.confirmed.file.cells.contains{$0.id==runtime.selectedCell && $0.kind == .math}
  }
  private func runSelected() {if let cell=runtime.selectedCell,canRun {runtime.run(cell)}}
  private var documentTitle:String {session.document.fileURL?.deletingPathExtension().lastPathComponent ?? session.drafts.confirmed.file.title}
  private var subtitle:String {
    let saved=UIStrings.text(session.document.isDocumentEdited ? "doc.edited":"doc.saved")
    if assistant.busy {return saved+"，"+assistant.status}
    if runtime.operation != nil {
      if runtime.cancelPending {return saved+"，正在停止…"}
      let index=session.drafts.confirmed.file.cells.firstIndex{$0.id==runtime.operationCell}.map{$0+1}
      return saved+"，"+UIStrings.text("cell.running")+(index.map{" 单元格 \($0)"} ?? "")
    }
    if runtime.unknownOperation != nil {return saved+"，计算结果待核对"}
    if runtime.message=="已停止" {return saved+"，"+UIStrings.text("cell.stopped")}
    return saved+"，"+UIStrings.text("kernel.ready")
  }
  private var outline:some View {
    NotebookOutline(workspace:workspace,session:session)
  }
  private var inspector:some View {
    VStack(alignment:.leading,spacing:0) {
      Picker("检查器",selection:Binding(get:{navigation.inspector},set:{navigation.inspector=$0})) {
        Text("助手").tag(WorkbenchDestination.assistant)
        Text("步骤").tag(WorkbenchDestination.steps)
        Text("变量").tag(WorkbenchDestination.variables)
      }.pickerStyle(.segmented).labelsHidden().padding(Theme.Space.sm)
        .accessibilityIdentifier("workspace-inspector-destination")
      Divider()
      ZStack(alignment:.topLeading) {
        NativeAssistantView(assistant:session.assistant).padding(Theme.Space.md)
          .opacity(navigation.inspector == .assistant ? 1:0)
          .allowsHitTesting(navigation.inspector == .assistant).accessibilityHidden(navigation.inspector != .assistant)
        ScrollView {
          VStack(alignment:.leading,spacing:Theme.Space.md) {inspectorDetails}
            .frame(maxWidth:.infinity,alignment:.leading).padding(Theme.Space.md)
        }.opacity(navigation.inspector == .assistant ? 0:1)
          .allowsHitTesting(navigation.inspector != .assistant).accessibilityHidden(navigation.inspector == .assistant)
      }
    }.frame(maxWidth:.infinity,maxHeight:.infinity,alignment:.topLeading).background(Theme.Color.chrome)
  }
  @ViewBuilder private var inspectorDetails:some View {
    switch navigation.inspector {
    case .document:
      VStack(alignment:.leading,spacing:12) {
        Text(session.document.fileURL?.lastPathComponent ?? session.drafts.confirmed.file.title).font(.title3).fontWeight(.medium)
        LabeledContent("单元格",value:String(session.drafts.confirmed.file.cells.count))
        LabeledContent("保存状态",value:session.document.statusText)
        Text(session.backup.status).font(.callout).foregroundStyle(.secondary)
        if let url=session.document.fileURL {Text(url.deletingLastPathComponent().path).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)}
      }
      DisclosureGroup("记录详情") {
        LabeledContent("源码版本",value:String(session.drafts.confirmed.revision.value)).font(.caption)
      }
    case .result:
      if let cell=runtime.selectedCell,let result=runtime.results[cell] {
        LabeledContent("状态",value:result.receipt.producer.terminal_status == .done ? "计算完成" : "计算有错误")
        LabeledContent("完成语句",value:String(result.receipt.producer.successful_statements.value))
        ForEach(Array(result.sources.enumerated()),id:\.offset) {_,text in
          Text(String(text.prefix(2000))).font(.system(.callout,design:.monospaced)).textSelection(.enabled)
        }
      } else {Text("运行数学单元格后，在这里查看结果详情。").foregroundStyle(.secondary)}
    case .assistant: EmptyView()
    case .steps:
      if let cell=runtime.selectedCell,let result=runtime.results[cell],let binding=result.bindings.last(where:{$0.out_index.value>0 && $0.view_id.value != nil}) {
        NativeStepsView(model:runtime.presenter(for:binding).stepsModel()).id(binding.result_ref)
      } else {Text("选择已运行的数学单元格，查看记录的步骤。").foregroundStyle(.secondary)}
    case .variables:
      if let error=runtime.projectionError {Text(error).foregroundStyle(.secondary)}
      else if let actual=runtime.projection,actual.active_checkpoint_id.value != nil {
        let current=actual.definitions_current && actual.execution_epoch.value==session.drafts.confirmed.execution_epoch.value && !session.drafts.confirmed.file.cells.contains {cell in
          guard cell.kind == .math,let draft=session.drafts.overlay(cell.id) else {return false};return draft.composing || !draft.source.utf8.elementsEqual(cell.source.utf8)
        }
        Text(current ? "计算定义与当前源码一致" : "源码已修改，计算定义需要重新确认")
        LabeledContent("计算版本",value:String(actual.kernel_state_revision.value))
        Text("变量值列表尚不可用。").foregroundStyle(.secondary)
      } else {Text("运行数学单元格后可确认定义状态。").foregroundStyle(.secondary)}
      Button("核对定义状态") {Task {await runtime.refreshProjection()}}.accessibilityIdentifier("workspace-refresh-definitions")
    }
  }

}

private struct NativeWorkbenchSplit:NSViewRepresentable {
  let outline:AnyView,notebook:AnyView,inspector:AnyView
  @ObservedObject var navigation:WorkbenchNavigation
  let notebookIdentity:ObjectIdentifier
  func makeNSView(context:Context)->WorkbenchSplitView {
    let view=WorkbenchSplitView(navigation:navigation)
    view.outline.rootView=outline;view.notebook.rootView=notebook;view.inspector.rootView=inspector
    view.notebook.identifier=NSUserInterfaceItemIdentifier("workspace-notebook")
    view.notebook.setAccessibilityIdentifier("workspace-notebook")
    view.notebookIdentity=notebookIdentity
    return view
  }
  func updateNSView(_ view:WorkbenchSplitView,context:Context) {
    view.outline.rootView=outline;view.inspector.rootView=inspector
    // The notebook already observes the document/workspace/runtime. Reassigning its AnyView on
    // every inspector update recreates SwiftUI scroll state, even with the same NSHostingView.
    if view.notebookIdentity != notebookIdentity {view.notebook.rootView=notebook;view.notebookIdentity=notebookIdentity;view.bindNavigation(navigation)}
    if view.showOutline != navigation.outlineVisible || view.showInspector != navigation.inspectorVisible {
      view.showOutline=navigation.outlineVisible;view.showInspector=navigation.inspectorVisible;view.needsLayout=true
    }
  }
}
@MainActor final class WorkbenchSplitView:NSSplitView,NSSplitViewDelegate {
  let outline=NSHostingView(rootView:AnyView(EmptyView())),notebook=NSHostingView(rootView:AnyView(EmptyView())),inspector=NSHostingView(rootView:AnyView(EmptyView()))
  var showOutline=true,showInspector=true
  var notebookIdentity:ObjectIdentifier?
  private weak var navigation:WorkbenchNavigation?
  private var lastLayout:WorkbenchLayout?
  private var lastWidth:CGFloat = -1
  private var arranging=false
  private(set) var outlineWidth:CGFloat=240,inspectorWidth:CGFloat=380
  init(navigation:WorkbenchNavigation) {
    self.navigation=navigation;super.init(frame:.zero)
    isVertical=true;dividerStyle = .thin;delegate=self
    addArrangedSubview(outline);addArrangedSubview(notebook);addArrangedSubview(inspector)
    setAccessibilityIdentifier("workspace-split")
  }
  required init?(coder:NSCoder) {fatalError("init(coder:) has not been implemented")}
  func bindNavigation(_ value:WorkbenchNavigation) {navigation=value;lastLayout=nil;needsLayout=true}
  override func layout() {
    super.layout();guard bounds.width>0,!arranging else {return}
    let mode=WorkbenchLayout.forWidth(bounds.width)
    if lastLayout != mode || lastWidth != bounds.width || outline.isHidden != !(showOutline && mode == .three) || inspector.isHidden != !(showInspector && mode != .compact) {
      arranging=true;defer {arranging=false}
      outline.isHidden = !(showOutline && mode == .three);inspector.isHidden = !(showInspector && mode != .compact)
      let left=outline.isHidden ? 0 : min(max(outlineWidth,220),260),right=inspector.isHidden ? 0 : min(max(inspectorWidth,360),420)
      outline.frame=NSRect(x:0,y:0,width:left,height:bounds.height)
      notebook.frame=NSRect(x:left+(left>0 ? dividerThickness : 0),y:0,width:max(1,bounds.width-left-right-(left>0 ? dividerThickness : 0)-(right>0 ? dividerThickness : 0)),height:bounds.height)
      inspector.frame=NSRect(x:bounds.width-right,y:0,width:right,height:bounds.height)
      lastLayout=mode;lastWidth=bounds.width
      if navigation?.layout != mode {DispatchQueue.main.async { [weak self] in self?.navigation?.layout=mode}}
    }
  }
  func splitView(_ splitView:NSSplitView,constrainMinCoordinate proposedMinimumPosition:CGFloat,ofSubviewAt dividerIndex:Int)->CGFloat {
    dividerIndex==0 ? (outline.isHidden ? 0 : 220) : notebook.frame.minX+400
  }
  func splitView(_ splitView:NSSplitView,constrainMaxCoordinate proposedMaximumPosition:CGFloat,ofSubviewAt dividerIndex:Int)->CGFloat {
    dividerIndex==0 ? min(260,bounds.width-(inspector.isHidden ? 0 : inspector.frame.width)-400) : bounds.width-360
  }
  func splitViewDidResizeSubviews(_ notification:Notification) {
    guard !arranging,lastWidth==bounds.width else {return}
    if !outline.isHidden {outlineWidth=max(220,outline.frame.width)}
    if !inspector.isHidden {inspectorWidth=max(360,inspector.frame.width)}
  }
}
