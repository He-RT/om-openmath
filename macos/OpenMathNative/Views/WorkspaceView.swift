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
  @State private var outlinePopover=false
  @State private var inspectorPopover=false
  init(workspace:NativeFileWorkspace,session:NativeDocumentSession) {self.workspace=workspace;self.session=session;_runtime=ObservedObject(wrappedValue:session.runtime);_navigation=ObservedObject(wrappedValue:session.runtime.navigation);_assistant=ObservedObject(wrappedValue:session.assistant)}
  var body:some View {
    VStack(spacing:0) {
      NativeWorkbenchSplit(outline:AnyView(outline),notebook:AnyView(NativeNotebookFileView(workspace:workspace,session:session)),inspector:AnyView(inspector),navigation:navigation,notebookIdentity:ObjectIdentifier(session))
      Divider()
      HStack(spacing:8) {
        if runtime.operation != nil {ProgressView().controlSize(.small)}
        Text(status).lineLimit(1).accessibilityIdentifier("workspace-operation-status")
        if runtime.unknownOperation != nil {Button("核对原运行") {Task {await runtime.reconcile()}}.disabled(runtime.operation != nil)}
        Spacer(minLength:12)
        Text(session.document.statusText).accessibilityIdentifier("notebook-file-status")
      }.font(.caption).foregroundStyle(.secondary).padding(.horizontal,18).padding(.vertical,8)
    }
    .toolbar {
      ToolbarItem(placement:.navigation) {
        Button("目录",systemImage:"sidebar.left") {
          if navigation.layout == .three {navigation.outlineVisible.toggle()} else {outlinePopover.toggle()}
        }.help("显示或隐藏目录").accessibilityIdentifier("workspace-outline-toggle")
          .popover(isPresented:$outlinePopover) {outline.frame(width:240,height:360)}
      }
      ToolbarItemGroup(placement:.primaryAction) {
        Menu("添加单元格",systemImage:"plus") {
          Button("数学单元格",systemImage:"function") {Task {await workspace.insertCell(.math,after:runtime.selectedCell)}}
          Button("文字单元格",systemImage:"text.alignleft") {Task {await workspace.insertCell(.text,after:runtime.selectedCell)}}
        }.disabled(!canChangeStructure).help("在当前单元格下方添加内容")
        Button("保存",systemImage:"square.and.arrow.down") {Task {await workspace.save()}}.disabled(workspace.busy).help("保存笔记本（⌘S）").accessibilityIdentifier("notebook-save")
        if runtime.operation != nil {
          Button("停止",systemImage:"stop.fill") {Task {await runtime.stop()}}.disabled(!runtime.canStop).accessibilityIdentifier("workspace-stop")
        } else {
          Button("运行",systemImage:"play.fill") {runSelected()}.disabled(!canRun).keyboardShortcut(.return,modifiers:[.command]).help("运行当前单元格（⌘↩）").accessibilityIdentifier("workspace-run")
        }
        Button("助手",systemImage:"bubble.left.and.bubble.right") {
          if navigation.layout != .compact,navigation.inspectorVisible,navigation.inspector == .assistant {navigation.inspectorVisible=false}
          else {show(.assistant)}
        }
          .help("打开数学助手").accessibilityIdentifier("workspace-assistant-toggle")
        Button("详情",systemImage:"sidebar.right") {
          if navigation.layout == .compact {navigation.inspector = .document;inspectorPopover.toggle()}
          else if navigation.inspectorVisible && navigation.inspector != .assistant {navigation.inspectorVisible=false}
          else {navigation.inspector = .document;navigation.inspectorVisible=true}
        }.help("文档、结果与步骤").accessibilityIdentifier("workspace-inspector-toggle")
          .popover(isPresented:$inspectorPopover) {inspector.frame(width:340,height:520)}
      }
    }
    .task {await runtime.refreshProjection()}
  }
  private var canChangeStructure:Bool {!workspace.busy && runtime.operation==nil && !assistant.busy}
  private func show(_ destination:WorkbenchDestination) {
    navigation.inspector=destination
    if navigation.layout == .compact {inspectorPopover=true} else {navigation.inspectorVisible=true}
  }
  private var canRun:Bool {
    runtime.canRun && !workspace.busy && !assistant.busy && session.drafts.confirmed.file.cells.contains{$0.id==runtime.selectedCell && $0.kind == .math}
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
    VStack(alignment:.leading,spacing:0) {
      Text("笔记本").font(.headline).padding(.horizontal,16).padding(.top,20).padding(.bottom,10)
      List(Array(session.drafts.confirmed.file.cells.enumerated()),id:\.element.id,selection:Binding(get:{runtime.selectedCell},set:{runtime.navigate(to:$0)})) {index,cell in
        HStack(alignment:.top,spacing:10) {
          Image(systemName:cell.kind == .math ? "function" : "text.alignleft").foregroundStyle(.secondary).frame(width:16)
          VStack(alignment:.leading,spacing:4) {
            Text(cell.source.split(whereSeparator:\.isNewline).first.map(String.init) ?? (cell.kind == .math ? "数学单元格" : "文字单元格")).lineLimit(2).font(.callout)
            Text("\(index+1) · \(cell.kind == .math ? "数学" : "文字")").font(.caption).foregroundStyle(.secondary)
          }
        }.padding(.vertical,5).tag(cell.id)
      }.listStyle(.sidebar).accessibilityIdentifier("workspace-outline")
    }.frame(maxWidth:.infinity,maxHeight:.infinity,alignment:.topLeading)
      .background(.bar)
  }
  private var inspector:some View {
    VStack(alignment:.leading,spacing:0) {
      if navigation.inspector == .assistant {
        NativeAssistantView(assistant:session.assistant).padding(16)
      } else {
        HStack {
          Text(navigation.inspector.title).font(.headline)
          Spacer()
          Menu("查看",systemImage:"ellipsis") {
            ForEach(WorkbenchDestination.allCases) {destination in Button(destination.title) {show(destination)}}
          }.labelStyle(.iconOnly).menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize().accessibilityIdentifier("workspace-inspector-destination")
        }.padding(16)
        Divider()
        ScrollView {
          VStack(alignment:.leading,spacing:16) {
            inspectorDetails
          }.frame(maxWidth:.infinity,alignment:.leading).padding(16)
        }
      }
    }.frame(maxWidth:.infinity,maxHeight:.infinity,alignment:.topLeading).background(.bar)
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
        WorkbenchStepsPreview(host:session.host,binding:binding).id(binding.result_ref)
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
  let notebookIdentity:ObjectIdentifier
  func makeNSView(context:Context)->WorkbenchSplitView {
    let view=WorkbenchSplitView(navigation:navigation)
    view.outline.rootView=outline;view.notebook.rootView=notebook;view.inspector.rootView=inspector
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
  private(set) var outlineWidth:CGFloat=208,inspectorWidth:CGFloat=320
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
      let left=outline.isHidden ? 0 : min(outlineWidth,240),right=inspector.isHidden ? 0 : min(inspectorWidth,400)
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
