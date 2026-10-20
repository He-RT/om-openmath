import Foundation
import CryptoKit

struct NativeDraft:Sendable {
  let cellID:String
  let editorGeneration:UInt64
  var sequence:UInt64
  var baseRevision:UInt64
  var baseSource:String
  var source:String
  var composing:Bool
  var conflict:Bool
  var requiresReview:Bool=false
  var selections:[DraftTextRange]=[DraftTextRange(NSRange(location:0,length:0))]
  var markedRange:DraftTextRange?=nil
}
struct DraftAcknowledgement:Sendable {
  let cellID:String
  let editorGeneration:UInt64
  let sequence:UInt64
}
struct DraftFence:Sendable {
  let value:NativeEditorFence
  let gate:PhysicalCommitGate
}
/// MainActor owns actual native editor overlays; confirmed source is a read-only owner projection.
/// Callers report real hasMarkedText and native generations, never a model-supplied boolean.
@MainActor final class DraftStore {
  private(set) var confirmed:NativeSourceSnapshot
  let runtime:String
  let documentGeneration:UInt64
  private let clock:@Sendable ()->UInt64
  private var drafts:[Data:NativeDraft]=[:]
  private var fences:[String:DraftFence]=[:]
  private var alive=true
  private(set) var persistenceRevision:UInt64=0
  private(set) var recoveredOrphans:[DraftBackupCell]=[]
  var persistenceChanged:(()->Void)?
  private func notePersistence() {
    precondition(persistenceRevision<HostSerial.maximum)
    persistenceRevision+=1;persistenceChanged?()
  }
  init(source:NativeSourceSnapshot,runtime:String,generation:UInt64,clock:@escaping @Sendable ()->UInt64 = {DispatchTime.now().uptimeNanoseconds/1_000_000}) {
    self.confirmed=source;self.runtime=runtime;self.documentGeneration=generation;self.clock=clock
  }
  func attachEditor(cell:String,generation:UInt64) throws {
    guard alive,let index=confirmed.file.cells.firstIndex(where:{$0.id.utf8.elementsEqual(cell.utf8)}) else {throw CommitPortError.sourceConflict}
    let key=Data(cell.utf8)
    if let old=drafts[key] {
      guard !old.composing else {throw CommitPortError.editingBusy}
      drafts[key]=NativeDraft(cellID:old.cellID,editorGeneration:generation,sequence:old.sequence,baseRevision:old.baseRevision,baseSource:old.baseSource,source:old.source,composing:false,conflict:old.conflict)
      drafts[key]?.requiresReview=old.requiresReview;drafts[key]?.selections=old.selections;drafts[key]?.markedRange=nil
      for fence in fences.values where fence.value.targets.contains(where:{$0.cell_id.utf8.elementsEqual(cell.utf8)}) {fence.gate.invalidateForNewInput()}
      notePersistence();return
    }
    let source=confirmed.file.cells[index].source
    drafts[key]=NativeDraft(cellID:cell,editorGeneration:generation,sequence:0,baseRevision:confirmed.cell_revisions[index].revision.value,baseSource:source,source:source,composing:false,conflict:false)
    notePersistence()
  }
  /// Called synchronously by the native editor input/marked-text path. Never submits/ends IME.
  func edited(cell:String,generation:UInt64,source:String,hasMarkedText:Bool,selections:[DraftTextRange]?=nil,markedRange:DraftTextRange?=nil) throws {
    let key=Data(cell.utf8)
    guard alive,var draft=drafts[key],draft.editorGeneration==generation,draft.sequence<HostSerial.maximum else {throw CommitPortError.sourceConflict}
    if let selections {for selection in selections {try selection.validate(in:source)}}
    if let markedRange {guard hasMarkedText else {throw CommitPortError.sourceConflict};try markedRange.validate(in:source)}
    if draft.source.utf8.elementsEqual(source.utf8),draft.composing==hasMarkedText {
      if let selections, selections != draft.selections || markedRange != draft.markedRange {draft.selections=selections;draft.markedRange=markedRange;drafts[key]=draft;notePersistence()}
      return
    }
    draft.sequence+=1;draft.source=source;draft.composing=hasMarkedText;draft.requiresReview=false
    draft.selections=selections ?? [DraftTextRange(NSRange(location:0,length:0))];draft.markedRange=markedRange;drafts[key]=draft
    notePersistence()
    for fence in fences.values where fence.value.targets.contains(where:{$0.cell_id.utf8.elementsEqual(cell.utf8)}) {fence.gate.invalidateForNewInput()}
  }
  func editorState() throws ->NativeEditorState {
    let records=try drafts.values.sorted{$0.cellID.utf8.lexicographicallyPrecedes($1.cellID.utf8)}.map { draft in
      NativeEditorTarget(cell_id:draft.cellID,editor_generation:try .init(draft.editorGeneration),draft_sequence:try .init(draft.sequence),base_cell_revision:try .init(draft.baseRevision),source_hash:SHA256.hash(data:Data(draft.source.utf8)).map{String(format:"%02x",$0)}.joined(),is_composing:draft.composing,is_dirty:!draft.source.utf8.elementsEqual(draft.baseSource.utf8) || draft.conflict || draft.requiresReview)
    }
    return NativeEditorState(runtime_instance_id:runtime,document_id:confirmed.document_id,document_generation:try .init(documentGeneration),source_revision:confirmed.revision,targets:records)
  }
  func overlay(_ cell:String)->NativeDraft? {drafts[Data(cell.utf8)]}
  var hasSavableEdits:Bool {drafts.values.contains{!$0.composing && !$0.requiresReview && !$0.conflict && !$0.source.utf8.elementsEqual($0.baseSource.utf8)}}
  /// A sync caller persists this exact raw source before reads/run/save/agent edits.
  func pendingEdits(excludingMarked:Bool=false) throws ->[(NativeSourceCell,DraftAcknowledgement)] {
    guard alive else {throw CommitPortError.sourceConflict}
    var edits:[(NativeSourceCell,DraftAcknowledgement)]=[]
    for cell in confirmed.file.cells {
      guard let draft=drafts[Data(cell.id.utf8)] else {continue}
      if draft.composing || draft.requiresReview {if excludingMarked {continue};throw CommitPortError.editingBusy}
      if draft.conflict {throw CommitPortError.sourceConflict}
      if !draft.source.utf8.elementsEqual(draft.baseSource.utf8) {
        var changed=cell;changed.source=draft.source
        edits.append((changed,.init(cellID:cell.id,editorGeneration:draft.editorGeneration,sequence:draft.sequence)))
      }
    }
    return edits
  }
  /// Applying a receipt cannot overwrite later typing or force-end composition. Own draft ACKs
  /// advance only the acknowledged sequence; external changes retain an explicit conflict overlay.
  func acknowledge(_ source:NativeSourceSnapshot,ownDrafts:[DraftAcknowledgement]=[]) throws {
    guard alive,source.document_id==confirmed.document_id,source.revision.value>=confirmed.revision.value else {throw CommitPortError.sourceConflict}
    try SourceValidation.snapshot(source)
    for (key,var draft) in drafts {
      guard let index=source.file.cells.firstIndex(where:{$0.id.utf8.elementsEqual(draft.cellID.utf8)}) else {
        if !draft.source.utf8.elementsEqual(draft.baseSource.utf8) || draft.composing {draft.conflict=true;drafts[key]=draft}
        else {drafts.removeValue(forKey:key)}
        continue
      }
      let actual=source.file.cells[index].source
      let own=ownDrafts.first(where:{$0.cellID.utf8.elementsEqual(draft.cellID.utf8) && $0.editorGeneration==draft.editorGeneration})
      if let own {
        guard own.sequence<=draft.sequence else {throw CommitPortError.sourceConflict}
        draft.baseSource=actual;draft.baseRevision=source.cell_revisions[index].revision.value
        if own.sequence==draft.sequence && !draft.composing {
          if !draft.source.utf8.elementsEqual(actual.utf8) {draft.selections=draft.selections.map{$0.clamped(to:actual)}}
          draft.source=actual;draft.conflict=false
        }
      } else if !actual.utf8.elementsEqual(draft.baseSource.utf8) {
        if !draft.source.utf8.elementsEqual(draft.baseSource.utf8) || draft.composing {draft.conflict=true}
        else {draft.source=actual;draft.selections=draft.selections.map{$0.clamped(to:actual)}}
        draft.baseSource=actual;draft.baseRevision=source.cell_revisions[index].revision.value
      }
      drafts[key]=draft
    }
    confirmed=source
    notePersistence()
  }
  func backupSnapshot(owner:DraftBackupOwner,sequence:UInt64)throws->DraftBackupSnapshot {
    guard alive,owner.documentID==confirmed.document_id,owner.runtime==runtime,owner.documentGeneration==documentGeneration else {throw CommitPortError.sourceConflict}
    let cells=drafts.values.sorted{$0.cellID.utf8.lexicographicallyPrecedes($1.cellID.utf8)}.map {draft in
      DraftBackupCell(cellID:draft.cellID,editorGeneration:draft.editorGeneration,draftSequence:draft.sequence,baseCellRevision:draft.baseRevision,
        baseUTF8:Data(draft.baseSource.utf8),sourceUTF8:Data(draft.source.utf8),wasComposing:draft.composing,conflict:draft.conflict,
        requiresReview:draft.requiresReview,selections:draft.selections,markedRange:draft.markedRange)
    } + recoveredOrphans
    let snapshot=DraftBackupSnapshot(codecVersion:1,owner:owner,sequence:sequence,sourceRevision:confirmed.revision.value,sourceSnapshotHash:confirmed.snapshot_hash,cells:cells)
    try DraftBackupValidation.snapshot(snapshot);return snapshot
  }
  /// Old native generations are provenance only. Restored text has no live system composition
  /// or write permission; a user action must confirm dirty text before it can be synchronized.
  func restoreBackup(_ stored:StoredDraftBackup)throws {
    guard alive,drafts.isEmpty,stored.snapshot.owner.documentID==confirmed.document_id else {throw CommitPortError.sourceConflict}
    try DraftBackupValidation.snapshot(stored.snapshot)
    for record in stored.snapshot.cells {
      guard let index=confirmed.file.cells.firstIndex(where:{$0.id.utf8.elementsEqual(record.cellID.utf8)}) else {recoveredOrphans.append(record);continue}
      let actual=confirmed.file.cells[index].source,raw=String(data:record.sourceUTF8,encoding:.utf8)!,base=String(data:record.baseUTF8,encoding:.utf8)!
      let alreadyConfirmed=actual.utf8.elementsEqual(raw.utf8) && !record.wasComposing && !record.conflict
      let conflict=record.conflict || (!actual.utf8.elementsEqual(base.utf8) && !alreadyConfirmed)
      var value=NativeDraft(cellID:record.cellID,editorGeneration:0,sequence:0,baseRevision:confirmed.cell_revisions[index].revision.value,baseSource:actual,source:raw,composing:false,conflict:conflict)
      value.requiresReview = !alreadyConfirmed || record.requiresReview
      value.selections=record.selections
      drafts[Data(record.cellID.utf8)]=value
    }
    notePersistence()
  }
  func reviewRecovered(cell:String,keep:Bool)throws {
    let key=Data(cell.utf8)
    guard alive,var draft=drafts[key],draft.requiresReview,!draft.composing else {throw CommitPortError.sourceConflict}
    if !keep {draft.source=draft.baseSource;draft.selections=[DraftTextRange(NSRange(location:0,length:0))]}
    draft.requiresReview=false;draft.conflict=false;draft.markedRange=nil;draft.sequence+=1;drafts[key]=draft;notePersistence()
  }
  /// Freeze current existing target owners after sync; TTL never spans CAS/network/animation.
  func acquireFence(before:NativeSourceSnapshot,targets:[String],ttlMS:UInt64=1000,manualAfter:NativeSourceSnapshot?=nil) throws ->DraftFence {
    guard alive,before.document_id==confirmed.document_id,before.snapshot_hash==confirmed.snapshot_hash,ttlMS>0,ttlMS<=2000,targets.count<=1000 else {throw CommitPortError.invalidFence}
    let now=clock();guard now<HostSerial.maximum,now+ttlMS<=HostSerial.maximum else {throw CommitPortError.invalidFence}
    var seen=Set<Data>(),records:[NativeEditorTarget]=[]
    for cellID in targets {
      guard seen.insert(Data(cellID.utf8)).inserted,let index=before.file.cells.firstIndex(where:{$0.id.utf8.elementsEqual(cellID.utf8)}) else {throw CommitPortError.invalidFence}
      let original=before.file.cells[index]
      let actual=manualAfter?.file.cells.first(where:{$0.id.utf8.elementsEqual(cellID.utf8)}) ?? original
      let draft=drafts[Data(cellID.utf8)]
      if let draft {
        guard !draft.composing,!draft.conflict,!draft.requiresReview,draft.source.utf8.elementsEqual(actual.source.utf8),draft.baseRevision==before.cell_revisions[index].revision.value else {throw CommitPortError.editingBusy}
      }
      records.append(.init(cell_id:cellID,editor_generation:try .init(draft?.editorGeneration ?? 0),draft_sequence:try .init(draft?.sequence ?? 0),base_cell_revision:before.cell_revisions[index].revision,source_hash:SHA256.hash(data:Data(actual.source.utf8)).map{String(format:"%02x",$0)}.joined(),is_composing:false,is_dirty:manualAfter != nil))
    }
    let id=UUID().uuidString.lowercased()
    let value=NativeEditorFence(protocol_version:1,fence_id:id,runtime_instance_id:runtime,document_id:confirmed.document_id,document_generation:try .init(documentGeneration),source_revision:confirmed.revision,source_snapshot_hash:confirmed.snapshot_hash,issued_ms:try .init(now),expires_ms:try .init(now+ttlMS),targets:records)
    let result=DraftFence(value:value,gate:PhysicalCommitGate(deadlineMS:now+ttlMS,now:clock));fences[id]=result;return result
  }
  func releaseFence(_ id:String) {fences.removeValue(forKey:id)?.gate.settle()}
  func invalidateDocument() {alive=false;for fence in fences.values {fence.gate.invalidateForNewInput()}}
}
