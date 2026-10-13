import AppKit
import SwiftUI

/// Clipboard success means the complete string was accepted by the system pasteboard.
@MainActor enum NativeValueClipboard {
  static func write(_ source:String,to pasteboard:NSPasteboard = .general)->Bool {
    pasteboard.clearContents()
    return pasteboard.setString(source,forType:.string) && pasteboard.string(forType:.string)?.utf8.elementsEqual(source.utf8)==true
  }
}
@MainActor final class NativeValueCell:NSTableCellView {
  let text=NSTextField(wrappingLabelWithString:"")
  let detail=NSTextField(labelWithString:"")
  let open=NSButton(title:"查看值",target:nil,action:nil)
  private var action:(()->Void)?
  override init(frame:NSRect) {
    super.init(frame:frame)
    identifier = .init("native-value-entry");textField=text
    text.font = .monospacedSystemFont(ofSize:12,weight:.regular);text.isSelectable=true
    text.maximumNumberOfLines=2;text.lineBreakMode = .byTruncatingTail
    detail.font = .systemFont(ofSize:10);detail.textColor = .secondaryLabelColor
    open.bezelStyle = .inline;open.controlSize = .small;open.target=self;open.action=#selector(activate)
    let footer=NSStackView(views:[detail,open]);footer.orientation = .horizontal;footer.spacing=6
    let stack=NSStackView(views:[text,footer]);stack.orientation = .vertical;stack.alignment = .leading;stack.spacing=3
    stack.translatesAutoresizingMaskIntoConstraints=false;addSubview(stack)
    NSLayoutConstraint.activate([stack.leadingAnchor.constraint(equalTo:leadingAnchor,constant:6),stack.trailingAnchor.constraint(equalTo:trailingAnchor,constant:-6),stack.topAnchor.constraint(equalTo:topAnchor,constant:5),stack.bottomAnchor.constraint(lessThanOrEqualTo:bottomAnchor,constant:-4)])
  }
  required init?(coder:NSCoder) {fatalError("not used")}
  func configure(_ entry:NativeValueEntry,page:NativeValuePage,action:@escaping()->Void) {
    text.stringValue=entry.excerpt
    detail.stringValue=[entry.nature.label,page.statusLabel(entry)].compactMap{$0}.joined(separator:" · ")
    self.action=action;open.isEnabled=entry.path.count<=32
    open.title=entry.path.count<=32 ? "查看值" : "深度已达上限"
    open.setAccessibilityLabel("查看值 · "+entry.excerpt)
    setAccessibilityIdentifier("value-entry-"+entry.id)
    text.setAccessibilityLabel(entry.excerpt+" · "+entry.nature.label)
  }
  @objc private func activate() {action?()}
}
@MainActor final class NativeValueLabelCell:NSTableCellView {
  override init(frame:NSRect) {
    super.init(frame:frame)
    identifier = .init("native-value-label")
    let field=NSTextField(wrappingLabelWithString:"");field.isSelectable=true;field.maximumNumberOfLines=2
    field.translatesAutoresizingMaskIntoConstraints=false;textField=field;addSubview(field)
    NSLayoutConstraint.activate([field.leadingAnchor.constraint(equalTo:leadingAnchor,constant:5),field.trailingAnchor.constraint(equalTo:trailingAnchor,constant:-5),field.centerYAnchor.constraint(equalTo:centerYAnchor)])
  }
  required init?(coder:NSCoder) {fatalError("not used")}
}

struct NativeValueTableView:NSViewRepresentable {
  let page:NativeValuePage
  let onOpen:(NativeValueEntry,String)->Void
  func makeCoordinator()->Coordinator {Coordinator()}
  func makeNSView(context:Context)->NSScrollView {
    let scroll=NSScrollView(),table=NSTableView()
    scroll.documentView=table;scroll.hasVerticalScroller=true;scroll.hasHorizontalScroller=true
    table.headerView=NSTableHeaderView();table.rowHeight=66;table.usesAlternatingRowBackgroundColors=true
    table.columnAutoresizingStyle = .noColumnAutoresizing;table.selectionHighlightStyle = .regular
    table.delegate=context.coordinator;table.dataSource=context.coordinator
    table.setAccessibilityLabel("结构化结果表格 · 行和列序号从 1 开始")
    context.coordinator.table=table;context.coordinator.update(page,onOpen:onOpen)
    return scroll
  }
  func updateNSView(_ view:NSScrollView,context:Context) {context.coordinator.update(page,onOpen:onOpen)}
  @MainActor final class Coordinator:NSObject,NSTableViewDataSource,NSTableViewDelegate {
    weak var table:NSTableView?
    var page:NativeValuePage?
    var onOpen:((NativeValueEntry,String)->Void)?
    private var identity:Data?
    func update(_ page:NativeValuePage,onOpen:@escaping(NativeValueEntry,String)->Void) {
      self.onOpen=onOpen
      let encoder=JSONEncoder();encoder.outputFormatting = .sortedKeys
      let next=try? encoder.encode(page)
      guard next != identity,let table else {return}
      identity=next;self.page=page
      let titles=[page.kind == .record ? "字段" : "序号"]+page.columns.map{[.list,.record].contains(page.kind) ? "值" : $0}
      if table.tableColumns.map(\.title) != titles {
        for column in table.tableColumns {table.removeTableColumn(column)}
        for (index,title) in titles.enumerated() {
          let column=NSTableColumn(identifier:.init("value-\(index)"));column.title=title
          column.width=index==0 ? 90 : 220;column.minWidth=index==0 ? 60 : 140;column.sortDescriptorPrototype=nil
          table.addTableColumn(column)
        }
      }
      table.reloadData()
    }
    func numberOfRows(in tableView:NSTableView)->Int {page?.rows.count ?? 0}
    func tableView(_ tableView:NSTableView,viewFor tableColumn:NSTableColumn?,row:Int)->NSView? {
      guard let page,row>=0,row<page.rows.count,let column=tableColumn,let index=tableView.tableColumns.firstIndex(of:column) else {return nil}
      let record=page.rows[row]
      if index==0 {
        let cell=(tableView.makeView(withIdentifier:.init("native-value-label"),owner:nil) as? NativeValueLabelCell) ?? NativeValueLabelCell(frame:.zero)
        cell.textField?.stringValue=record.label;return cell
      }
      guard index-1<record.cells.count else {return nil}
      let entry=record.cells[index-1],cell=(tableView.makeView(withIdentifier:.init("native-value-entry"),owner:nil) as? NativeValueCell) ?? NativeValueCell(frame:.zero)
      cell.configure(entry,page:page) {[weak self] in self?.onOpen?(entry,record.label+" / "+page.columns[index-1])}
      return cell
    }
  }
}

