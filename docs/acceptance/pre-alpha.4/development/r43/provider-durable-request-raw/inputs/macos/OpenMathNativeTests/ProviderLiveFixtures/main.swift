import Foundation
import Darwin

enum LiveFixtureError:Error {case invalidConfig,unexpectedResult}
@main struct ProviderLiveFixtures {
  static func main()async throws {
    guard CommandLine.arguments.count==2 else {throw LiveFixtureError.invalidConfig}
    let path=CommandLine.arguments[1],attributes=try FileManager.default.attributesOfItem(atPath:path)
    guard attributes[.type] as? FileAttributeType == .typeRegular,
      (attributes[.ownerAccountID] as? NSNumber)?.uint32Value==getuid(),
      let permissions=attributes[.posixPermissions] as? NSNumber,permissions.intValue & 0o077 == 0,
      let config=try JSONSerialization.jsonObject(with:Data(contentsOf:URL(fileURLWithPath:path))) as? [String:Any],
      let base=config["base_url"] as? String,let key=config["api_key"] as? String,
      let models=config["models"] as? [String],models==["gpt-6.1-sol","gpt-6-luna"],
      let endpoint=URL(string:base.trimmingCharacters(in:CharacterSet(charactersIn:"/"))+"/chat/completions") else {throw LiveFixtureError.invalidConfig}
    let data=Data(key.utf8),host=try await NativeHostClient.open()
    // Explicit user-authorized testing credential; no Registry import/old Keychain/profile scan.
    let secrets=SecretStore(testReader:{_ in (0,data)}),transport=ProviderTransport(secrets:secrets)
    var reports:[[String:Any]]=[]
    for name in models {
      let binding:[String:Any] = ["provider_ref":"explicit-test-provider","route_ref":"explicit-test-route","model_ref":name,"adapter_version":"single-turn-1","protocol":"chat_completions"]
      let message:[String:Any] = ["record_type":"canonical_agent_message","message_version":1,"message_id":"user","role":"user","task_id":"probe-task","agent_turn_id":"probe-turn","parts":[["kind":"text","part_id":"text","text":"Return only the single ASCII digit equal to 2+2."]]]
      let request:[String:Any] = ["binding":binding,"api_model":name,"messages":[["canonical":message,"replay":NSNull()]],"results":[],"tools":[],"max_output_tokens":64,"temperature":NSNull(),"stream":true]
      let requestJSON=String(decoding:try JSONSerialization.data(withJSONObject:request),as:UTF8.self)
      let nonce=UUID().uuidString.lowercased(),model=try await ProviderModelSession.prepare(host:host,requestJSON:requestJSON,request:"probe-"+nonce,context:"context-"+nonce,task:"probe-task",turn:"probe-turn",message:"assistant-"+nonce)
      let route=try ProviderRoute(binding:model.binding,endpoint:endpoint,authentication:.bearer,credentialReference:"explicit-user-test",timeouts:try ProviderTimeouts(firstResponse:60,idle:30,total:90))
      let result=try await transport.send(model,route:route,journal:.init(record:{_ in},responseBytes:{_ in}),onText:{_ in})
      let text=result.message.parts.compactMap {part->String? in if case .agent_text_part(let p)=part{return p.text};return nil}.joined()
      guard result.finish=="complete",text.trimmingCharacters(in:.whitespacesAndNewlines)=="4" else {throw LiveFixtureError.unexpectedResult}
      reports.append(["model":name,"matched":true,"finish":result.finish,"response":"4","usage":try JSONSerialization.jsonObject(with:result.usageJSON),"scope":"authorized Swift URLSession/Rust C ABI development smoke; injected explicit credential, no Registry or durable request ledger claim"])
    }
    try await host.close()
    print(String(decoding:try JSONSerialization.data(withJSONObject:reports,options:[.sortedKeys]),as:UTF8.self))
  }
}
