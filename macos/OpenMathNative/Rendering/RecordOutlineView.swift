import AppKit
import SwiftUI

@MainActor final class NativeValueOutline:NSOutlineView {
  var openSelected:(()->Void)?
  override func keyDown(with event:NSEvent) {
    if event.keyCode==36,event.modifierFlags.intersection([.command,.control,.option]).isEmpty {openSelected?()}
    else {super.keyDown(with:event)}
  }
}
/// The only outline children are actual entries in the current bounded page. Opening a
/// container requests its real immutable path, rather than inventing/loading an entire tree.
struct NativeRecordOutlineView:NSViewRepresentable {
  let page:NativeValuePage
  let onOpen:(NativeValueEntry,String)->Void
  func makeCoordinator()->Coordinator {Coordinator()}
  func makeNSView(context:Context)->NSScrollView {
    let scroll=NSScrollView(),outline=NativeValueOutline()
    scroll.documentView=outline;scroll.hasVerticalScroller=true;scroll.hasHorizontalScroller=true
    let field=NSTableColumn(identifier:.init("field"));field.title="字段";field.width=190;field.minWidth=100
    let value=NSTableColumn(identifier:.init("value"));value.title="值";value.width=400;value.minWidth=220
    outline.addTableColumn(field);outline.addTableColumn(value);outline.outlineTableColumn=field
    outline.headerView=NSTableHeaderView();outline.rowHeight=66;outline.usesAlternatingRowBackgroundColors=true
    outline.columnAutoresizingStyle = .noColumnAutoresizing
    outline.delegate=context.coordinator;outline.dataSource=context.coordinator
    outline.target=context.coordinator;outline.doubleAction=#selector(Coordinator.openSelection(_:))
    outline.openSelected = {[weak coordinator=context.coordinator] in coordinator?.openSelection(nil)}
    outline.setAccessibilityLabel("记录字段与实际值 · 分页读取")
    context.coordinator.outline=outline;context.coordinator.update(page,onOpen:onOpen)
    return scroll
  }
  func updateNSView(_ view:NSScrollView,context:Context) {context.coordinator.update(page,onOpen:onOpen)}
  @MainActor final class Coordinator:NSObject,NSOutlineViewDataSource,NSOutlineViewDelegate {
    final class Leaf {let entry:NativeValueEntry;init(_ entry:NativeValueEntry) {self.entry=entry}}
    final class Field {
      let row:NativeValueRow,leaves:[Leaf]
      init(_ row:NativeValueRow) {self.row=row;leaves=row.cells.map(Leaf.init)}
    }
    weak var outline:NSOutlineView?
    private var fields:[Field]=[]
    private var page:NativeValuePage?
    private var onOpen:((NativeValueEntry,String)->Void)?
    private var identity:Data?
    func update(_ page:NativeValuePage,onOpen:@escaping(NativeValueEntry,String)->Void) {
      self.onOpen=onOpen
      let encoder=JSONEncoder();encoder.outputFormatting = .sortedKeys
      let next=try? encoder.encode(page)
      guard next != identity else {return}
      identity=next;self.page=page;fields=page.rows.map(Field.init)
      outline?.reloadData()
    }
    func outlineView(_ outlineView:NSOutlineView,numberOfChildrenOfItem item:Any?)->Int {
      if item==nil {return fields.count}
      return (item as? Field)?.leaves.count ?? 0
    }
    func outlineView(_ outlineView:NSOutlineView,child index:Int,ofItem item:Any?)->Any {
      if let field=item as? Field {return field.leaves[index]}
      return fields[index]
    }
    func outlineView(_ outlineView:NSOutlineView,isItemExpandable item:Any)->Bool {(item as? Field)?.row.cells.isEmpty==false}
    func outlineViewSelectionDidChange(_ notification:Notification) {
      guard let outline,outline.selectedRow>=0 else {return}
      outline.window?.makeFirstResponder(outline)
    }
    @objc func openSelection(_ sender:Any?) {
      guard let outline,outline.selectedRow>=0 else {return}
      if let field=outline.item(atRow:outline.selectedRow) as? Field {
        if outline.isItemExpanded(field) {outline.collapseItem(field)} else {outline.expandItem(field)}
      } else if let leaf=outline.item(atRow:outline.selectedRow) as? Leaf,leaf.entry.path.count<=32 {
        let label=fields.first{$0.row.cells.contains{$0.id.utf8.elementsEqual(leaf.entry.id.utf8)}}?.row.label ?? "值"
        onOpen?(leaf.entry,label)
      }
    }
    func outlineView(_ outlineView:NSOutlineView,viewFor tableColumn:NSTableColumn?,item:Any)->NSView? {
      guard let page,let column=tableColumn else {return nil}
      if let field=item as? Field {
        guard column.identifier.rawValue=="field" else {return nil}
        let translated=page.fieldLabel(field.row.label)
        let cell=(outlineView.makeView(withIdentifier:.init("native-value-label"),owner:nil) as? NativeValueLabelCell) ?? NativeValueLabelCell(frame:.zero)
        cell.textField?.stringValue=translated==field.row.label ? translated : translated+" · "+field.row.label
        return cell
      }
      guard let leaf=item as? Leaf else {return nil};let entry=leaf.entry
      if column.identifier.rawValue=="field" {
        let cell=(outlineView.makeView(withIdentifier:.init("native-value-label"),owner:nil) as? NativeValueLabelCell) ?? NativeValueLabelCell(frame:.zero)
        cell.textField?.stringValue=entry.kind.label;return cell
      }
      let cell=(outlineView.makeView(withIdentifier:.init("native-value-entry"),owner:nil) as? NativeValueCell) ?? NativeValueCell(frame:.zero)
      let field=fields.first{$0.row.cells.contains{$0.id.utf8.elementsEqual(entry.id.utf8)}}?.row.label ?? "值"
      cell.configure(entry,page:page) {[weak self] in self?.onOpen?(entry,field)}
      return cell
    }
  }
}