struct NativeStructuredValueView:View {
  @ObservedObject var presenter:ResultPresenter
  let page:NativeValuePage
  @State private var copyStatus:String?
  var body:some View {
    VStack(alignment:.leading,spacing:8) {
      HStack {
        Text(page.kind.label).font(.headline)
        if page.authoritative,let origin=page.origin {Text(origin.name+" · 实际计算").font(.caption).foregroundStyle(.secondary)}
        Spacer()
        if !presenter.valueTrail.isEmpty {Button("返回上层") {Task {await presenter.backValue()}}.disabled(presenter.busy)}
      }
      if !page.path.isEmpty {Text(presenter.valueRoute.label).font(.caption).foregroundStyle(.secondary)}
      if page.column_count>0 {
        Text("\(page.row_count) 行/字段 · \(page.column_count) 列 · 序号从 1 开始").font(.caption).foregroundStyle(.secondary)
        if page.kind == .record {
          Text("展开字段，双击值行或按 Return 查看。").font(.caption).foregroundStyle(.secondary)
          NativeRecordOutlineView(page:page,onOpen:open).frame(height:min(360,CGFloat(page.rows.count)*66+28))
        } else {
          NativeValueTableView(page:page,onOpen:open).frame(height:min(360,max(70,CGFloat(page.rows.count)*66+28)))
        }
        if page.rows.isEmpty {Text("没有数据行").foregroundStyle(.secondary)}
        pagination
      } else if let source=page.source {
        if let nature=presenter.valueRoute.nature {Text(nature.label).font(.caption).foregroundStyle(.secondary)}
        NativeMathView(request:.init(identity:presenter.original.result_ref+"/value/"+page.path.map(String.init).joined(separator:"."),originalLatex:source.latex,wolfram:source.input_form,modern:source.modern_form.value))
        if page.kind != .scalar {Text("保留原始定义，可复制完整源码。").font(.caption).foregroundStyle(.secondary)}
      }
      HStack {
        Menu("复制完整当前值") {
          Button("Wolfram 源式") {copy(.input_form)}
          Button("Modern 源式") {copy(.modern)}
          Button("LaTeX") {copy(.latex)}
        }.disabled(presenter.busy)
        if !page.rows.isEmpty {Button("复制页面摘录") {copyStatus=NativeValueClipboard.write(page.excerpt) ? "页面摘录已复制。" : "剪贴板未接纳摘录，请重试。"}}
      }
      if let copyStatus {Text(copyStatus).font(.caption).foregroundStyle(.secondary).accessibilityIdentifier("native-value-copy-status")}
    }.accessibilityElement(children:.contain).accessibilityLabel("结构化计算结果")
  }
  private var pagination:some View {
    VStack(alignment:.leading,spacing:6) {
      if page.row_count>UInt32(page.rows.count) {
        HStack {
          Button("上一页") {load(offset:page.offset>32 ? page.offset-32 : 0,column:page.column_offset)}.disabled(page.offset==0 || presenter.busy)
          Text("行 \(page.row_count==0 ? 0 : UInt64(page.offset)+1)–\(page.rowEnd) / \(page.row_count)").font(.caption)
          Button("下一页") {load(offset:page.rowEnd,column:page.column_offset)}.disabled(page.rowEnd>=page.row_count || presenter.busy)
        }
      }
      if page.column_count>UInt32(page.columns.count) {
        HStack {
          Button("前列") {load(offset:page.offset,column:page.column_offset>8 ? page.column_offset-8 : 0)}.disabled(page.column_offset==0 || presenter.busy)
          Text("列 \(page.column_count==0 ? 0 : UInt64(page.column_offset)+1)–\(page.columnEnd) / \(page.column_count)").font(.caption)
          Button("后列") {load(offset:page.offset,column:page.columnEnd)}.disabled(page.columnEnd>=page.column_count || presenter.busy)
        }
      }
    }
  }
  private func load(offset:UInt32,column:UInt32) {copyStatus=nil;Task {await presenter.loadValue(path:page.path,offset:offset,columnOffset:column)}}
  private func open(_ entry:NativeValueEntry,_ label:String) {copyStatus=nil;Task {await presenter.openValue(entry,label:label)}}
  private func copy(_ format:InspectResultSourceFormat) {
    copyStatus=nil
    Task {
      if let source=await presenter.readValueSource(format:format) {copyStatus=NativeValueClipboard.write(source) ? "完整当前值已复制。" : "剪贴板未接纳完整值，请重试。"}
    }
  }
}
