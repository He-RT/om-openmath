import Foundation

struct AgentPreparedRequest:Sendable {let context:String,model:String}
struct AgentNativeModelPort:Sendable {
  let prepare:@Sendable (NativeAgentRequestIntent)async throws->AgentPreparedRequest
  let stream:@Sendable (NativeAgentRequestIntent)async throws->AsyncThrowingStream<AgentModelEventPayload,any Error>
}
