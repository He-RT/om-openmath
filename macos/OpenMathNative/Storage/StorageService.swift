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
  private var references:BlobReferences?
  private var kernel:KernelStore?
  private var saves:SaveStore?
  private var effects:LibraryStore?
  private var outbox:DocumentOutbox?
  private var drafts:DraftBackupStore?
  private var sessions:AgentSessionStore?
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
      let databaseURL=folder.appendingPathComponent("Generations").appendingPathComponent(String(format:"%06lld",opened.1.generation)).appendingPathComponent(kind=="document" ? "authority.sqlite" : "library.sqlite")
      writer.references=try BlobReferences(db:opened.0,url:databaseURL)
      if kind=="document",(3...5).contains(opened.1.storeVersion) {writer.kernel=try KernelStore(db:opened.0,url:databaseURL,identity:opened.1)}
      if kind=="document",(4...5).contains(opened.1.storeVersion) {writer.saves=try SaveStore(db:opened.0,url:databaseURL,identity:opened.1)}
      if kind=="document",opened.1.storeVersion==5 {writer.drafts=try DraftBackupStore(db:opened.0,url:databaseURL,identity:opened.1)}
      if kind=="library",(2...4).contains(opened.1.storeVersion) {writer.effects=try LibraryStore(db:opened.0,url:databaseURL,identity:opened.1)}
      if kind=="library",(3...4).contains(opened.1.storeVersion) {writer.sessions=try AgentSessionStore(db:opened.0,url:databaseURL,identity:opened.1)}
      if kind=="document",let source=writer.source {writer.outbox=DocumentOutbox(db:opened.0,url:databaseURL,identity:opened.1,source:source,kernel:writer.kernel,saves:writer.saves)}
      return StoreOpenInfo(identity:opened.1,runtime:opened.2)
    }
  }
  nonisolated(nonsending) func header() async throws ->StoreIdentity {
    try await perform { writer in guard let db=writer.db else { throw StorageError.closing };return try StorageBootstrap.readHeader(db) }
  }
  nonisolated(nonsending) func sourceCall<T:Sendable>(_ work:@escaping @Sendable (DocumentStore)throws->T) async throws ->T {
    try await perform { writer in guard let source=writer.source else { throw StorageError.closing };return try work(source) }
  }
  nonisolated(nonsending) func referenceCall<T:Sendable>(_ work:@escaping @Sendable (BlobReferences)throws->T) async throws ->T {
    try await perform { writer in guard let references=writer.references else {throw StorageError.closing};return try work(references) }
  }
  nonisolated(nonsending) func kernelCall<T:Sendable>(_ work:@escaping @Sendable (KernelStore)throws->T) async throws ->T {
    try await perform {writer in guard let kernel=writer.kernel else {throw StorageError.unsupportedVersion};return try work(kernel)}
  }
  nonisolated(nonsending) func saveCall<T:Sendable>(_ work:@escaping @Sendable (SaveStore)throws->T) async throws ->T {
    try await perform {writer in guard let saves=writer.saves else {throw StorageError.unsupportedVersion};return try work(saves)}
  }
  nonisolated(nonsending) func effectCall<T:Sendable>(_ work:@escaping @Sendable (LibraryStore)throws->T) async throws ->T {
    try await perform {writer in guard let effects=writer.effects else {throw StorageError.unsupportedVersion};return try work(effects)}
  }
  nonisolated(nonsending) func outboxCall<T:Sendable>(_ work:@escaping @Sendable (DocumentOutbox)throws->T) async throws ->T {
    try await perform {writer in guard let outbox=writer.outbox else {throw StorageError.closing};return try work(outbox)}
  }
  nonisolated(nonsending) func draftCall<T:Sendable>(_ work:@escaping @Sendable (DraftBackupStore)throws->T) async throws ->T {
    try await perform {writer in guard let drafts=writer.drafts else {throw StorageError.unsupportedVersion};return try work(drafts)}
  }
  nonisolated(nonsending) func sessionCall<T:Sendable>(_ work:@escaping @Sendable (AgentSessionStore)throws->T) async throws->T {
    try await perform {writer in guard let sessions=writer.sessions else {throw StorageError.unsupportedVersion};return try work(sessions)}
  }
  /// Failed Library open is not a clean shutdown or a license to replace the original DB.
  nonisolated(nonsending) func abandon() async {
    _ = try? await perform {writer in
      try writer.db?.close();writer.db=nil;writer.source=nil;writer.references=nil;writer.kernel=nil;writer.saves=nil;writer.effects=nil;writer.outbox=nil;writer.drafts=nil;writer.sessions=nil
    }
  }
  nonisolated(nonsending) func close() async throws {
    try await perform { writer in
      if let db=writer.db {
        try db.transaction { try db.statement("UPDATE store_state SET last_clean_shutdown=1 WHERE singleton=1") }
        try db.checkpoint();try db.close();writer.db=nil;writer.source=nil;writer.references=nil;writer.kernel=nil;writer.saves=nil;writer.effects=nil;writer.outbox=nil;writer.drafts=nil;writer.sessions=nil
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
  private var blobs:BlobStore?
  private var library:StoreWriter?
  private var libraryState:NativeLibraryAvailability = .unavailable(.closing)
  private var documents:[String:StoreWriter]=[:]
  private var closeTask:Task<Void,any Error>?
  private init(paths:NativeStoragePaths,lease:RootLease) { self.paths=paths;self.lease=lease;self.blobs=BlobStore(root:paths.root,lease:lease) }
  static func open(paths:NativeStoragePaths,faults:StorageFaults = .init()) async throws ->(StorageService,StoreOpenInfo) {
    let lease=try await rootLease(paths:paths,faults:faults)
    let service=StorageService(paths:paths,lease:lease)
    let writer=StoreWriter("library",lease:lease)
    let info=try await writer.open(folder:paths.root.appendingPathComponent("Library",isDirectory:true),kind:"library",document:nil,faults:faults)
    await service.installLibrary(writer,info:info)
    return (service,info)
  }
  private static func rootLease(paths:NativeStoragePaths,faults:StorageFaults) async throws->RootLease {
    let queue=DispatchQueue(label:"org.openmath.storage.bootstrap",qos:.utility)
    return try await withCheckedThrowingContinuation { continuation in
      queue.async {
        precondition(!Thread.isMainThread)
        do { continuation.resume(returning:try StorageBootstrap.root(paths,faults:faults)) }
        catch { continuation.resume(throwing:error) }
      }
    }
  }
  /// Native workspace can retain a healthy document service when Library alone is damaged.
  /// Root/channel/lock failures never take this fallback, and no empty replacement is created.
  static func openWorkspace(paths:NativeStoragePaths,faults:StorageFaults = .init()) async throws->StorageService {
    let lease=try await rootLease(paths:paths,faults:faults),service=StorageService(paths:paths,lease:lease)
    let writer=StoreWriter("library",lease:lease)
    do {
      let info=try await writer.open(folder:paths.root.appendingPathComponent("Library",isDirectory:true),kind:"library",document:nil,faults:faults)
      await service.installLibrary(writer,info:info)
    } catch {
      await writer.abandon()
      await service.unavailableLibrary((error as? StorageError) ?? .corruptIdentity)
    }
    return service
  }
  private func installLibrary(_ writer:StoreWriter,info:StoreOpenInfo) {library=writer;libraryState=info.identity.storeVersion==4 ? .ready(info) : .legacy(info)}
  private func unavailableLibrary(_ error:StorageError) {libraryState = .unavailable(error)}
  func libraryAvailability()->NativeLibraryAvailability {libraryState}
  func startupDocuments(after:String?=nil,limit:Int=32) async throws->StartupDocumentPage {
    guard closeTask==nil,let lease else {throw StorageError.closing}
    let root=paths.root
    return try await Task.detached { [lease] in _=lease;return try StartupDocuments.page(root:root,after:after,limit:limit)}.value
  }
  func documentIdentity(_ document:String) async throws->StoreIdentity {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.header()
  }
  func beginDraftOwner(document:String,owner:String,runtime:String,generation:UInt64,faults:StorageFaults = .init()) async throws->DraftBackupOwner {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.draftCall {try $0.beginOwner(owner,runtime:runtime,generation:generation,faults:faults)}
  }
  func backupDrafts(_ snapshot:DraftBackupSnapshot,faults:StorageFaults = .init()) async throws->DraftBackupReceipt {
    guard closeTask==nil,let writer=documents[snapshot.owner.documentID] else {throw StorageError.closing}
    return try await writer.draftCall {try $0.write(snapshot,faults:faults)}
  }
  func draftBackupReceipt(document:String,owner:String,sequence:UInt64) async throws->DraftBackupReceipt? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.draftCall {try $0.query(owner:owner,sequence:sequence)}
  }
  func draftBackup(document:String) async throws->StoredDraftBackup? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.draftCall {try $0.head()}
  }
  func documentOutbox(document:String,kind:DocumentEffectKind,after:Int64=0,limit:Int=16) async throws->DocumentOutboxPage {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.outboxCall {try $0.page(kind,after:after,limit:limit)}
  }
  /// Return only after Library durable readback, then mark the original document event.
  /// Repeating after a crash reuses its stable store/event identity, including projected=1.
  func projectDocumentEffect(_ effect:DocumentEffect,faults:StorageFaults = .init()) async throws->LibraryProjectionReceipt {
    guard closeTask==nil,let writer=documents[effect.documentID] else {throw StorageError.closing}
    guard let library else {
      if case .unavailable(let error)=libraryState {throw error};throw StorageError.unsupportedVersion
    }
    let actual=try await writer.outboxCall {try $0.verify(effect)}
    let receipt=try await library.effectCall {try $0.append(actual,faults:faults)}
    try faults.reach("library_projection_confirmed")
    try await writer.outboxCall {try $0.acknowledge(effect,receipt:receipt,faults:faults)}
    return receipt
  }
  func libraryEffects(after:Int64=0,limit:Int=16) async throws->[StoredDocumentEffect] {
    guard closeTask==nil,let library else {throw StorageError.closing}
    return try await library.effectCall {try $0.page(after:after,limit:limit)}
  }
  func createAgentSession(_ id:String,faults:StorageFaults = .init()) async throws->AgentSessionIdentity {
    guard closeTask==nil,let library else {throw StorageError.closing};return try await library.sessionCall {try $0.create(id,faults:faults)}
  }
  func appendSessionEvent(_ input:SessionAppendInput,faults:StorageFaults = .init()) async throws->SessionAppendReceipt {
    guard closeTask==nil,let library else {throw StorageError.closing};return try await library.sessionCall {try $0.append(input,faults:faults)}
  }
  func appendSessionResource(_ input:SessionAppendInput,publication:BlobPublication,owner:BlobOwner,faults:StorageFaults = .init()) async throws->SessionAppendReceipt {
    guard closeTask==nil,let library,let blobs,owner.kind == .context else {throw StorageError.closing}
    let transfer=try await blobs.retain(publication)
    do {
      let reader=try await blobs.openReader(transfer);try await blobs.closeReader(reader)
      let reference=BlobReference(owner:owner,descriptor:transfer.descriptor,mediaType:"application/octet-stream",codecVersion:"agent-wire-v1")
      let receipt=try await library.sessionCall {try $0.append(input,resource:reference,faults:faults)}
      try await blobs.release(transfer);return receipt
    } catch {try? await blobs.release(transfer);throw error}
  }
  func originalOperationEffect(_ target:SessionOperationTarget) async throws->DocumentEffect? {
    guard closeTask==nil,let writer=documents[target.documentID] else {throw StorageError.closing}
    return try await writer.outboxCall {try $0.operation(target)}
  }
  /// Reconciliation may inspect an unopened managed document, never create a guessed target.
  func reconcileOriginalOperationEffect(_ target:SessionOperationTarget) async throws->DocumentEffect? {
    try SessionValidation.target(target)
    if documents[target.documentID] != nil {
      let effect=try await originalOperationEffect(target)
      if let effect {_ = try await projectDocumentEffect(effect)}
      return effect
    }
    guard closeTask==nil,let lease else {throw StorageError.closing}
    let selected=paths.root.appendingPathComponent("Documents/\(target.documentID)/active.json")
    let exists=try await Task.detached { [lease] in _=lease;return try ManagedFiles.check(selected,directory:false)}.value
    guard exists else {throw StorageError.recoveryRequired}
    let opened=try await openDocument(target.documentID)
    do {
      guard opened.identity.storeID==target.storeID else {throw SessionLedgerError.wrongSourceEffect}
      let effect=try await originalOperationEffect(target)
      if let effect {_ = try await projectDocumentEffect(effect)}
      try await closeDocument(target.documentID);return effect
    } catch {try? await closeDocument(target.documentID);throw error}
  }
  func sessionEvent(_ session:String,event:String) async throws->StoredSessionEvent? {
    guard closeTask==nil,let library else {throw StorageError.closing};return try await library.sessionCall {try $0.query(session,event:event)}
  }
  func sessionEvents(_ session:String,after:UInt64=0,limit:Int=16) async throws->[StoredSessionEvent] {
    guard closeTask==nil,let library else {throw StorageError.closing};return try await library.sessionCall {try $0.page(session,after:after,limit:limit)}
  }
  func sessionRecovery(_ session:String,cursor:SessionRecoveryCursor?=nil,limit:Int=16) async throws->SessionRecoveryPage {
    guard closeTask==nil,let library else {throw StorageError.closing};return try await library.sessionCall {try $0.recover(session,cursor:cursor,limit:limit)}
  }
  func agentSessions(after:String?=nil,limit:Int=16) async throws->[SessionListRecord] {
    guard closeTask==nil,let library else {throw StorageError.closing};return try await library.sessionCall {try $0.list(after:after,limit:limit)}
  }
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
  func closeDocument(_ id:String) async throws {
    guard let writer=documents[id] else {return}
    try await writer.close()
    documents.removeValue(forKey:id)
  }
  func committedSource(_ id:String) async throws ->NativeSourceSnapshot? {
    guard closeTask==nil,let writer=documents[id] else { throw StorageError.closing }
    return try await writer.sourceCall { try $0.readHead() }
  }
  func admitSource(_ plan:NativeSourceCommit,runtime:String,faults:StorageFaults = .init()) async throws ->NativeSourceAdmission {
    guard closeTask==nil,let writer=documents[plan.commit.document_id] else {throw StorageError.closing}
    return try await writer.sourceCall {try $0.admit(plan,runtime:runtime,faults:faults)}
  }
  func sourceAdmission(document:String,operation:String) async throws ->NativeSourceAdmission? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.sourceCall {try $0.admission(operation)}
  }
  func settleSourceAdmission(document:String,operation:String,cancelled:Bool) async throws ->NativeSourceAdmission {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.sourceCall {try $0.settleAdmission(operation,cancelled:cancelled)}
  }
  func commitSource(_ plan:NativeSourceCommit,faults:StorageFaults = .init(),controls:SourceCommitControls?=nil) async throws ->NativeDurableSourceReceipt {
    guard closeTask==nil,let writer=documents[plan.commit.document_id] else { throw StorageError.closing }
    return try await writer.sourceCall { try $0.commit(plan,faults:faults,controls:controls) }
  }
  func calculationSettings(document:String) async throws ->CalculationReceipt? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.sourceCall {try $0.calculationSettings()}
  }
  func sourceReceipt(document:String,operation:String) async throws ->NativeDurableSourceReceipt? {
    guard closeTask==nil,let writer=documents[document] else { throw StorageError.closing }
    return try await writer.sourceCall { try $0.query(operation) }
  }
  func fileHead(document:String) async throws ->SavedFileHead? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing};return try await writer.saveCall {try $0.readHead()}
  }
  func bindFile(_ binding:FileBinding,opened:SaveSnapshot?=nil,faults:StorageFaults = .init()) async throws ->SavedFileHead {
    guard closeTask==nil,let writer=documents[binding.documentID] else {throw StorageError.closing};return try await writer.saveCall {try $0.choose(binding,opened:opened,faults:faults)}
  }
  func saveIntent(document:String,operation:String) async throws ->SaveIntent? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing};return try await writer.saveCall {try $0.readIntent(operation)}
  }
  func pendingSaveIntents(document:String) async throws->[SaveIntent] {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.saveCall {try $0.pendingIntents()}
  }
  func confirmSaveNotWritten(document:String,operation:String) async throws {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    try await writer.saveCall {try $0.confirmNotWritten(operation)}
  }
  func saveReceipt(document:String,operation:String) async throws ->SaveReceipt? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing};return try await writer.saveCall {try $0.query(operation)}
  }
  func completeSave(_ actual:SaveReceipt,faults:StorageFaults = .init()) async throws ->SaveReceipt {
    guard closeTask==nil,let writer=documents[actual.documentID] else {throw StorageError.closing};return try await writer.saveCall {try $0.completed(actual,faults:faults)}
  }
  func prepareSave(snapshot:SaveSnapshot,binding:FileBinding,operation:String,publication:BlobPublication,faults:StorageFaults = .init()) async throws ->SaveIntent {
    guard closeTask==nil,let blobs,let writer=documents[snapshot.source.document_id] else {throw StorageError.closing}
    let transfer=try await blobs.retain(publication)
    do {
      guard transfer.descriptor.hash==snapshot.fileHash,transfer.descriptor.byteLength==UInt64(snapshot.bytes.count) else {throw BlobError.corrupt}
      let actual=try await writer.saveCall {try $0.prepare(snapshot:snapshot,binding:binding,operation:operation,descriptor:transfer.descriptor,faults:faults)}
      try await blobs.release(transfer);return actual
    } catch {try? await blobs.release(transfer);throw error}
  }
  func kernelReceipt(document:String,operation:String) async throws ->NativeKernelReceipt? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.kernelCall {try $0.query(operation)}
  }
  func acceptedKernelHead(document:String) async throws ->NativeKernelReceipt? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.kernelCall {try $0.readHead()}
  }
  func kernelRecoveryHead(document:String) async throws->StoredKernelRecovery? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.kernelCall {try $0.recoveryHead()}
  }
  func confirmedNoKernelCommit(_ plan:NativeKernelCommit) async throws ->NativeKernelNoCommit {
    guard closeTask==nil,let writer=documents[plan.producer.document_id] else {throw StorageError.closing}
    return try await writer.kernelCall {try $0.confirmedNoCommit(plan)}
  }
  /// Verify real immutable publication, retain it, then reference/accept in the same document DB.
  func commitKernel(_ plan:NativeKernelCommit,publication:BlobPublication,controls:KernelCommitControls,faults:StorageFaults = .init()) async throws ->NativeKernelReceipt {
    guard closeTask==nil,let blobs,let writer=documents[plan.producer.document_id] else {throw StorageError.closing}
    try KernelValidation.plan(plan)
    let transfer=try await blobs.retain(publication)
    do {
      guard transfer.descriptor.hash==plan.checkpoint_blob_hash,transfer.descriptor.byteLength==plan.checkpoint_byte_length.value else {throw BlobError.corrupt}
      // openReader rehashes/checks actual original bytes before any database pointer can reference it.
      let reader=try await blobs.openReader(transfer);try await blobs.closeReader(reader)
      let receipt=try await writer.kernelCall {try $0.commit(plan,descriptor:transfer.descriptor,faults:faults,controls:controls)}
      try await blobs.release(transfer);return receipt
    } catch {try? await blobs.release(transfer);throw error}
  }
  func publishBlob(data:Data,expectedHash:String?=nil,faults:StorageFaults = .init()) async throws ->BlobPublication {
    guard closeTask==nil,let blobs else {throw StorageError.closing}
    return try await blobs.publish(data:data,expectedHash:expectedHash,faults:faults)
  }
  func sourceTransactions(document:String,ids:[String],faults:StorageFaults = .init()) async throws ->[NativeStoredSourceTransaction] {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.sourceCall {try $0.transactions(ids,faults:faults)}
  }
  func sourceUndoMetadata(document:String,operation:String) async throws ->NativeUndoGroup? {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.sourceCall {try $0.undoMetadata(operation)}
  }
  func sourceHistory(document:String) async throws ->[SourceHistoryEntry] {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.sourceCall {try $0.historyEntries()}
  }
  func pinSourceTransaction(document:String,transaction:String,kind:String,owner:String,enabled:Bool) async throws {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    try await writer.sourceCall {try $0.pinTransaction(transaction,kind:kind,owner:owner,enabled:enabled)}
  }
  func compactSourceHistory(document:String,faults:StorageFaults = .init()) async throws ->SourceHistoryPrune {
    guard closeTask==nil,let writer=documents[document] else {throw StorageError.closing}
    return try await writer.sourceCall {try $0.compactHistory(faults:faults)}
  }
  func publishBlob(file:URL,expectedHash:String?=nil,faults:StorageFaults = .init()) async throws ->BlobPublication {
    guard closeTask==nil,let blobs else {throw StorageError.closing}
    return try await blobs.publish(file:file,expectedHash:expectedHash,faults:faults)
  }
  func releaseBlob(_ publication:BlobPublication) async throws {
    guard closeTask==nil,let blobs else {throw StorageError.closing};try await blobs.release(publication)
  }
  func attachBlob(_ publication:BlobPublication,owner:BlobOwner,mediaType:String,codec:String,document:String?=nil,faults:StorageFaults = .init()) async throws ->BlobReference {
    guard closeTask==nil,let blobs,let writer=document.flatMap({documents[$0]}) ?? (document==nil ? library : nil) else {throw StorageError.closing}
    let transfer=try await blobs.retain(publication)
    do {
      // Incoming pin may be removed while IO is running. The transfer pin is private and independent.
      let reference=BlobReference(owner:owner,descriptor:transfer.descriptor,mediaType:mediaType,codecVersion:codec)
      let stored=try await writer.referenceCall {try $0.attach(reference,faults:faults)}
      try await blobs.release(transfer);return stored
    } catch {try? await blobs.release(transfer);throw error}
  }
  func blobReference(owner:BlobOwner,hash:String,document:String?=nil) async throws ->BlobReference? {
    guard closeTask==nil,let writer=document.flatMap({documents[$0]}) ?? (document==nil ? library : nil) else {throw StorageError.closing}
    return try await writer.referenceCall {try $0.reference(owner,hash:hash)}
  }
  func openBlob(_ publication:BlobPublication) async throws ->BlobReadLease {
    guard closeTask==nil,let blobs else {throw StorageError.closing};return try await blobs.openReader(publication)
  }
  func openReferencedBlob(owner:BlobOwner,hash:String,document:String?=nil) async throws ->BlobReadLease {
    guard closeTask==nil,let blobs else {throw StorageError.closing}
    guard let record=try await blobReference(owner:owner,hash:hash,document:document) else {throw BlobError.missing}
    return try await blobs.openReference(record.descriptor)
  }
  func readBlob(_ lease:BlobReadLease,offset:UInt64,count:Int) async throws ->Data {
    guard closeTask==nil,let blobs else {throw StorageError.closing};return try await blobs.read(lease,offset:offset,count:count)
  }
  func closeBlob(_ lease:BlobReadLease) async throws {
    guard closeTask==nil,let blobs else {throw StorageError.closing};try await blobs.closeReader(lease)
  }
  func inspectBlob(_ descriptor:BlobDescriptor) async throws ->BlobInspection {
    guard closeTask==nil,let blobs,let library else {throw StorageError.closing}
    var count=try await library.referenceCall {try $0.count(descriptor.hash)}
    let checked=documents
    for writer in checked.values {count+=try await writer.referenceCall {try $0.count(descriptor.hash)}}
    let root=paths.root
    let known=Set(checked.keys)
    let complete=try await Task.detached { () throws ->Bool in
      let folder=root.appendingPathComponent("Documents")
      guard try ManagedFiles.check(folder,directory:true) else {return true}
      let entries=try FileManager.default.contentsOfDirectory(atPath:folder.path)
      return entries.count<=10000 && Set(entries)==known
    }.value
    return try await blobs.inspect(descriptor,references:count,allStoresChecked:complete)
  }
  func close() async throws {
    if let closeTask { return try await closeTask.value }
    let task=Task { try await self.finishClosing() };closeTask=task
    try await task.value
  }
  private func finishClosing() async throws {
    blobs?.beginClose() // stop blob admission/copy immediately, independent of SQLite shutdown
    for writer in documents.values { try await writer.close() }
    if let library { try await library.close() }
    if let blobs {await blobs.close()};blobs=nil
    documents.removeAll();library=nil;lease=nil
  }
}
