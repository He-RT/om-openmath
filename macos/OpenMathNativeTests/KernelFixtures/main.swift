import Foundation
import SQLite3

@main struct KernelFixtures {
  static func main() async throws {
    let fullsyncFault=CommandLine.arguments.contains("fullsync-fault")
    if fullsyncFault {precondition(om_fixture_install_sync_probe()==SQLITE_OK)}
    let mode=CommandLine.arguments[1],fixture=URL(fileURLWithPath:CommandLine.arguments[2]),root=URL(fileURLWithPath:CommandLine.arguments[3],isDirectory:true)
    let initial=try JSONDecoder().decode(NativeSourceSnapshot.self,from:Data(contentsOf:fixture.appendingPathComponent("source.json")))
    let (storage,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    let info=try await storage.openDocument(initial.document_id)
    if mode=="initialize" {
      precondition(info.identity.storeVersion==3 && info.identity.minimumReaderVersion==3)
      _ = try await storage.initializeSource(initial)
      try JSONEncoder().encode(info).write(to:fixture.appendingPathComponent("store-info.json"))
      try await storage.close();return
    }
    let stage=CommandLine.arguments[4],folder=fixture.appendingPathComponent(stage)
    let plan=try JSONDecoder().decode(NativeKernelCommit.self,from:Data(contentsOf:folder.appendingPathComponent("plan.json")))
    let bytes=try Data(contentsOf:folder.appendingPathComponent("checkpoint.bin"))
    try KernelValidation.plan(plan)
    if mode=="inspect-crash" {
      let record=try await storage.kernelReceipt(document:initial.document_id,operation:plan.operation_id)
      let head=try await storage.acceptedKernelHead(document:initial.document_id)!
      let reference=try await storage.blobReference(owner:.init(kind:.checkpoint,id:plan.checkpoint_id),hash:plan.checkpoint_blob_hash,document:initial.document_id)
      if let record {
        try KernelValidation.receipt(record,plan:plan)
        precondition(head.checkpoint_id==record.checkpoint_id && reference != nil)
        print("same-accepted-candidate")
      } else {
        precondition(head.checkpoint_id==plan.expected_parent_checkpoint_id.value && reference==nil)
        print("previous-accepted-parent")
      }
      try await storage.close();return
    }
    precondition(KernelHashes.data(bytes)==plan.checkpoint_blob_hash)
    let publication=try await storage.publishBlob(data:bytes,expectedHash:plan.checkpoint_blob_hash)
    let before=try await storage.acceptedKernelHead(document:initial.document_id)
    let ordinary=KernelCommitControls(ownerBarrier:{ }) // physical-only fixture; Rust lifecycle tested separately
    if mode=="crash" {
      let point=CommandLine.arguments[5]
      let fault=StorageFaults { current in
        if current==point {
          FileHandle.standardOutput.write(Data((point+"\n").utf8))
          // Disposable external test child is killed at this real boundary by interruption.py.
          while true {Thread.sleep(forTimeInterval:0.1)}
        }
      }
      _ = try await storage.commitKernel(plan,publication:publication,controls:ordinary,faults:fault)
      fatalError("did not reach crash boundary")
    }
    if stage != "bootstrap" {
      let failed=StorageFaults {point in if point=="kernel_checkpoint_inserted" {throw StorageError.system(ENOSPC,"actual-kernel-rollback")} }
      do {_ = try await storage.commitKernel(plan,publication:publication,controls:ordinary,faults:failed);fatalError("partial checkpoint acknowledged")}
      catch StorageError.system { }
      let absent=try await storage.kernelReceipt(document:initial.document_id,operation:plan.operation_id)
      let head=try await storage.acceptedKernelHead(document:initial.document_id)
      let reference=try await storage.blobReference(owner:.init(kind:.checkpoint,id:plan.checkpoint_id),hash:plan.checkpoint_blob_hash,document:initial.document_id)
      precondition(absent==nil && head?.checkpoint_id==before?.checkpoint_id && reference==nil)
      let cancelled=KernelCommitControls(ownerBarrier:{throw CommitPortError.cancelled})
      do {_ = try await storage.commitKernel(plan,publication:publication,controls:cancelled);fatalError("cancelled checkpoint committed")}
      catch CommitPortError.cancelled { }
      let cancelledReceipt=try await storage.kernelReceipt(document:initial.document_id,operation:plan.operation_id);precondition(cancelledReceipt==nil)
    }
    var receipt:NativeKernelReceipt
    if stage=="b" && fullsyncFault {
      let actualSyncFault=StorageFaults {point in if point=="before_kernel_commit" {om_fixture_fail_fullsync(1)} }
      do {_ = try await storage.commitKernel(plan,publication:publication,controls:ordinary,faults:actualSyncFault);fatalError("actual failed fullsync acknowledged")}
      catch StorageError.unknownCommit { }
      do {_ = try await storage.kernelReceipt(document:initial.document_id,operation:plan.operation_id);fatalError("unstable kernel bytes certified")}
      catch StorageError.unknownCommit { }
      om_fixture_fail_fullsync(0)
      receipt=try await storage.kernelReceipt(document:initial.document_id,operation:plan.operation_id)!
    } else if stage=="b" {
      let lost=StorageFaults {point in if point=="kernel_committed" {throw StorageError.system(EIO,"actual-kernel-lost-ack")} }
      do {_ = try await storage.commitKernel(plan,publication:publication,controls:ordinary,faults:lost);fatalError("lost ACK returned success")}
      catch StorageError.unknownCommit { }
      receipt=try await storage.kernelReceipt(document:initial.document_id,operation:plan.operation_id)!
    } else {receipt=try await storage.commitKernel(plan,publication:publication,controls:ordinary)}
    try KernelValidation.receipt(receipt,plan:plan)
    if stage != "bootstrap" {
      var stale=plan
      stale.operation_id="stale-operation-\(stage)";stale.checkpoint_id="stale-checkpoint-\(stage)";stale.outbox_event_id="stale-event-\(stage)";stale.result_id = .init("stale-result-\(stage)")
      stale.request_hash=KernelHashes.request(stale)
      do {_ = try await storage.commitKernel(stale,publication:publication,controls:ordinary);fatalError("old parent overwrote newer accepted state")}
      catch StorageError.staleRevision { }
    }
    let duplicated=try await storage.commitKernel(plan,publication:publication,controls:.init(ownerBarrier:{fatalError("duplicate invoked barrier")}))
    precondition(duplicated.checkpoint_id==receipt.checkpoint_id && duplicated.committed_at==receipt.committed_at)
    var conflict=plan;conflict.outbox_event_id="different-kernel-event";conflict.request_hash=KernelHashes.request(conflict)
    do {_ = try await storage.commitKernel(conflict,publication:publication,controls:ordinary);fatalError("different original-ID request accepted")}
    catch StorageError.idempotencyConflict { }
    try JSONEncoder().encode(receipt).write(to:folder.appendingPathComponent("receipt.json"))
    let source=try await storage.committedSource(initial.document_id)!
    precondition(source.snapshot_hash==initial.snapshot_hash && source.revision.value==0)
    try await storage.releaseBlob(publication)
    try await storage.close()
    let (reopened,_)=try await StorageService.open(paths:.init(root:root,channel:.preview))
    _ = try await reopened.openDocument(initial.document_id)
    let restored=try await reopened.kernelReceipt(document:initial.document_id,operation:plan.operation_id)!
    let restoredHead=try await reopened.acceptedKernelHead(document:initial.document_id)!
    precondition(restored.checkpoint_id==receipt.checkpoint_id && restoredHead.checkpoint_id==receipt.checkpoint_id)
    let lease=try await reopened.openReferencedBlob(owner:.init(kind:.checkpoint,id:receipt.checkpoint_id),hash:receipt.checkpoint_blob_hash,document:initial.document_id)
    var actual=Data(),offset:UInt64=0
    while offset<receipt.checkpoint_byte_length.value {
      let chunk=try await reopened.readBlob(lease,offset:offset,count:min(64*1024,Int(receipt.checkpoint_byte_length.value-offset)))
      precondition(!chunk.isEmpty);actual.append(chunk);offset+=UInt64(chunk.count)
    }
    precondition(actual==bytes);try await reopened.closeBlob(lease);try await reopened.close()
    let database=root.appendingPathComponent("Documents/\(initial.document_id)/Generations/000001/authority.sqlite")
    let acceptedCheckpoint=receipt.checkpoint_id
    let counts=try await Task.detached { () throws->[Int64] in
      let reader=try SQLiteDatabase(url:database,readonly:true);defer {try? reader.close()}
      let tables=["accepted_checkpoints","kernel_operations","kernel_transitions","kernel_outbox","kernel_head","accepted_results"]
      let counts=try tables.map {try reader.integer("SELECT COUNT(*) FROM \($0)")}
      var pointer:String?
      try reader.statement("SELECT active_checkpoint_ref FROM document_head") {pointer=try SQLColumn.text($0,0)}
      let integrity=try reader.text("PRAGMA integrity_check")
      precondition(pointer==acceptedCheckpoint && integrity=="ok")
      return counts
    }.value
    let expected:Int64=receipt.kernel_state_revision.value==0 ? 1 : Int64(receipt.kernel_state_revision.value+1)
    precondition(counts==[expected,expected,expected,expected,1,expected-1])
    if stage=="error" {
      try await Task.detached {
        let writer=try SQLiteDatabase(url:database);defer {try? writer.close()};_ = try writer.configure()
        try writer.transaction {try writer.statement("UPDATE kernel_outbox SET payload_hash=? WHERE operation_id=?",[.text(String(repeating:"0",count:64)),.text(plan.operation_id)])}
      }.value
      let (damaged,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));_ = try await damaged.openDocument(initial.document_id)
      do {_ = try await damaged.kernelReceipt(document:initial.document_id,operation:plan.operation_id);fatalError("broken receipt graph looked absent or accepted")}
      catch StorageError.corruptIdentity { }
      try await damaged.close()
      try await Task.detached {
        let writer=try SQLiteDatabase(url:database);defer {try? writer.close()};_ = try writer.configure()
        try writer.transaction {try writer.statement("UPDATE kernel_outbox SET payload_hash=(SELECT receipt_hash FROM kernel_operations WHERE document_id=kernel_outbox.document_id AND operation_id=kernel_outbox.operation_id) WHERE operation_id=?",[.text(plan.operation_id)])}
      }.value
      let blob=root.appendingPathComponent("Blobs/sha256/\(plan.checkpoint_blob_hash.prefix(2))/\(plan.checkpoint_blob_hash)")
      let original=try Data(contentsOf:blob);var corrupt=original;corrupt[0]^=1;try corrupt.write(to:blob)
      let (brokenBlob,_)=try await StorageService.open(paths:.init(root:root,channel:.preview));_ = try await brokenBlob.openDocument(initial.document_id)
      do {_ = try await brokenBlob.openReferencedBlob(owner:.init(kind:.checkpoint,id:plan.checkpoint_id),hash:plan.checkpoint_blob_hash,document:initial.document_id);fatalError("damaged actual checkpoint bytes were opened")}
      catch BlobError.corrupt { }
      try await brokenBlob.close();try original.write(to:blob)
    }
    print("Actual kernel \(stage): SQLite/Blob/source/head/outbox/receipt \(counts), rollback/cancel/original-ID/reopen bytes verified")
  }
}
