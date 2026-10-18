import SwiftUI

struct NativeAssistantView:View {
  @ObservedObject var assistant:NativeAssistant
  @Environment(\.openSettings) private var openSettings
  var documentSaved:Bool?=nil
  var body:some View {
    GeometryReader {geometry in
    VStack(alignment:.leading,spacing:0) {
      HStack(spacing:12) {
        Text(assistant.conversationTitle).font(.system(size:12.5,weight:.medium)).lineLimit(1).truncationMode(.tail)
        Spacer(minLength:4)
        Menu("历史对话",systemImage:"clock.arrow.circlepath") {
          if assistant.conversations.isEmpty {Text("没有与当前文稿绑定的历史对话")}
          ForEach(assistant.conversations) {conversation in
            Button {Task {await assistant.selectConversation(conversation.id)}} label: {
              if conversation.id==assistant.conversationID {Label(conversation.title,systemImage:"checkmark")}
              else {Text(conversation.title)}
            }
          }
          Divider()
          Button("重新读取历史") {Task {await assistant.refreshHistory()}}
          Button("模型设置…") {assistant.showSettings=true}.accessibilityIdentifier("assistant-settings")
        }.labelStyle(.iconOnly).menuStyle(.borderlessButton).fixedSize()
          .disabled(assistant.busy || assistant.undoing || assistant.historyLoading || assistant.unknownActionUndo != nil || assistant.unknownRoundChange != nil).accessibilityIdentifier("assistant-history")
        Button("新对话",systemImage:"square.and.pencil") {assistant.newConversation()}.labelStyle(.iconOnly)
          .disabled(assistant.busy || assistant.undoing || assistant.historyLoading || assistant.unknownActionUndo != nil || assistant.unknownRoundChange != nil).accessibilityIdentifier("assistant-new-conversation")
      }.buttonStyle(.borderless).foregroundStyle(Theme.Color.ink2).padding(.horizontal,16).padding(.vertical,12)
      if let error=assistant.historyError {Text(error).font(.caption).foregroundStyle(.secondary).padding(.horizontal,16).padding(.bottom,8)}
      if assistant.messages.isEmpty {
        VStack(alignment:.leading,spacing:8) {
          Text("用实际内核核对数学，协助阅读和修改当前笔记本。").font(.callout).foregroundStyle(.secondary)
          if !assistant.configured {Button("添加模型",systemImage:"plus") {assistant.showSettings=true}.accessibilityIdentifier("assistant-add-model")}
        }.padding(.horizontal,16).padding(.vertical,12)
      }
      NativeAssistantTranscript(messages:assistant.messages,busy:assistant.busy,conversationID:assistant.conversationID,
        onInsertCode:assistant.busy ? nil:{code,language in Task {await assistant.insertCodeBlock(code,language:language)}},actionModel:assistant) {message in
        if let outcome=assistant.outcomes.first(where:{$0.id==message.taskID}),message.id==assistant.messages.last(where:{$0.taskID==message.taskID})?.id {
          NativeTurnOutcomeSummary(assistant:assistant,outcome:outcome,documentSaved:documentSaved)
        }
      }.frame(minHeight:100,maxHeight:.infinity)
      if let error=assistant.transcriptError {Text(error).font(.caption).foregroundStyle(.secondary).padding(.horizontal,16)}
      NativeAssistantComposer(assistant:assistant,panelHeight:geometry.size.height).padding(12)
    }}.task {await assistant.load()}.onChange(of:assistant.showSettings) {
      if assistant.showSettings {assistant.showSettings=false;UserDefaults.standard.set("models",forKey:"settings.section");openSettings()}
    }
  }
}
