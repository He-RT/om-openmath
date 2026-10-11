import AppKit
import Foundation
import SQLite3

private struct Manifest:Codable {
  let documentID:String,storeID:String,snapshotHash:String,checkpointID:String,checkpointHash:String,fileHash:String
  let kernelRevision:UInt64
}
private func actualOutput(_ store:StorageService,_ receipt:NativeKernelReceipt,cell:String) async throws->String? {
  let reader=try await store.openReferencedBlob(owner:.init(kind:.checkpoint,id:receipt.checkpoint_id),hash:receipt.checkpoint_blob_hash,document:receipt.document_id)
  var bytes=Data(),offset:UInt64=0
  while offset<receipt.checkpoint_byte_length.value {
    let chunk=try await store.readBlob(reader,offset:offset,count:min(65536,Int(receipt.checkpoint_byte_length.value-offset)));bytes.append(chunk);offset+=UInt64(chunk.count)
  }
  try await store.closeBlob(reader)
  precondition(KernelHashes.data(bytes)==receipt.checkpoint_blob_hash && bytes.prefix(5)==Data([79,77,75,83,1]))
  let size=bytes[5..<9].enumerated().reduce(UInt32(0)) {$0 | (UInt32($1.element) << UInt32($1.offset*8))}
  let meta=try JSONSerialization.jsonObject(with:bytes.subdata(in:21..<21+Int(size))) as! [String:Any]
  let actual=(meta["cells"] as! [[String:Any]]).first{($0["input"] as? [String:Any])?["id"] as? String==cell}!
  let output=actual["output"] as? [String:Any],items=output?["items"] as? [[String:Any]]
  return items?.last?["input_form"] as? String
}
@MainActor @main struct RecoveryFixtures {
  private static func prepare(_ root:URL) async throws->(StorageService,NativeDocumentSession,KernelCommitPort,Manifest) {
    let user=root.appendingPathComponent("user-files",isDirectory:true);try FileManager.default.createDirectory(at:user,withIntermediateDirectories:true)
    let target=user.appendingPathComponent("恢复🙂.omnb")
    let file=NativeSourceFile(version:1,title:"恢复测试",cells:[.init(id:"a",kind:.math,source:"let a=2",dialect:.modern),.init(id:"b",kind:.math,source:"a+1",dialect:.modern)])
    try NotebookFileCodec.encode(file).write(to:target)
    let (store,info)=try await StorageService.open(paths:.init(root:root.appendingPathComponent("managed"),channel:.preview))
    precondition(info.identity.storeVersion==2 && info.identity.minimumReaderVersion==2)
    let session=try await NativeDocumentSession.open(storage:store,url:target),kernel=KernelCommitPort(client:session.host,storage:store,source:session.source)
    let two=try await kernel.runCell("a",operation:"recovery-a2"),three=try await kernel.runCell("b",operation:"recovery-b3")
    let value2=try await actualOutput(store,two,cell:"a"),value3=try await actualOutput(store,three,cell:"b");precondition(value2=="2" && value3=="3")
    let editor=try session.editor("a");editor.textView.string="let a=5";editor.textView.didChangeText()
    let five=try await kernel.runCell("a",operation:"recovery-a5"),six=try await kernel.runCell("b",operation:"recovery-b6")
    let value5=try await actualOutput(store,five,cell:"a"),value6=try await actualOutput(store,six,cell:"b");precondition(value5=="5" && value6=="6")
    let saved=try await session.document.saveNative(operation:"recovery-save"),identity=try await store.documentIdentity(session.drafts.confirmed.document_id)
    let manifest=Manifest(documentID:identity.documentID!,storeID:identity.storeID,snapshotHash:session.drafts.confirmed.snapshot_hash,checkpointID:six.checkpoint_id,checkpointHash:six.checkpoint_blob_hash,fileHash:saved.fileHash,kernelRevision:six.kernel_state_revision.value)
    try ManagedFiles.publish(manifest,to:root.appendingPathComponent("manifest.json"),faults:.init())
    let source=try await store.documentOutbox(document:manifest.documentID,kind:.source)
    let math=try await store.documentOutbox(document:manifest.documentID,kind:.kernel)
    let files=try await store.documentOutbox(document:manifest.documentID,kind:.save)
    precondition(source.entries.count==1 && math.entries.count==5 && files.entries.count==1)
    let bootstrap=try JSONDecoder().decode(NativeKernelReceipt.self,from:math.entries[0].effect.payload)
    precondition(bootstrap.kernel_state_revision.value==0 && bootstrap.producer.cell_id.value==nil && bootstrap.result_id.value==nil)
    precondition(math.entries.dropFirst().map(\.effect.operationID)==["recovery-a2","recovery-b3","recovery-a5","recovery-b6"])
    return (store,session,kernel,manifest)
  }
  private static func recovered(_ root:URL) async throws {
    let manifest=try JSONDecoder().decode(Manifest.self,from:Data(contentsOf:root.appendingPathComponent("manifest.json")))
    func mark(_ text:String) {FileHandle.standardError.write(Data((text+"\n").utf8))}
    mark("recovery-storage-open")
    let (store,_)=try await StorageService.open(paths:.init(root:root.appendingPathComponent("managed"),channel:.preview))
    mark("recovery-document-open");_ = try await store.openDocument(manifest.documentID)
    mark("recovery-source-and-prefix")
    let before=try await store.committedSource(manifest.documentID)!,head=try await store.acceptedKernelHead(document:manifest.documentID)!
    precondition(before.snapshot_hash==manifest.snapshotHash && head.checkpoint_id==manifest.checkpointID && head.checkpoint_blob_hash==manifest.checkpointHash && head.kernel_state_revision.value==manifest.kernelRevision)
    // This branch creates no NativeHostClient, KernelCommitPort, editor or file writer.
    mark("recovery-project-original-effects")
    let recovery=RecoveryService(storage:store),result=try await recovery.projectDocument(manifest.documentID)
    let events=try await store.libraryEffects();precondition(result.complete && result.checkedEvents==7 && events.count==7)
    let repeated=try await recovery.projectDocument(manifest.documentID);precondition(repeated.complete && repeated.insertedEvents==0)
    let after=try await store.committedSource(manifest.documentID)!,headAfter=try await store.acceptedKernelHead(document:manifest.documentID)!
    precondition(after.snapshot_hash==before.snapshot_hash && headAfter.checkpoint_id==head.checkpoint_id && headAfter.kernel_state_revision.value==head.kernel_state_revision.value)
    let retainedFile=try Data(contentsOf:root.appendingPathComponent("user-files/恢复🙂.omnb"))
    precondition(KernelHashes.data(retainedFile)==manifest.fileHash)
    try await store.close()
    print("Actual restart: 7 original effects including bootstrap, no duplicates, source/accepted prefix/file bytes unchanged; no evaluator or HTTP created")
  }
  private static func recoveredSession(_ root:URL) async throws {
    let manifest=try JSONDecoder().decode(Manifest.self,from:Data(contentsOf:root.appendingPathComponent("manifest.json")))
    let (store,_)=try await StorageService.open(paths:.init(root:root.appendingPathComponent("managed"),channel:.preview))
    let page=try await store.startupDocuments();precondition(page.records.count==1 && page.records[0].id==manifest.documentID && page.records[0].recoverable)
    let session=try await NativeDocumentSession.recover(storage:store,documentID:manifest.documentID)
    let state=try await session.host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
    precondition(state.active_checkpoint_id.value==manifest.checkpointID && state.kernel_state_revision.value==manifest.kernelRevision && state.definitions_current)
    let original=try await store.acceptedKernelHead(document:manifest.documentID)!
    precondition(original.checkpoint_id==manifest.checkpointID && session.drafts.confirmed.snapshot_hash==manifest.snapshotHash)
    precondition(session.recoveryMessages.contains{$0.contains("没有执行单元格")})
    // Now explicitly run only b. It must use the restored a=5 definition, without rerunning a.
    let kernel=KernelCommitPort(client:session.host,storage:store,source:session.source)
    let next=try await kernel.runCell("b",operation:"explicit-run-after-recovery")
    let value=try await actualOutput(store,next,cell:"b");precondition(value=="6" && next.kernel_state_revision.value==manifest.kernelRevision+1)
    let events=try await store.documentOutbox(document:manifest.documentID,kind:.kernel);precondition(events.entries.count==6)
    await kernel.close();try await session.close();try await store.close()
    print("Actual native startup selection restores original accepted checkpoint/revision with no replay; explicit b-only run uses a=5 and returns6")
  }
  static func main() async throws {
    precondition(om_fixture_install_sync_probe()==SQLITE_OK)
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    if CommandLine.arguments.count>2,CommandLine.arguments[2]=="recover" {try await recovered(root);return}
    if CommandLine.arguments.count>2,CommandLine.arguments[2]=="session-recover" {try await recoveredSession(root);return}
    if CommandLine.arguments.count>2,CommandLine.arguments[2]=="session-corrupt" {
      let manifest=try JSONDecoder().decode(Manifest.self,from:Data(contentsOf:root.appendingPathComponent("manifest.json")))
      let blob=root.appendingPathComponent("managed/Blobs/sha256/\(manifest.checkpointHash.prefix(2))/\(manifest.checkpointHash)")
      let original=try Data(contentsOf:blob);var damaged=original;damaged[damaged.count-1]^=1;try damaged.write(to:blob)
      let (storage,_)=try await StorageService.open(paths:.init(root:root.appendingPathComponent("managed"),channel:.preview))
      let recovered=try await NativeDocumentSession.recover(storage:storage,documentID:manifest.documentID)
      let state=try await recovered.host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
      precondition(state.active_checkpoint_id.value==nil && recovered.recoveryMessages.contains{$0.contains("计算状态未恢复")})
      precondition(recovered.drafts.confirmed.snapshot_hash==manifest.snapshotHash)
      let physical=try await storage.acceptedKernelHead(document:manifest.documentID)!
      precondition(physical.checkpoint_id==manifest.checkpointID && physical.kernel_state_revision.value==manifest.kernelRevision)
      try await recovered.close();try await storage.close()
      let persistedDamage=try Data(contentsOf:blob);precondition(persistedDamage==damaged)
      print("Damaged original checkpoint refused: source and historical receipt retained, no empty calculation claimed recovered")
      return
    }
    let (store,session,kernel,manifest)=try await prepare(root)
    let sourceEvent=try await store.documentOutbox(document:manifest.documentID,kind:.source).entries[0].effect
    if CommandLine.arguments.count>3,CommandLine.arguments[2]=="crash" {
      let boundary=CommandLine.arguments[3]
      _ = try await RecoveryService(storage:store).projectDocument(manifest.documentID,faults:.init {point in
        if point==boundary {FileHandle.standardOutput.write(Data((point+"\n").utf8));raise(SIGSTOP)}
      });fatalError("actual projection crash boundary was not reached")
    }
    let rollback=StorageFaults {point in if point=="library_projection_before_commit" {throw StorageError.system(ENOSPC,"projection-fixture")}}
    do {_ = try await store.projectDocumentEffect(sourceEvent,faults:rollback);fatalError("rollback falsely acknowledged")} catch StorageError.system { }
    let empty=try await store.libraryEffects();precondition(empty.isEmpty)
    let lost=StorageFaults {point in if point=="library_projection_committed" {throw StorageError.system(EIO,"lost-library-ack")}}
    do {_ = try await store.projectDocumentEffect(sourceEvent,faults:lost);fatalError("lost ACK falsely acknowledged")} catch StorageError.unknownCommit { }
    let actualLost=try await store.libraryEffects();precondition(actualLost.count==1)
    let recoveredOriginal=try await store.projectDocumentEffect(sourceEvent);precondition(!recoveredOriginal.inserted && recoveredOriginal.librarySequence==1)
    let save=try await store.documentOutbox(document:manifest.documentID,kind:.save).entries[0].effect
    let projection=try JSONSerialization.jsonObject(with:save.payload) as! [String:Any]
    precondition(Set(projection.keys)==["codecVersion","operationID","documentID","sourceRevision","sourceSnapshotHash","bindingRevision","fileHash","recovered","completedAt"])
    var changed=projection;changed["fileHash"]=String(repeating:"a",count:64)
    let fake=DocumentEffect(codecVersion:save.codecVersion,sourceStoreID:save.sourceStoreID,sourceGeneration:save.sourceGeneration,documentID:save.documentID,eventID:save.eventID,operationID:save.operationID,kind:save.kind,rawReceiptHash:save.rawReceiptHash,payload:try JSONSerialization.data(withJSONObject:changed))
    do {_ = try await store.projectDocumentEffect(fake);fatalError("fake native receipt reached Library")} catch StorageError.corruptIdentity { }
    let afterFake=try await store.libraryEffects();precondition(afterFake.count==1)
    let math=try await store.documentOutbox(document:manifest.documentID,kind:.kernel).entries[0].effect
    let unknown=StorageFaults {point in if point=="library_projection_before_commit" {om_fixture_fail_fullsync(1)}}
    do {_ = try await store.projectDocumentEffect(math,faults:unknown);fatalError("actual F_FULLFSYNC failure acknowledged")} catch StorageError.unknownCommit { }
    om_fixture_fail_fullsync(0)
    let confirmed=try await store.projectDocumentEffect(math);precondition(!confirmed.inserted)
    let recovery=RecoveryService(storage:store)
    var cursor:DocumentProjectionCursor?,checked=0,inserted=0,complete=false
    for _ in 0..<12 {
      let result=try await recovery.projectDocument(manifest.documentID,startingAt:cursor,maximumEvents:1)
      checked+=result.checkedEvents;inserted+=result.insertedEvents;cursor=result.nextCursor;complete=result.complete
      if complete {break}
    }
    precondition(complete && checked==7 && inserted==5)
    let repeated=try await recovery.projectDocument(manifest.documentID);precondition(repeated.insertedEvents==0 && repeated.checkedEvents==7)
    let firstPage=try await store.libraryEffects(limit:1);precondition(firstPage.count==1)
    let secondPage=try await store.libraryEffects(after:firstPage[0].sequence,limit:1);precondition(secondPage.count==1 && secondPage[0].sequence>firstPage[0].sequence)
    let clone=try await NativeDocumentSession.open(storage:store,url:root.appendingPathComponent("user-files/恢复🙂.omnb"))
    _ = try await clone.document.saveNative(operation:"recovery-save")
    let secondDocument=try await RecoveryService(storage:store).projectDocument(clone.drafts.confirmed.document_id)
    precondition(secondDocument.insertedEvents==1)
    let all=try await store.libraryEffects();precondition(all.count==8 && all.filter{$0.effect.eventID==save.eventID}.count==2)
    try await clone.close();await kernel.close();try await session.close();try await store.close()
    let database=root.appendingPathComponent("managed/Library/Generations/000001/library.sqlite")
    try await Task.detached {
      // This independent header copy is solely a file-change guard fixture, never a backup
      // or restoration input. Do not delete real WAL files to manufacture a test condition.
      let guardFile=root.appendingPathComponent("readonly-guard-fixture.sqlite")
      try FileManager.default.copyItem(at:database,to:guardFile)
      let reader=try SQLiteDatabase(url:guardFile,readonly:true)
      let version=try reader.integer("PRAGMA user_version");precondition(version==2)
      // Deliberately violate the serialized-reader contract in this disposable fixture.
      // No data bytes change; the reader still must reject a changed underlying file.
      try FileManager.default.setAttributes([.modificationDate:Date(timeIntervalSince1970:1)],ofItemAtPath:guardFile.path)
      do {_ = try reader.integer("PRAGMA user_version");fatalError("changed immutable view returned stale data")} catch StorageError.recoveryRequired { }
      try reader.close()
    }.value
    // Corrupt an actual complete Library record. Read-only validation rejects it before writer
    // setup; workspace keeps the original DB bytes and can still use the healthy document.
    try await Task.detached {let db=try SQLiteDatabase(url:database);_ = try db.configure();try db.transaction {try db.statement("UPDATE document_effect_events SET record=? WHERE sequence=1",[.blob(Data("broken full record".utf8))])};try db.checkpoint();try db.close()}.value
    let brokenBytes=try Data(contentsOf:database)
    do {_ = try await StorageService.open(paths:.init(root:root.appendingPathComponent("managed"),channel:.preview));fatalError("bad record swallowed")} catch StorageError.corruptIdentity { }
    let workspace=try await StorageService.openWorkspace(paths:.init(root:root.appendingPathComponent("managed"),channel:.preview))
    let availability=await workspace.libraryAvailability();guard case .unavailable(.corruptIdentity)=availability else {fatalError("bad Library reported ready")}
    _ = try await workspace.openDocument(manifest.documentID)
    let retained=try await workspace.committedSource(manifest.documentID)!,prefix=try await workspace.acceptedKernelHead(document:manifest.documentID)!
    precondition(retained.snapshot_hash==manifest.snapshotHash && prefix.checkpoint_id==manifest.checkpointID)
    let independent=try await NativeDocumentSession.open(storage:workspace,url:root.appendingPathComponent("user-files/恢复🙂.omnb"))
    let actualSave=try await independent.document.saveNative(to:root.appendingPathComponent("user-files/库损坏仍可保存.omnb"),operation:"degraded-document-save")
    precondition(actualSave.fileHash==manifest.fileHash)
    try await independent.close();try await workspace.close()
    let stillBroken=try Data(contentsOf:database);precondition(stillBroken==brokenBytes)
    print("Actual Library/source/kernel/file projections: durable rollback/lost ACK/F_FULLFSYNC recovery, bounded cursor, namespace separation, forged receipt refusal, private bookmark isolation and complete-record corruption/degraded document saves passed")
  }
}
