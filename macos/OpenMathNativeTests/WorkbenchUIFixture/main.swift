import AppKit
import Security
import SwiftUI

private struct LiveWorkbenchCase:Decodable {
  let root:String,scenario:String,model:String,prompt:String
  let seed:[String]
  var url:URL {URL(fileURLWithPath:root,isDirectory:true)}
  @MainActor func prepare(_ session:NativeDocumentSession)async throws {
    let api=URL(fileURLWithPath:"/Users/hert/.config/openmath/testing/pre-alpha-4-api.json")
    let config=try JSONSerialization.jsonObject(with:Data(contentsOf:api)) as! [String:Any]
    guard let base=config["base_url"] as? String,["https://4Router.net/v1","http://localhost:10100/v1"].contains(base),let key=config["api_key"] as? String,
      let models=config["models"] as? [String],models.contains(model) else {throw AssistantSetupError.invalid}
    let secret=Data(key.utf8)
    try session.assistant.configure(.init(baseURL:base,model:model,credentialReference:"authorized-live-api"),secrets:SecretStore(testReader:{_ in (errSecSuccess,secret)}))
    session.assistant.allowsChanges=scenario != "S6"
    var receipts:[NativeKernelReceipt]=[]
    for cell in seed {receipts.append(try await session.runtime.kernel.runCell(cell,operation:"live-seed-"+cell))}
    await session.runtime.displayAgentReceipts(receipts)
    session.assistant.draft=prompt
  }
}
private struct LiveWorkbenchObservation:View {
  @ObservedObject var assistant:NativeAssistant
  let session:NativeDocumentSession,live:LiveWorkbenchCase
  var body:some View {
    Color.clear.frame(width:0,height:0)
      .onChange(of:assistant.busy) {if !assistant.busy {Task {await save()}}}
      .onChange(of:assistant.undoneTurns) {Task {await save()}}
  }
  @MainActor private func save()async {
    do {
      var values:[String:String]=[:]
      if ["S1","S4","S5"].contains(live.scenario) {
        let client=ResultClient(host:session.host)
        for receipt in await session.agent.currentKernelReceipts() {
          guard let cell=receipt.producer.cell_id.value,let result=receipt.result_id.value else {continue}
          let manifest=try await client.manifest(result)
          if let binding=manifest.bindings.last {try client.select(binding);if let value=try await client.completeSource(format:.input_form,path:[]) {values[cell]=String(value.prefix(2048))}}
        }
      }
      let encoder=JSONEncoder();encoder.outputFormatting=[.prettyPrinted,.sortedKeys]
      let source=try JSONSerialization.jsonObject(with:encoder.encode(session.drafts.confirmed))
      let outcomes=try JSONSerialization.jsonObject(with:encoder.encode(assistant.outcomes))
      let messages=try JSONSerialization.jsonObject(with:encoder.encode(assistant.messages))
      let result:[String:Any]=["scenario":live.scenario,"model":live.model,"busy":assistant.busy,"status":assistant.status,
        "source":source,"outcomes":outcomes,"messages":messages,"values":values,"undone_turns":assistant.undoneTurns.sorted()]
      try JSONSerialization.data(withJSONObject:result,options:[.prettyPrinted,.sortedKeys,.withoutEscapingSlashes]).write(to:live.url.appendingPathComponent("summary.json"),options:.atomic)
    } catch {try? Data("Live result observation unavailable; original state retained".utf8).write(to:live.url.appendingPathComponent("observation-error.txt"))}
  }
}

@main struct WorkbenchUIFixture:App {
  @StateObject private var workspace:NativeFileWorkspace
  private let fixture:URL
  private let transcript:[AssistantMessage]?
  private let live:LiveWorkbenchCase?
  init() {
    guard let resource=Bundle.main.url(forResource:"FixtureRoot",withExtension:"txt"),let value=try? String(contentsOf:resource,encoding:.utf8) else {fatalError("fixture root missing")}
    let root=URL(fileURLWithPath:value.trimmingCharacters(in:.whitespacesAndNewlines),isDirectory:true)
    let liveURL=root.appendingPathComponent("current-case.json")
    live=FileManager.default.fileExists(atPath:liveURL.path) ? try? JSONDecoder().decode(LiveWorkbenchCase.self,from:Data(contentsOf:liveURL)):nil
    let caseRoot=live?.url ?? root
    precondition(caseRoot.standardizedFileURL.path==root.standardizedFileURL.path || caseRoot.standardizedFileURL.path.hasPrefix(root.standardizedFileURL.path+"/"))
    fixture=caseRoot.appendingPathComponent("fixture.omnb")
    transcript=Bundle.main.url(forResource:"ActionTranscript",withExtension:"json").flatMap{try? JSONDecoder().decode([AssistantMessage].self,from:Data(contentsOf:$0))}
    _workspace=StateObject(wrappedValue:NativeFileWorkspace(storage:NativeAppStorage(paths:.init(root:caseRoot.appendingPathComponent("storage"),channel:.preview))))
  }
  var body:some Scene {
    Window("OpenMath Workbench Fixture",id:"fixture") {
      Group {
        if let transcript {
          ScrollView {VStack(alignment:.leading,spacing:18) {
            Text("操作记录组件预览（回环数据，不调用模型）").font(.caption).foregroundStyle(.secondary)
            ForEach(transcript) {message in NativeAssistantMessageBody(message:message)}
          }.padding(20)}.frame(width:400)
        }
        else if let session=workspace.session {
          WorkspaceView(workspace:workspace,session:session)
            .background {if let live {LiveWorkbenchObservation(assistant:session.assistant,session:session,live:live)}}
        }
        else if let error=workspace.errorText {Text("测试笔记本未打开："+error).textSelection(.enabled).padding()}
        else {ProgressView("正在打开测试笔记本")}
      }.frame(minWidth:transcript==nil ? 640:400,minHeight:560)
        .background(NativeWindowAttachment(workspace:workspace).frame(width:0,height:0))
        .task {
          if transcript==nil {
            _ = await workspace.open(fixture)
            if let live,let session=workspace.session {
              do {
                try await live.prepare(session)
                session.runtime.navigation.inspector = .assistant
                session.runtime.navigation.inspectorVisible = true
              } catch {try? Data("Authorized live setup failed; no model request was started".utf8).write(to:live.url.appendingPathComponent("setup-error.txt"))}
            }
          }
        }
    }.defaultSize(width:transcript==nil ? 1440:400,height:900)
      .commands {WorkbenchCommands(workspace:workspace)}
  }
}
