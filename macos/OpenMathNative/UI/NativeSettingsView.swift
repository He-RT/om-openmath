import AppKit
import SwiftUI

struct NativeSettingsView:View {
  @ObservedObject var model:NativeSettingsModel
  @ObservedObject var workspace:NativeFileWorkspace
  @AppStorage("settings.section") private var section="general"
  @AppStorage("workbench.appearance") private var appearance="system"
  @AppStorage("assistant.send-shortcut") private var sendShortcut="enter"
  @AppStorage("assistant.default-mode") private var defaultMode="discuss"
  @State private var deletingProvider:String?
  @State private var showRecovery=false
  @State private var showSessions=false
  var body:some View {
    VStack(spacing:0) {
      TabView(selection:$section) {
        general.tabItem {Label("通用",systemImage:"gearshape")}.tag("general")
        editing.tabItem {Label("编辑与计算",systemImage:"function")}.tag("editing")
        models.tabItem {Label("模型",systemImage:"cpu")}.tag("models")
        assistant.tabItem {Label("助手",systemImage:"bubble.left.and.bubble.right")}.tag("assistant")
        data.tabItem {Label("数据",systemImage:"internaldrive")}.tag("data")
      }
      if section=="models" || section=="assistant" {
        Divider()
        HStack(spacing:12) {
          if model.saving || model.loading {WorkbenchProgress().controlSize(.small)}
          VStack(alignment:.leading,spacing:3) {
            if let error=model.error {Text(error).foregroundStyle(Theme.Color.conditional)}
            else {Text(model.dirty ? "模型配置有未保存的修改":model.saveStatus ?? "密钥保存在系统钥匙串中").foregroundStyle(.secondary)}
          }.font(.caption).frame(maxWidth:.infinity,alignment:.leading)
          if !model.loaded {Button("重试") {Task {await model.load()}}.disabled(model.loading)}
          Button("还原") {model.revert()}.disabled(!model.dirty || model.saving)
          Button("保存") {Task {await model.save()}}.buttonStyle(.borderedProminent).disabled(!model.loaded || !model.dirty || model.saving).accessibilityIdentifier("settings-save-models")
        }.padding(16)
      }
    }.frame(minWidth:820,idealWidth:960,minHeight:620,idealHeight:720)
      .tint(Theme.Color.verified).task {model.workspace=workspace;await model.load()}
      .onChange(of:sendShortcut) {workspace.session?.assistant.sendShortcut=AssistantSendShortcut(rawValue:sendShortcut) ?? .enter}
      .confirmationDialog("移除供应商及其模型？",isPresented:Binding(get:{deletingProvider != nil},set:{if !$0 {deletingProvider=nil}}),titleVisibility:.visible) {
        Button("移除",role:.destructive) {if let id=deletingProvider {model.removeProvider(id)};deletingProvider=nil}
        Button("取消",role:.cancel) {deletingProvider=nil}
      } message: {Text("更改在保存后生效；当前回合继续使用冻结的配置。笔记本和历史对话保留。")}
      .sheet(isPresented:$showRecovery) {NativeRecoveryList(workspace:workspace,isPresented:$showRecovery)}
      .sheet(isPresented:$showSessions) {NativeSessionRecoveryList(workspace:workspace,isPresented:$showSessions)}
  }
  private var general:some View {
    Form {
      LabeledContent("OpenMath",value:Bundle.main.object(forInfoDictionaryKey:"OpenMathReleaseVersion") as? String ?? "开发版")
      LabeledContent("平台",value:"Apple Silicon · macOS 27 或更新版本")
      Picker("外观",selection:$appearance) {Text("跟随系统").tag("system");Text("浅色").tag("light");Text("深色").tag("dark")}
        .accessibilityIdentifier("settings-appearance")
      Text("在本地笔记本中计算、推导与记录。").foregroundStyle(.secondary)
    }.formStyle(.grouped).padding(16)
  }
  private var editing:some View {
    Form {
      LabeledContent("运行方式",value:"手动运行单元格")
      Text("编辑后已有结果会标为过期；运行当前单元格使用 ⌘↩，停止使用 ⌘.。").foregroundStyle(.secondary)
    }.formStyle(.grouped).padding(16)
  }
  private var assistant:some View {
    Form {
      Picker("发送快捷键",selection:$sendShortcut) {Text("Enter 发送 · ⇧Enter 换行").tag("enter");Text("⌘↩ 发送 · Enter 换行").tag("commandReturn")}
      Picker("新对话的默认模式",selection:$defaultMode) {Text("讨论 · 只读取和临时计算").tag("discuss");Text("执行 · 可修改和运行，每轮可撤销").tag("execute")}
      Picker("新对话的默认模型",selection:$model.defaultModel) {
        if model.providers.flatMap(\.models).isEmpty {Text("请先添加模型").tag(String?.none)}
        ForEach(model.providers) {provider in
          ForEach(provider.models) {item in Text(provider.name+" · "+item.displayName).tag(String?.some(item.id))}
        }
      }.disabled(!model.loaded || model.saving)
      Text("默认模型随模型配置保存；模式和发送快捷键立即保存。当前回合使用开始时的模式和模型。").font(.caption).foregroundStyle(.secondary)
    }.formStyle(.grouped).padding(16)
  }
  private var data:some View {
    Form {
      Text("笔记本保存在你选择的 .omnb 文件；本地备份与助手历史由应用保留。").foregroundStyle(.secondary)
      Button("恢复本地笔记本…") {showRecovery=true;Task {await workspace.refreshRecoverableDocuments()}}.disabled(workspace.busy)
      Button("恢复助手会话…") {showSessions=true;Task {await workspace.refreshRecoverableSessions()}}.disabled(workspace.busy)
      Button("核对当前笔记本保存状态") {Task {await workspace.reconcile()}}.disabled(workspace.busy || workspace.session==nil)
    }.formStyle(.grouped).padding(16)
  }
  private var models:some View {
    HSplitView {
      VStack(spacing:0) {
        List(selection:$model.selectedProvider) {
          ForEach(model.providers) {provider in
            VStack(alignment:.leading,spacing:3) {Text(provider.name.isEmpty ? "未命名供应商":provider.name);Text("\(provider.models.count) 个模型").font(.caption).foregroundStyle(.secondary)}.padding(.vertical,3).tag(provider.id)
          }
        }.accessibilityIdentifier("settings-providers")
        Divider()
        HStack {Button("添加供应商",systemImage:"plus") {model.addProvider()}.labelStyle(.iconOnly).help("添加供应商").accessibilityIdentifier("settings-add-provider");Button("移除供应商",systemImage:"minus") {deletingProvider=model.selectedProvider}.labelStyle(.iconOnly).help("移除选中的供应商").disabled(model.selectedProvider==nil);Spacer()}.padding(10).disabled(!model.loaded || model.saving)
      }.frame(minWidth:180,idealWidth:210,maxWidth:240)
      if let id=model.selectedProvider,let index=model.providers.firstIndex(where:{$0.id==id}) {
        providerForm(index:index).id(id).frame(minWidth:540)
      } else {ContentUnavailableView("添加供应商",systemImage:"cpu",description:Text("配置兼容 Chat Completions 的服务，再加入要使用的模型。")).frame(maxWidth:.infinity,maxHeight:.infinity)}
    }.disabled(model.saving)
  }
  private func providerForm(index:Int)->some View {
    let provider=model.providers[index]
    return Form {
      Section("连接") {
        TextField("显示名",text:$model.providers[index].name).accessibilityIdentifier("settings-provider-name")
        TextField("API 地址",text:$model.providers[index].baseURL,prompt:Text("https://example.com/v1")).accessibilityIdentifier("settings-provider-url")
        SecureField(provider.credentialReference.isEmpty ? "API Key":"API Key（留空保留已存密钥）",text:$model.providers[index].key).accessibilityIdentifier("settings-provider-key")
        Text("本机回环地址支持 HTTP，其他地址使用 HTTPS。密钥仅写入系统钥匙串。").font(.caption).foregroundStyle(.secondary)
        HStack {Button("测试连接") {Task {await model.testConnection(provider.id)}}.disabled(model.testingProvider != nil);if model.testingProvider==provider.id {WorkbenchProgress().controlSize(.small)}}
        if let status=model.connectionStatus[provider.id] {Text(status).font(.caption).foregroundStyle(.secondary)}
      }
      Section {
        ForEach(provider.models.indices,id:\.self) {modelIndex in
          modelForm(provider:index,model:modelIndex)
        }
        Button("添加模型",systemImage:"plus") {model.addModel(provider.id)}.accessibilityIdentifier("settings-add-model")
      } header: {Text("模型")}
    }.formStyle(.grouped).padding(.horizontal,8)
  }
  private func modelForm(provider:Int,model index:Int)->some View {
    let item=model.providers[provider].models[index],providerID=model.providers[provider].id
    return VStack(alignment:.leading,spacing:10) {
      HStack {Text(item.displayName.isEmpty ? "未命名模型":item.displayName).font(.headline);Spacer();Button("移除模型",systemImage:"minus.circle") {model.removeModel(item.id,provider:providerID)}.labelStyle(.iconOnly).help("从此供应商移除模型")}
      TextField("API 名称",text:$model.providers[provider].models[index].apiName).accessibilityIdentifier("settings-model-api-"+item.id)
      TextField("显示名",text:$model.providers[provider].models[index].displayName)
      HStack {Toggle("工具调用",isOn:$model.providers[provider].models[index].tools);Toggle("图片输入",isOn:$model.providers[provider].models[index].images)}.toggleStyle(.checkbox)
      TextField("上下文上限",text:$model.providers[provider].models[index].context,prompt:Text("留空表示未知"))
    }.padding(.vertical,6)
  }
}
