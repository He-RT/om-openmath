import CryptoKit
import Foundation

/// Length-prefixed UTF-8 framing, byte-for-byte identical to Rust document/hashes.rs.
/// Swift String equality normalizes canonically equivalent scalars, so source checks use bytes.
struct SourceHashFramer {
  private var digest=SHA256()
  init(_ domain:String) { text(domain) }
  mutating func number(_ number:UInt64) { var big=number.bigEndian;withUnsafeBytes(of:&big){digest.update(bufferPointer:$0)} }
  mutating func text(_ text:String) { let data=Data(text.utf8);number(UInt64(data.count));digest.update(data:data) }
  func finish()->String { digest.finalize().map{String(format:"%02x",$0)}.joined() }
}
enum SourceHashes {
  static func snapshot(_ source:NativeSourceSnapshot)->String {
    var hash=SourceHashFramer("openmath-source-v1")
    hash.text(source.document_id);hash.number(source.revision.value);hash.number(source.execution_epoch.value)
    hash.number(UInt64(source.file.version));hash.text(source.file.title);hash.number(UInt64(source.file.cells.count))
    for (index,cell) in source.file.cells.enumerated() {
      hash.text(cell.id);hash.text(cell.kind.rawValue);hash.text(cell.source);hash.text(cell.dialect.rawValue)
      hash.number(index<source.cell_revisions.count ? source.cell_revisions[index].revision.value : .max)
    }
    return hash.finish()
  }
  static func request(_ plan:NativeSourceCommit)->String {
    request(commit:plan.commit,change:plan.calculation_change.value,group:plan.undo_group,inputGroup:plan.input_group_id)
  }
  static func request(commit:DocumentCommit,change:NativeCalculationChange?,group:NativeUndoGroup?,inputGroup:String?=nil)->String {
    var hash=SourceHashFramer("openmath-source-operation-v1")
    hash.text(commit.document_id);hash.text(commit.operation_id)
    hash.text(commit.inverse_plan_hash);hash.text(commit.snapshot_hash);hash.text(commit.actor.rawValue)
    hash.text(commit.task_id.value ?? "");hash.text(commit.undo_of.value ?? "")
    if let group {
      hash.text("undo-group-v1");hash.text(group.group_id);hash.number(UInt64(group.transaction_ids.count))
      for id in group.transaction_ids {hash.text(id)}
    }
    if let inputGroup {hash.text("input-group-v1");hash.text(inputGroup)}
    if let change {
      hash.text("calculation-change-v1")
      for setting in [change.before,change.after] {
        hash.text(setting.dialect.rawValue);hash.text(setting.constants.rawValue)
        for value in [setting.reactive,setting.auto_run_dependents,setting.show_steps,setting.auto_plot] {hash.number(value ? 1 : 0)}
        hash.number(setting.eval_timeout_ms.value)
      }
    }
    return hash.finish()
  }
  static func settingsChanged(_ change:NativeCalculationChange)->Bool {
    let a=change.before,b=change.after
    return a.dialect != b.dialect || a.constants != b.constants || a.reactive != b.reactive || a.auto_run_dependents != b.auto_run_dependents
      || a.show_steps != b.show_steps || a.auto_plot != b.auto_plot || a.eval_timeout_ms.value != b.eval_timeout_ms.value
  }
  static func same(_ a:NativeSourceCell,_ b:NativeSourceCell)->Bool {
    a.id.utf8.elementsEqual(b.id.utf8) && a.kind==b.kind && a.dialect==b.dialect && a.source.utf8.elementsEqual(b.source.utf8)
  }
  static func mathChanged(_ before:NativeSourceFile,_ after:NativeSourceFile)->Bool {
    let left=before.cells.filter{$0.kind == .math},right=after.cells.filter{$0.kind == .math}
    guard left.count==right.count else {return true}
    return !zip(left,right).allSatisfy{same($0,$1)}
  }
  static func changes(_ a:NativeSourceFile,_ b:NativeSourceFile)->[String] {
    var ids:[Data:String]=[:]
    for (index,cell) in a.cells.enumerated() where index>=b.cells.count || !same(cell,b.cells[index]) { ids[Data(cell.id.utf8)]=cell.id }
    for (index,cell) in b.cells.enumerated() where index>=a.cells.count || !same(cell,a.cells[index]) { ids[Data(cell.id.utf8)]=cell.id }
    return ids.values.sorted { $0.utf8.lexicographicallyPrecedes($1.utf8) }
  }
}
enum SourceValidation {
  static func identity(_ id:String)->Bool {
    !id.isEmpty && id.utf8.count<=256 && id.utf8.enumerated().allSatisfy { index,byte in
      (byte>=48 && byte<=57) || (byte>=65 && byte<=90) || (byte>=97 && byte<=122) || (index>0 && [46,95,58,45].contains(byte))
    }
  }
  static func snapshot(_ source:NativeSourceSnapshot) throws {
    guard source.cell_revisions.allSatisfy({$0.revision.value<=source.revision.value}),source.codec_version==1,source.file.version==1,identity(source.document_id),source.file.title.utf8.count<=16384,
      source.file.cells.count<=10000,source.cell_revisions.count==source.file.cells.count,
      Set(source.file.cells.map{Data($0.id.utf8)}).count==source.file.cells.count,
      zip(source.file.cells,source.cell_revisions).allSatisfy({$0.id.utf8.elementsEqual($1.cell_id.utf8) && !$0.id.isEmpty && $0.id.utf8.count<=256}),
      source.file.title.utf8.count+source.file.cells.reduce(0,{$0+$1.source.utf8.count+$1.id.utf8.count})<=2*1024*1024,
      source.snapshot_hash==SourceHashes.snapshot(source) else { throw StorageError.corruptIdentity }
  }
  static func commit(_ plan:NativeSourceCommit) throws {
    try snapshot(plan.before);try snapshot(plan.after)
    let commit=plan.commit
    let mathChange=SourceHashes.mathChanged(plan.before.file,plan.after.file) || plan.calculation_change.value.map(SourceHashes.settingsChanged)==true
    guard plan.protocol_version==1,plan.generation.value>0,identity(commit.operation_id),identity(commit.transaction_id),identity(commit.outbox_event_id),
      commit.document_id==plan.before.document_id,commit.document_id==plan.after.document_id,
      commit.base_revision.value==plan.before.revision.value,commit.committed_revision.value==plan.after.revision.value,
      plan.before.revision.value<HostSerial.maximum,plan.after.revision.value==plan.before.revision.value+1,
      (!mathChange || plan.before.execution_epoch.value<HostSerial.maximum),
      plan.after.execution_epoch.value==plan.before.execution_epoch.value+(mathChange ? 1 : 0),
      commit.execution_epoch.value==plan.after.execution_epoch.value,commit.snapshot_hash==plan.after.snapshot_hash,
      commit.inverse_plan_hash==plan.before.snapshot_hash,commit.request_hash==SourceHashes.request(plan),commit.snapshot_blob_hash.value==nil,
      commit.changed_cell_ids==SourceHashes.changes(plan.before.file,plan.after.file) else { throw StorageError.corruptIdentity }
    for (index,cell) in plan.after.file.cells.enumerated() {
      let prior=plan.before.file.cells.firstIndex(where:{$0.id.utf8.elementsEqual(cell.id.utf8)})
      var expected:UInt64=0
      if let prior {
        expected=plan.before.cell_revisions[prior].revision.value
        if !SourceHashes.same(cell,plan.before.file.cells[prior]) {
          guard expected<HostSerial.maximum else { throw StorageError.corruptIdentity };expected += 1
        }
      }
      guard plan.after.cell_revisions[index].revision.value==expected else { throw StorageError.corruptIdentity }
    }
    if commit.actor == .agent { guard let task=commit.task_id.value,identity(task) else { throw StorageError.corruptIdentity } }
    if commit.actor == .undo { guard let undo=commit.undo_of.value,identity(undo) else { throw StorageError.corruptIdentity } }
    if let input=plan.input_group_id {guard commit.actor == .manual,identity(input) else {throw StorageError.corruptIdentity}}
    if let group=plan.undo_group {
      guard commit.actor == .undo,identity(group.group_id),!group.transaction_ids.isEmpty,group.transaction_ids.count<=32,
        group.transaction_ids.allSatisfy(identity),Set(group.transaction_ids).count==group.transaction_ids.count,
        commit.undo_of.value==group.transaction_ids.first else {throw StorageError.corruptIdentity}
    }
  }
}
