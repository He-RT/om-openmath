import Foundation
import CryptoKit

/// Exact byte-framed identity shared with Rust kernel/acceptance/contract.rs.
enum KernelHashes {
  static func request(_ plan:NativeKernelCommit)->String {
    var h=SourceHashFramer("openmath-kernel-acceptance-v1")
    h.text(plan.kind.rawValue)
    for value in [plan.store_id,plan.operation_id,plan.checkpoint_id,plan.expected_parent_checkpoint_id.value ?? "",plan.checkpoint_blob_hash] {h.text(value)}
    h.number(plan.expected_kernel_state_revision.value);h.number(plan.checkpoint_byte_length.value);h.number(UInt64(plan.codec_version))
    let p=plan.producer
    h.text(p.document_id);h.number(p.document_generation.value);h.number(p.source_revision.value);h.number(p.execution_epoch.value);h.number(p.kernel_state_revision.value)
    h.text(p.source_snapshot_hash);h.text(p.kernel_build);h.number(p.config_revision.value)
    h.text(p.calculation.dialect.rawValue);h.text(p.calculation.constants.rawValue)
    for value in [p.calculation.reactive,p.calculation.auto_run_dependents,p.calculation.show_steps,p.calculation.auto_plot] {h.number(value ? 1 : 0)}
    h.number(p.calculation.eval_timeout_ms.value);h.text(p.general_hash);h.text(p.cell_id.value ?? "");h.text(p.cell_source_hash.value ?? "")
    h.text(p.terminal_status.rawValue);h.number(p.successful_statements.value);h.number(p.out_index.value?.value ?? 0)
    h.text(plan.source.snapshot_hash);h.text(plan.acceptance_source.snapshot_hash);h.text(plan.result_id.value ?? "");h.text(plan.outbox_event_id)
    return h.finish()
  }
  static func data(_ data:Data)->String {SHA256.hash(data:data).map{String(format:"%02x",$0)}.joined()}
}
enum KernelValidation {
  static func hash(_ hash:String)->Bool {hash.utf8.count==64 && hash.utf8.allSatisfy{($0>=48 && $0<=57)||($0>=97 && $0<=102)}}
  static func plan(_ plan:NativeKernelCommit) throws {
    try SourceValidation.snapshot(plan.source);try SourceValidation.snapshot(plan.acceptance_source)
    let p=plan.producer
    guard plan.protocol_version==1,plan.codec_version==1,
      [plan.store_id,plan.runtime_instance_id,plan.operation_id,plan.checkpoint_id,plan.outbox_event_id].allSatisfy(SourceValidation.identity),
      hash(plan.checkpoint_blob_hash),hash(p.general_hash),!p.kernel_build.isEmpty,p.kernel_build.utf8.count<=128,
      (21...64*1024*1024).contains(plan.checkpoint_byte_length.value),p.document_generation.value>0,
      p.document_id==plan.source.document_id,p.document_id==plan.acceptance_source.document_id,
      p.source_revision.value==plan.source.revision.value,p.source_snapshot_hash==plan.source.snapshot_hash,
      p.execution_epoch.value==plan.source.execution_epoch.value,p.execution_epoch.value==plan.acceptance_source.execution_epoch.value,
      plan.acceptance_source.revision.value>=plan.source.revision.value,!SourceHashes.mathChanged(plan.source.file,plan.acceptance_source.file),
      p.successful_statements.value<=10000,p.out_index.value.map({$0.value>0 && $0.value<=10000}) ?? true,
      p.calculation.eval_timeout_ms.value>0,plan.request_hash==KernelHashes.request(plan) else {throw StorageError.corruptIdentity}
    switch plan.kind {
    case .bootstrap:
      guard plan.expected_parent_checkpoint_id.value==nil,plan.expected_kernel_state_revision.value==0,p.kernel_state_revision.value==0,
        p.cell_id.value==nil,p.cell_source_hash.value==nil,p.terminal_status == .unexecuted,p.successful_statements.value==0,
        p.out_index.value==nil,plan.result_id.value==nil else {throw StorageError.corruptIdentity}
    case .execute_cell:
      guard let parent=plan.expected_parent_checkpoint_id.value,SourceValidation.identity(parent),parent != plan.checkpoint_id,
        plan.expected_kernel_state_revision.value<HostSerial.maximum,p.kernel_state_revision.value==plan.expected_kernel_state_revision.value+1,
        let result=plan.result_id.value,SourceValidation.identity(result),p.terminal_status != .unexecuted,
        (p.successful_statements.value==0)==(p.out_index.value==nil),
        let cell=plan.source.file.cells.first(where:{$0.id.utf8.elementsEqual(p.cell_id.value?.utf8 ?? "".utf8)}),cell.kind == .math,
        p.cell_source_hash.value==KernelHashes.data(Data(cell.source.utf8)) else {throw StorageError.corruptIdentity}
    }
  }
  static func receipt(_ receipt:NativeKernelReceipt,plan:NativeKernelCommit) throws {
    try self.plan(plan)
    let encoder=JSONEncoder();encoder.outputFormatting=[.sortedKeys]
    guard receipt.protocol_version==1,receipt.codec_version==1,receipt.store_id==plan.store_id,receipt.document_id==plan.producer.document_id,
      receipt.operation_id==plan.operation_id,receipt.request_hash==plan.request_hash,receipt.checkpoint_id==plan.checkpoint_id,
      receipt.checkpoint_blob_hash==plan.checkpoint_blob_hash,receipt.checkpoint_byte_length.value==plan.checkpoint_byte_length.value,
      receipt.accepted_source_revision.value==plan.acceptance_source.revision.value,receipt.accepted_snapshot_hash==plan.acceptance_source.snapshot_hash,
      receipt.kernel_state_revision.value==plan.producer.kernel_state_revision.value,receipt.result_id.value==plan.result_id.value,
      receipt.outbox_event_id==plan.outbox_event_id,try encoder.encode(receipt.producer)==encoder.encode(plan.producer),
      (20...64).contains(receipt.committed_at.utf8.count),receipt.committed_at.utf8.allSatisfy({$0<128}),
      receipt.committed_at.hasSuffix("Z"),receipt.committed_at.contains("T") else {throw StorageError.corruptIdentity}
  }
}
