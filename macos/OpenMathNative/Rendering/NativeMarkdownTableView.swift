import AppKit
import SwiftUI

/// Tables use their own native table surface. NSTextTable would force the surrounding macOS
/// text view into TextKit 1 and invalidate both formula providers and TextKit 2 height metrics.
struct NativeMarkdownTableView:View {
  let table:MarkdownRenderTable,dark:Bool,fontSize:CGFloat
  var body:some View {
    VStack(alignment:.leading,spacing:6) {
      MarkdownNativeTable(table:table,dark:dark,fontSize:fontSize)
      HStack {
        if table.rows.count>101 || table.columnCount>32 {Text("显示前100行、32列；复制包含完整表格。").font(.caption).foregroundStyle(.secondary)}
        Spacer()
        Button("复制表格",systemImage:"doc.on.doc") {NSPasteboard.general.clearContents();NSPasteboard.general.setString(table.tsv,forType:.string)}.buttonStyle(.borderless).font(.caption)
      }
    }
  }
}
private struct MarkdownNativeTable:NSViewRepresentable {
  let table:MarkdownRenderTable,dark:Bool,fontSize:CGFloat
  func makeCoordinator()->Coordinator {Coordinator()}
  func makeNSView(context:Context)->NSScrollView {
    let scroll=NSScrollView();scroll.hasHorizontalScroller=true;scroll.hasVerticalScroller=false;scroll.autohidesScrollers=true;scroll.drawsBackground=false
    let native=NSTableView();native.headerView=nil;native.backgroundColor = .textBackgroundColor
    native.gridStyleMask=[.solidHorizontalGridLineMask,.solidVerticalGridLineMask];native.intercellSpacing=NSSize(width:12,height:8)
    native.selectionHighlightStyle = .regular;native.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
    native.delegate=context.coordinator;native.dataSource=context.coordinator;scroll.documentView=native
    context.coordinator.configure(table,native:native,dark:dark,size:fontSize);return scroll
  }
  func updateNSView(_ scroll:NSScrollView,context:Context) {if let native=scroll.documentView as? NSTableView {context.coordinator.configure(table,native:native,dark:dark,size:fontSize)}}
  func sizeThatFits(_ proposal:ProposedViewSize,nsView:NSScrollView,context:Context)->CGSize? {
    guard let native=nsView.documentView as? NSTableView else {return nil}
    let width=max(1,proposal.width ?? 600),columns=max(1,native.tableColumns.count),columnWidth=max(150,(width-CGFloat(columns)*12-2)/CGFloat(columns))
    for column in native.tableColumns {column.width=columnWidth}
    native.frame.size.width=max(width,CGFloat(columns)*(columnWidth+12))
    native.noteHeightOfRows(withIndexesChanged:IndexSet(integersIn:0..<native.numberOfRows))
    var height:CGFloat=0
    for row in 0..<native.numberOfRows {height+=context.coordinator.tableView(native,heightOfRow:row)+8}
    height+=nsView.hasHorizontalScroller ? NSScroller.scrollerWidth(for:.small,scrollerStyle:.legacy):0
    native.frame.size.height=height
    return CGSize(width:width,height:ceil(height))
  }
  @MainActor final class Coordinator:NSObject,NSTableViewDataSource,NSTableViewDelegate {
    var data:MarkdownRenderTable?,dark=false,size:CGFloat=16
    private var key:Data?
    private var heights:[String:CGFloat]=[:]
    func configure(_ data:MarkdownRenderTable,native:NSTableView,dark:Bool,size:CGFloat) {
      let actual=Data((data.sourceDigest+"/table-\(data.id)/\(dark)/\(size)").utf8)
      guard actual != key else {return};key=actual;heights=[:];self.data=data;self.dark=dark;self.size=size
      for column in native.tableColumns {native.removeTableColumn(column)}
      for index in 0..<min(32,data.columnCount) {let column=NSTableColumn(identifier:.init(String(index)));column.minWidth=150;column.width=180;native.addTableColumn(column)}
      native.reloadData()
    }
    func numberOfRows(in tableView:NSTableView)->Int {min(101,data?.rows.count ?? 0)}
    func tableView(_ tableView:NSTableView,viewFor tableColumn:NSTableColumn?,row:Int)->NSView? {
      guard let data,let tableColumn,let column=Int(tableColumn.identifier.rawValue) else {return nil}
      let id=NSUserInterfaceItemIdentifier("markdown-cell")
      let view=(tableView.makeView(withIdentifier:id,owner:self) as? MarkdownTextView) ?? MarkdownTextView(usingTextLayoutManager:true)
      view.identifier=id;view.configure();view.install(data.cell(row,column),dark:dark,fontSize:size)
      return view
    }
    func tableView(_ tableView:NSTableView,heightOfRow row:Int)->CGFloat {
      guard let data else {return 24}
      let key=String(row)+"/"+tableView.tableColumns.map{String(Double($0.width))}.joined(separator:",")
      if let height=heights[key] {return height}
      var height:CGFloat=24
      for (column,model) in tableView.tableColumns.enumerated() {
        let measure=MarkdownTextView(usingTextLayoutManager:true);measure.configure();measure.install(data.cell(row,column),dark:dark,fontSize:size)
        height=max(height,measure.measuredHeight(width:model.width))
      }
      heights[key]=height;return height
    }
  }
}
