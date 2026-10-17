import SwiftUI

struct NativeAssistantTranscript<Footer:View>:View {
  let messages:[AssistantMessage],busy:Bool,conversationID:String
  var onInsertCode:((String,String)->Void)?=nil
  var actionModel:NativeAssistant?=nil
  @ViewBuilder let footer:(AssistantMessage)->Footer
  @State private var scroll=AssistantTranscriptScrollState()
  @State private var userScrolling=false
  var body:some View {
    GeometryReader {geometry in
      ScrollViewReader {reader in
        ScrollView {
          LazyVStack(alignment:.leading,spacing:20) {
            ForEach(messages) {message in
              NativeAssistantMessageBody(message:message,streaming:busy && message.id==messages.last?.id,
                onInsertCode:onInsertCode,maximumUserWidth:max(1,(geometry.size.width-32)*0.88),actionModel:actionModel)
                .id(message.id)
              footer(message)
            }
            Color.clear.frame(height:1).id("assistant-transcript-tail")
          }.padding(.horizontal,16).padding(.vertical,16)
        }.accessibilityIdentifier("assistant-transcript")
          .defaultScrollAnchor(.bottom,for:.initialOffset)
          .onScrollPhaseChange {_,phase in userScrolling = phase == .interacting || phase == .decelerating || phase == .tracking}
          .onScrollGeometryChange(for:AssistantTranscriptGeometry.self) {value in
            .init(offset:value.contentOffset.y,height:value.contentSize.height,viewport:value.containerSize)
          } action: {old,new in
            scroll.observe(old,new,userScrolling:userScrolling)
            if scroll.following,!userScrolling,old.height != new.height {showTail(reader)}
          }
          .onChange(of:messages.last) {
            scroll.contentChanged()
            if scroll.following {showTail(reader)}
          }
          .onChange(of:conversationID) {scroll.showLatest();showTail(reader)}
          .overlay(alignment:.bottom) {
            if scroll.unseen {
              Button("新消息",systemImage:"chevron.down") {scroll.showLatest();showTail(reader)}
                .buttonStyle(.bordered).buttonBorderShape(.capsule).padding(.bottom,8)
                .accessibilityIdentifier("assistant-new-messages")
            }
          }
      }
    }
  }
  private func showTail(_ reader:ScrollViewProxy) {reader.scrollTo("assistant-transcript-tail",anchor:.bottom)}
}
