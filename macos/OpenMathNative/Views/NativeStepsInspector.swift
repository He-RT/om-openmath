import SwiftUI

struct NativeStepsInspector:View {
  @ObservedObject var presenter:ResultPresenter
  let ordinal:Int
  var body:some View {
    VStack(alignment:.leading,spacing:12) {
      Text("单元格 \(ordinal) 的推导步骤").font(.system(size:12)).foregroundStyle(.secondary)
      if let input=presenter.originalCellSource {
        ScrollView(.horizontal) {Text(input).font(.system(size:12,design:.monospaced)).textSelection(.enabled).fixedSize(horizontal:true,vertical:false)}
      } else if let error=presenter.message {Text(error).font(.caption).foregroundStyle(.secondary)}
      NativeStepsView(model:presenter.stepsModel())
    }.task {if presenter.originalCellSource==nil {await presenter.readOriginalInput(cellSource:true)}}
  }
}
