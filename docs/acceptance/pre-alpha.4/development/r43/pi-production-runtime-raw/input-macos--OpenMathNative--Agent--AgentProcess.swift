import Foundation
import CryptoKit
import Security

enum AgentProcessError:Error,Sendable {case unavailable,invalidManifest,architecture,component,signature}
private struct HelperFile:Decodable {let path:String,sha256:String,bytes:UInt64}
private struct HelperManifest:Decodable {
  let schema_version:Int,kind:String,architecture:String,node_version:String,core_version:String,ai_version:String,adapter_version:String,protocol_version:Int,schema_hash:String,node:String,entry:String,production_packages:Int,system_node_fallback:Bool,node_release_signer:String,files:[HelperFile]
}
struct AgentLaunchSpec:Sendable {let executable:URL,entry:URL,root:URL,schemaHash:String}
/// Bundled absolute entry only. There is no PATH/node/npm discovery or arbitrary plugin argument.
struct AgentProcess:Sendable {
  let spec:AgentLaunchSpec
  static func bundled(_ bundle:Bundle = .main) async throws->AgentProcess {
    guard let package=bundle.url(forResource:"AgentRuntime",withExtension:nil) else {throw AgentProcessError.unavailable}
    return try await verified(package:package,signatureRoot:bundle.bundleURL)
  }
  /// Explicit unsigned package root is used only by standalone development fixtures.
  static func verified(package:URL,signatureRoot:URL?=nil) async throws->AgentProcess {
    try await Task.detached {
      let root=package.standardizedFileURL.resolvingSymlinksInPath()
      let manifestURL=root.appendingPathComponent("HelperManifest.json"),bytes=try Data(contentsOf:manifestURL)
      guard bytes.count<=2*1024*1024 else {throw AgentProcessError.invalidManifest}
      let manifest=try JSONDecoder().decode(HelperManifest.self,from:bytes)
      guard manifest.schema_version==1,manifest.kind=="helper_production_closure",manifest.node_version=="26.11.1",manifest.core_version=="1.0.4",manifest.ai_version=="1.0.4",manifest.adapter_version=="openmath-adapter-1",manifest.protocol_version==1,!manifest.system_node_fallback,manifest.production_packages==86,manifest.node_release_signer=="5BE8A3F6C8A5C01D106C0AD820B1A390B168D356",manifest.node=="Node/node",manifest.entry=="Agent/main.mjs",manifest.files.count>0,manifest.files.count<=20000 else {throw AgentProcessError.invalidManifest}
      guard manifest.architecture=="arm64" else {throw AgentProcessError.architecture}
      var seen=Set<String>(),total:UInt64=0
      for item in manifest.files {
        guard !item.path.hasPrefix("/"),!item.path.split(separator:"/",omittingEmptySubsequences:false).contains(where:{$0==".."||$0=="."||$0.isEmpty}),seen.insert(item.path).inserted,KernelValidation.hash(item.sha256),item.bytes<=256*1024*1024 else {throw AgentProcessError.component}
        let file=root.appendingPathComponent(item.path),resolved=file.resolvingSymlinksInPath()
        guard resolved.path.hasPrefix(root.path+"/"),resolved==file,try file.resourceValues(forKeys:[.isRegularFileKey]).isRegularFile==true else {throw AgentProcessError.component}
        let actual=try file.resourceValues(forKeys:[.fileSizeKey]).fileSize
        guard actual.map(UInt64.init)==item.bytes else {throw AgentProcessError.component}
        let handle=try FileHandle(forReadingFrom:file);defer {try? handle.close()}
        var hash=SHA256()
        while let chunk=try handle.read(upToCount:128*1024),!chunk.isEmpty {hash.update(data:chunk)}
        guard hash.finalize().map({String(format:"%02x",$0)}).joined()==item.sha256 else {throw AgentProcessError.component}
        total+=item.bytes;guard total<=512*1024*1024 else {throw AgentProcessError.component}
      }
      let enumerator=FileManager.default.enumerator(at:root,includingPropertiesForKeys:[.isRegularFileKey,.isSymbolicLinkKey])
      var actualPaths=Set<String>()
      while let file=enumerator?.nextObject() as? URL {
        let attributes=try file.resourceValues(forKeys:[.isRegularFileKey,.isSymbolicLinkKey])
        guard attributes.isSymbolicLink != true else {throw AgentProcessError.component}
        if attributes.isRegularFile==true {actualPaths.insert(String(file.path.dropFirst(root.path.count+1)))}
      }
      guard actualPaths==seen.union(["HelperManifest.json"]),manifest.files.first(where:{$0.path=="Agent/docs/design/native-agent.schema.json"})?.sha256==manifest.schema_hash else {throw AgentProcessError.component}
      guard seen.contains(manifest.node),seen.contains(manifest.entry),seen.contains("Agent/docs/design/native-agent.schema.json"),seen.contains("Agent/docs/design/agent-tools.schema.json") else {throw AgentProcessError.component}
      let executable=root.appendingPathComponent(manifest.node),entry=root.appendingPathComponent(manifest.entry)
      let node=try FileHandle(forReadingFrom:executable);let header=try node.read(upToCount:8);try node.close()
      guard header==Data([0xcf,0xfa,0xed,0xfe,0x0c,0,0,1]) else {throw AgentProcessError.architecture}
      guard FileManager.default.isExecutableFile(atPath:executable.path) else {throw AgentProcessError.component}
      try checkSignature(executable)
      if let signatureRoot {try checkSignature(signatureRoot)}
      return AgentProcess(spec:.init(executable:executable,entry:entry,root:root,schemaHash:manifest.schema_hash))
    }.value
  }
  private static func checkSignature(_ url:URL)throws {
    var code:SecStaticCode?
    guard SecStaticCodeCreateWithPath(url as CFURL,SecCSFlags(rawValue:0),&code)==errSecSuccess,let code,
      SecStaticCodeCheckValidity(code,SecCSFlags(rawValue:kSecCSStrictValidate|kSecCSCheckAllArchitectures|kSecCSCheckNestedCode),nil)==errSecSuccess else {throw AgentProcessError.signature}
  }
  func launch() async throws->AgentPrivatePipe {
    let directory=FileManager.default.temporaryDirectory.appendingPathComponent("OpenMath-Agent-"+UUID().uuidString,isDirectory:true)
    try FileManager.default.createDirectory(at:directory,withIntermediateDirectories:false,attributes:[.posixPermissions:0o700])
    do {return try await AgentPrivatePipe.launch(executable:spec.executable,arguments:["--permission","--allow-fs-read="+spec.root.path,"--no-addons",spec.entry.path],directory:directory,environment:["PATH":""],removeDirectoryOnClose:true)}
    catch {try? FileManager.default.removeItem(at:directory);throw error}
  }
}
