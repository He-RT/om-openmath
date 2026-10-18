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
  init(file:URL?=nil)throws {self.file=try file ?? NativeStoragePaths.system(.preview).root.appendingPathComponent("models.json")}
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
    try encoder.encode(document).write(to:file,options:.atomic)
    try FileManager.default.setAttributes([.posixPermissions:0o600],ofItemAtPath:file.path)
  }
  static func validate(_ document:ModelRegistryDocument)throws {
    let models=document.providers.flatMap(\.models)
    guard document.codecVersion==1,document.providers.count<=64,models.count<=512,
      Set(document.providers.map(\.id)).count==document.providers.count,Set(models.map(\.id)).count==models.count,
      document.defaultModelID==nil || models.contains(where:{$0.id==document.defaultModelID}) else {throw AssistantSetupError.invalid}
    for provider in document.providers {
      guard SourceValidation.identity(provider.id),!provider.name.isEmpty,provider.name.utf8.count<=256,
        !provider.credentialReference.isEmpty,provider.credentialReference.utf8.count<=256 else {throw AssistantSetupError.invalid}
      for model in provider.models {
        try AssistantSettingsStore.validate(provider.baseURL,model:model.apiName)
        guard SourceValidation.identity(model.id),!model.displayName.isEmpty,model.displayName.utf8.count<=256,
          model.contextLimit.map({$0>0 && $0<=100_000_000}) ?? true else {throw AssistantSetupError.invalid}
      }
    }
  }
}
