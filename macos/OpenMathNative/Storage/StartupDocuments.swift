import Foundation
import SQLite3

struct StartupDocumentRecord:Identifiable,Sendable {
  let id:String
  let title:String
  let revision:UInt64?
  let hasDrafts:Bool
  let fileDirty:Bool
  let recoverable:Bool
  let warning:String?
}
struct StartupDocumentPage:Sendable {let records:[StartupDocumentRecord];let next:String?}
enum StartupDocuments {
  /// Only active selectors in this already locked managed root; no user-file scan, source
  /// evaluation, file bookmark resolution, implicit generation selection or database writer.
  static func page(root:URL,after:String?,limit:Int)throws->StartupDocumentPage {
    guard limit>0,limit<=32 else {throw StorageError.corruptIdentity}
    let folder=root.appendingPathComponent("Documents",isDirectory:true)
    guard try ManagedFiles.check(folder,directory:true) else {return .init(records:[],next:nil)}
    let names=try FileManager.default.contentsOfDirectory(atPath:folder.path)
    guard names.count<=10000 else {throw StorageError.recoveryRequired}
    let ids=names.filter{UUID(uuidString:$0) != nil && $0==$0.lowercased()}.sorted().filter{after==nil || $0>after!}
    var records:[StartupDocumentRecord]=[]
    for id in ids.prefix(limit) {
      do {
        let path=folder.appendingPathComponent(id,isDirectory:true)
        guard try ManagedFiles.check(path,directory:true) else {throw StorageError.recoveryRequired}
        let selector=try ManagedFiles.readJSON(StoreSelector.self,path.appendingPathComponent("active.json"),keys:StorageBootstrap.selectorKeys)
        guard selector.formatVersion==1,selector.generation>0,selector.generation<=999999 else {throw StorageError.unsupportedVersion}
        let url=path.appendingPathComponent("Generations/\(String(format:"%06lld",selector.generation))/authority.sqlite")
        let db=try SQLiteDatabase(url:url,readonly:true);defer {try? db.close()}
        let header=try StorageBootstrap.readHeader(db)
        guard header.kind=="document",header.documentID==id,header.storeID==selector.storeID,header.generation==selector.generation,try header.hash==selector.headerHash else {throw StorageError.corruptIdentity}
        var snapshot:NativeSourceSnapshot?
        try db.statement("SELECT r.snapshot FROM document_head h JOIN document_revisions r ON h.document_id=r.document_id AND h.revision=r.revision AND h.snapshot_hash=r.snapshot_hash") {row in
          guard snapshot==nil else {throw StorageError.corruptIdentity}
          snapshot=try JSONDecoder().decode(NativeSourceSnapshot.self,from:SQLColumn.data(row,0))
        }
        guard let snapshot,snapshot.document_id==id else {throw StorageError.recoveryRequired};try SourceValidation.snapshot(snapshot)
        var hasDrafts=false
        if header.storeVersion==5,let backup=try DraftBackupStore(db:db,url:url,identity:header).head() {
          hasDrafts=backup.snapshot.cells.contains{cell in
            cell.wasComposing || cell.requiresReview || cell.conflict || !cell.sourceUTF8.elementsEqual(cell.baseUTF8)
          }
        }
        var dirty=true
        if header.storeVersion>=4 {
          try db.statement("SELECT saved_revision,saved_snapshot_hash FROM file_head WHERE document_id=?",[.text(id)]) {row in
            if sqlite3_column_type(row,0) != SQLITE_NULL {let savedHash=try SQLColumn.text(row,1);dirty=sqlite3_column_int64(row,0) != Int64(snapshot.revision.value) || savedHash != snapshot.snapshot_hash}
          }
        }
        records.append(.init(id:id,title:snapshot.file.title,revision:snapshot.revision.value,hasDrafts:hasDrafts,fileDirty:dirty,recoverable:header.storeVersion==5,warning:header.storeVersion==5 ? nil : "此原生数据格式需要升级，原件已保留。"))
      } catch {records.append(.init(id:id,title:"需要恢复的笔记本",revision:nil,hasDrafts:false,fileDirty:true,recoverable:false,warning:"本地记录无法确认，原件已保留；可打开用户文件或选择备份。"))}
    }
    return .init(records:records,next:ids.count>limit ? records.last?.id : nil)
  }
}
