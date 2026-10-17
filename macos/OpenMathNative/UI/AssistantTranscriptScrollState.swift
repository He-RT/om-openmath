import Foundation
import CoreGraphics

struct AssistantTranscriptGeometry:Equatable {
  let offset:CGFloat,height:CGFloat,viewport:CGSize
  var nearBottom:Bool {offset+viewport.height>=height-24}
}
/// User intent survives content growth and asynchronous Markdown layout.
struct AssistantTranscriptScrollState {
  var following=true,unseen=false
  mutating func contentChanged() {if !following {unseen=true}}
  mutating func observe(_ old:AssistantTranscriptGeometry,_ new:AssistantTranscriptGeometry,userScrolling:Bool) {
    if userScrolling || (old.viewport==new.viewport && new.offset<old.offset-1 && new.height==old.height) {
      following=new.nearBottom
      if following {unseen=false}
    }
  }
  mutating func showLatest() {following=true;unseen=false}
}
