import Foundation

struct NativeDecodedEvent: Sendable {
  var envelope: EventEnvelope
  var operation: HostOperationStatus?
}
struct NativeDecodedBatch: Sendable {
  var needsResync: Bool
  var lastRustSequence:UInt64=0
  var events: [NativeDecodedEvent]
}
struct HostConfirmedProjection: Sendable {
  let runtime: String
  var hostPhase: HostPhase = .starting
  var rustSequence: UInt64 = 0
  var uiSequence: UInt64 = 0
  var needsResync = true
  var documentBinding: DocumentBinding?
  var storageReady = false
  var operations: [String: HostOperationStatus] = [:]
}

/// Pure scope/sequence reducer. It handles facts, never executes or rewrites notebook source.
/// Rust and each local producer keep separate counters. uiSequence is assigned by this router.
struct AppEventRouter: Sendable {
  private(set) var projection: HostConfirmedProjection
  private var requiredRustSequence:UInt64=0
  private var terminalOrder: [String] = []
  private var localSources: [String: (identity:String,generation:UInt64,sequence:UInt64)] = [:]
  init(runtime: String, document: DocumentBinding? = nil) {
    projection = .init(runtime: runtime, documentBinding: document)
  }
  mutating func registerLocalProducer(_ name:String, identity:String, generation:UInt64) {
    localSources[name] = (identity,generation,0)
  }
  mutating func applyLocal(_ name:String, identity:String, generation:UInt64, sequence:UInt64) throws -> Bool {
    guard projection.hostPhase != .closed,projection.hostPhase != .failed,let source=localSources[name], source.identity==identity, source.generation==generation,
      sequence > source.sequence, sequence <= HostSerial.maximum else { return false }
    guard sequence==source.sequence+1 else { return false } // that source must supply its own baseline
    localSources[name]=(identity,generation,sequence)
    try changed()
    return true
  }
  mutating func install(_ snapshot:HostSnapshot) throws -> Bool {
    guard (projection.hostPhase != .closed || snapshot.host_phase == .closed),snapshot.protocol_version==1, snapshot.runtime_instance_id==projection.runtime,
      snapshot.rust_event_sequence.value>=max(projection.rustSequence,requiredRustSequence),
      sameDocumentGeneration(snapshot.document_binding.value,projection.documentBinding) else { return false }
    if let old=projection.documentBinding, let new=snapshot.document_binding.value {
      guard new.document_revision.value>=old.document_revision.value,
        new.execution_epoch.value>=old.execution_epoch.value else { return false }
    }
    var next:[String:HostOperationStatus]=[:]
    for var operation in snapshot.operations {
      // Terminal math is immutable. A previously confirmed local copy stays usable when the
      // server omits details; its original provenance is retained, not recomputed or replayed.
      if operation.result.value==nil, let previous=projection.operations[operation.operation_ref],
        previous.phase==operation.phase, previous.phase.isTerminal {
        operation.result=previous.result
      }
      next[operation.operation_ref]=operation
    }
    projection.operations=next
    terminalOrder=terminalOrder.filter { next[$0] != nil }
    for op in snapshot.operations where op.phase.isTerminal && !terminalOrder.contains(op.operation_ref) {
      terminalOrder.append(op.operation_ref)
    }
    projection.hostPhase=snapshot.host_phase
    projection.documentBinding=snapshot.document_binding.value
    projection.storageReady=snapshot.document_storage_ready
    projection.rustSequence=snapshot.rust_event_sequence.value
    projection.needsResync=false
    trim()
    try changed()
    return true
  }
  /// Returned terminal facts can resolve a request only after its separate admission is confirmed.
  mutating func apply(_ batch:NativeDecodedBatch) throws -> [HostOperationStatus] {
    requiredRustSequence=max(requiredRustSequence,batch.lastRustSequence)
    guard projection.hostPhase != .closed else { return [] }
    if batch.needsResync { projection.needsResync=true }
    var terminal:[HostOperationStatus]=[]
    for event in batch.events {
      let envelope=event.envelope
      guard envelope.protocol_version==1, envelope.runtime_instance_id==projection.runtime else { continue }
      let sequence=envelope.rust_event_sequence.value
      requiredRustSequence=max(requiredRustSequence,sequence)
      guard sequence>projection.rustSequence else { continue }
      guard !projection.needsResync, sequence==projection.rustSequence+1 else {
        projection.needsResync=true
        continue
      }
      // Consume the global Rust counter even for old document scope, without applying its data.
      projection.rustSequence=sequence
      guard sameDocument(envelope.document_binding.value,projection.documentBinding) else { continue }
      switch envelope.event_kind {
      case .host_ready: projection.hostPhase = .ready; try changed()
      case .host_failed: projection.hostPhase = .failed; try changed()
      case .operation_progress, .operation_finished:
        guard let operation=event.operation, envelope.operation_ref.value==operation.operation_ref else {
          projection.needsResync=true; continue
        }
        if let old=projection.operations[operation.operation_ref], old.phase.isTerminal {
          if old.phase != operation.phase { projection.needsResync=true }
          continue
        }
        projection.operations[operation.operation_ref]=operation
        if operation.phase.isTerminal {
          terminalOrder.append(operation.operation_ref)
          terminal.append(operation)
        }
        trim(); try changed()
      default:
        // A new owner event requires its confirmed baseline until its real delta adapter exists.
        projection.needsResync=true
      }
    }
    return terminal
  }
  mutating func confirmClosed() throws { projection.hostPhase = .closed; try changed() }
  mutating func markFailed() throws { projection.hostPhase = .failed; try changed() }
  private mutating func changed() throws {
    guard projection.uiSequence<HostSerial.maximum else { throw HostContractError.invalidSerial }
    projection.uiSequence += 1
  }
  private mutating func trim() {
    while terminalOrder.count>32 {
      let expired=terminalOrder.removeFirst()
      if projection.operations[expired]?.phase.isTerminal==true { projection.operations.removeValue(forKey:expired) }
    }
  }
}
extension HostOperationStatusPhase {
  var isTerminal:Bool { self == .completed || self == .cancelled || self == .failed }
}
func sameDocumentGeneration(_ a:DocumentBinding?,_ b:DocumentBinding?)->Bool {
  switch (a,b) {
  case (nil,nil): return true
  case (.some(let a),.some(let b)): return a.document_id==b.document_id && a.generation.value==b.generation.value
  default: return false
  }
}
func sameDocument(_ a:DocumentBinding?,_ b:DocumentBinding?)->Bool {
  sameDocumentGeneration(a,b) && a?.document_revision.value==b?.document_revision.value
    && a?.execution_epoch.value==b?.execution_epoch.value
}
