import Foundation
import SwiftUI

struct ModelSettingsDraft:Identifiable,Equatable {
  let id:String
  var apiName:String,displayName:String,tools:Bool,images:Bool,context:String
  init(_ model:RegisteredModel) {id=model.id;apiName=model.apiName;displayName=model.displayName;tools=model.tools;images=model.images;context=model.contextLimit.map(String.init) ?? ""}
  init() {id="model-"+UUID().uuidString.lowercased();apiName="";displayName="新模型";tools=true;images=false;context=""}
  func value()throws->RegisteredModel {
    let text=context.trimmingCharacters(in:.whitespacesAndNewlines)
    guard text.isEmpty || text.utf8.allSatisfy({(48...57).contains($0)}) && Int(text).map({$0>0 && $0<=100_000_000})==true else {throw ModelSettingsError.context}
    return .init(id:id,apiName:apiName.trimmingCharacters(in:.whitespacesAndNewlines),displayName:displayName.trimmingCharacters(in:.whitespacesAndNewlines),tools:tools,images:images,contextLimit:text.isEmpty ? nil:Int(text))
  }
}
struct ProviderSettingsDraft:Identifiable,Equatable {
  let id:String
  var name:String,baseURL:String,credentialReference:String,models:[ModelSettingsDraft]
  // Deliberately neither Codable nor written to the registry; only the explicit save path sees it.
  var key=""
  init(_ provider:RegisteredProvider) {id=provider.id;name=provider.name;baseURL=provider.baseURL;credentialReference=provider.credentialReference;models=provider.models.map(ModelSettingsDraft.init)}
  init() {id="provider-"+UUID().uuidString.lowercased();name="新供应商";baseURL="";credentialReference="";models=[]}
  func value()throws->RegisteredProvider {
    .init(id:id,name:name.trimmingCharacters(in:.whitespacesAndNewlines),baseURL:baseURL.trimmingCharacters(in:.whitespacesAndNewlines).trimmingCharacters(in:CharacterSet(charactersIn:"/")),credentialReference:credentialReference,models:try models.map{try $0.value()})
  }
}
enum ModelSettingsError:Error {case context,credential}

