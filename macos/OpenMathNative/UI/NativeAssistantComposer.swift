import AppKit
import SwiftUI

struct NativeAssistantComposer:View {
  @ObservedObject var assistant:NativeAssistant
  let panelHeight:CGFloat
  @State private var editorHeight:CGFloat=46
  @State private var focused=false
  @State private var showReferences=false
  @State private var showModels=false
  @State private var showContext=false
  @State private var showInput=false
  @State private var query=""
  @State private var highlight:String?
  @State private var inputChoice=0
  @State private var displayedInput:AssistantInputSnapshot?
  @State private var readableInput=""
  @State private var rawInput=false
  @FocusState private var searchFocused:Bool
  @FocusState private var buttonFocused:Control?
  private enum Control:Hashable {case models,context}
  private var modelName:String {assistant.selectedModel?.displayName ?? "添加模型"}
  private var filtered:[(RegisteredProvider,[RegisteredModel])] {
    assistant.registry.providers.compactMap {provider in
      let models=provider.models.filter{query.isEmpty || (provider.name+" "+$0.displayName+" "+$0.apiName).localizedCaseInsensitiveContains(query)}
      return models.isEmpty ? nil:(provider,models)
    }
  }
  private var eligible:[RegisteredModel] {filtered.flatMap(\.1).filter{!assistant.allowsChanges || $0.tools}}
  var body:some View {
    VStack(alignment:.leading,spacing:6) {
      VStack(alignment:.leading,spacing:6) {
        if !assistant.references.isEmpty {
          ScrollView(.horizontal) {
            HStack(spacing:6) {ForEach(assistant.references) {reference in
              HStack(spacing:5) {
                Text(reference.label).lineLimit(1)
                Button("移除"+reference.label,systemImage:"xmark") {assistant.references.removeAll{$0.id==reference.id}}
                  .labelStyle(.iconOnly).font(.system(size:9))
              }.font(.system(size:11.5)).padding(.horizontal,9).padding(.vertical,5)
                .background(Theme.Color.well,in:Capsule()).help(reference.summary)
            }}
          }.scrollIndicators(.hidden).accessibilityIdentifier("assistant-reference-chips")
        }
        AssistantComposerEditor(assistant:assistant,maximumHeight:max(46,panelHeight*0.35),height:$editorHeight,focused:$focused)
          .frame(height:min(max(46,editorHeight),max(46,panelHeight*0.35)))
          .overlay(alignment:.topLeading) {if assistant.draft.isEmpty,!assistant.composerMarked {Text("询问当前笔记本…").font(.system(size:13.5)).foregroundStyle(Theme.Color.ink3).padding(.top,6).allowsHitTesting(false)}}
        HStack(spacing:9) {
          Menu("添加",systemImage:"plus") {
            Button("文件或图片…") {assistant.addFilesRequested?()}.disabled(assistant.addFilesRequested==nil)
            Button("引用单元格…") {showReferences=true}
          }.labelStyle(.iconOnly).menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize().help("添加附件或引用单元格")
          Menu(assistant.allowsChanges ? "执行":"讨论") {
            Button {assistant.setMode(execute:true)} label: {Label("执行 · 可修改和运行，每轮可撤销",systemImage:assistant.allowsChanges ? "checkmark":"pencil")}
            Button {assistant.setMode(execute:false)} label: {Label("讨论 · 只读取和临时计算",systemImage:assistant.allowsChanges ? "bubble":"checkmark")}
          }.menuStyle(.borderlessButton).fixedSize().accessibilityIdentifier("assistant-mode")
          Spacer(minLength:0)
          Button {showContext=true} label: {contextRing}
            .buttonStyle(.plain).focused($buttonFocused,equals:.context).help("上下文用量和本轮完整输入").accessibilityLabel("上下文用量").accessibilityIdentifier("assistant-context")
          Button {query="";highlight=assistant.selectedModelID;showModels=true} label: {
            HStack(spacing:3) {Text(modelName).lineLimit(1).truncationMode(.middle).frame(maxWidth:90);Image(systemName:"chevron.down").font(.system(size:8))}
          }.buttonStyle(.plain).focused($buttonFocused,equals:.models).help(modelName).accessibilityIdentifier("assistant-model-picker")
          if assistant.busy {
            Button {Task {await assistant.stop()}} label: {Image(systemName:"stop.fill").font(.system(size:11,weight:.semibold)).frame(width:28,height:28).foregroundStyle(Theme.Color.paper).background(Theme.Color.ink,in:Circle())}
              .buttonStyle(.plain).disabled(assistant.stopping).accessibilityLabel("停止").accessibilityIdentifier("assistant-stop")
          } else {
            Button {assistant.send()} label: {Image(systemName:"arrow.up").font(.system(size:13,weight:.semibold)).frame(width:28,height:28).foregroundStyle(Theme.Color.paper).background(assistant.canSend ? Theme.Color.verified:Theme.Color.ink3.opacity(0.3),in:Circle())}
              .buttonStyle(.plain).disabled(!assistant.canSend).accessibilityLabel("发送").accessibilityIdentifier("assistant-send")
          }
        }.font(.system(size:11.5)).foregroundStyle(Theme.Color.ink2).buttonStyle(.borderless)
      }.padding(10).background(Theme.Color.paper,in:RoundedRectangle(cornerRadius:14))
        .overlay(RoundedRectangle(cornerRadius:14).stroke(focused ? Theme.Color.verified:Theme.Color.lineStrong,lineWidth:1))
        .background(RoundedRectangle(cornerRadius:17).stroke(Theme.Color.verified.opacity(focused ? 0.12:0),lineWidth:3).padding(-2))
      if let note=assistant.modelChangeNote {Text(note).font(.system(size:11)).foregroundStyle(.secondary)}
      else if assistant.allowsChanges,assistant.selectedModel?.tools==false {Text("当前模型不支持工具调用，请切换模型或选择讨论").font(.system(size:11)).foregroundStyle(Theme.Color.conditional)}
      else if !assistant.allowsChanges {Text("讨论模式不会修改或运行笔记本").font(.system(size:11)).foregroundStyle(.secondary)}
      if assistant.busy || assistant.status.hasPrefix("请求失败") || assistant.status.hasPrefix("操作") || assistant.status.hasPrefix("撤销结果") {
        HStack(spacing:6) {if assistant.busy {ProgressView().controlSize(.mini)};Text(assistant.status).lineLimit(3)}.font(.system(size:11)).foregroundStyle(.secondary).accessibilityIdentifier("assistant-status")
      }
    }.popover(isPresented:$showReferences,arrowEdge:.bottom) {referencePanel}
      .popover(isPresented:$showModels,arrowEdge:.bottom) {modelPanel}
      .popover(isPresented:$showContext,arrowEdge:.bottom) {contextPanel}
      .sheet(isPresented:$showInput) {inputPanel}
      .onChange(of:showModels) {if showModels {searchFocused=true} else {buttonFocused = .models}}
      .onChange(of:showContext) {if !showContext {buttonFocused = .context}}
  }
  private var contextRing:some View {
    ZStack {
      Circle().stroke(Theme.Color.ink3.opacity(0.4),style:StrokeStyle(lineWidth:2,dash:assistant.selectedModel?.contextLimit==nil ? [2,2]:[]))
      if let limit=assistant.selectedModel?.contextLimit,let estimate=assistant.inputPreview?.estimatedTokens {
        Circle().trim(from:0,to:min(1,Double(estimate)/Double(limit))).stroke(estimate>limit ? Theme.Color.conditional:Theme.Color.verified,lineWidth:2).rotationEffect(.degrees(-90))
      }
    }.frame(width:14,height:14)
  }
  private var modelPanel:some View {
    VStack(alignment:.leading,spacing:10) {
      TextField("搜索模型",text:$query).textFieldStyle(.roundedBorder).focused($searchFocused)
        .onSubmit {if let model=eligible.first(where:{$0.id==highlight}) ?? eligible.first {assistant.selectModel(model.id);showModels=false}}
        .onMoveCommand {direction in
          guard !eligible.isEmpty,direction == .up || direction == .down else {return}
          let current=eligible.firstIndex{$0.id==highlight} ?? (direction == .down ? -1:0)
          highlight=eligible[(current+(direction == .down ? 1:-1)+eligible.count)%eligible.count].id
        }
      ScrollView {
        VStack(alignment:.leading,spacing:9) {ForEach(filtered,id:\.0.id) {provider,models in
          Text(provider.name).font(.caption).foregroundStyle(.secondary)
          ForEach(models) {model in
            Button {assistant.selectModel(model.id);showModels=false} label: {
              VStack(alignment:.leading,spacing:3) {
                HStack {Text(model.displayName).fontWeight(.medium);Spacer();if model.id==assistant.selectedModelID {Image(systemName:"checkmark")}}
                Text((model.tools ? "工具调用":"无工具调用")+" · "+(model.images ? "图片输入":"文字输入")).font(.caption).foregroundStyle(.secondary)
                if assistant.allowsChanges,!model.tools {Text("不支持工具调用，切换到讨论模式后可用").font(.caption).foregroundStyle(.secondary)}
              }.frame(maxWidth:.infinity,alignment:.leading).padding(7).background(highlight==model.id ? Theme.Color.well:Color.clear,in:RoundedRectangle(cornerRadius:6))
            }.buttonStyle(.plain).disabled(assistant.allowsChanges && !model.tools)
          }
        }}
      }.frame(maxHeight:300)
      if filtered.isEmpty {Text("没有匹配的已配置模型").font(.callout).foregroundStyle(.secondary)}
      Divider()
      Button("管理模型… ⌘,") {showModels=false;assistant.showSettings=true}
    }.padding(14).frame(width:310).onExitCommand {showModels=false}
  }
  private var referencePanel:some View {
    VStack(alignment:.leading,spacing:10) {
      Text("引用当前笔记本中的单元格").font(.headline)
      ScrollView {VStack(alignment:.leading,spacing:4) {ForEach(assistant.referenceCandidates?() ?? []) {reference in
        Button {assistant.addReference(reference.id);showReferences=false} label: {
          VStack(alignment:.leading,spacing:3) {Text(reference.label);Text(reference.summary).font(.caption.monospaced()).lineLimit(1).foregroundStyle(.secondary)}.frame(maxWidth:.infinity,alignment:.leading).padding(6)
        }.buttonStyle(.plain)
      }}}.frame(maxHeight:320)
    }.padding(14).frame(width:310).onExitCommand {showReferences=false}
  }
  private var contextPanel:some View {
    VStack(alignment:.leading,spacing:12) {
      Text("上下文用量").font(.headline)
      if let preview=assistant.inputPreview {
        LabeledContent("系统与工具说明",value:"约 \(Int(ceil(Double(preview.systemCharacters)/1.6))) tokens")
        LabeledContent("对话与工具结果（含草稿）",value:"约 \(Int(ceil(Double(preview.conversationCharacters)/1.6))) tokens")
        LabeledContent("本条附件",value:"\(preview.attachmentCharacters) 字符")
        Text("以上为字符数 ÷ 1.6 的预估").font(.caption).foregroundStyle(.secondary)
      } else {Text(assistant.contextError ?? "正在读取本轮输入…").foregroundStyle(.secondary)}
      if let limit=assistant.selectedModel?.contextLimit {LabeledContent("上下文上限",value:"\(limit) tokens（模型配置）")}
      else {Text("上限未知").foregroundStyle(.secondary)}
      Divider()
      Text("上一轮请求的实际用量").fontWeight(.medium)
      if let usage=assistant.lastUsage {
        LabeledContent("输入",value:usage.input.map(String.init) ?? "未报告")
        LabeledContent("输出",value:usage.output.map(String.init) ?? "未报告")
        if let cache=usage.cacheRead {LabeledContent("缓存读取",value:String(cache))}
        if let cache=usage.cacheWrite {LabeledContent("缓存写入",value:String(cache))}
        Text("来自供应商返回，缓存项目不与输入相加").font(.caption).foregroundStyle(.secondary)
      } else {Text("尚无已报告用量").foregroundStyle(.secondary)}
      Button("查看本轮完整输入") {
        showContext=false;inputChoice=assistant.lastRequestInput==nil ? 0:1
        displayedInput=assistant.lastRequestInput ?? assistant.inputPreview;readableInput="";showInput=true
      }
      Picker("发送快捷键",selection:$assistant.sendShortcut) {Text("Enter 发送").tag(AssistantSendShortcut.enter);Text("⌘↩ 发送").tag(AssistantSendShortcut.commandReturn)}.pickerStyle(.menu)
    }.font(.system(size:12)).padding(16).frame(width:340).onExitCommand {showContext=false}
  }
  private var inputPanel:some View {
    VStack(alignment:.leading,spacing:12) {
      HStack {Text("本轮完整输入").font(.headline);Spacer();Button("关闭") {showInput=false}.keyboardShortcut(.cancelAction)}
      Picker("输入版本",selection:$inputChoice) {Text("发送前预览（含草稿）").tag(0);if assistant.lastRequestInput != nil {Text("最近实际请求").tag(1)}}.pickerStyle(.segmented)
      Text(inputChoice==1 ? "实际请求正文，不含认证头":"发送前预览；执行时将冻结消息和配置").font(.caption).foregroundStyle(.secondary)
      Toggle("显示原始请求 JSON",isOn:$rawInput).font(.caption)
      AssistantReadonlyInput(text:displayedInput.map{rawInput || readableInput.isEmpty ? $0.fullInput:readableInput} ?? assistant.contextError ?? "暂无可读取的输入")
        .task(id:displayedInput?.fullInput) {
          guard let original=displayedInput?.fullInput else {return}
          let readable=await Task.detached(priority:.userInitiated) {AssistantInputPresentation.readable(original)}.value
          if !Task.isCancelled {readableInput=readable}
        }
    }.onChange(of:inputChoice) {
      displayedInput=inputChoice==1 ? assistant.lastRequestInput:assistant.inputPreview;readableInput=""
    }.padding(18).frame(minWidth:640,idealWidth:800,minHeight:500)
  }
}
private struct AssistantReadonlyInput:NSViewRepresentable {
  let text:String
  func makeNSView(context:Context)->NSScrollView {
    let scroll=NSScrollView(),view=NSTextView(usingTextLayoutManager:true)
    scroll.hasVerticalScroller=true;view.isEditable=false;view.isSelectable=true;view.font = .monospacedSystemFont(ofSize:11.5,weight:.regular)
    view.isVerticallyResizable=true;view.autoresizingMask=[.width];view.frame=NSRect(x:0,y:0,width:640,height:500)
    view.textContainer?.widthTracksTextView=true;scroll.documentView=view;view.string=text;return scroll
  }
  func updateNSView(_ view:NSScrollView,context:Context) {if let textView=view.documentView as? NSTextView,textView.string != text {textView.string=text}}
}
