import AppKit
import Foundation
import UniformTypeIdentifiers

extension UTType {static let openMathNotebook=UTType(exportedAs:"org.openmath.notebook",conformingTo:.json)}
private final class NativeDocumentSnapshot:@unchecked Sendable {
  private let lock=NSLock()
  private var bytes:Data?
  func read()throws->Data {try lock.withLock {guard let bytes else {throw SaveError.sourceChanged};return bytes}}
  func replace(_ bytes:Data) {lock.withLock {self.bytes=bytes}}
}
/// Native document façade. Source authority/IME lives in the existing ports; this class owns only
/// file/window presentation and frozen bytes for AppKit's data API, never an evaluator copy.
@objc(OMNativeDocument) @MainActor final class NativeDocument:NSDocument {
  private nonisolated let frozen=NativeDocumentSnapshot()
  private var port:NativeFilePort?
  private(set) var loaded:OpenedNotebook?
  private var edited=true
  private var saving=false
  private var dead=false
  private var closeApproved=false
  private var bindingGeneration:UInt64=0
  private(set) var statusText="未保存"
  var changed:(()->Void)?
  var beforeSave:(()async throws->Void)?
  private var autosaveOwner:NativeAutosave?
  private var autosave:NativeAutosave {
    if let autosaveOwner {return autosaveOwner}
    let owner=NativeAutosave(idleDelay:.seconds(2),maximumDelay:.seconds(10)) { [weak self] in
      guard let self,!self.dead,let port=self.port,self.fileURL != nil else {return true}
      if self.saving {return false}
      let state=try await port.state()
      guard state.hasSavableWork else {return true}
      _ = try await self.saveNative(automatic:true)
      return !(try await port.state()).hasSavableWork
    }
    autosaveOwner=owner;return owner
  }
  override class var autosavesInPlace:Bool {false}
  // AppKit's default timer can create an "autosave elsewhere" writer with an opaque URL.
  // The owned autosave controller is the sole timer and always uses the durable intent port.
  override func scheduleAutosaving() {}
  override nonisolated class var readableTypes:[String] {["org.openmath.notebook"]}
  override nonisolated class var writableTypes:[String] {["org.openmath.notebook"]}
  override nonisolated class func isNativeType(_ type:String)->Bool {type=="org.openmath.notebook"}
  override func defaultDraftName()->String {"未命名笔记本"}
  override var isDocumentEdited:Bool {edited || super.isDocumentEdited}
  func attach(_ port:NativeFilePort,opened:OpenedNotebook?=nil) {self.port=port;self.loaded=opened;fileType="org.openmath.notebook";if let opened {bindingGeneration=opened.binding.revision;fileURL=URL(fileURLWithPath:opened.binding.displayPath);frozen.replace(opened.bytes);edited=false;updateChangeCount(.changeCleared);statusText="已打开"}}
  /// Real native editor/confirmed-source notifications call this; programmatic echoes do not.
  func noteEdit() {guard !dead else {return};closeApproved=false;edited=true;updateChangeCount(.changeDone);statusText="未保存";changed?();if fileURL != nil {autosave.edited()}}
  override nonisolated func data(ofType typeName:String)throws->Data {try frozen.read()}
  override nonisolated func read(from data:Data,ofType typeName:String)throws {
    _ = try NotebookFileCodec.decode(data);frozen.replace(data)
  }
  static func opened(_ url:URL,document:String,files:NativeFileService) async throws->OpenedNotebook {try await files.open(url,document:document)}
  /// Native host uses the same private target and save operation whether invoked by Cmd-S,
  /// menu/panel or document close. No background URL comes from a model argument.
  func saveNative(to target:URL?=nil,operation:String="save-"+UUID().uuidString.lowercased(),faults:StorageFaults = .init(),automatic:Bool=false) async throws->SaveReceipt {
    guard !dead,!saving,let port else {throw SaveError.busy}
    saving=true;statusText="正在保存";changed?();defer {saving=false;changed?()}
    let countToken=changeCountToken(for:.saveOperation)
    do {
      if let target {
        let head=try await port.chooseTarget(target,faults:faults)
        bindingGeneration=head.binding.revision
        // The explicitly chosen target is visible immediately, but stays dirty until confirmed.
        fileURL=target;fileType="org.openmath.notebook";edited=true
      }
      let startedBinding=bindingGeneration
      try await beforeSave?()
      let snapshot=try await port.snapshot();frozen.replace(snapshot.bytes)
      let receipt=try await port.save(operation:operation,snapshot:snapshot,faults:faults)
      let state=try await port.state()
      if startedBinding==bindingGeneration,receipt.bindingRevision==state.head?.binding.revision {
        edited=state.dirty;updateChangeCount(withToken:countToken,for:.saveOperation)
        edited=edited || super.isDocumentEdited
        if edited {statusText="已保存快照，后续修改尚未保存"} else {statusText="已保存"}
      } else {edited=true;statusText="原目标已保存，当前目标尚未保存"}
      if !automatic {autosave.manualSaved(clean:!state.hasSavableWork)}
      return receipt
    } catch {
      edited=true
      statusText=(error as? SaveError) == .externalConflict ? "文件已被其他应用修改，请另存为或核对" : "保存失败，修改已保留"
      if (error as? SaveError) == .unknownOutcome {statusText="保存结果待核对，修改已保留"}
      autosave.suspend()
      throw error
    }
  }
  func reconcileSave() async throws {
    guard !dead,!saving,let port else {throw SaveError.busy}
    let before=try await port.state()
    guard let operation=before.unknownOperation else {return}
    _ = try await port.reconcile(operation)
    let state=try await port.state();edited=state.dirty
    if !edited {updateChangeCount(.changeCleared)}
    statusText=edited ? "已核对，仍有修改尚未保存" : "已核对并保存"
    changed?();autosave.manualSaved(clean:!state.hasSavableWork)
  }
  /// System save panel grants the target. Cancel leaves the original binding and local source.
  func saveAsWithPanel() async throws->SaveReceipt? {
    guard !dead else {throw SaveError.closing}
    NSApplication.shared.activate()
    let panel=NSSavePanel();panel.allowedContentTypes=[.openMathNotebook];panel.canCreateDirectories=true
    panel.nameFieldStringValue=fileURL?.lastPathComponent ?? "未命名.omnb"
    let response=await withCheckedContinuation {continuation in
      if let window=windowControllers.first?.window {panel.beginSheetModal(for:window){continuation.resume(returning:$0)}} else {panel.begin{continuation.resume(returning:$0)}}
    }
    guard response == .OK,let url=panel.url else {return nil}
    return try await saveNative(to:url)
  }
  static func openWithPanel() async->URL? {
    NSApplication.shared.activate()
    let panel=NSOpenPanel();panel.allowedContentTypes=[.openMathNotebook];panel.allowsMultipleSelection=false;panel.canChooseDirectories=false
    let response=await withCheckedContinuation {continuation in panel.begin{continuation.resume(returning:$0)}}
    return response == .OK ? panel.url : nil
  }
  override func save(_ sender:Any?) {
    Task {do {if fileURL==nil {_ = try await saveAsWithPanel()} else {_ = try await saveNative()}} catch {presentError(error)}}
  }
  override func saveAs(_ sender:Any?) {Task {do {_ = try await saveAsWithPanel()} catch {presentError(error)}}}
  override func save(to url:URL,ofType typeName:String,for saveOperation:NSDocument.SaveOperationType,completionHandler:@escaping (Error?)->Void) {
    Task {
      defer {unblockUserInteraction()}
      do {
        guard saveOperation == .saveOperation || saveOperation == .saveAsOperation || saveOperation == .autosaveInPlaceOperation else {throw SaveError.unsafeTarget}
        if saveOperation == .autosaveInPlaceOperation, url != fileURL {throw SaveError.bindingChanged}
        _ = try await saveNative(to:saveOperation == .saveAsOperation || fileURL==nil ? url : nil,automatic:saveOperation == .autosaveInPlaceOperation)
        completionHandler(nil)
      } catch {completionHandler(error)}
    }
  }
  /// Close guard keeps local data/error visible; a failed save never dismisses the document.
  func prepareToClose() async->Bool {
    closeApproved=false
    autosave.suspend()
    while saving {try? await Task.sleep(for:.milliseconds(5))}
    guard let port,let state=try? await port.state() else {return false}
    if !state.dirty,state.unknownOperation==nil {closeApproved=true;return true}
    if state.unknownOperation != nil {statusText="保存结果待核对，请先核对后关闭";changed?();return false}
    let alert=NSAlert();alert.messageText="关闭前保存笔记本？";alert.informativeText=state.hasDrafts ? "尚有未提交输入。请保存或继续编辑，当前输入不会被强制结束。" : "当前源码已提交到本地，尚未写入用户文件。"
    alert.addButton(withTitle:"保存");alert.addButton(withTitle:"继续编辑");alert.addButton(withTitle:"关闭，保留本地修改")
    if state.hasDrafts {alert.buttons.last?.isEnabled=false}
    let result=alert.runModal()
    if result == .alertSecondButtonReturn {return false}
    if result == .alertThirdButtonReturn {closeApproved=true;return true}
    do {if fileURL==nil {guard try await saveAsWithPanel() != nil else {return false}} else {_ = try await saveNative()};closeApproved = !(try await port.state()).dirty;return closeApproved} catch {presentError(error);return false}
  }
  func closeNative(approvalRequired:Bool=false) async throws {
    if approvalRequired && !closeApproved {throw SaveError.sourceChanged}
    await autosave.finish()
    if approvalRequired && !closeApproved {throw SaveError.sourceChanged}
    dead=true;await port?.close();super.close()
  }
}
