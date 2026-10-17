import SwiftUI

/// Message identity and ordered parts stay the same during streaming and durable confirmation.
struct NativeAssistantMessageBody:View {
  let message:AssistantMessage
  var streaming=false
  var onInsertCode:((String,String)->Void)?=nil
  var maximumUserWidth:CGFloat=320
  var body:some View {
    Group {
      if message.role=="user" {
        HStack(alignment:.top,spacing:0) {
          Spacer(minLength:0)
          parts.padding(.horizontal,12).padding(.vertical,10)
            .background(Theme.Color.well,in:RoundedRectangle(cornerRadius:Theme.Radius.bubble))
            .frame(maxWidth:maximumUserWidth,alignment:.trailing)
        }
      } else {
        VStack(alignment:.leading,spacing:8) {
          HStack(alignment:.firstTextBaseline,spacing:7) {
            Text("助手").foregroundStyle(Theme.Color.agent).fontWeight(.medium)
            Text(message.context?.model ?? "模型信息未记录").foregroundStyle(Theme.Color.ink2)
            Text(message.context.map{$0.mode=="execute" ? "执行":"讨论"} ?? "").foregroundStyle(Theme.Color.ink2)
          }.font(.system(size:11.5)).accessibilityIdentifier("assistant-message-signature-"+message.id)
          parts
        }.frame(maxWidth:.infinity,alignment:.leading)
      }
    }.accessibilityElement(children:.contain).accessibilityIdentifier("assistant-message-"+message.id)
  }
  private var parts:some View {
    VStack(alignment:.leading,spacing:8) {
      ForEach(Array(message.parts.enumerated()),id:\.offset) {_,part in
        switch part.value {
        case .text(let text):
          if message.role=="assistant" {NativeMarkdownView(source:text,fontSize:13.5,onInsertCode:onInsertCode)}
          else {Text(text).font(Theme.Font.conversation).textSelection(.enabled)}
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
