import SwiftUI

struct NativeAssistantView:View {
  @ObservedObject var assistant:NativeAssistant
  var body:some View {
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
          .disabled(assistant.busy || assistant.undoing || assistant.historyLoading).accessibilityIdentifier("assistant-history")
        Button("新对话",systemImage:"square.and.pencil") {assistant.newConversation()}.labelStyle(.iconOnly)
          .disabled(assistant.busy || assistant.undoing || assistant.historyLoading).accessibilityIdentifier("assistant-new-conversation")
      }.buttonStyle(.borderless).foregroundStyle(Theme.Color.ink2).padding(.horizontal,16).padding(.vertical,12)
      if let error=assistant.historyError {Text(error).font(.caption).foregroundStyle(.secondary).padding(.horizontal,16).padding(.bottom,8)}
      if assistant.messages.isEmpty {
        VStack(alignment:.leading,spacing:8) {
          Text("用实际内核核对数学，协助阅读和修改当前笔记本。").font(.callout).foregroundStyle(.secondary)
          if !assistant.configured {Button("添加模型",systemImage:"plus") {assistant.showSettings=true}.accessibilityIdentifier("assistant-add-model")}
        }.padding(.horizontal,16).padding(.vertical,12)
      }
      NativeAssistantTranscript(messages:assistant.messages,busy:assistant.busy,conversationID:assistant.conversationID,
        onInsertCode:assistant.busy ? nil:{code,language in Task {await assistant.insertCodeBlock(code,language:language)}}) {message in
        if let outcome=assistant.outcomes.first(where:{$0.id==message.taskID}),message.id==assistant.messages.last(where:{$0.taskID==message.taskID})?.id {
          NativeTurnOutcomeSummary(assistant:assistant,outcome:outcome)
        }
      }.frame(minHeight:100,maxHeight:.infinity)
      if let error=assistant.transcriptError {Text(error).font(.caption).foregroundStyle(.secondary).padding(.horizontal,16)}
      composer.padding(12)
    }.task {await assistant.load()}.sheet(isPresented:$assistant.showSettings) {settings}
  }
  private var composer:some View {
    VStack(alignment:.leading,spacing:10) {
      Divider()
      Toggle("允许修改并运行笔记本",isOn:$assistant.allowsChanges).font(.caption).disabled(assistant.busy).accessibilityIdentifier("assistant-allow-changes")
      TextEditor(text:$assistant.draft).font(.body).frame(minHeight:70,maxHeight:110).scrollContentBackground(.hidden)
        .padding(5).background(.background,in:RoundedRectangle(cornerRadius:6)).overlay(RoundedRectangle(cornerRadius:6).stroke(.quaternary))
        .overlay(alignment:.topLeading) {if assistant.draft.isEmpty {Text("询问当前笔记本…").foregroundStyle(.tertiary).padding(10).allowsHitTesting(false)}}
        .accessibilityLabel("给助手的问题").accessibilityIdentifier("assistant-input").disabled(assistant.busy)
      HStack {
        Text(assistant.status).font(.caption).foregroundStyle(.secondary).lineLimit(3).accessibilityIdentifier("assistant-status")
        Spacer()
        if assistant.busy {Button("停止",systemImage:"stop.fill") {Task {await assistant.stop()}}.disabled(assistant.stopping).accessibilityIdentifier("assistant-stop")}
        else {Button("发送",systemImage:"arrow.up") {assistant.send()}.disabled(assistant.undoing || assistant.historyLoading || assistant.draft.trimmingCharacters(in:.whitespacesAndNewlines).isEmpty).accessibilityIdentifier("assistant-send").keyboardShortcut(.return,modifiers:[.command,.shift])}
      }
    }
  }
  private var settings:some View {
    VStack(alignment:.leading,spacing:16) {
      Text("模型设置").font(.title2)
      Text("连接兼容 Chat Completions 的服务；本机回环地址支持 HTTP，其他地址使用 HTTPS。密钥保存在系统钥匙串中。").font(.callout).foregroundStyle(.secondary)
      Form {
        TextField("API 基础地址",text:$assistant.baseURL,prompt:Text("https://example.com/v1")).accessibilityIdentifier("assistant-base-url")
        TextField("模型名称",text:$assistant.model).accessibilityIdentifier("assistant-model")
        SecureField(assistant.configured ? "API Key（留空保留现有密钥）":"API Key",text:$assistant.apiKey).accessibilityIdentifier("assistant-api-key")
      }
      Text(assistant.status).font(.caption).foregroundStyle(.secondary)
      HStack {Spacer();Button("取消") {assistant.apiKey="";assistant.showSettings=false}.keyboardShortcut(.cancelAction);Button("保存") {Task {await assistant.saveSettings()}}.keyboardShortcut(.defaultAction).accessibilityIdentifier("assistant-save-settings")}
    }.padding(24).frame(width:500)
  }
}
