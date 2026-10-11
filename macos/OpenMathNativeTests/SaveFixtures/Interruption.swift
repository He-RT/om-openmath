import AppKit
import Foundation
import SQLite3

/// Disposable child processes stop only at real save boundaries. Restart reads original source,
/// intent, user file and receipt; it never executes cells or rewrites an uncertain user target.
@MainActor enum SaveInterruption {
  static func run() async throws {
    let args=CommandLine.arguments,root=URL(fileURLWithPath:args[1],isDirectory:true)
    let mode=args[2],target=root.appendingPathComponent("user.omnb")
    let id="00000000-0000-0000-0000-000000000412",operation="sigkill-save"
    let (storage,_)=try await StorageService.open(paths:.init(root:root.appendingPathComponent("managed"),channel:.preview))
    let info=try await storage.openDocument(id)
    let initial:NativeSourceSnapshot
    if mode.hasPrefix("inspect") {
      guard let actual=try await storage.committedSource(id) else {fatalError("source absent after death")};initial=actual
    } else {
      let original=NativeSourceFile(version:1,title:"crash",cells:[.init(id:"a",kind:.math,source:"let a=2",dialect:.modern)])
      try NotebookFileCodec.encode(original).write(to:target)
      var value=NativeSourceSnapshot(codec_version:1,document_id:id,revision:try HostSerial(0),execution_epoch:try HostSerial(0),file:original,cell_revisions:[.init(cell_id:"a",revision:try HostSerial(0))],snapshot_hash:"")
      value.snapshot_hash=SourceHashes.snapshot(value);initial=try await storage.initializeSource(value)
    }
    let host=try await NativeHostClient.open()
    let owner=try await host.sourceCommand(.source_open(.init(type:.source_open,snapshot:initial,store_id:info.identity.storeID,calculation:.init(nil),config_revision:try HostSerial(0))))
    let clock=SourceOwnerClock();clock.calibrate(owner.owner_time_ms.value)
    let drafts=DraftStore(source:initial,runtime:host.runtimeInstanceID,generation:owner.document_generation.value,clock:{clock.now()})
    let source=DocCommitPort(client:host,storage:storage,drafts:drafts,clock:clock)
    let files=NativeFilePort(client:host,storage:storage,source:source,drafts:drafts,document:id)
    if mode.hasPrefix("inspect") {
      let before=try await storage.saveReceipt(document:id,operation:operation)
      let intent=try await storage.saveIntent(document:id,operation:operation)
      precondition(intent != nil && initial.file.cells[0].source=="let a=7" && initial.revision.value==1)
      if mode=="inspect-conflict" {
        do {_ = try await files.reconcile(operation);fatalError("mismatched displaced external file acknowledged")}
        catch SaveError.externalConflict { }
        let receipt=try await storage.saveReceipt(document:id,operation:operation)
        precondition(receipt==nil)
        let displaced=try NotebookFileCodec.decode(Data(contentsOf:root.appendingPathComponent(".openmath-save-"+operation)))
        precondition(displaced.title=="外部修改原件")
        print("{\"saved\":false,\"external_original_preserved\":true,\"conflict\":true}")
        await files.close();await source.close();try await host.close();try await storage.close();return
      }
      let recovered=try await files.reconcile(operation)
      let actual=try NotebookFileCodec.decode(Data(contentsOf:target))
      let state=try await files.state()
      let kernel=try await host.kernelCommand(.kernel_state(.init(type:.kernel_state)))
      precondition(kernel.active_checkpoint_id.value==nil && kernel.kernel_state_revision.value==0)
      if let recovered {precondition(actual.cells[0].source=="let a=7" && recovered.sourceRevision==1 && !state.dirty)}
      else {precondition(actual.cells[0].source=="let a=2" && state.dirty)}
      let record:[String:Any]=["file_source":actual.cells[0].source,"saved":recovered != nil,"prior_receipt":before != nil,"dirty":state.dirty,"kernel_executed":false,"source_revision":initial.revision.value]
      let output=try JSONSerialization.data(withJSONObject:record,options:[.sortedKeys]);print(String(decoding:output,as:UTF8.self))
      await files.close();await source.close();try await host.close();try await storage.close()
    } else {
      _ = try await files.chooseTarget(target)
      try drafts.attachEditor(cell:"a",generation:1);try drafts.edited(cell:"a",generation:1,source:"let a=7",hasMarkedText:false)
      let faults=StorageFaults {point in
        if point==mode {
          print(point);fflush(stdout)
          while true {Thread.sleep(forTimeInterval:1)}
        }
      }
      _ = try await files.save(operation:operation,faults:faults)
      fatalError("requested real boundary not reached")
    }
  }
}
