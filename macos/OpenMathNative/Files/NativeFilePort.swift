import Foundation

/// Serial native physical file IO; NSFileCoordinator/file read/write/hash never execute on UI.
actor NativeFileService {
  private let queue=DispatchQueue(label:"org.openmath.native.document.files",qos:.utility)
  private let authorizations=DispatchQueue(label:"org.openmath.native.document.file-authorizations",qos:.utility)
  private var closing=false
  nonisolated(nonsending) private func background<T:Sendable>(_ body:@escaping @Sendable ()throws->T) async throws->T {
    try await withCheckedThrowingContinuation {continuation in queue.async {do {continuation.resume(returning:try body())} catch {continuation.resume(throwing:error)}}}
  }
  func open(_ url:URL,document:String) async throws->OpenedNotebook {guard !closing else {throw SaveError.closing};return try await background {try SaveOperations.open(url,document:document)}}
  func authorize(_ url:URL,document:String,revision:UInt64) async throws->FileBinding {
    guard !closing else {throw SaveError.closing}
    return try await withCheckedThrowingContinuation {continuation in authorizations.async {do {continuation.resume(returning:try SaveOperations.authorize(url,document:document,revision:revision))} catch {continuation.resume(throwing:error)}}}
  }
  func write(_ snapshot:SaveSnapshot,intent:SaveIntent,faults:StorageFaults) async throws->SaveReceipt {guard !closing else {throw SaveError.closing};return try await background {try SaveOperations.write(snapshot,intent:intent,faults:faults)}}
  func reconcile(_ intent:SaveIntent) async throws->SaveReceipt? {guard !closing else {throw SaveError.closing};return try await background {try SaveOperations.reconcile(intent)}}
  func close() async {closing=true;_ = try? await background { ()->Bool in true };await withCheckedContinuation {continuation in authorizations.async {continuation.resume()}}}
}
struct FileSaveState:Sendable {
  let head:SavedFileHead?
  let sourceRevision:UInt64
  let sourceSnapshotHash:String
  let hasDrafts:Bool
  let hasSavableWork:Bool
  let dirty:Bool
  let unknownOperation:String?
}
/// File adapter uses the original source authority and one physical document writer. Saving source
/// N never reads newer draft data from a mutable view; N+1 remains dirty after N's real receipt.
actor NativeFilePort {
  private let client:NativeHostClient
  private let storage:StorageService
  private let source:DocCommitPort
  private let drafts:DraftStore
  private let files:NativeFileService
  private let document:String
  private var active:Set<String>=[]
  private var preparing:Set<String>=[]
  private var closing=false
  private var unknown:String?
  init(client:NativeHostClient,storage:StorageService,source:DocCommitPort,drafts:DraftStore,files:NativeFileService=NativeFileService(),document:String) {
    self.client=client;self.storage=storage;self.source=source;self.drafts=drafts;self.files=files;self.document=document
  }
  func state() async throws->FileSaveState {
    let snapshot=await drafts.confirmed
    let editor=try await drafts.editorState()
    let pending=editor.targets.contains{$0.is_dirty||$0.is_composing}
    let head=try await storage.fileHead(document:document)
    let dirty=head?.savedRevision != snapshot.revision.value || head?.savedSnapshotHash != snapshot.snapshot_hash || pending
    let savable=head?.savedRevision != snapshot.revision.value || head?.savedSnapshotHash != snapshot.snapshot_hash || editor.targets.contains{!$0.is_composing && $0.is_dirty}
    return .init(head:head,sourceRevision:snapshot.revision.value,sourceSnapshotHash:snapshot.snapshot_hash,hasDrafts:pending,hasSavableWork:savable,dirty:dirty,unknownOperation:unknown)
  }
  /// User panel-selected binding. A new target becomes dirty independently from previous Save ACKs.
  func chooseTarget(_ url:URL,faults:StorageFaults = .init()) async throws->SavedFileHead {
    guard !closing,unknown==nil,preparing.count<32 else {throw SaveError.busy}
    let old=try await storage.fileHead(document:document),revision=(old?.binding.revision ?? 0)+1
    let binding=try await files.authorize(url,document:document,revision:revision)
    return try await storage.bindFile(binding,faults:faults)
  }
  /// Initial open is considered saved only when the actual source-only bytes match the installed
  /// revision0 source. This never restores outputs or automatically executes notebook cells.
  func adoptOpened(_ opened:OpenedNotebook) async throws->SavedFileHead {
    guard !closing,opened.binding.documentID==document,opened.binding.revision==1 else {throw SaveError.bindingChanged}
    let snapshot=await drafts.confirmed
    let same=try await Task.detached {try NotebookFileCodec.encode(opened.file)==NotebookFileCodec.encode(snapshot.file)}.value
    guard snapshot.revision.value==0,same,KernelHashes.data(opened.bytes)==opened.binding.expected?.byteHash else {throw SaveError.sourceChanged}
    let frozen=try await Task.detached {try SaveSnapshot(source:snapshot)}.value
    return try await storage.bindFile(opened.binding,opened:frozen)
  }
  private func freeze() async throws->SaveSnapshot {
    try await source.synchronizeDrafts(excludingMarked:true)
    let actual=try await client.sourceCommand(.source_read(.init(type:.source_read)))
    guard let snapshot=actual.snapshot.value,snapshot.document_id==document else {throw SaveError.sourceChanged}
    return try await Task.detached {try SaveSnapshot(source:snapshot)}.value
  }
  /// A frozen snapshot is explicit and inspectable by native document/export clients.
  func snapshot() async throws->SaveSnapshot {guard !closing else {throw SaveError.closing};return try await freeze()}
  /// Original ID reconciles a missing ACK; no new file replacement is attempted while unknown.
  func reconcile(_ operation:String,faults:StorageFaults = .init()) async throws->SaveReceipt? {
    guard !active.contains(operation),!preparing.contains(operation) else {throw SaveError.busy}
    if let actual=try await storage.saveReceipt(document:document,operation:operation) {if unknown==operation {unknown=nil};return actual}
    guard let intent=try await storage.saveIntent(document:document,operation:operation) else {throw SaveError.unknownOutcome}
    if let receipt=try await files.reconcile(intent) {
      let completed=try await storage.completeSave(receipt,faults:faults);if unknown==operation {unknown=nil};return completed
    }
    if unknown==operation {unknown=nil};return nil
  }
  func save(operation:String="save-"+UUID().uuidString.lowercased(),snapshot frozen:SaveSnapshot?=nil,faults:StorageFaults = .init()) async throws->SaveReceipt {
    guard !closing else {throw SaveError.closing}
    guard unknown==nil,!active.contains(operation),preparing.isEmpty,active.isEmpty else {throw SaveError.busy}
    if let actual=try await storage.saveReceipt(document:document,operation:operation) {return actual}
    if try await storage.saveIntent(document:document,operation:operation) != nil {throw SaveError.unknownOutcome}
    preparing.insert(operation);defer {preparing.remove(operation)}
    guard let head=try await storage.fileHead(document:document) else {throw SaveError.permissionDenied}
    let snapshot:SaveSnapshot
    if let frozen {guard frozen.source.document_id==document else {throw SaveError.sourceChanged};snapshot=frozen} else {snapshot=try await freeze()}
    guard !closing else {throw SaveError.closing}
    let publication=try await storage.publishBlob(data:snapshot.bytes,expectedHash:snapshot.fileHash,faults:faults)
    var intent:SaveIntent
    do {intent=try await storage.prepareSave(snapshot:snapshot,binding:head.binding,operation:operation,publication:publication,faults:faults)}
    catch {
      let original=error;try? await storage.releaseBlob(publication)
      let persisted:SaveIntent?
      do {persisted=try await storage.saveIntent(document:document,operation:operation)}
      catch {unknown=operation;throw SaveError.unknownOutcome}
      if persisted==nil {throw original}
      unknown=operation;throw SaveError.unknownOutcome
    }
    preparing.remove(operation);active.insert(operation);defer {active.remove(operation)}
    do {
      let actual=try await files.write(snapshot,intent:intent,faults:faults)
      let receipt=try await storage.completeSave(actual,faults:faults)
      try await storage.releaseBlob(publication);return receipt
    } catch {
      let original=error
      if let known=original as? SaveError,known != .unknownOutcome {
        // SaveOperations wraps every uncertain post-replacement IO failure as unknown. A known
        // pre-publication rejection/conflict never becomes a fake save or an unnecessary retry.
        try? await storage.releaseBlob(publication);throw known
      }
      // The IO writer has settled. Reconcile only the original save/target; external changes and
      // missing permission are explicit, and no unknown outcome is replayed under another ID.
      unknown=operation
      do {
        if let actual=try await storage.saveReceipt(document:document,operation:operation) {unknown=nil;try? await storage.releaseBlob(publication);return actual}
        if let actual=try await files.reconcile(intent) {
          let receipt=try await storage.completeSave(actual);unknown=nil;try? await storage.releaseBlob(publication);return receipt
        }
        unknown=nil;try? await storage.releaseBlob(publication);throw original
      } catch SaveError.externalConflict {try? await storage.releaseBlob(publication);unknown=nil;throw SaveError.externalConflict}
      catch {try? await storage.releaseBlob(publication);throw unknown==nil ? original : SaveError.unknownOutcome}
    }
  }
  /// Closing settles original physical operations; no future autosave or discarded input is written.
  func close() async {closing=true;while !active.isEmpty || !preparing.isEmpty {try? await Task.sleep(for:.milliseconds(5))};await files.close()}
}
