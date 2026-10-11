import Foundation
import Combine

/// MainActor freezes native text; storage owns serialization and actual durability off-thread.
/// Debounce is 500 ms/max 2 s. Original sequence readback resolves unknown ACKs before retry.
@MainActor final class NativeDraftBackup:ObservableObject {
  @Published private(set) var status="尚未本地备份"
  private let storage:StorageService,store:DraftStore,owner:DraftBackupOwner
  private let faults:StorageFaults
  private var sequence:UInt64=0
  private struct Pending {let snapshot:DraftBackupSnapshot;let revision:UInt64;let hash:String}
  private var unknown:Pending?
  private var confirmedRevision:UInt64?
  private var idle:Task<Void,Never>?,maximum:Task<Void,Never>?,running:Task<Void,any Error>?
  private var stopped=false,paused=false
  private var runSerial:UInt64=0
  private(set) var failure:(any Error)?
  init(storage:StorageService,store:DraftStore,owner:DraftBackupOwner,faults:StorageFaults = .init()) {
    self.storage=storage;self.store=store;self.owner=owner
    self.faults=faults
    store.persistenceChanged={ [weak self] in self?.changed() }
  }
  var isCurrent:Bool {confirmedRevision==store.persistenceRevision && unknown==nil}
  private func changed() {
    guard !stopped else {return};status=paused ? (unknown==nil ? "本地备份失败，输入已保留" : "本地备份结果待核对，输入已保留") : "尚未本地备份"
    guard !paused else {return}
    idle?.cancel();idle=Task { [weak self] in do {try await Task.sleep(for:.milliseconds(500))} catch {return};self?.fire()}
    if maximum==nil {maximum=Task { [weak self] in do {try await Task.sleep(for:.seconds(2))} catch {return};self?.fire()}}
  }
  private func fire() {
    idle?.cancel();idle=nil;maximum?.cancel();maximum=nil
    guard !stopped,!paused,running==nil else {return}
    let (task,serial)=startWrite()
    Task { [weak self] in guard let self else {return};do {try await task.value} catch {};if self.runSerial==serial {self.running=nil};if !self.paused,!self.isCurrent {self.changed()}}
  }
  private func startWrite()->(Task<Void,any Error>,UInt64) {
    if let running {return (running,runSerial)}
    runSerial+=1;let serial=runSerial
    let task=Task { [weak self] in guard let self else {return};try await self.writeCurrent()};running=task;return (task,serial)
  }
  private func writeCurrent() async throws {
    guard !stopped else {throw StorageError.closing}
    do {
      if let unknown {
        var receipt=try await storage.draftBackupReceipt(document:owner.documentID,owner:owner.ownerID,sequence:unknown.snapshot.sequence)
        if receipt==nil {receipt=try await storage.backupDrafts(unknown.snapshot,faults:faults)} // Same frozen ID, after durable absence readback.
        guard receipt?.snapshotHash==unknown.hash else {throw StorageError.recoveryRequired}
        confirmedRevision=unknown.revision
        self.unknown=nil
      }
      if isCurrent {failure=nil;paused=false;status="已本地备份";return}
      guard sequence<HostSerial.maximum else {throw StorageError.corruptIdentity};sequence+=1
      let revision=store.persistenceRevision,snapshot=try store.backupSnapshot(owner:owner,sequence:sequence)
      status="正在本地备份"
      let hash=try await Task.detached {KernelHashes.data(try DraftBackupValidation.encode(snapshot))}.value
      let receipt:DraftBackupReceipt
      do {receipt=try await storage.backupDrafts(snapshot,faults:faults)}
      catch StorageError.unknownCommit {unknown=Pending(snapshot:snapshot,revision:revision,hash:hash);throw StorageError.unknownCommit(EIO)}
      guard receipt.owner==owner,receipt.sequence==snapshot.sequence,receipt.snapshotHash==hash else {throw StorageError.corruptIdentity}
      confirmedRevision=revision;status=isCurrent ? "已本地备份" : "新输入尚未本地备份";failure=nil;paused=false
    } catch {failure=error;paused=true;status=unknown==nil ? "本地备份失败，输入已保留" : "本地备份结果待核对，输入已保留";throw error}
  }
  func flush() async throws {
    idle?.cancel();idle=nil;maximum?.cancel();maximum=nil
    while running != nil || !isCurrent {
      let (task,serial)=startWrite()
      do {try await task.value} catch {if runSerial==serial {running=nil};throw error}
      if runSerial==serial {running=nil}
    }
  }
  func finish() async throws {try await flush();stop()}
  func stop() {stopped=true;idle?.cancel();maximum?.cancel();idle=nil;maximum=nil;store.persistenceChanged=nil}
}
