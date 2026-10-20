import SwiftUI

struct CellDiagnostics:View {
  let session:NativeDocumentSession,cell:NativeSourceCell
  @ObservedObject var workspace:NativeFileWorkspace
  @ObservedObject private var runtime:NotebookRuntime
  init(session:NativeDocumentSession,cell:NativeSourceCell,workspace:NativeFileWorkspace) {
    self.session=session;self.cell=cell;self.workspace=workspace
    _runtime=ObservedObject(wrappedValue:session.runtime)
  }
  private var source:String {session.drafts.overlay(cell.id)?.source ?? cell.source}
  private var diagnostics:[NotebookSourceDiagnostic] {
    guard runtime.sourceProjection?.source.file.cells.first(where:{$0.id==cell.id})?.source.utf8.elementsEqual(source.utf8)==true else {return []}
    return runtime.sourceDiagnostics[Data(cell.id.utf8)]?.filter{$0.severity=="Error"} ?? []
  }
  var body:some View {
    if !diagnostics.isEmpty {
      VStack(alignment:.leading,spacing:8) {
        ForEach(Array(diagnostics.enumerated()),id:\.offset) {_,diagnostic in
          HStack(alignment:.firstTextBaseline,spacing:8) {
            Image(systemName:Theme.Symbol.error).foregroundStyle(Theme.Color.red)
            if let map=try? SourceIndexMap(source:source,draftSequence:0),let position=try? map.position(forUTF8:Int(diagnostic.span.start)) {
              Text("\(position.line):\(position.column)").font(.system(size:12,design:.monospaced))
            }
            Text(diagnostic.message).font(.callout).textSelection(.enabled)
            Spacer(minLength:4)
            Button("定位") {session.locateDiagnostic(cell.id,source:source,start:diagnostic.span.start,end:diagnostic.span.end)}
              .buttonStyle(.borderless).font(.system(size:12))
          }
        }
      }.padding(12).frame(maxWidth:.infinity,alignment:.leading)
        .background(Theme.Color.red.opacity(0.07),in:RoundedRectangle(cornerRadius:6))
        .overlay(RoundedRectangle(cornerRadius:6).stroke(Theme.Color.red.opacity(0.35)))
        .accessibilityIdentifier("notebook-diagnostics-"+cell.id)
    }
  }
}
