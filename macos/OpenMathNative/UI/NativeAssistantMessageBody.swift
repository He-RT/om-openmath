import SwiftUI

/// The current transcript body; D1/D2 refine its styling without changing the ordered data.
struct NativeAssistantMessageBody:View {
  let message:AssistantMessage
  var streaming=false
  var body:some View {
    VStack(alignment:.leading,spacing:4) {
      Text(message.role=="user" ? "你":"助手").font(.caption).foregroundStyle(.secondary)
      ForEach(message.parts) {part in
        switch part.value {
        case .text(let text):
          if message.role=="assistant",!streaming {NativeMarkdownView(source:text,fontSize:14)}
          else {Text(text).textSelection(.enabled).frame(maxWidth:.infinity,alignment:.leading)}
        case .actions(let actions):
          ForEach(AgentActionGroup.rows(actions)) {group in
            if group.actions.count>1 {
              DisclosureGroup(group.summary) {ForEach(group.actions) {action in actionRow(action)}}.font(.caption)
            } else if let action=group.actions.first {actionRow(action)}
          }
        }
      }
    }
  }
  private func actionRow(_ action:AgentAction)->some View {
    VStack(alignment:.leading,spacing:3) {
      HStack(alignment:.firstTextBaseline,spacing:6) {
        Text(action.verb).fontWeight(.medium)
        Text(action.object).foregroundStyle(.secondary)
        Spacer(minLength:0)
        let state=switch action.status {case .running:"进行中";case .ok:"已返回";case .failed:"失败";case .unknown:"待核对";case .cancelled:"已取消"}
        Text(state).foregroundStyle(action.status == .failed ? Color.red:Color.secondary)
      }
      if let error=action.error {Text(error.code+"："+error.message+(error.recovery.isEmpty ? "":"；"+error.recovery)).foregroundStyle(action.status == .failed ? Color.red:Color.secondary)}
    }.font(.caption).textSelection(.enabled).accessibilityIdentifier("agent-action-"+action.callID)
  }
}
