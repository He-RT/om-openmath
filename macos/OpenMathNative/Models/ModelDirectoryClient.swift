import Foundation

enum ModelDirectoryError:Error {case authentication,redirect,http(Int),invalid,limit}
private final class ModelDirectoryRedirects:NSObject,URLSessionTaskDelegate {
  func urlSession(_ session:URLSession,task:URLSessionTask,willPerformHTTPRedirection response:HTTPURLResponse,newRequest request:URLRequest,completionHandler:@escaping @Sendable(URLRequest?)->Void) {completionHandler(nil)}
}
actor ModelDirectoryClient {
  /// A single user-requested GET, with no redirect, retry, model inference or persisted response.
  func test(baseURL:String,secret:ProviderSecret)async throws->Int {
    try AssistantSettingsStore.validate(baseURL,model:"directory")
    var parts=URLComponents(string:baseURL)!
    if parts.scheme=="http",parts.host?.lowercased()=="localhost" {parts.host="127.0.0.1"}
    let configuration=URLSessionConfiguration.ephemeral
    configuration.httpShouldSetCookies=false;configuration.urlCache=nil;configuration.timeoutIntervalForRequest=15;configuration.timeoutIntervalForResource=20
    let redirects=ModelDirectoryRedirects(),session=URLSession(configuration:configuration,delegate:redirects,delegateQueue:nil)
    defer {session.invalidateAndCancel()}
    var request=URLRequest(url:parts.url!.appendingPathComponent("models"));request.httpMethod="GET"
    request.setValue("application/json",forHTTPHeaderField:"Accept")
    for (name,value) in secret.authenticationHeader(.bearer) {request.setValue(value,forHTTPHeaderField:name)}
    let (stream,response)=try await session.bytes(for:request)
    guard let response=response as? HTTPURLResponse else {throw ModelDirectoryError.invalid}
    switch response.statusCode {
    case 200..<300:break
    case 301...399:throw ModelDirectoryError.redirect
    case 401,403:throw ModelDirectoryError.authentication
    default:throw ModelDirectoryError.http(response.statusCode)
    }
    var data=Data()
    for try await byte in stream {guard data.count<512*1024 else {throw ModelDirectoryError.limit};data.append(byte)}
    guard let root=try JSONSerialization.jsonObject(with:data) as? [String:Any],let models=root["data"] as? [[String:Any]],models.count<=10000,models.allSatisfy({$0["id"] is String}) else {throw ModelDirectoryError.invalid}
    return models.count
  }
}
