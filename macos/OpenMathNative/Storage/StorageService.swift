import Foundation

struct StoreOpenInfo:Sendable,Codable { let identity:StoreIdentity;let runtime:SQLiteRuntimeInfo }
/// The final transfer retains the root lease until queue-confined SQLite destruction finishes.
private final class FinalDatabaseOwner:@unchecked Sendable {
  let database:SQLiteDatabase
  let lease:RootLease
  init(_ database:SQLiteDatabase,lease:RootLease) { self.database=database;self.lease=lease }
}
/// Every connection belongs to exactly one serial queue. No database pointer crosses the API.
private final class StoreWriter:@unchecked Sendable {
  let queue:DispatchQueue
  private let lease:RootLease
  private var db:SQLiteDatabase?
  private var identity:StoreIdentity?
  private var source:DocumentStore?
  init(_ name:String,lease:RootLease) { self.lease=lease;queue=DispatchQueue(label:"org.openmath.storage."+name,qos:.utility) }
  nonisolated(nonsending) func perform<T:Sendable>(_ body:@escaping @Sendable (StoreWriter)throws->T) async throws ->T {
    try await withCheckedThrowingContinuation { continuation in
      queue.async {
        precondition(!Thread.isMainThread)
        do { continuation.resume(returning:try body(self)) }
        catch { continuation.resume(throwing:error) }
      }
    }
  }
  nonisolated(nonsending) func open(folder:URL,kind:String,document:String?,faults:StorageFaults) async throws ->StoreOpenInfo {
    try await perform { writer in
      dispatchPrecondition(condition:.onQueue(writer.queue))
      let opened=try StorageBootstrap.openStore(folder:folder,kind:kind,document:document,faults:faults)
      writer.db=opened.0;writer.identity=opened.1
      if kind=="document" { writer.source=try DocumentStore(db:opened.0,url:folder.appendingPathComponent("Generations").appendingPathComponent(String(format:"%06lld",opened.1.generation)).appendingPathComponent("authority.sqlite"),identity:opened.1) }
      return StoreOpenInfo(identity:opened.1,runtime:opened.2)
    }
  }
  nonisolated(nonsending) func header() async throws ->StoreIdentity {
    try await perform { writer in guard let db=writer.db else { throw StorageError.closing };return try StorageBootstrap.readHeader(db) }
  }
  nonisolated(nonsending) func sourceCall<T:Sendable>(_ work:@escaping @Sendable (DocumentStore)throws->T) async throws ->T {
    try await perform { writer in guard let source=writer.source else { throw StorageError.closing };return try work(source) }
  }
  nonisolated(nonsending) func close() async throws {
    try await perform { writer in
      if let db=writer.db {
        try db.transaction { try db.statement("UPDATE store_state SET last_clean_shutdown=1 WHERE singleton=1") }
        try db.checkpoint();try db.close();writer.db=nil;writer.source=nil
      }
    }
  }
  deinit {
    // A forgotten explicit close still releases SQLite off the UI thread; no clean-shutdown claim.
    if let db { let finalOwner=FinalDatabaseOwner(db,lease:lease);queue.async { try? finalOwner.database.close() } }
  }
}
/// Host-native physical storage. Rust owns document semantics and sends typed commit requests.
actor StorageService {
  private let paths:NativeStoragePaths
  private var lease:RootLease?
  private var library:StoreWriter?
  private var documents:[String:StoreWriter]=[:]
  private var closeTask:Task<Void,any Error>?
  private init(paths:NativeStoragePaths,lease:RootLease) { self.paths=paths;self.lease=lease }
  static func open(paths:NativeStoragePaths,faults:StorageFaults = .init()) async throws ->(StorageService,StoreOpenInfo) {
    let queue=DispatchQueue(label:"org.openmath.storage.bootstrap",qos:.utility)
    let lease:RootLease=try await withCheckedThrowingContinuation { continuation in
      queue.async {
        precondition(!Thread.isMainThread)
        do { continuation.resume(returning:try StorageBootstrap.root(paths,faults:faults)) }
        catch { continuation.resume(throwing:error) }
      }
    }
    let service=StorageService(paths:paths,lease:lease)
    let writer=StoreWriter("library",lease:lease)
    let info=try await writer.open(folder:paths.root.appendingPathComponent("Library",isDirectory:true),kind:"library",document:nil,faults:faults)
    await service.installLibrary(writer)
    return (service,info)
  }
  private func installLibrary(_ writer:StoreWriter) { library=writer }
  func readLibraryHeader() async throws ->StoreIdentity {
    guard let library,closeTask==nil else { throw StorageError.closing }
    return try await library.header()
  }
  func openDocument(_ id:String,faults:StorageFaults = .init()) async throws ->StoreOpenInfo {
    guard closeTask==nil,UUID(uuidString:id) != nil,id==id.lowercased() else { throw StorageError.unsafePath }
    guard documents[id]==nil else { throw StorageError.inUse }
    guard let lease else { throw StorageError.closing }
    let writer=StoreWriter("document."+id,lease:lease)
    documents[id]=writer // reserve before suspension; never create a second writer
    do { return try await writer.open(folder:paths.root.appendingPathComponent("Documents",isDirectory:true).appendingPathComponent(id,isDirectory:true),kind:"document",document:id,faults:faults) }
    catch { documents.removeValue(forKey:id);throw error }
  }
  func initializeSource(_ source:NativeSourceSnapshot) async throws ->NativeSourceSnapshot {
    guard closeTask==nil,let writer=documents[source.document_id] else { throw StorageError.closing }
    return try await writer.sourceCall { try $0.initialize(source) }
  }
  func committedSource(_ id:String) async throws ->NativeSourceSnapshot? {
    guard closeTask==nil,let writer=documents[id] else { throw StorageError.closing }
    return try await writer.sourceCall { try $0.readHead() }
  }
  func commitSource(_ plan:NativeSourceCommit,faults:StorageFaults = .init()) async throws ->NativeDurableSourceReceipt {
    guard closeTask==nil,let writer=documents[plan.commit.document_id] else { throw StorageError.closing }
    return try await writer.sourceCall { try $0.commit(plan,faults:faults) }
  }
  func sourceReceipt(document:String,operation:String) async throws ->NativeDurableSourceReceipt? {
    guard closeTask==nil,let writer=documents[document] else { throw StorageError.closing }
    return try await writer.sourceCall { try $0.query(operation) }
  }
  func close() async throws {
    if let closeTask { return try await closeTask.value }
    let task=Task { try await self.finishClosing() };closeTask=task
    try await task.value
  }
  private func finishClosing() async throws {
    for writer in documents.values { try await writer.close() }
    if let library { try await library.close() }
    documents.removeAll();library=nil;lease=nil
  }
}
