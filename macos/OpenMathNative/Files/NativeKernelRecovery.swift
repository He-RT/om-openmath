import Foundation
enum NativeKernelRecoveryError:Error,Sendable {case unavailable}

enum NativeKernelRecovery {
  private struct Metadata:Encodable {
    let protocol_version:UInt32
    let plan:NativeKernelCommit
    let receipt:NativeKernelReceipt
  }
  /// Trusted storage/ABI adapter only. The original producer, raw codec bytes and actual
  /// receipt are checked in the background; import creates no new durable kernel operation.
  static func restore(storage:StorageService,client:NativeHostClient,document:String,expectedSourceHash:String) async throws->NativeKernelReceipt? {
    guard let stored=try await storage.kernelRecoveryHead(document:document) else {return nil}
    let receipt=stored.receipt
    guard receipt.checkpoint_byte_length.value<=64*1024*1024 else {throw StorageError.corruptIdentity}
    let reader=try await storage.openReferencedBlob(owner:.init(kind:.checkpoint,id:receipt.checkpoint_id),hash:receipt.checkpoint_blob_hash,document:document)
    let blob:Data
    do {
      var bytes=Data(),offset:UInt64=0
      while offset<receipt.checkpoint_byte_length.value {
        try Task.checkCancellation()
        let chunk=try await storage.readBlob(reader,offset:offset,count:min(65536,Int(receipt.checkpoint_byte_length.value-offset)))
        guard !chunk.isEmpty else {throw BlobError.corrupt};bytes.append(chunk);offset+=UInt64(chunk.count)
      }
      try await storage.closeBlob(reader);blob=bytes
    } catch {try? await storage.closeBlob(reader);throw error}
    let payload=try await Task.detached {
      guard KernelHashes.data(blob)==receipt.checkpoint_blob_hash else {throw BlobError.corrupt}
      let metadata=try RecoveryValidation.encode(Metadata(protocol_version:1,plan:stored.plan,receipt:receipt))
      guard metadata.count<=16*1024*1024 else {throw StorageError.corruptIdentity}
      var bytes=Data([79,77,75,82,1]);let length=UInt32(metadata.count)
      for index in 0..<4 {bytes.append(UInt8((length>>UInt32(index*8))&255))}
      bytes.append(metadata);bytes.append(blob);return (bytes,KernelHashes.data(bytes))
    }.value
    let operation="restore-"+UUID().uuidString.lowercased()
    return try await withTaskCancellationHandler {
      do {
        let start=try await client.kernelCommand(.kernel_restore_start(.init(type:.kernel_restore_start,operation_id:operation,expected_source_hash:expectedSourceHash,byte_length:try HostSerial(UInt64(payload.0.count)),payload_hash:payload.1)))
        guard start.phase == .blob,start.blob_offset.value?.value==0 else {throw NativeHostClientError.invalidReply}
        var offset=0
        while offset<payload.0.count {
          try Task.checkCancellation()
          let count=min(65536,payload.0.count-offset),chunk=Array(payload.0[offset..<offset+count]).map(UInt32.init)
          let actual=try await client.kernelCommand(.kernel_restore_write(.init(type:.kernel_restore_write,operation_id:operation,offset:try HostSerial(UInt64(offset)),bytes:chunk)))
          offset+=count
          guard actual.phase == .blob,actual.blob_offset.value?.value==UInt64(offset) else {throw NativeHostClientError.invalidReply}
        }
        var reply=try await client.kernelCommand(.kernel_restore_finish(.init(type:.kernel_restore_finish,operation_id:operation)))
        while reply.phase == .queued || reply.phase == .running {
          try Task.checkCancellation();try await Task.sleep(for:.milliseconds(10))
          reply=try await client.kernelCommand(.kernel_status(.init(type:.kernel_status,operation_id:operation)))
        }
        guard reply.phase == .completed,reply.active_checkpoint_id.value==receipt.checkpoint_id,
          reply.kernel_state_revision.value==receipt.kernel_state_revision.value,reply.receipt.value?.checkpoint_blob_hash==receipt.checkpoint_blob_hash else {throw NativeKernelRecoveryError.unavailable}
        return receipt
      } catch {
        let originalError=error
        _ = try? await client.cancel(operation:operation)
        // Cancellation must settle the original import worker before giving editing back;
        // otherwise its gate could remain held with nobody left to advance its completion.
        while let outcome=try? await client.kernelCommand(.kernel_status(.init(type:.kernel_status,operation_id:operation))) {
          if outcome.phase == .queued || outcome.phase == .running {try? await Task.sleep(for:.milliseconds(10));continue}
          if outcome.phase == .completed,outcome.active_checkpoint_id.value==receipt.checkpoint_id,
            outcome.receipt.value?.checkpoint_blob_hash==receipt.checkpoint_blob_hash {return receipt}
          break
        }
        throw originalError
      }
    } onCancel: {Task {_ = try? await client.cancel(operation:operation)}}
  }
}
