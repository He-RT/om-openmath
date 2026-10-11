import AppKit
import UniformTypeIdentifiers

/// Deliberately no SwiftUI/NSDocument/kernel/storage. This fixture separates the actual system
/// picker from the document owner; its only writable data is a synthetic acceptance file.
@MainActor final class FilePanelFixture:NSObject,NSApplicationDelegate {
  let folder:URL
  private var window:NSWindow!
  private var panel:NSSavePanel?
  private let label=NSTextField(labelWithString:"尚未选择文件")
  init(folder:URL) {self.folder=folder}
  private func record(_ value:[String:Any]) {
    do {let data=try JSONSerialization.data(withJSONObject:value,options:[.sortedKeys]);try data.write(to:folder.deletingLastPathComponent().appendingPathComponent("last-panel-receipt.json"),options:.atomic)}
    catch {NSLog("Fixture receipt unavailable: %@",error.localizedDescription)}
  }
  func applicationDidFinishLaunching(_ notification:Notification) {
    window=NSWindow(contentRect:NSRect(x:0,y:0,width:680,height:280),styleMask:[.titled,.closable,.miniaturizable,.resizable],backing:.buffered,defer:false)
    window.title="OpenMath File Panel Fixture";window.center()
    let stack=NSStackView();stack.orientation = .vertical;stack.spacing=16;stack.alignment = .leading
    let plain=NSButton(title:"标准 JSON 保存",target:self,action:#selector(plainSave));plain.setAccessibilityIdentifier("fixture-json-save")
    let custom=NSButton(title:"OpenMath 类型保存",target:self,action:#selector(customSave));custom.setAccessibilityIdentifier("fixture-omnb-save")
    let open=NSButton(title:"打开合成笔记本",target:self,action:#selector(openFile));open.setAccessibilityIdentifier("fixture-open")
    let modal=NSButton(title:"无类型限制的模态保存",target:self,action:#selector(modalSave));modal.setAccessibilityIdentifier("fixture-modal-save")
    for view in [plain,custom,open,modal,label] {stack.addArrangedSubview(view)}
    stack.translatesAutoresizingMaskIntoConstraints=false;window.contentView!.addSubview(stack)
    NSLayoutConstraint.activate([stack.leadingAnchor.constraint(equalTo:window.contentView!.leadingAnchor,constant:24),stack.topAnchor.constraint(equalTo:window.contentView!.topAnchor,constant:24),stack.trailingAnchor.constraint(lessThanOrEqualTo:window.contentView!.trailingAnchor,constant:-24)])
    let menu=NSMenu(),item=NSMenuItem();menu.addItem(item);let appMenu=NSMenu();appMenu.addItem(withTitle:"退出",action:#selector(NSApplication.terminate(_:)),keyEquivalent:"q");item.submenu=appMenu;NSApplication.shared.mainMenu=menu
    window.makeKeyAndOrderFront(nil);NSApplication.shared.activate()
  }
  @objc private func plainSave() {save(type:.json,name:"fixture.json")}
  @objc private func customSave() {save(type:UTType(exportedAs:"org.openmath.notebook",conformingTo:.json),name:"fixture.omnb")}
  @objc private func modalSave() {
    let chosen=NSSavePanel();chosen.directoryURL=folder;chosen.nameFieldStringValue="unrestricted.omnb"
    let response=chosen.runModal();label.stringValue="无限制模态response=\(response.rawValue)"
    record(["mode":"unrestricted_modal","response":response.rawValue,"url":chosen.url?.path ?? "","window_visible":window.isVisible])
  }
  private func save(type:UTType,name:String) {
    let chosen=NSSavePanel();chosen.allowedContentTypes=[type];chosen.directoryURL=folder
    chosen.nameFieldStringValue=name;chosen.canCreateDirectories=true;panel=chosen
    label.stringValue="目录可写：\(FileManager.default.isWritableFile(atPath:folder.path))；类型：\(type.identifier)"
    chosen.beginSheetModal(for:window) { [weak self,chosen] response in
      MainActor.assumeIsolated {
        guard let self else {return}
        if response == .OK,let url=chosen.url {
          do {try Data("{\"version\":1,\"title\":\"panel fixture\",\"cells\":[]}".utf8).write(to:url);self.label.stringValue="实际写入："+url.lastPathComponent}
          catch {self.label.stringValue="写入失败："+error.localizedDescription}
        } else {self.label.stringValue="已取消；response=\(response.rawValue)"}
        self.record(["mode":type.identifier,"response":response.rawValue,"url":chosen.url?.path ?? "","window_visible":self.window.isVisible])
        self.panel=nil
      }
    }
  }
  @objc private func openFile() {
    let chosen=NSOpenPanel();chosen.allowedContentTypes=[UTType(exportedAs:"org.openmath.notebook",conformingTo:.json)];chosen.directoryURL=folder;panel=chosen
    chosen.beginSheetModal(for:window) { [weak self,chosen] response in
      MainActor.assumeIsolated {self?.label.stringValue=response == .OK ? "已选择："+(chosen.url?.lastPathComponent ?? "") : "已取消";self?.panel=nil}
    }
  }
  func applicationShouldTerminateAfterLastWindowClosed(_ sender:NSApplication)->Bool {true}
  func applicationShouldHandleReopen(_ sender:NSApplication,hasVisibleWindows flag:Bool)->Bool {window.makeKeyAndOrderFront(nil);return true}
}
@main struct FilePanelMain {
  @MainActor static func main() {
    let app=NSApplication.shared;app.setActivationPolicy(.regular)
    let folder=Bundle.main.bundleURL.deletingLastPathComponent().appendingPathComponent("user-files")
    let delegate=FilePanelFixture(folder:folder);app.delegate=delegate
    withExtendedLifetime(delegate) {app.run()}
  }
}
