import Foundation
private struct Fixture:Decodable {
  let initial:NativeSourceSnapshot
  let plan:NativeSourceCommit
  let preview:NativePreviewData
  let prepared_only:Bool
}
@main struct PreviewFixtures {
  static func main() async throws {
    let fixture=try JSONDecoder().decode(Fixture.self,from:Data(contentsOf:URL(fileURLWithPath:CommandLine.arguments[1])))
    let root=URL(fileURLWithPath:CommandLine.arguments[2],isDirectory:true)
    let output=URL(fileURLWithPath:CommandLine.arguments[3])
    precondition(fixture.preview.valid && fixture.preview.preview_ref.value != nil)
    let assigned=fixture.preview.assigned_cell_ids["new-result"]!
    precondition(fixture.plan.after.file.cells[1].id==assigned)
    let (store,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    _ = try await store.openDocument(fixture.initial.document_id)
    _ = try await store.initializeSource(fixture.initial)
    let result=try await store.commitSource(fixture.plan)
    let duplicate=try await store.commitSource(fixture.plan)
    precondition(result.receipt.operation_id==duplicate.receipt.operation_id && result.receipt.transaction_id.value==duplicate.receipt.transaction_id.value)
    let committed=try await store.committedSource(fixture.initial.document_id)!
    precondition(committed.file.cells.count==2 && committed.file.cells[0].source=="let a=5" && committed.file.cells[1].source=="a+1")
    precondition(committed.file.cells[1].id==assigned && committed.snapshot_hash==fixture.plan.after.snapshot_hash)
    try JSONEncoder().encode(result).write(to:output)
    try await store.close()
    print("Actual Rust frozen preview -> Swift SQLite commit/readback: original plan and host cell IDs retained, duplicate submission one effect, no reparse of a later draft")
  }
}
