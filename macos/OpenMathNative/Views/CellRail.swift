import SwiftUI

struct CellRail:View {
  let state:CellPresentationState
  @Environment(\.accessibilityReduceMotion) private var systemReducedMotion
  private var reduceMotion:Bool {systemReducedMotion || WorkbenchMotion.fixtureReduced}
  var body:some View {
    Group {
      if state.status == .running && !reduceMotion {
        TimelineView(.animation(minimumInterval:1.0/30.0)) {context in
          line(phase:CGFloat(context.date.timeIntervalSinceReferenceDate.truncatingRemainder(dividingBy:0.7)/0.7)*12)
        }
      } else {line(phase:0)}
    }.overlay(alignment:.top) {
      if state.byAgent {RoundedRectangle(cornerRadius:1.5).fill(Theme.Color.agent).frame(width:3,height:18)}
    }.frame(width:3).accessibilityHidden(true)
  }
  private func line(phase:CGFloat)->some View {
    GeometryReader {geometry in
      Path {path in path.move(to:CGPoint(x:1.5,y:0));path.addLine(to:CGPoint(x:1.5,y:geometry.size.height))}
        .stroke(state.color,style:StrokeStyle(lineWidth:state.status == .pending ? 1:3,lineCap:.round,dash:state.dashed ? [5,4]:[],dashPhase:phase))
    }
  }
}
