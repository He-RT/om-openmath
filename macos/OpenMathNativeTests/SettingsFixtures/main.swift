import AppKit
import SwiftUI
import Darwin
import Combine

@MainActor private final class SettingsFixtureState:ObservableObject {
  let root:URL,services:NativeAppStorage,workspace:NativeFileWorkspace,model:NativeSettingsModel,registry:ModelRegistry
  @Published var ready=false
  var ownCredential:String?
  private var started=false
  private var links=[AnyCancellable]()
  let ui=Bundle.main.object(forInfoDictionaryKey:"OpenMathSettingsFixture") as? Bool==true
  init() {
    let base=Bundle.main.object(forInfoDictionaryKey:"OpenMathFixtureRoot") as? String ?? CommandLine.arguments.first(where:{$0.hasPrefix("--root=")}).map{String($0.dropFirst(7))}!
    root=URL(fileURLWithPath:base).appendingPathComponent("run-"+UUID().uuidString.lowercased())
    services=NativeAppStorage(paths:.init(root:root.appendingPathComponent("storage"),channel:.preview))
    workspace=NativeFileWorkspace(storage:services);registry=try! ModelRegistry(file:root.appendingPathComponent("models.json"));model=NativeSettingsModel(registry:registry)
    // Match production AppHost's two StateObjects so native command validation sees edits.
    model.objectWillChange.sink { [weak self] in self?.objectWillChange.send() }.store(in:&links)
    workspace.objectWillChange.sink { [weak self] in self?.objectWillChange.send() }.store(in:&links)
  }
  func start()async {
    guard !started else {return};started=true
    do {
      try FileManager.default.createDirectory(at:root,withIntermediateDirectories:true)
      let firstRoot=root.appendingPathComponent("settings-before-notebook")
      let firstServices=NativeAppStorage(paths:.init(root:firstRoot,channel:.preview))
      let firstWorkspace=NativeFileWorkspace(storage:firstServices)
      let firstRegistry=try ModelRegistry(file:firstRoot.appendingPathComponent("models.json"))
      let firstModel=NativeSettingsModel(registry:firstRegistry)
      firstModel.workspace=firstWorkspace;await firstModel.load();precondition(firstModel.loaded)
      firstModel.addProvider();firstModel.providers[0].name="首次配置"
      firstModel.providers[0].baseURL="http://127.0.0.1:10104/v1"
      firstModel.providers[0].credentialReference="fixture-unused-first-run"
      firstModel.addModel(firstModel.providers[0].id)
      firstModel.providers[0].models[0].apiName="fixture-first-run"
      await firstModel.save();precondition(firstModel.error==nil)
      precondition(FileManager.default.fileExists(atPath:firstRoot.appendingPathComponent("store.json").path))
      _ = try await firstServices.open();try await firstServices.close()
      print("First launch: settings saved before any notebook, managed storage root remains openable; no credential or model request")
      let source=NativeSourceFile(version:1,title:"设置验收",cells:[.init(id:"math",kind:.math,source:"2 + 2",dialect:.modern)])
      let sourceURL=root.appendingPathComponent("设置验收.omnb"),original=try JSONEncoder().encode(source);try original.write(to:sourceURL)
      guard await workspace.open(sourceURL),let assistant=workspace.session?.assistant else {fatalError("fixture document failed to open")}
      let server=try DelayedLoopbackServer(response:Data("{\"data\":[{\"id\":\"fixture-primary\"},{\"id\":\"fixture-secondary\"}]}".utf8),delay:0,contentType:"application/json")
      let configuration=AssistantConfiguration(baseURL:"http://127.0.0.1:\(server.port)/v1",model:"fixture-primary",credentialReference:"fixture-unused-"+UUID().uuidString.lowercased())
      try assistant.configure(configuration)
      var seed=ModelRegistryDocument.current(configuration);seed.providers[0].name="隔离测试服务"
      try await registry.save(seed)
      model.workspace=workspace;await model.load();precondition(model.loaded)
      model.addModel(seed.providers[0].id)
      model.providers[0].models[1].apiName="fixture-secondary";model.providers[0].models[1].displayName="备用图片模型"
      model.providers[0].models[1].tools=false;model.providers[0].models[1].images=true
      model.providers[0].models[1].context="invalid"
      await model.save();precondition(model.error != nil && model.providers[0].models[1].context=="invalid")
      let unchanged=try await registry.load();precondition(unchanged==seed)
      model.providers[0].models[1].context="32000"
      precondition(model.error==nil,"correcting a draft kept a stale validation error")
      let ownKey="settings-fixture-"+UUID().uuidString.lowercased();model.providers[0].key=ownKey
      await model.save();precondition(model.error==nil && model.providers[0].key.isEmpty)
      let saved=try await registry.load();ownCredential=saved.providers[0].credentialReference
      precondition(saved.providers[0].models.count==2 && saved.providers[0].models[1].contextLimit==32000)
      let stored=try Data(contentsOf:root.appendingPathComponent("models.json"));precondition(stored.range(of:Data(ownKey.utf8))==nil)
      let secret=try await SecretStore().read(reference:ownCredential!)
      precondition(secret.authenticationHeader(.bearer)["Authorization"]=="Bearer "+ownKey)
      precondition(assistant.registry==saved && assistant.selectedModelID==saved.defaultModelID)
      assistant.setMode(execute:true);assistant.selectModel(saved.providers[0].models[1].id)
      precondition(assistant.selectedModelID==saved.defaultModelID)
      assistant.setMode(execute:false);assistant.selectModel(saved.providers[0].models[1].id)
      precondition(assistant.selectedModelID==saved.providers[0].models[1].id && assistant.nextTools.isEmpty)
      let storage=try await services.open(),task="no-tool-fixture",conversation="no-tool-"+UUID().uuidString.lowercased()
      _ = try await storage.createAgentSession(conversation,documentID:workspace.session!.drafts.confirmed.document_id)
      let schemaRoot=Bundle.main.object(forInfoDictionaryKey:"OpenMathRepository") as? String ?? FileManager.default.currentDirectoryPath
      let context=AssistantContext(host:workspace.session!.host,storage:storage,session:conversation,task:task,configuration:configuration,mode:.discuss,tools:assistant.nextTools,schema:URL(fileURLWithPath:schemaRoot).appendingPathComponent("docs/design/agent-tools.schema.json"))
      let input=AgentCanonicalMessage(record_type:.canonical_agent_message,message_version:1,message_id:"no-tool-question",role:.user,agent_turn_id:.init(nil),task_id:.init(task),parts:[.agent_text_part(.init(kind:.text,part_id:"question",text:"普通讨论"))])
      let preview=try await context.preview(input),body=try JSONSerialization.jsonObject(with:Data(preview.fullInput.utf8)) as! [String:Any]
      precondition((body["tools"] as? [Any])?.isEmpty==true && preview.fullInput.contains("本轮未提供工具"))
      await model.testConnection(saved.providers[0].id)
      precondition(model.connectionStatus[saved.providers[0].id]?.contains("目录返回 2 个模型")==true)
      precondition(server.receivedBody && String(decoding:server.requestBytes,as:UTF8.self).hasPrefix("GET /v1/models HTTP/1.1"))
      model.addProvider();model.providers[1].name="未保存的连接";model.providers[1].baseURL="https://example.invalid/v1"
      await model.save();precondition(model.error != nil && model.providers.count==2)
      let retained=try await registry.load();precondition(retained==saved)
      model.revert();precondition(model.providers.count==1 && model.error==nil && !model.dirty)
      let bytes=try Data(contentsOf:sourceURL);precondition(bytes==original)
      assistant.setMode(execute:true);assistant.selectModel(saved.defaultModelID!)
      workspace.session?.runtime.navigation.inspectorVisible=true
      workspace.session?.runtime.navigation.inspector = .assistant
      ready=true
      print("Native Settings: invalid context/new-provider key retained without overwriting config; actual Keychain/JSON roundtrip with no key bytes in JSON; two configured models/capabilities reached composer; actual loopback GET models returned 2; revert preserved notebook bytes. No model inference.")
      if !ui {await cleanup();Darwin.exit(0)}
    } catch {FileHandle.standardError.write(Data("Native settings fixture failed: \(type(of:error))\n".utf8));fatalError("native settings fixture failed")}
  }
  func cleanup()async {
    _ = await workspace.shutdown()
    // Only this test's freshly generated exact account; never enumerate Keychain.
    if let ownCredential {SecretStore.remove(reference:ownCredential)}
    try? await services.close()
  }
}
@MainActor private final class SettingsFixtureDelegate:NSObject,NSApplicationDelegate {
  weak var state:SettingsFixtureState?
  private var stopping=false
  func applicationShouldTerminate(_ sender:NSApplication)->NSApplication.TerminateReply {
    guard !stopping,let state else {return .terminateNow};stopping=true
    Task {await state.cleanup();sender.reply(toApplicationShouldTerminate:true)};return .terminateLater
  }
}
private struct SettingsFixtureControls:View {
  let state:SettingsFixtureState
  @Environment(\.openSettings) private var openSettings
  var body:some View {
    HStack {
      Text("隔离设置测试 · 仅本机目录 GET，无模型推理");Spacer()
      Button("切换测试主题") {NSApp.appearance=NSAppearance(named:NSApp.effectiveAppearance.bestMatch(from:[.aqua,.darkAqua]) == .darkAqua ? .aqua:.darkAqua)}.accessibilityIdentifier("fixture-settings-theme")
      Button("打开模型设置") {UserDefaults.standard.set("models",forKey:"settings.section");openSettings()}.accessibilityIdentifier("fixture-open-settings")
    }.font(.caption).padding(8)
  }
}
@main struct SettingsFixtureApp:App {
  @NSApplicationDelegateAdaptor(SettingsFixtureDelegate.self) private var delegate
  @StateObject private var state=SettingsFixtureState()
  var body:some Scene {
    Window("OpenMath 设置验收",id:"fixture-workspace") {
      VStack(spacing:0) {
        if let session=state.workspace.session {WorkspaceView(workspace:state.workspace,session:session)} else {ProgressView("正在准备隔离设置…").frame(maxWidth:.infinity,maxHeight:.infinity)}
        SettingsFixtureControls(state:state)
      }.background(NativeWindowAttachment(workspace:state.workspace).frame(width:0,height:0))
        .task {delegate.state=state;NSApp.setActivationPolicy(state.ui ? .regular:.accessory);await state.start()}
    }.defaultSize(width:1180,height:780).windowToolbarStyle(.unified).commands {
      WorkbenchCommands(workspace:state.workspace)
      CommandGroup(replacing:.undoRedo) {Button("撤销") {state.workspace.undo()}.keyboardShortcut("z").disabled(!state.workspace.canUndo);Button("重做") {state.workspace.redo()}.keyboardShortcut("z",modifiers:[.command,.shift]).disabled(!state.workspace.canRedo)}
    }
    Settings {NativeSettingsView(model:state.model,workspace:state.workspace)}
  }
}