@MainActor final class NativeSettingsModel:ObservableObject {
  @Published var providers:[ProviderSettingsDraft]=[] {
    didSet {
      guard !loading,!saving else {return}
      error=nil;saveStatus=nil
      for provider in providers {
        if let previous=oldValue.first(where:{$0.id==provider.id}),previous.baseURL != provider.baseURL || previous.key != provider.key || previous.credentialReference != provider.credentialReference {connectionStatus[provider.id]="连接配置待验证"}
      }
    }
  }
  @Published var selectedProvider:String?
  @Published var defaultModel:String?
  @Published private(set) var committed:ModelRegistryDocument?
  @Published private(set) var loaded=false
  @Published private(set) var saving=false
  @Published private(set) var loading=false
  @Published private(set) var testingProvider:String?
  @Published private(set) var error:String?
  @Published private(set) var connectionStatus:[String:String]=[:]
  @Published private(set) var saveStatus:String?
  weak var workspace:NativeFileWorkspace?
  private let registry:ModelRegistry?
  private let secrets:SecretStore
  private var retired=Set<String>(),retireTask:Task<Void,Never>?
  init(registry:ModelRegistry?=nil,secrets:SecretStore=SecretStore()) {self.registry=registry ?? (try? ModelRegistry());self.secrets=secrets}
  var dirty:Bool {
    guard let committed else {return false}
    return providers != committed.providers.map(ProviderSettingsDraft.init) || defaultModel != committed.defaultModelID
  }
  func load()async {
    guard !loading,!loaded else {return}
    guard let registry else {error="模型配置目录暂不可用，现有数据未被覆盖。";return}
    loading=true;defer {loading=false}
    do {let document=try await registry.loadInitial();apply(document);error=nil;loaded=true}
    catch {self.error="模型配置无法读取。现有文件未被覆盖，请重试。"}
  }
  private func apply(_ document:ModelRegistryDocument) {
    let selected=selectedProvider
    committed=document;providers=document.providers.map(ProviderSettingsDraft.init);defaultModel=document.defaultModelID
    selectedProvider=providers.contains(where:{$0.id==selected}) ? selected:providers.first?.id
  }
  func addProvider() {guard loaded,!saving else {return};let provider=ProviderSettingsDraft();providers.append(provider);selectedProvider=provider.id;saveStatus=nil}
  func removeProvider(_ id:String) {
    guard !saving else {return};let removed=Set(providers.first(where:{$0.id==id})?.models.map(\.id) ?? [])
    providers.removeAll{$0.id==id};if removed.contains(defaultModel ?? "") {defaultModel=providers.first?.models.first?.id}
    selectedProvider=providers.first?.id;saveStatus=nil
  }
  func addModel(_ provider:String) {guard let index=providers.firstIndex(where:{$0.id==provider}),!saving else {return};let model=ModelSettingsDraft();providers[index].models.append(model);if defaultModel==nil {defaultModel=model.id}}
  func removeModel(_ id:String,provider:String) {
    guard let index=providers.firstIndex(where:{$0.id==provider}),!saving else {return};providers[index].models.removeAll{$0.id==id}
    if defaultModel==id {defaultModel=providers.flatMap(\.models).first?.id}
  }
  func revert() {guard !saving,let committed else {return};apply(committed);error=nil;saveStatus=nil}
  func save()async {
    guard loaded,!saving,let registry else {return};saving=true;defer {saving=false}
    do {
      for provider in providers where provider.credentialReference.isEmpty && provider.key.isEmpty {throw ModelSettingsError.credential}
      let document=ModelRegistryDocument(providers:try providers.map{try $0.value()},defaultModelID:defaultModel)
      let keys=Dictionary(uniqueKeysWithValues:providers.filter{!$0.key.isEmpty}.map{($0.id,Data($0.key.utf8))})
      let saved=try await registry.commit(document,keys:keys)
      apply(saved.document);loaded=true;error=nil;saveStatus="模型配置已保存"
      workspace?.session?.assistant.applyRegistry(saved.document)
      retired.formUnion(saved.retired);retireWhenIdle()
    } catch ModelSettingsError.context {error="上下文上限请输入 1–100000000 的整数，留空表示未知。草稿已保留。"}
    catch ModelSettingsError.credential {error="新供应商需要 API Key。密钥只保存在系统钥匙串，草稿已保留。"}
    catch {self.error="保存尚未确认，草稿已保留。请检查服务地址、模型名称、能力配置和钥匙串状态后重试。"}
  }
  private func retireWhenIdle() {
    retireTask?.cancel()
    retireTask=Task { [weak self] in
      guard let self else {return}
      while workspace?.session?.assistant.busy==true {do {try await Task.sleep(for:.milliseconds(250))} catch {return}}
      let current=Set(committed?.providers.map(\.credentialReference) ?? [])
      for reference in retired.subtracting(current) {SecretStore.remove(reference:reference)}
      retired=[]
    }
  }
  func testConnection(_ id:String)async {
    guard testingProvider==nil,let provider=providers.first(where:{$0.id==id}) else {return}
    testingProvider=id;connectionStatus[id]="正在读取模型目录…"
    defer {
      testingProvider=nil
      if providers.first(where:{$0.id==id}) != provider {connectionStatus[id]="配置已更改，请重新测试连接。"}
    }
    do {
      let secret:ProviderSecret
      if provider.key.isEmpty {secret=try await secrets.read(reference:provider.credentialReference)}
      else {secret=try ProviderSecret(Data(provider.key.utf8))}
      let count=try await ModelDirectoryClient().test(baseURL:provider.baseURL.trimmingCharacters(in:.whitespacesAndNewlines).trimmingCharacters(in:CharacterSet(charactersIn:"/")),secret:secret)
      connectionStatus[id]=providers.first(where:{$0.id==id})==provider ? "已连接，目录返回 \(count) 个模型；没有自动添加模型。":"配置已更改，请重新测试连接。"
    } catch ModelDirectoryError.authentication {connectionStatus[id]="认证被拒绝，请检查 API Key。"}
    catch ModelDirectoryError.redirect {connectionStatus[id]="服务返回重定向；请填写最终 API 地址后重试。"}
    catch ModelDirectoryError.http(let status) {connectionStatus[id]="模型目录返回 HTTP \(status)，可手动配置模型。"}
    catch ModelDirectoryError.invalid {connectionStatus[id]="服务未返回兼容的模型目录，可手动配置模型。"}
    catch {connectionStatus[id]="连接未能验证。请检查地址、密钥和网络；当前草稿已保留。"}
  }
}
