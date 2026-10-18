import Foundation

struct RegisteredModel:Identifiable,Codable,Equatable,Sendable {
  let id:String
  var apiName:String,displayName:String,tools:Bool,images:Bool,contextLimit:Int?
}
struct RegisteredProvider:Identifiable,Codable,Equatable,Sendable {
  let id:String
  var name:String,baseURL:String,credentialReference:String,models:[RegisteredModel]
}
struct ModelRegistryDocument:Codable,Equatable,Sendable {
  var codecVersion=1,providers:[RegisteredProvider]=[],defaultModelID:String?
  static func current(_ configuration:AssistantConfiguration)->Self {
    let model=RegisteredModel(id:"current-model",apiName:configuration.model,displayName:configuration.model,tools:true,images:false,contextLimit:nil)
    return .init(providers:[.init(id:"current-provider",name:URLComponents(string:configuration.baseURL)?.host ?? "已配置服务",baseURL:configuration.baseURL,credentialReference:configuration.credentialReference,models:[model])],defaultModelID:model.id)
  }
  func selection(_ id:String?)->(RegisteredProvider,RegisteredModel)? {
    for provider in providers {if let model=provider.models.first(where:{$0.id==id}) {return (provider,model)}};return nil
  }
}
/// This application's current native configuration only; credentials never enter this JSON.
actor ModelRegistry {
  let file:URL
  private let currentApplication:Bool
  init(file:URL?=nil)throws {self.file=try file ?? NativeStoragePaths.system(.preview).root.appendingPathComponent("models.json");currentApplication=file==nil}
  func loadInitial()async throws->ModelRegistryDocument {
    if FileManager.default.fileExists(atPath:file.path) || !currentApplication {return try load()}
    let current=try AssistantSettingsStore(),seed=try await current.load()
    return try load(seed:seed)
  }
  func load(seed:AssistantConfiguration?=nil)throws->ModelRegistryDocument {
    if FileManager.default.fileExists(atPath:file.path) {
      let bytes=try Data(contentsOf:file);guard bytes.count<=512*1024 else {throw AssistantSetupError.invalid}
      let document=try JSONDecoder().decode(ModelRegistryDocument.self,from:bytes);try Self.validate(document);return document
    }
    guard let seed else {return .init()}
    let document=ModelRegistryDocument.current(seed);try save(document);return document
  }
  func save(_ document:ModelRegistryDocument)throws {
    try Self.validate(document)
    try FileManager.default.createDirectory(at:file.deletingLastPathComponent(),withIntermediateDirectories:true,attributes:[.posixPermissions:0o700])
    let encoder=JSONEncoder();encoder.outputFormatting=[.sortedKeys,.prettyPrinted]
    let bytes=try encoder.encode(document);guard bytes.count<=512*1024 else {throw AssistantSetupError.invalid}
    try bytes.write(to:file,options:.atomic)
    try FileManager.default.setAttributes([.posixPermissions:0o600],ofItemAtPath:file.path)
  }
  struct Commit:Sendable {let document:ModelRegistryDocument,retired:Set<String>}
  func commit(_ proposed:ModelRegistryDocument,keys:[String:Data])throws->Commit {
    let previous=try load();var candidate=proposed,inserted:[String]=[]
    guard Set(keys.keys).isSubset(of:Set(candidate.providers.map(\.id))) else {throw AssistantSetupError.invalid}
    for index in candidate.providers.indices where keys[candidate.providers[index].id] != nil {
      _ = try ProviderSecret(keys[candidate.providers[index].id]!)
      candidate.providers[index].credentialReference="provider-"+UUID().uuidString.lowercased()
    }
    try Self.validate(candidate)
    do {
      for provider in candidate.providers where keys[provider.id] != nil {
        try SecretStore.insert(reference:provider.credentialReference,bytes:keys[provider.id]!);inserted.append(provider.credentialReference)
      }
      try save(candidate)
    } catch {
      // Only discard fresh credentials after confirming the registry kept its previous bytes.
      // An uncertain atomic-file ACK must not leave a published registry pointing at a removed key.
      if let actual=try? load(),actual==previous {for reference in inserted {SecretStore.remove(reference:reference)}}
      throw error
    }
    let retained=Set(candidate.providers.map(\.credentialReference))
    // The current single-model seed still owns its assistant-* account. Only retire
    // accounts created by this registry, so an older frozen owner keeps its credential.
    let retired=Set(previous.providers.map(\.credentialReference)).subtracting(retained).filter{$0.hasPrefix("provider-")}
    return .init(document:candidate,retired:retired)
  }
  static func validate(_ document:ModelRegistryDocument)throws {
    let models=document.providers.flatMap(\.models)
    guard document.codecVersion==1,document.providers.count<=64,models.count<=512,
      Set(document.providers.map(\.id)).count==document.providers.count,Set(models.map(\.id)).count==models.count,
      document.defaultModelID==nil || models.contains(where:{$0.id==document.defaultModelID}) else {throw AssistantSetupError.invalid}
    for provider in document.providers {
      guard SourceValidation.identity(provider.id),!provider.name.isEmpty,provider.name.utf8.count<=256,
        provider.baseURL.utf8.count<=4096,SourceValidation.identity(provider.credentialReference) else {throw AssistantSetupError.invalid}
      try AssistantSettingsStore.validate(provider.baseURL,model:"configured-model")
      for model in provider.models {
        try AssistantSettingsStore.validate(provider.baseURL,model:model.apiName)
        guard SourceValidation.identity(model.id),!model.displayName.isEmpty,model.displayName.utf8.count<=256,
          model.contextLimit.map({$0>0 && $0<=100_000_000}) ?? true else {throw AssistantSetupError.invalid}
      }
    }
  }
}
