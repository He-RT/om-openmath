import AppKit
import Foundation

/// One app root lease shared by scratch-independent document sessions. No old Tauri profile is
/// consulted; Preview owns its isolated channel. Initialization is reserved before suspension.
actor NativeAppStorage {
  private let paths:NativeStoragePaths?
  private var starting:Task<StorageService,any Error>?
  private var closed=false
  init(paths:NativeStoragePaths?=nil) {self.paths=paths}
  func open() async throws->StorageService {
    guard !closed else {throw StorageError.closing}
    if let starting {return try await starting.value}
    let chosen=try paths ?? NativeStoragePaths.system(.preview)
    let task=Task {try await StorageService.openWorkspace(paths:chosen)}
    starting=task
    do {return try await task.value} catch {starting=nil;throw error}
  }
  func close() async throws {closed=true;if let store=try? await starting?.value {try await store.close()};starting=nil}
}

/// Actual opened source, native overlays and file presentation. Construction performs no CAS.
/// Closing settles file/source work before releasing the original host and document writer.
@MainActor final class NativeDocumentSession {
  let document:NativeDocument
  let host:NativeHostClient
  let source:DocCommitPort
  let files:NativeFilePort
  let drafts:DraftStore
  let undo:UndoCoordinator
  let backup:NativeDraftBackup
  private(set) var recoveryMessages:[String]=[]
  private(set) var recoveryHistoryCursor:DocumentProjectionCursor?
  private let storage:StorageService
  private var editors:[Data:DraftTextViewAdapter]=[:]
  private var closed=false
  private init(document:NativeDocument,host:NativeHostClient,source:DocCommitPort,files:NativeFilePort,drafts:DraftStore,storage:StorageService,backupOwner:DraftBackupOwner) {
    self.document=document;self.host=host;self.source=source;self.files=files;self.drafts=drafts;self.storage=storage
    self.undo=UndoCoordinator {transactions,group,operation in try await source.undo(transactions:transactions,group:group,operation:operation)}
    self.backup=NativeDraftBackup(storage:storage,store:drafts,owner:backupOwner)
    document.beforeSave={ [weak self] in try await self?.backup.flush();try await self?.flushSource() }
    document.beforeClose={ [weak self] in try await self?.backup.flush() }
    document.draftsBackedUp={ [weak self] in self?.backup.isCurrent==true }
  }
  static func open(storage:StorageService,url:URL?=nil) async throws->NativeDocumentSession {
    let id=UUID().uuidString.lowercased(),physical=NativeFileService()
    let opened:OpenedNotebook?
    do {opened=try await url.map {target in Task {try await physical.open(target,document:id)}}?.value}
    catch {await physical.close();throw error}
    let file=opened?.file ?? NativeSourceFile(version:1,title:"未命名",cells:[.init(id:UUID().uuidString.lowercased(),kind:.math,source:"",dialect:.modern)])
    let info=try await storage.openDocument(id)
    var host:NativeHostClient?
    do {
      let snapshot=try await Task.detached {
        var value=NativeSourceSnapshot(codec_version:1,document_id:id,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:file,cell_revisions:try file.cells.map{.init(cell_id:$0.id,revision:try HostSerial(0))},snapshot_hash:"")
        value.snapshot_hash=SourceHashes.snapshot(value);return value
      }.value
      let initial=try await storage.initializeSource(snapshot)
      let client=try await NativeHostClient.open();host=client
      let reply=try await client.sourceCommand(.source_open(.init(type:.source_open,snapshot:initial,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
      let clock=SourceOwnerClock();clock.calibrate(reply.owner_time_ms.value)
      let drafts=DraftStore(source:initial,runtime:client.runtimeInstanceID,generation:reply.document_generation.value,clock:{clock.now()})
      let backupOwner=try await storage.beginDraftOwner(document:id,owner:UUID().uuidString.lowercased(),runtime:client.runtimeInstanceID,generation:reply.document_generation.value)
      let source=DocCommitPort(client:client,storage:storage,drafts:drafts,clock:clock)
      let files=NativeFilePort(client:client,storage:storage,source:source,drafts:drafts,files:physical,document:id)
      if let opened {_ = try await files.adoptOpened(opened)}
      let native=NativeDocument();native.attach(files,opened:opened)
      return NativeDocumentSession(document:native,host:client,source:source,files:files,drafts:drafts,storage:storage,backupOwner:backupOwner)
    } catch {
      if let host {try? await host.close()};await physical.close();try? await storage.closeDocument(id);throw error
    }
  }
  func editor(_ cell:String)throws->DraftTextViewAdapter {
    guard !closed else {throw SaveError.closing}
    let key=Data(cell.utf8)
    if let existing=editors[key] {return existing}
    let editor=try DraftTextViewAdapter(store:drafts,cell:cell,generation:1)
    editor.bindDocumentUndo(undo.manager);editor.didEdit={ [weak document] in document?.noteEdit() }
    editors[key]=editor;return editor
  }
  /// Explicit native recovery selection. The local document identity is retained, runtime/grants
  /// are new, and no cell execution or model request is started by construction.
  static func recover(storage:StorageService,documentID:String) async throws->NativeDocumentSession {
    let info=try await storage.openDocument(documentID),physical=NativeFileService()
    var host:NativeHostClient?
    do {
      guard info.identity.storeVersion==5,let snapshot=try await storage.committedSource(documentID) else {throw StorageError.unsupportedVersion}
      let storedDraft=try await storage.draftBackup(document:documentID),settings=try await storage.calculationSettings(document:documentID)
      let client=try await NativeHostClient.open();host=client
      let opened=try await client.sourceCommand(.source_open(.init(type:.source_open,snapshot:snapshot,store_id:info.identity.storeID,calculation:.init(settings?.settings),config_revision:try HostSerial(settings?.revision ?? 0))))
      let clock=SourceOwnerClock();clock.calibrate(opened.owner_time_ms.value)
      let drafts=DraftStore(source:snapshot,runtime:client.runtimeInstanceID,generation:opened.document_generation.value,clock:{clock.now()})
      if let storedDraft {try drafts.restoreBackup(storedDraft)}
      let source=DocCommitPort(client:client,storage:storage,drafts:drafts,clock:clock),files=NativeFilePort(client:client,storage:storage,source:source,drafts:drafts,files:physical,document:documentID)
      let owner=try await storage.beginDraftOwner(document:documentID,owner:UUID().uuidString.lowercased(),runtime:client.runtimeInstanceID,generation:opened.document_generation.value)
      let native=NativeDocument();native.attach(files)
      let session=NativeDocumentSession(document:native,host:client,source:source,files:files,drafts:drafts,storage:storage,backupOwner:owner)
      let pending=try await files.recoverPendingSaves(),state=try await files.state()
      try native.restoreBinding(state.head,snapshot:snapshot,dirty:state.dirty)
      if !pending.isEmpty {session.recoveryMessages.append("\(pending.count) 项保存结果待核对；未重新覆盖文件。")}
      do {
        if try await NativeKernelRecovery.restore(storage:storage,client:client,document:documentID,expectedSourceHash:snapshot.snapshot_hash) != nil {session.recoveryMessages.append("已恢复已接纳的计算状态，没有执行单元格。")}
      } catch {session.recoveryMessages.append("计算状态未恢复；源码和历史记录已保留。可另存为并打开为新文档重新运行。")}
      if drafts.confirmed.file.cells.contains(where:{drafts.overlay($0.id)?.requiresReview==true}) {session.recoveryMessages.append("已恢复待确认草稿；请先查看并选择使用内容。")}
      if !drafts.recoveredOrphans.isEmpty {session.recoveryMessages.append("\(drafts.recoveredOrphans.count) 份已删除单元格的草稿已保留，可复制另存。")}
      if case .ready=await storage.libraryAvailability() {await session.reconcileRecoveryHistory()}
      try await session.backup.flush()
      return session
    } catch {
      if let host {try? await host.close()};await physical.close();try? await storage.closeDocument(documentID);throw error
    }
  }
  func reviewDraft(_ cell:String,keep:Bool)throws {
    try drafts.reviewRecovered(cell:cell,keep:keep);editors[Data(cell.utf8)]?.reflectAcknowledgedSource()
    if !drafts.confirmed.file.cells.contains(where:{drafts.overlay($0.id)?.requiresReview==true}) {recoveryMessages.removeAll{$0=="已恢复待确认草稿；请先查看并选择使用内容。"}}
    document.noteEdit()
  }
  func reconcileRecoveryHistory() async {
    do {
      let result=try await RecoveryService(storage:storage).projectDocument(drafts.confirmed.document_id,startingAt:recoveryHistoryCursor)
      recoveryHistoryCursor=result.nextCursor
      recoveryMessages.removeAll{$0.hasPrefix("历史投影")}
      if !result.complete {recoveryMessages.append("历史投影尚有记录待核对，可继续核对；不会重做原操作。")}
    } catch {recoveryMessages.removeAll{$0.hasPrefix("历史投影")};recoveryMessages.append("历史投影核对失败，原文档记录已保留。")}
    document.changed?()
  }
  private func flushSource() async throws {
    let owner=drafts.confirmed.file.cells.first {cell in
      guard let draft=drafts.overlay(cell.id) else {return false}
      return !draft.composing && !draft.source.utf8.elementsEqual(draft.baseSource.utf8)
    }
    let group=owner.flatMap{editors[Data($0.id.utf8)]?.inputGroup} ?? UUID().uuidString.lowercased()
    if let actual=try await source.synchronizeDrafts(inputGroup:group,excludingMarked:true) {try undo.registerCommitted(actual,group:group,name:"编辑笔记本")}
    for editor in editors.values {editor.reflectAcknowledgedSource()}
  }
  func close(approvalRequired:Bool=false) async throws {
    guard !closed else {return}
    for editor in editors.values {editor.textView.isEditable=false}
    do {try document.checkCloseApproval(approvalRequired);try await document.closeNative(approvalRequired:approvalRequired);backup.stop()}
    catch {for editor in editors.values {editor.textView.isEditable=true};throw error}
    closed=true;await source.close();try await host.close()
    try await storage.closeDocument(drafts.confirmed.document_id)
  }
}
