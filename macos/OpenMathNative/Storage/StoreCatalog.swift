import Foundation
import SQLite3

struct CatalogStore:Sendable {
  let folder:URL,database:URL,identity:StoreIdentity
  let counts:[String:Int64],references:[BlobDescriptor]
}
/// Called under the installation lease and maintenance barrier. A missing selector, unknown
/// format, incomplete reference table or invalid FK stops the entire scan, never means empty.
enum StoreCatalog {
  static func stores(_ root:URL,retained:Bool=false)throws->[CatalogStore] {
    var folders:[(URL,String,String?)]=[(root.appendingPathComponent("Library"),"library",nil)]
    let documents=root.appendingPathComponent("Documents")
    if try ManagedFiles.check(documents,directory:true) {
      let names=try FileManager.default.contentsOfDirectory(atPath:documents.path)
      guard names.count<=10000 else {throw BlobError.budgetExceeded}
      for name in names.sorted() {
        guard UUID(uuidString:name) != nil,name==name.lowercased() else {throw StorageError.corruptIdentity}
        folders.append((documents.appendingPathComponent(name),"document",name))
      }
    }
    var result:[CatalogStore]=[]
    for (folder,kind,document) in folders {
      let selector=try ManagedFiles.readJSON(StoreSelector.self,folder.appendingPathComponent("active.json"),keys:StorageBootstrap.selectorKeys)
      guard selector.formatVersion==1,selector.generation>0,selector.generation<=999999 else {throw StorageError.unsupportedVersion}
      let generations=folder.appendingPathComponent("Generations")
      guard try ManagedFiles.check(generations,directory:true) else {throw StorageError.recoveryRequired}
      let selected=String(format:"%06lld",selector.generation)
      let names=retained ? try FileManager.default.contentsOfDirectory(atPath:generations.path) : [selected]
      guard names.count<=10000,names.contains(selected) else {throw StorageError.recoveryRequired}
      for name in names.sorted() {
        // Interrupted unpublished migration directories are retained recovery resources.
        // Do not guess their references or collect original bytes while one is unresolved.
        guard name.count==6,name.utf8.allSatisfy({$0>=48 && $0<=57}),let generation=Int64(name),generation>0 else {throw StorageError.recoveryRequired}
        let database=generations.appendingPathComponent(name).appendingPathComponent(kind=="library" ? "library.sqlite" : "authority.sqlite")
        let db=try SQLiteDatabase(url:database,readonly:true);defer {try? db.close()}
        let identity=try StorageBootstrap.readHeader(db)
        guard identity.kind==kind,identity.documentID==document,identity.generation==generation,identity.storeID==selector.storeID else {throw StorageError.corruptIdentity}
        if name==selected {guard try identity.hash==selector.headerHash else {throw StorageError.corruptIdentity}}
        try validate(db,identity:identity,url:database)
        result.append(.init(folder:folder,database:database,identity:identity,counts:try counts(db),references:try references(db)))
      }
    }
    guard Set(result.filter{$0.identity.kind=="library"}.map{$0.identity.storeID}).count==1 else {throw StorageError.corruptIdentity}
    return result
  }
  static func counts(_ db:SQLiteDatabase)throws->[String:Int64] {
    var tables:[String]=[]
    try db.statement("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name") {tables.append(try SQLColumn.text($0,0))}
    guard tables.count<=128 else {throw StorageError.corruptIdentity}
    var counts:[String:Int64]=[:]
    for table in tables {
      guard table.utf8.allSatisfy({($0>=97 && $0<=122)||$0==95}) else {throw StorageError.corruptIdentity}
      counts[table]=try db.integer("SELECT count(*) FROM \(table)")
    };return counts
  }
  static func validate(_ db:SQLiteDatabase,identity:StoreIdentity,url:URL)throws {
    var broken=false;try db.statement("PRAGMA foreign_key_check") {_ in broken=true}
    guard !broken else {throw StorageError.corruptIdentity}
    if identity.kind=="library",identity.storeVersion>=2 {_ = try LibraryStore(db:db,url:url,identity:identity)}
    if identity.kind=="library",identity.storeVersion>=3 {try AgentSessionStore(db:db,url:url,identity:identity).validateAll()}
    if identity.kind=="document",identity.storeVersion==5 {_ = try DraftBackupStore(db:db,url:url,identity:identity)}
    // Schema supported by the selected version must exist; a partial schema is corruption.
    let tables=Set(try counts(db).keys)
    var required:Set<String>=["store_header","store_state","blob_objects","blob_refs"]
    if identity.kind=="document" {
      required.formUnion(DocumentSchema.tables)
      required.formUnion(["source_admissions","source_admission_transitions"])
      if identity.storeVersion>=2 {required.formUnion(["source_history","source_history_pins"])}
      if identity.storeVersion>=3 {required.formUnion(KernelSchema.tables)}
      if identity.storeVersion>=4 {required.formUnion(["file_bindings","file_head","save_intents","save_receipts","save_outbox"])}
    }
    guard required.isSubset(of:tables) else {throw StorageError.recoveryRequired}
    if identity.kind=="document" {
      try db.statement("SELECT document_id,revision,snapshot_hash,snapshot FROM document_revisions") {row in
        let snapshot=try JSONDecoder().decode(NativeSourceSnapshot.self,from:SQLColumn.data(row,3));try SourceValidation.snapshot(snapshot)
        guard snapshot.document_id==identity.documentID,snapshot.document_id==(try SQLColumn.text(row,0)),Int64(snapshot.revision.value)==sqlite3_column_int64(row,1),snapshot.snapshot_hash==(try SQLColumn.text(row,2)) else {throw StorageError.corruptIdentity}
      }
      if identity.storeVersion>=3 {guard try db.integer("SELECT count(*) FROM accepted_checkpoints c WHERE NOT EXISTS(SELECT 1 FROM blob_refs r WHERE r.owner_kind='checkpoint' AND r.owner_id=c.checkpoint_id AND r.blob_hash=c.blob_hash)")==0 else {throw StorageError.corruptIdentity}}
      if identity.storeVersion>=4 {guard try db.integer("SELECT count(*) FROM save_intents s WHERE NOT EXISTS(SELECT 1 FROM blob_refs r WHERE r.owner_kind='export' AND r.owner_id=s.operation_id AND r.blob_hash=s.file_hash)")==0 else {throw StorageError.corruptIdentity}}
    }
    for (table,payload,hash) in [("document_effect_events","record","record_hash"),("draft_owners","record","record_hash"),("draft_batches","receipt","receipt_hash"),("file_bindings","record","record_hash"),("save_intents","record","record_hash"),("save_receipts","record","record_hash"),("outbox","payload","payload_hash"),("kernel_outbox","payload","payload_hash"),("save_outbox","payload","payload_hash"),("kernel_operations","receipt","receipt_hash"),("source_history","summary","summary_hash")] where tables.contains(table) {
      try db.statement("SELECT \(payload),\(hash) FROM \(table)") {row in guard KernelHashes.data(try SQLColumn.data(row,0))==(try SQLColumn.text(row,1)) else {throw StorageError.corruptIdentity}}
    }
  }
  static func references(_ db:SQLiteDatabase)throws->[BlobDescriptor] {
    guard try db.integer("SELECT count(*) FROM sqlite_schema WHERE type='table' AND name IN ('blob_objects','blob_refs')")==2 else {throw StorageError.recoveryRequired}
    var descriptors:[BlobDescriptor]=[]
    try db.statement("SELECT DISTINCT o.blob_hash,o.byte_length FROM blob_refs r JOIN blob_objects o ON r.blob_hash=o.blob_hash ORDER BY o.blob_hash") {row in
      let hash=try SQLColumn.text(row,0),size=sqlite3_column_int64(row,1)
      guard KernelValidation.hash(hash),size>=0,size<=128*1024*1024 else {throw StorageError.corruptIdentity}
      descriptors.append(.init(hash:hash,byteLength:UInt64(size)))
      guard descriptors.count<=100000 else {throw BlobError.budgetExceeded}
    };return descriptors
  }
  static func union(_ stores:[CatalogStore])throws->[BlobDescriptor] {
    var found:[String:BlobDescriptor]=[:]
    for store in stores {for descriptor in store.references {
      if let old=found[descriptor.hash],old != descriptor {throw StorageError.corruptIdentity}
      found[descriptor.hash]=descriptor
    }};return found.values.sorted{$0.hash<$1.hash}
  }
}
