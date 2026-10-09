import Foundation

/// Native UndoManager consumes its command only after the actual inverse transaction is durable.
private final class DocumentUndoManager:UndoManager {
  weak var owner:UndoCoordinator?
  private var consuming=false
  override var canUndo:Bool {
    let available=super.canUndo
    return MainActor.assumeIsolated {available && (consuming || owner?.permits(.undo)==true)}
  }
  override var canRedo:Bool {
    let available=super.canRedo
    return MainActor.assumeIsolated {available && (consuming || owner?.permits(.redo)==true)}
  }
  override func undo() {MainActor.assumeIsolated {owner?.request(.undo)}}
  override func redo() {MainActor.assumeIsolated {owner?.request(.redo)}}
  @MainActor func consume(_ direction:UndoCoordinator.Direction) {
    consuming=true;defer {consuming=false}
    if direction == .undo {super.undo()} else {super.redo()}
  }
}
@MainActor private final class UndoEntry:NSObject {
  let group:String
  let name:String
  var transactions:[String]
  init(group:String,name:String,transactions:[String]) {self.group=group;self.name=name;self.transactions=transactions}
}
/// The native undo stack stores confirmed transaction identities, never an old notebook snapshot.
/// The callback must use DocCommitPort; an animation/native UndoManager callback cannot prove IO.
@MainActor final class UndoCoordinator {
  enum Direction {case undo,redo}
  enum Phase {case ready,pending,unknown,failed,closed}
  private let native=DocumentUndoManager()
  private let perform:@Sendable ([String],String,String) async throws ->NativeCommitState
  private var undoEntries:[UndoEntry]=[],redoEntries:[UndoEntry]=[]
  private var seen:[String:String]=[:]
  private var queued:[(NativeCommitState,String,String)]=[]
  private var pending:(entry:UndoEntry,direction:Direction,operation:String)?
  private var confirmedInverse:NativeCommitState?
  private var alive=true
  private(set) var phase:Phase = .ready
  private(set) var lastOperation:String?
  private(set) var error:String?
  var manager:UndoManager {native}
  init(perform:@escaping @Sendable ([String],String,String) async throws ->NativeCommitState) {
    self.perform=perform;native.owner=self;native.groupsByEvent=false;native.levelsOfUndo=200
  }
  /// Called only for an actual completed source receipt. Echo of the same operation is ignored.
  func registerCommitted(_ state:NativeCommitState,group:String,name:String) throws {
    guard alive,state.phase == .completed,let receipt=state.receipt.value,
      let transaction=receipt.receipt.transaction_id.value,receipt.receipt.operation_id==state.operation_id,
      receipt.receipt.request_hash==state.request_hash else {throw CommitPortError.sourceConflict}
    if let hash=seen[state.operation_id] {
      guard hash==state.request_hash else {throw StorageError.idempotencyConflict};return
    }
    if pending?.operation==state.operation_id {return}
    guard seen.count<4096 else {throw CommitPortError.sourceConflict}
    if pending != nil {
      if queued.contains(where:{$0.0.operation_id==state.operation_id}) {return}
      guard queued.count<128 else {throw CommitPortError.operationInProgress}
      queued.append((state,group,name));return
    }
    seen[state.operation_id]=state.request_hash
    if let entry=undoEntries.last,entry.group==group,entry.transactions.count<32 {
      entry.transactions.append(transaction);return
    }
    let entry=UndoEntry(group:group,name:name,transactions:[transaction])
    undoEntries.append(entry);redoEntries.removeAll()
    if undoEntries.count>200 {undoEntries.removeFirst()}
    register(entry,initial:true)
  }
  private func register(_ entry:UndoEntry,initial:Bool) {
    if initial {native.beginUndoGrouping()}
    native.registerUndo(withTarget:entry) { [weak self] entry in
      MainActor.assumeIsolated {self?.consume(entry)}
    }
    native.setActionName(entry.name)
    if initial {native.endUndoGrouping()}
  }
  fileprivate func permits(_ direction:Direction)->Bool {
    alive && pending==nil && !(direction == .undo ? undoEntries : redoEntries).isEmpty
  }
  fileprivate func request(_ direction:Direction) {
    guard permits(direction),let entry=(direction == .undo ? undoEntries : redoEntries).last else {return}
    pending=(entry,direction,UUID().uuidString.lowercased());start()
  }
  /// Unknown outcomes retry the original ID and let DocCommitPort read the actual stored receipt.
  func reconcile() {guard pending != nil,phase == .unknown else {return};start()}
  private func start() {
    guard let pending,phase != .pending else {return}
    phase = .pending;error=nil;lastOperation=pending.operation
    let transactions=pending.entry.transactions,group=pending.entry.group,operation=pending.operation
    Task { [weak self,perform] in
      do {
        let actual=try await perform(transactions,group,operation)
        guard let self,self.alive,self.pending?.operation==operation else {return}
        guard actual.phase == .completed,actual.receipt.value?.receipt.transaction_id.value != nil else {
          self.phase = .failed;self.error="撤销未提交";self.pending=nil;try? self.flushQueued();return
        }
        self.confirmedInverse=actual
        self.native.consume(pending.direction)
        self.confirmedInverse=nil;self.pending=nil;self.phase = .ready
        try self.flushQueued()
      } catch CommitPortError.unknownOutcome {
        guard let self,self.alive else {return};self.phase = .unknown;self.error="撤销结果待核对"
      } catch {
        guard let self,self.alive else {return};self.phase = .failed;self.error="撤销未提交，原内容保留";self.pending=nil
        try? self.flushQueued()
      }
    }
  }
  private func consume(_ entry:UndoEntry) {
    guard let pending,pending.entry===entry,let actual=confirmedInverse,
      let transaction=actual.receipt.value?.receipt.transaction_id.value else {return}
    seen[actual.operation_id]=actual.request_hash
    let inverse=UndoEntry(group:UUID().uuidString.lowercased(),name:entry.name,transactions:[transaction])
    if pending.direction == .undo {undoEntries.removeLast();redoEntries.append(inverse)}
    else {redoEntries.removeLast();undoEntries.append(inverse)}
    register(inverse,initial:false)
  }
  private func flushQueued() throws {
    let ready=queued;queued.removeAll()
    for (state,group,name) in ready {try registerCommitted(state,group:group,name:name)}
  }
  func close() {alive=false;phase = .closed;native.removeAllActions();queued.removeAll();undoEntries.removeAll();redoEntries.removeAll()}
}
