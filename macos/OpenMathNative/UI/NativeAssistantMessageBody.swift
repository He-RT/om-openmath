import SwiftUI

/// Message identity and ordered parts stay the same during streaming and durable confirmation.
struct NativeAssistantMessageBody:View {
  let message:AssistantMessage
  var streaming=false
  var onInsertCode:((String,String)->Void)?=nil
  var maximumUserWidth:CGFloat=320
  var actionModel:NativeAssistant?=nil
  @State private var actionsExpanded:Bool?
  private var expanded:Bool {AgentActionGroup.expanded(message.actions,streaming:streaming,preference:actionsExpanded)}
  var body:some View {
    Group {
      if message.role=="user" {
        HStack(alignment:.top,spacing:0) {
          Spacer(minLength:0)
          VStack(alignment:.trailing,spacing:6) {
            if let attachments=message.context?.attachments,!attachments.isEmpty {
              ScrollView(.horizontal) {HStack(spacing:6) {ForEach(attachments) {item in AssistantAttachmentCard(attachment:item)}}}.scrollIndicators(.hidden).frame(height:48)
            }
            if !message.text.isEmpty || message.context?.references?.isEmpty==false {
              parts.padding(.horizontal,12).padding(.vertical,10)
                .background(Theme.Color.well,in:RoundedRectangle(cornerRadius:Theme.Radius.bubble))
            }
          }.frame(maxWidth:maximumUserWidth,alignment:.trailing)
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
      if message.role=="user",let references=message.context?.references,!references.isEmpty {
        Text(references.map{"［"+$0.label+"］"}.joined(separator:" ")).font(.system(size:11.5)).foregroundStyle(Theme.Color.ink2)
      }
      ForEach(Array(message.parts.enumerated()),id:\.offset) {index,part in
        switch part.value {
        case .text(let text):
          if message.role=="assistant" {NativeMarkdownView(source:text,fontSize:13.5,onInsertCode:onInsertCode)}
          else {Text(text).font(Theme.Font.conversation).textSelection(.enabled)}
        case .actions(let actions):
          if index==message.parts.firstIndex(where:{if case .actions=$0.value {return true};return false}) {
            Button {actionsExpanded = !expanded} label: {
              HStack(spacing:6) {Image(systemName:expanded ? "chevron.down":"chevron.right");Text(AgentActionGroup.summary(message.actions)).multilineTextAlignment(.leading)}
            }.font(.system(size:11.5)).buttonStyle(.plain).foregroundStyle(Theme.Color.ink2)
              .disabled(streaming || message.actions.contains{$0.status == .running}).accessibilityIdentifier("agent-actions-toggle-"+message.id)
          }
          let visible=AgentActionGroup.visible(actions,expanded:expanded)
          if !visible.isEmpty {AgentActionList(actions:visible,model:actionModel,active:streaming)}
        }
      }
    }
  }
}
