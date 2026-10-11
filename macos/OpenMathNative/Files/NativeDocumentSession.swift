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
    let task=Task {let (store,_)=try await StorageService.open(paths:chosen);return store}
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
  private let storage:StorageService
  private var editors:[Data:DraftTextViewAdapter]=[:]
  private var closed=false
  private init(document:NativeDocument,host:NativeHostClient,source:DocCommitPort,files:NativeFilePort,drafts:DraftStore,storage:StorageService) {
    self.document=document;self.host=host;self.source=source;self.files=files;self.drafts=drafts;self.storage=storage
    self.undo=UndoCoordinator {transactions,group,operation in try await source.undo(transactions:transactions,group:group,operation:operation)}
    document.beforeSave={ [weak self] in try await self?.flushSource() }
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
      let source=DocCommitPort(client:client,storage:storage,drafts:drafts,clock:clock)
      let files=NativeFilePort(client:client,storage:storage,source:source,drafts:drafts,files:physical,document:id)
      if let opened {_ = try await files.adoptOpened(opened)}
      let native=NativeDocument();native.attach(files,opened:opened)
      return NativeDocumentSession(document:native,host:client,source:source,files:files,drafts:drafts,storage:storage)
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
    do {try await document.closeNative(approvalRequired:approvalRequired)}
    catch {for editor in editors.values {editor.textView.isEditable=true};throw error}
    closed=true;await source.close();try await host.close()
    try await storage.closeDocument(drafts.confirmed.document_id)
  }
}
