import AppKit
import SwiftUI

// Native material study. Preset notebook, no kernel, networking, files, or model calls.
private let accent = Color(red: 29 / 255, green: 117 / 255, blue: 80 / 255)
private let curveBlue = Color(red: 47 / 255, green: 107 / 255, blue: 216 / 255)
private let agentPurple = Color(red: 85 / 255, green: 81 / 255, blue: 207 / 255)
private let originalSource = "solve(x^2 - 5*x + 6 = 0, x)"

enum Panel: String, CaseIterable, Identifiable {
  case assistant = "助手", steps = "步骤", variables = "变量"
  var id: String { rawValue }
}
enum Scene: String, CaseIterable, Identifiable {
  case reading = "阅读与图形", steps = "推导步骤", assistant = "助手", editing = "公式编辑"
  var id: String { rawValue }
}
enum Orientation: String, CaseIterable, Identifiable {
  case landscape = "横屏", portrait = "竖屏", compact = "窄窗口"
  var id: String { rawValue }
  var size: CGSize {
    switch self {
    case .landscape: CGSize(width: 1194, height: 834)
    case .portrait: CGSize(width: 834, height: 1112)
    case .compact: CGSize(width: 640, height: 834)
    }
  }
}
@MainActor final class Study: ObservableObject {
  @Published var view = 3
  @Published var scene: Scene = .reading
  @Published var orientation: Orientation = .landscape
  @Published var dark = false
  @Published var clear = false
  @Published var phonePanel: Panel?
  @Published var padPanel: Panel? = .steps
  @Published var phoneSource = originalSource
  @Published var padSource = originalSource
  @Published var editing = false
  @Published var phoneDraft = ""
  @Published var padDraft = ""
  @Published var duoPose: DuoPose = .flat
  @Published var duoRotated = false
  @Published var duoWidth: Double = 890
  @Published var duoSource = originalSource
  @Published var duoPanel: Panel? = .steps
  @Published var duoDraft = ""
  @Published var duoAnchor: String?
  @Published var notice = ""
  var paper: Color { dark ? Color(red: 0.105, green: 0.116, blue: 0.113) : .white }
  var well: Color { dark ? .white.opacity(0.055) : .black.opacity(0.032) }
  var line: Color { dark ? .white.opacity(0.1) : .black.opacity(0.09) }
  var green: Color { dark ? Color(red: 0.37, green: 0.82, blue: 0.61) : accent }
  var glass: Glass { clear ? .clear : .regular }
  func selectScene() {
    editing = scene == .editing
    if scene == .steps { duoPanel = .steps }
    if scene == .assistant { duoPanel = .assistant }
    if scene == .editing { duoSource = "solve(x^2 - 5*x + 4 = 0, x)" }
    switch scene {
    case .reading: phonePanel = nil; padPanel = .steps
    case .steps: phonePanel = .steps; padPanel = .steps
    case .assistant: phonePanel = .assistant; padPanel = .assistant
    case .editing:
      phonePanel = nil; padPanel = .steps
      phoneSource = "solve(x^2 - 5*x + 4 = 0, x)"
      padSource = phoneSource
    }
  }
  func hint(_ text: String) {
    notice = text
    Task { try? await Task.sleep(for: .seconds(2.7)); if notice == text { notice = "" } }
  }
}

@main struct GlassWorkbenchApp: App {
  @StateObject private var study = Study()
  var body: some SwiftUI.Scene {
    WindowGroup("OpenMath · iOS 27 / iPadOS 27 草稿 02") {
      ReviewBoard().environmentObject(study)
        .preferredColorScheme(study.dark ? .dark : .light)
        .frame(minWidth: 1100, minHeight: 730)
    }
    .defaultSize(width: 1440, height: 880)
    .windowStyle(.hiddenTitleBar)
    .commands { CommandGroup(replacing: .newItem) {} }
  }
}

struct ReviewBoard: View {
  @EnvironmentObject private var study: Study
  var body: some View {
    VStack(spacing: 0) {
      HStack(spacing: 16) {
        Image(systemName: "sum").font(.system(size: 22, weight: .medium))
          .foregroundStyle(study.green)
        Text("OpenMath").font(.system(size: 17, weight: .semibold))
        Text("/").foregroundStyle(.tertiary)
        Text("iOS & iPadOS").font(.system(size: 13)).foregroundStyle(.secondary)
        Spacer()
        Picker("设备", selection: $study.view) {
          Text("总览").tag(0); Text("iPhone").tag(1); Text("iPad").tag(2); Text("Duo").tag(3)
        }.pickerStyle(.segmented).labelsHidden().frame(width: 260)
        Picker("场景", selection: $study.scene) {
          ForEach(Scene.allCases) { Text($0.rawValue).tag($0) }
        }.labelsHidden().frame(width: 130)
        Picker("材质", selection: $study.clear) {
          Text("常规玻璃").tag(false); Text("通透玻璃").tag(true)
        }.labelsHidden().frame(width: 120)
        Toggle("深色", isOn: $study.dark).toggleStyle(.switch).controlSize(.small)
      }
      .padding(.horizontal, 32).padding(.top, 24).padding(.bottom, 17)
      Divider().opacity(0.6)
      HStack(alignment: .bottom) {
        VStack(alignment: .leading, spacing: 7) {
          Text("NATIVE MATERIAL STUDY / 02")
            .font(.system(size: 9, weight: .medium, design: .monospaced))
            .tracking(1.8).foregroundStyle(study.green)
          Text("数学在纸上，操作浮于其上。")
            .font(.system(size: 25, weight: .medium)).tracking(-0.8)
        }
        Spacer()
        VStack(alignment: .trailing, spacing: 4) {
          Text("系统 Liquid Glass · 按压 / 滚动 / 展开")
          Text("布局草稿 · 示例数据 · 不连接内核或模型")
        }.font(.system(size: 10)).foregroundStyle(.secondary)
      }.padding(.horizontal, 38).padding(.top, 24).padding(.bottom, 23)
      GeometryReader { geometry in
        let phoneColumn = study.view == 1 ? geometry.size.width : min(320.0, geometry.size.width * 0.25)
        let padColumn = study.view == 2 ? geometry.size.width : geometry.size.width - phoneColumn - 34
        let maxHeight = geometry.size.height - 65
        if study.view == 3 {
          DuoReviewBoard(available: geometry.size)
        } else {
        HStack(alignment: .top, spacing: 34) {
          if study.view != 2 {
            DeviceStudy(title: "01  iPhone", caption: "单栏笔记本 · 悬浮操作", width: phoneColumn) {
              let scale = min(min(phoneColumn / 420, maxHeight / 892), 1)
              PhoneWorkbench().frame(width: 402, height: 874)
                .clipShape(RoundedRectangle(cornerRadius: 44))
                .padding(9).background(.black, in: RoundedRectangle(cornerRadius: 53))
                .overlay { RoundedRectangle(cornerRadius: 53).strokeBorder(.white.opacity(0.2), lineWidth: 1) }
                .shadow(color: .black.opacity(0.12), radius: 17, y: 14)
                .scaleEffect(scale, anchor: .topLeading)
                .frame(width: 420 * scale, height: 892 * scale, alignment: .topLeading)
            }
          }
          if study.view != 1 {
            VStack(spacing: 0) {
              HStack {
                Text("02  iPad").font(.system(size: 12, weight: .medium))
                Spacer()
                Picker("方向", selection: $study.orientation) {
                  ForEach(Orientation.allCases) { Text($0.rawValue).tag($0) }
                }.pickerStyle(.segmented).labelsHidden().frame(width: 177).controlSize(.small)
              }.frame(height: 27).padding(.bottom, 8)
              let size = study.orientation.size
              let scale = min(min(padColumn / (size.width + 18), maxHeight / (size.height + 18)), 1)
              PadWorkbench().frame(width: size.width, height: size.height)
                .clipShape(RoundedRectangle(cornerRadius: 20))
                .padding(9).background(.black, in: RoundedRectangle(cornerRadius: 29))
                .overlay { RoundedRectangle(cornerRadius: 29).strokeBorder(.white.opacity(0.2), lineWidth: 1) }
                .shadow(color: .black.opacity(0.12), radius: 17, y: 14)
                .scaleEffect(scale, anchor: .topLeading)
                .frame(width: (size.width + 18) * scale, height: (size.height + 18) * scale, alignment: .topLeading)
              Text("目录与检查器保留结构；玻璃用于上层操作。")
                .font(.system(size: 10)).foregroundStyle(.secondary).padding(.top, 14)
            }.frame(width: padColumn, alignment: .top)
          }
        }.frame(maxWidth: .infinity, alignment: .top)
        }
      }.padding(.horizontal, 38)
      HStack {
        Text("浅灰源码槽 / 精确结果 / 绿色状态左轨")
        Spacer()
        if !study.notice.isEmpty {
          Text(study.notice).foregroundStyle(study.green)
        } else {
          Text("macOS 27 原生材质预览载体 · iPhone / Duo / iPad 布局示意")
        }
      }.font(.system(size: 10)).foregroundStyle(.secondary)
        .padding(.horizontal, 38).padding(.vertical, 18)
    }
    .background(study.dark ? Color(red: 0.12, green: 0.14, blue: 0.13) : Color(red: 0.91, green: 0.925, blue: 0.913))
    .onChange(of: study.scene) { study.selectScene() }
  }
}
// Review fixtures, not UIScreen metrics or hardware fold detection. Layout below
// reads its container; production iOS 27.1 must supply actual reserved regions.
enum DuoDivision { case vertical, horizontal }
enum DuoPose: String, CaseIterable, Identifiable {
  case folded = "合拢", flat = "展开", book = "书本半折", tabletop = "桌面半折"
  var id: String { rawValue }
  var canvas: CGSize {
    switch self {
    case .folded: CGSize(width: 466, height: 678)
    case .flat, .book: CGSize(width: 890, height: 626)
    case .tabletop: CGSize(width: 626, height: 890)
    }
  }
  func division(rotated: Bool) -> DuoDivision? {
    switch self {
    case .folded, .flat: nil
    case .book: rotated ? .horizontal : .vertical
    case .tabletop: rotated ? .vertical : .horizontal
    }
  }
}
struct DuoReviewBoard: View {
  @EnvironmentObject private var study: Study
  let available: CGSize
  private var canvas: CGSize {
    let preset = study.duoPose.canvas
    return CGSize(width: study.duoWidth, height: study.duoRotated ? preset.width : preset.height)
  }
  var body: some View {
    VStack(spacing: 10) {
      HStack(spacing: 16) {
        Text("03  iPhone Duo").font(.system(size: 13, weight: .medium))
        Spacer()
        Picker("Duo 姿态", selection: $study.duoPose) {
          ForEach(DuoPose.allCases) { Text($0.rawValue).tag($0) }
        }.pickerStyle(.segmented).labelsHidden().frame(width: 338)
        Button {
          study.duoRotated.toggle()
          study.duoWidth = study.duoRotated ? study.duoPose.canvas.height : study.duoPose.canvas.width
        } label: { Label("旋转", systemImage: "rotate.right") }
        .buttonStyle(.bordered).accessibilityLabel("旋转 Duo 画布")
      }.frame(height: 28)
      HStack(spacing: 10) {
        Text("可用宽度").foregroundStyle(.secondary)
        Slider(value: $study.duoWidth, in: 340...1024).frame(width: 190)
          .accessibilityLabel("Duo 可用宽度")
        Text("\(Int(canvas.width)) × \(Int(canvas.height))").monospacedDigit().frame(width: 91, alignment: .leading)
        Text("评审画布 · 非设备安全区测量").foregroundStyle(.tertiary)
        Spacer()
        Text("同一文稿 / 同一草稿 / 连续阅读").foregroundStyle(study.green)
      }.font(.system(size: 10)).frame(height: 21)
      let scale = min(min(available.width / (canvas.width + 18), (available.height - 101) / (canvas.height + 18)), 1)
      DuoWorkbench(division: study.duoPose.division(rotated: study.duoRotated))
        .frame(width: canvas.width, height: canvas.height)
        .clipShape(RoundedRectangle(cornerRadius: 29))
        .padding(9).background(.black, in: RoundedRectangle(cornerRadius: 38))
        .overlay { RoundedRectangle(cornerRadius: 38).strokeBorder(.white.opacity(0.2), lineWidth: 1) }
        .shadow(color: .black.opacity(0.13), radius: 18, y: 12)
        .scaleEffect(scale, anchor: .top)
        .frame(width: (canvas.width + 18) * scale, height: (canvas.height + 18) * scale, alignment: .top)
        .frame(maxWidth: .infinity)
      Text("纵向玻璃操作组留在安全边缘；文稿连续滚动，检查内容随可用区域呈现。")
        .font(.system(size: 10)).foregroundStyle(.secondary)
    }
    .onChange(of: study.duoPose) {
      study.duoWidth = study.duoRotated ? study.duoPose.canvas.height : study.duoPose.canvas.width
    }
  }
}
struct DuoWorkbench: View {
  @EnvironmentObject private var study: Study
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  let division: DuoDivision?
  @State private var scrolled = false
  @State private var inspectorVisible = true
  @State private var sheetVisible = false
  @State private var outline = false
  @State private var menu = false
  var body: some View {
    GeometryReader { geometry in
      let size = geometry.size
      let rail: CGFloat = 72 // Review-only safe-edge fixture, not a device inset.
      let usable = max(0, size.width - rail)
      let split = usable >= 700 && size.width > size.height
      let tabletop = division == .horizontal && !split
      let primary = split && inspectorVisible ? min(usable - 290, max(360, size.width / 2 - 8)) : usable
      let secondary = max(0, usable - primary - 16)
      ZStack(alignment: .topLeading) {
        study.paper
        // Keep this ScrollView at one structural identity through every resize.
        Notebook(phone: usable < 600, source: $study.duoSource,
                 panel: Binding(get: { study.duoPanel }, set: { value in
                   study.duoPanel = value
                   if split || tabletop { inspectorVisible = true } else { sheetVisible = true }
                 }), scrolled: $scrolled, scrollAnchor: $study.duoAnchor,
                 topInset: 29, bottomInset: tabletop && inspectorVisible ? size.height / 2 + 36 : 40,
                 deviceName: "iPhone Duo")
          .frame(width: primary, height: size.height)
        if split && inspectorVisible {
          inspector(phone: false, close: { withAnimation { inspectorVisible = false } })
            .padding(.top, 18).frame(width: secondary, height: size.height)
            .background(.regularMaterial).offset(x: primary + 16)
        }
        if tabletop && inspectorVisible {
          inspector(phone: true, close: { withAnimation { inspectorVisible = false } })
            .padding(.top, 15).frame(width: usable, height: max(0, size.height / 2 - 13))
            .background(.regularMaterial, in: UnevenRoundedRectangle(topLeadingRadius: 22, topTrailingRadius: 22))
            .offset(y: size.height / 2 + 13)
        }
        if let division {
          fold(division, size: size, usable: usable)
        }
        if study.editing {
          KeyboardStudy().frame(width: primary).frame(height: size.height, alignment: .bottom)
        }
        if (!split && !tabletop && sheetVisible) || outline {
          Color.black.opacity(0.13).frame(width: usable, height: size.height)
            .onTapGesture { withAnimation { sheetVisible = false; outline = false } }
          VStack(spacing: 0) {
            if outline {
              SheetTitle(title: "目录") { outline = false }
              Outline(close: { outline = false })
            } else {
              Capsule().fill(.secondary.opacity(0.35)).frame(width: 34, height: 5).padding(.vertical, 12)
              inspector(phone: true, close: { withAnimation { sheetVisible = false } })
            }
          }.frame(width: usable, height: min(640, size.height - 28))
            .background(.regularMaterial, in: UnevenRoundedRectangle(topLeadingRadius: 27, topTrailingRadius: 27))
            .offset(y: max(28, size.height - 640))
            .transition(.move(edge: .bottom).combined(with: .opacity))
        }
        DuoRail(outer: study.duoPose == .folded, outline: { withAnimation { outline.toggle() } },
                inspector: {
                  withAnimation(reduceMotion ? nil : .spring(response: 0.35, dampingFraction: 0.9)) {
                    if split || tabletop { inspectorVisible.toggle() } else { sheetVisible.toggle() }
                  }
                }, more: { withAnimation { menu.toggle() } })
          .frame(width: rail, height: size.height).offset(x: usable)
        if menu {
          FloatingMenu(close: { menu = false }, add: { menu = false; study.hint("草稿仅展示单元格入口。") }, outline: { menu = false; outline = true })
            .frame(width: min(260, usable - 24)).offset(x: max(12, usable - 272), y: max(14, size.height - 198))
        }
      }
      .clipped()
      .animation(reduceMotion ? nil : .easeInOut(duration: 0.28), value: split)
      .onChange(of: split) { _, wide in
        if wide && sheetVisible { inspectorVisible = true; sheetVisible = false }
      }
      .onChange(of: study.scene) {
        if study.scene == .steps || study.scene == .assistant {
          if split || tabletop { inspectorVisible = true } else { sheetVisible = true }
        }
      }
      .onChange(of: study.duoPanel) { _, value in
        if value != nil {
          if split || tabletop { inspectorVisible = true } else { sheetVisible = true }
        }
      }
    }
  }
  private func inspector(phone: Bool, close: @escaping () -> Void) -> some View {
    Inspector(panel: Binding(get: { study.duoPanel ?? .steps }, set: { study.duoPanel = $0 }),
              draft: $study.duoDraft, phone: phone, close: close)
  }
  @ViewBuilder private func fold(_ axis: DuoDivision, size: CGSize, usable: CGFloat) -> some View {
    if axis == .vertical {
      Rectangle().fill(LinearGradient(colors: [.clear, .black.opacity(0.06), .clear], startPoint: .leading, endPoint: .trailing))
        .frame(width: 16, height: size.height).offset(x: size.width / 2 - 8)
        .accessibilityLabel("竖向折叠保留区示意").allowsHitTesting(false)
    } else {
      Rectangle().fill(LinearGradient(colors: [.clear, .black.opacity(0.07), .clear], startPoint: .top, endPoint: .bottom))
        .frame(width: usable, height: 20).offset(y: size.height / 2 - 10)
        .accessibilityLabel("横向折叠保留区示意").allowsHitTesting(false)
    }
  }
}
struct DuoRail: View {
  @EnvironmentObject private var study: Study
  var outer: Bool
  var outline: () -> Void, inspector: () -> Void, more: () -> Void
  var body: some View {
    VStack(spacing: 10) {
      VStack(spacing: 9) {
        if outer { Capsule().fill(.black).frame(width: 27, height: 13) }
        Text("9:41").font(.system(size: 10, weight: .semibold))
        HStack(spacing: 4) {
          Image(systemName: "wifi"); Image(systemName: "battery.100percent")
        }.font(.system(size: 9))
      }.frame(height: 65)
      GlassIcon(symbol: "chevron.left", label: "Duo 文稿") { outline() }
      GlassIcon(symbol: "list.bullet", label: "Duo 目录", action: outline)
      Spacer(minLength: 14)
      GlassEffectContainer(spacing: 9) {
        VStack(spacing: 9) {
          GlassIcon(symbol: "play.fill", label: "Duo 运行", tint: true) { study.hint("预置求解示例；未调用内核。") }
          GlassIcon(symbol: "plus", label: "Duo 添加单元格") { study.hint("草稿仅展示单元格入口。") }
          GlassIcon(symbol: "sidebar.right", label: "Duo 检查面板", tint: true, action: inspector)
          GlassIcon(symbol: "ellipsis", label: "Duo 更多", action: more)
        }
      }
    }.padding(.top, 10).padding(.bottom, 20)
      .frame(maxWidth: .infinity).background(study.paper.opacity(0.7))
  }
}

struct DeviceStudy<Content: View>: View {
  let title: String, caption: String
  let width: CGFloat
  @ViewBuilder var content: () -> Content
  var body: some View {
    VStack(spacing: 0) {
      HStack { Text(title).font(.system(size: 12, weight: .medium)); Spacer(); Text("原生玻璃").font(.system(size: 10)).foregroundStyle(.secondary) }
        .frame(height: 27).padding(.bottom, 8)
      content()
      Text(caption).font(.system(size: 10)).foregroundStyle(.secondary).padding(.top, 14)
    }.frame(width: width, alignment: .top)
  }
}

struct GlassIcon: View {
  @EnvironmentObject private var study: Study
  let symbol: String, label: String
  var tint = false
  var action: () -> Void
  var body: some View {
    Button(action: action) {
      Image(systemName: symbol).font(.system(size: 18, weight: .medium))
        .foregroundStyle(tint ? study.green : .primary)
        .frame(width: 46, height: 46).contentShape(Circle())
    }.buttonStyle(.plain)
      .glassEffect(tint ? study.glass.tint(study.green.opacity(0.12)).interactive() : study.glass.interactive(), in: .circle)
      .accessibilityLabel(label).help(label)
  }
}
struct StatusBar: View {
  var phone = false
  var body: some View {
    HStack(spacing: 6) {
      Text(phone ? "9:41" : "9:41　10月22日 周四")
        .font(.system(size: phone ? 16 : 10, weight: .semibold))
      Spacer()
      if phone { Image(systemName: "cellularbars").font(.system(size: 12, weight: .bold)) }
      Image(systemName: "wifi").font(.system(size: phone ? 13 : 10, weight: .semibold))
      Image(systemName: "battery.100percent").font(.system(size: phone ? 20 : 14))
    }.padding(.horizontal, phone ? 30 : 18)
      .frame(height: phone ? 54 : 26)
      .overlay(alignment: .top) {
        if phone { Capsule().fill(.black).frame(width: 118, height: 31).padding(.top, 11) }
      }
  }
}

struct PhoneWorkbench: View {
  @EnvironmentObject private var study: Study
  @State private var menu = false
  @State private var outline = false
  @State private var add = false
  @State private var scrolled = false
  var body: some View {
    ZStack(alignment: .top) {
      study.paper
      Notebook(phone: true, source: $study.phoneSource, panel: $study.phonePanel, scrolled: $scrolled)
      VStack(spacing: 0) {
        StatusBar(phone: true)
        HStack(spacing: 9) {
          GlassIcon(symbol: "chevron.left", label: "文稿") { outline = true }
          Spacer(minLength: 2)
          VStack(spacing: 2) {
            Text("二次方程与图形").font(.system(size: 14, weight: .semibold))
            Text("本地已保存").font(.system(size: 10)).foregroundStyle(.secondary)
          }.frame(maxWidth: .infinity)
          GlassIcon(symbol: "play.fill", label: "运行", tint: true) { study.hint("预置示例：x = 2，x = 3；未调用内核。") }
          GlassIcon(symbol: "ellipsis", label: "更多") { withAnimation(.spring(response: 0.36, dampingFraction: 0.83)) { menu.toggle() } }
        }.padding(.horizontal, 16).padding(.top, 3)
        Spacer()
        if !study.editing {
          PhoneDock(scrolled: scrolled, outline: { outline = true }, add: { add = true }, inspector: {
            withAnimation(.spring(response: 0.42, dampingFraction: 0.9)) { study.phonePanel = .steps }
          }).padding(.bottom, 29)
        }
      }.allowsHitTesting(true)
      if menu {
        Color.clear.contentShape(Rectangle()).onTapGesture { withAnimation { menu = false } }
        FloatingMenu(close: { menu = false }, add: { menu = false; add = true }, outline: { menu = false; outline = true })
          .frame(width: 240).padding(.top, 112).padding(.trailing, 16)
          .frame(maxWidth: .infinity, alignment: .trailing)
          .transition(.scale(scale: 0.12, anchor: .topTrailing).combined(with: .opacity))
      }
      if let panel = study.phonePanel {
        Color.black.opacity(0.16).onTapGesture { withAnimation { study.phonePanel = nil } }
        PhoneSheet {
          Inspector(panel: Binding(get: { study.phonePanel ?? panel }, set: { study.phonePanel = $0 }), draft: $study.phoneDraft, phone: true, close: { withAnimation { study.phonePanel = nil } })
        }.transition(.move(edge: .bottom).combined(with: .opacity))
      }
      if outline {
        Color.black.opacity(0.16).onTapGesture { withAnimation { outline = false } }
        PhoneSheet {
          VStack(spacing: 0) {
            SheetTitle(title: "目录") { outline = false }
            Outline(close: { outline = false })
          }
        }.transition(.move(edge: .bottom))
      }
      if add {
        Color.black.opacity(0.16).onTapGesture { add = false }
        VStack { Spacer(); VStack(spacing: 0) {
          SheetTitle(title: "添加单元格") { add = false }
          ForEach([("function", "数学"), ("text.alignleft", "文字")], id: \.1) { item in
            Button { add = false; study.hint("草稿仅展示添加入口，不修改真实文稿。") } label: {
              Label(item.1, systemImage: item.0).frame(maxWidth: .infinity, alignment: .leading).padding(20)
            }.buttonStyle(.plain)
          }
        }.padding(.bottom, 28).background(.regularMaterial, in: UnevenRoundedRectangle(topLeadingRadius: 28, topTrailingRadius: 28)) }
      }
      if study.editing { KeyboardStudy().frame(maxHeight: .infinity, alignment: .bottom) }
      Capsule().fill(.primary).frame(width: 130, height: 5).frame(maxHeight: .infinity, alignment: .bottom).padding(.bottom, 8).allowsHitTesting(false)
    }.clipped()
  }
}
struct PhoneDock: View {
  @EnvironmentObject private var study: Study
  var scrolled: Bool
  var outline: () -> Void, add: () -> Void, inspector: () -> Void
  var body: some View {
    GlassEffectContainer(spacing: 14) {
      HStack(spacing: 12) {
        HStack(spacing: 2) {
          Button(action: outline) { Image(systemName: "list.bullet").frame(width: 49, height: 52) }.help("目录")
          Button(action: add) {
            HStack(spacing: 7) {
              Image(systemName: "plus").fontWeight(.medium)
              if !scrolled { Text("单元格").font(.system(size: 14, weight: .medium)).transition(.opacity) }
            }.padding(.horizontal, 14).frame(height: 52)
          }.help("添加单元格")
        }.font(.system(size: 20)).buttonStyle(.plain).padding(.horizontal, 6)
          .glassEffect(study.glass.interactive(), in: .capsule)
        Button(action: inspector) {
          Image(systemName: "sidebar.right").font(.system(size: 21)).frame(width: 54, height: 54)
        }.buttonStyle(.plain).foregroundStyle(study.green)
          .glassEffect(study.glass.tint(study.green.opacity(0.10)).interactive(), in: .circle)
          .help("检查面板：助手、步骤、变量")
      }
    }.animation(.spring(response: 0.36, dampingFraction: 0.83), value: scrolled)
  }
}
struct FloatingMenu: View {
  @EnvironmentObject private var study: Study
  var close: () -> Void, add: () -> Void, outline: () -> Void
  var body: some View {
    VStack(spacing: 0) {
      menuItem("添加单元格", "plus", action: add)
      Divider().padding(.horizontal, 15)
      menuItem("文稿目录", "list.bullet", action: outline)
      Divider().padding(.horizontal, 15)
      menuItem("分享与导出", "square.and.arrow.up") { close(); study.hint("此处将打开系统分享面板。") }
    }.padding(.vertical, 6).glassEffect(study.glass, in: RoundedRectangle(cornerRadius: 24))
  }
  private func menuItem(_ text: String, _ icon: String, action: @escaping () -> Void) -> some View {
    Button(action: action) { HStack { Text(text); Spacer(); Image(systemName: icon) }.font(.system(size: 15)).padding(.horizontal, 20).frame(height: 49) }.buttonStyle(.plain)
  }
}
struct PhoneSheet<Content: View>: View {
  @ViewBuilder var content: () -> Content
  var body: some View {
    VStack { Spacer(minLength: 0); VStack(spacing: 0) {
      Capsule().fill(.secondary.opacity(0.4)).frame(width: 35, height: 5).padding(.top, 10).padding(.bottom, 9)
      content()
    }.frame(height: 735).background(.regularMaterial, in: UnevenRoundedRectangle(topLeadingRadius: 29, topTrailingRadius: 29)) }
  }
}
struct SheetTitle: View {
  let title: String
  var close: () -> Void
  var body: some View {
    HStack { Text(title).font(.system(size: 21, weight: .semibold)); Spacer(); GlassIcon(symbol: "xmark", label: "关闭", action: close) }
      .padding(.horizontal, 23).padding(.vertical, 12)
  }
}

struct PadWorkbench: View {
  @EnvironmentObject private var study: Study
  @State private var sidebar = true
  @State private var scrolled = false
  @State private var more = false
  private var compact: Bool { study.orientation == .compact }
  private var threeColumns: Bool { study.orientation == .landscape }
  var body: some View {
    ZStack(alignment: .top) {
      study.paper
      HStack(spacing: 0) {
        if threeColumns && sidebar {
          Outline(close: {}).frame(width: 192).padding(.top, 97)
            .frame(maxHeight: .infinity).background(.regularMaterial)
        }
        Notebook(phone: false, source: $study.padSource, panel: $study.padPanel, scrolled: $scrolled)
        if !compact, let panel = study.padPanel {
          Inspector(panel: Binding(get: { study.padPanel ?? panel }, set: { study.padPanel = $0 }), draft: $study.padDraft, phone: false, close: { withAnimation { study.padPanel = nil } })
            .padding(.top, 91).frame(width: threeColumns ? 310 : 298)
            .frame(maxHeight: .infinity).background(.regularMaterial)
        }
      }
      VStack(spacing: 0) {
        StatusBar()
        HStack(spacing: 12) {
          GlassEffectContainer(spacing: 10) {
            HStack(spacing: 5) {
              Button { study.hint("OpenMath · 示例文稿") } label: { Image(systemName: "sum").font(.system(size: 20, weight: .medium)).frame(width: 44, height: 44) }
              Button { withAnimation { sidebar.toggle() } } label: { Image(systemName: "sidebar.left").font(.system(size: 18)).frame(width: 44, height: 44) }
            }.buttonStyle(.plain).foregroundStyle(study.green).padding(.horizontal, 4)
              .glassEffect(study.glass.interactive(), in: .capsule)
          }
          VStack(alignment: .leading, spacing: 3) {
            Text("二次方程与图形").font(.system(size: 15, weight: .semibold))
            Label("已保存 · 内核就绪", systemImage: "checkmark").font(.system(size: 10)).foregroundStyle(.secondary)
          }.padding(.leading, 4)
          Spacer(minLength: 5)
          HStack(spacing: 2) {
            Button { study.padSource = originalSource; study.hint("已恢复示例源码") } label: { Image(systemName: "arrow.uturn.backward").frame(width: 45, height: 44) }
            Button { study.hint("此处将打开系统分享面板。") } label: { Image(systemName: "square.and.arrow.up").frame(width: 45, height: 44) }
            Button { study.hint("草稿仅展示添加入口。") } label: { Image(systemName: "plus").frame(width: 45, height: 44) }
          }.font(.system(size: 18)).buttonStyle(.plain).padding(.horizontal, 5).glassEffect(study.glass.interactive(), in: .capsule)
          Button { study.hint("预置示例：x = 2，x = 3；未调用内核。") } label: {
            Label("运行", systemImage: "play.fill").font(.system(size: 14, weight: .semibold)).padding(.horizontal, 17).frame(height: 44)
          }.buttonStyle(.plain).foregroundStyle(study.green).glassEffect(study.glass.tint(study.green.opacity(0.13)).interactive(), in: .capsule)
          GlassIcon(symbol: "sidebar.right", label: "检查面板") { withAnimation { study.padPanel = study.padPanel == nil ? .steps : nil } }
        }.padding(.horizontal, 17).padding(.top, 4)
        Spacer()
        if compact {
          PhoneDock(scrolled: scrolled, outline: { study.hint("窄窗口目录以面板打开。") }, add: { study.hint("草稿仅展示添加入口。") }, inspector: { withAnimation { study.padPanel = .steps; more = true } })
            .padding(.bottom, 24)
        }
      }
      if compact && more, let panel = study.padPanel {
        Color.black.opacity(0.16).onTapGesture { more = false }
        PhoneSheet {
          Inspector(panel: Binding(get: { study.padPanel ?? panel }, set: { study.padPanel = $0 }), draft: $study.padDraft, phone: true, close: { more = false })
        }.transition(.move(edge: .bottom))
      }
      Capsule().fill(.primary.opacity(0.7)).frame(width: 205, height: 4).frame(maxHeight: .infinity, alignment: .bottom).padding(.bottom, 7).allowsHitTesting(false)
    }.clipped()
  }
}

struct Notebook: View {
  @EnvironmentObject private var study: Study
  let phone: Bool
  @Binding var source: String
  @Binding var panel: Panel?
  @Binding var scrolled: Bool
  var scrollAnchor: Binding<String?> = .constant(nil)
  var topInset: CGFloat? = nil
  var bottomInset: CGFloat? = nil
  var deviceName: String? = nil
  var stale: Bool { source != originalSource }
  var body: some View {
    ScrollViewReader { proxy in
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          Text("数学笔记 / 代数").id("intro").font(.system(size: 10)).foregroundStyle(.secondary).padding(.bottom, 10)
          Text("二次方程与图形").font(.system(size: phone ? 28 : 31, weight: .semibold, design: .serif)).padding(.bottom, 9)
          HStack(spacing: 7) {
            Text("今天 14:02"); Text("·"); Circle().fill(study.green).frame(width: 5, height: 5); Text("本地已保存")
          }.font(.system(size: 10)).foregroundStyle(.secondary).padding(.bottom, phone ? 29 : 32)
          Text("求解").font(.system(size: 21, weight: .semibold, design: .serif)).padding(.bottom, 10)
          Text("先求方程的根，再从图形中观察它们与 x 轴交点的关系。")
            .font(.system(size: 16, design: .serif)).foregroundStyle(.secondary).lineSpacing(6).fixedSize(horizontal: false, vertical: true).padding(.bottom, 21)
          HStack(alignment: .top, spacing: 11) {
            Rectangle().fill(stale ? Color.secondary.opacity(0.5) : study.green).frame(width: 3)
            VStack(alignment: .leading, spacing: 0) {
              if study.editing {
                VStack(alignment: .leading, spacing: 9) {
                  HStack { Text("Modern"); Spacer(); Text("正在编辑").foregroundStyle(study.green) }.font(.system(size: 10))
                  TextEditor(text: $source).font(.system(size: 13, design: .monospaced))
                    .scrollContentBackground(.hidden).frame(height: 59).padding(.horizontal, -4)
                    .accessibilityLabel((deviceName ?? (phone ? "iPhone" : "iPad")) + " 数学源码")
                }.padding(12).background(study.well, in: RoundedRectangle(cornerRadius: 9))
                  .overlay { RoundedRectangle(cornerRadius: 9).strokeBorder(study.green.opacity(0.6), lineWidth: 1) }
              } else {
                SourceWell(text: source, phone: phone).onTapGesture { study.editing = true }
              }
              HStack(spacing: 28) {
                Text("𝑥 = 2,"); Text("𝑥 = 3")
              }.font(.system(size: 27, design: .serif)).opacity(stale ? 0.34 : 1)
                .padding(.horizontal, 11).padding(.top, 20).padding(.bottom, 15)
              Label(stale ? "源码已更改 · 上次结果，需重新运行" : "精确解，已代入验证", systemImage: stale ? "clock" : "checkmark.seal")
                .font(.system(size: phone ? 10 : 11)).foregroundStyle(stale ? Color.secondary : study.green).padding(.leading, 11)
              HStack(spacing: 16) {
                Button("步骤 ⌄") { withAnimation(.spring(response: 0.42, dampingFraction: 0.9)) { panel = .steps } }.foregroundStyle(study.green)
                Button("绘图") { withAnimation { proxy.scrollTo("plot", anchor: .top) } }
                Button("复制") { NSPasteboard.general.clearContents(); NSPasteboard.general.setString("x = 2, x = 3", forType: .string); study.hint("已复制示例解集") }
                Spacer(minLength: 2)
                Text("2 个解 · 9 ms").font(.system(size: 9, design: .monospaced)).foregroundStyle(.tertiary)
              }.buttonStyle(.plain).font(.system(size: 11)).foregroundStyle(.secondary).frame(height: 44).padding(.leading, 11)
            }
          }.fixedSize(horizontal: false, vertical: true).id("solve")
          Text("看见这两个根").font(.system(size: 21, weight: .semibold, design: .serif)).padding(.top, 29).padding(.bottom, 13).id("plot")
          HStack(alignment: .top, spacing: 11) {
            Rectangle().fill(study.green).frame(width: 3)
            VStack(spacing: 0) {
              SourceWell(text: "plot(x^2 - 5*x + 6, x, 0, 5)", phone: phone)
              Parabola().frame(height: phone ? 219 : 230).padding(.top, 8)
              HStack { Text("与 x 轴交于 2、3"); Spacer(); Button("图形信息") { study.hint("预置图形：x ∈ [0, 5]，y ∈ [−1, 4]。") }.buttonStyle(.plain).foregroundStyle(study.green) }
                .font(.system(size: 10)).foregroundStyle(.secondary).padding(.horizontal, 7).padding(.vertical, 8)
            }
          }.fixedSize(horizontal: false, vertical: true)
          Text("观察与结论").font(.system(size: 20, weight: .semibold, design: .serif)).padding(.top, 30).padding(.bottom, 10)
          Text("图形的两个零点，与精确求解得到的根一致。")
            .font(.system(size: 16, design: .serif)).foregroundStyle(.secondary).lineSpacing(6)
          Text("代数给出精确值，图形帮助我们看清关系。")
            .font(.system(size: 14, design: .serif)).foregroundStyle(.secondary).padding(.top, 22)
        }
        .scrollTargetLayout()
        .padding(.horizontal, phone ? 23 : 37).padding(.top, topInset ?? (phone ? 140 : 119)).padding(.bottom, bottomInset ?? (phone ? 125 : 65))
        .frame(maxWidth: 800, alignment: .leading).frame(maxWidth: .infinity)
      }.scrollIndicators(.hidden)
        .scrollPosition(id: scrollAnchor, anchor: .top)
        .onScrollGeometryChange(for: Bool.self) { $0.contentOffset.y > 30 } action: { _, value in scrolled = value }
    }
  }
}
struct SourceWell: View {
  @EnvironmentObject private var study: Study
  let text: String, phone: Bool
  var body: some View {
    ScrollView(.horizontal) {
      Text(text).font(.system(size: phone ? 12.5 : 13, design: .monospaced)).foregroundStyle(.primary.opacity(0.9)).fixedSize()
        .padding(.horizontal, 12).padding(.vertical, 14)
    }.scrollIndicators(.hidden).background(study.well, in: RoundedRectangle(cornerRadius: 8))
  }
}
struct Parabola: View {
  @EnvironmentObject private var study: Study
  var body: some View {
    Canvas { context, size in
      let left = 29.0, right = size.width - 21, top = 20.0, bottom = size.height - 37
      func x(_ value: Double) -> Double { left + value / 5 * (right - left) }
      func y(_ value: Double) -> Double { bottom - (value + 1) / 5 * (bottom - top) }
      var grid = Path()
      for i in 0...5 { grid.move(to: CGPoint(x: x(Double(i)), y: top)); grid.addLine(to: CGPoint(x: x(Double(i)), y: bottom)) }
      for i in -1...4 { grid.move(to: CGPoint(x: left, y: y(Double(i)))); grid.addLine(to: CGPoint(x: right, y: y(Double(i)))) }
      context.stroke(grid, with: .color(study.line), lineWidth: 0.65)
      var axis = Path(); axis.move(to: CGPoint(x: left, y: y(0))); axis.addLine(to: CGPoint(x: right + 6, y: y(0)))
      axis.move(to: CGPoint(x: left, y: bottom + 4)); axis.addLine(to: CGPoint(x: left, y: top - 8))
      context.stroke(axis, with: .color(.secondary.opacity(0.6)), lineWidth: 0.8)
      var curve = Path()
      for i in 0...180 {
        let value = 0.44 + Double(i) / 180 * 4.12
        let point = CGPoint(x: x(value), y: y(value * value - 5 * value + 6))
        if i == 0 { curve.move(to: point) } else { curve.addLine(to: point) }
      }
      context.stroke(curve, with: .color(curveBlue), style: StrokeStyle(lineWidth: 2.3, lineCap: .round))
      for value in [2.0, 3.0] {
        let circle = Path(ellipseIn: CGRect(x: x(value)-3.6, y: y(0)-3.6, width: 7.2, height: 7.2))
        context.fill(circle, with: .color(study.paper)); context.stroke(circle, with: .color(.orange), lineWidth: 1.8)
      }
      for i in 1...5 { context.draw(Text("\(i)").font(.system(size: 9)).foregroundStyle(.secondary), at: CGPoint(x: x(Double(i)), y: y(0)+14)) }
      for i in [0,2,4] { context.draw(Text("\(i)").font(.system(size: 9)).foregroundStyle(.secondary), at: CGPoint(x: left-13, y: y(Double(i)))) }
      context.draw(Text("x").font(.system(size: 12, design: .serif).italic()).foregroundStyle(.secondary), at: CGPoint(x: right+12, y: y(0)))
      context.draw(Text("y").font(.system(size: 12, design: .serif).italic()).foregroundStyle(.secondary), at: CGPoint(x: left, y: top-12))
      context.draw(Text("—   y = x² − 5x + 6").font(.system(size: 10)).foregroundStyle(curveBlue), at: CGPoint(x: size.width/2, y: size.height-9))
    }.accessibilityLabel("二次函数图，零点为二和三")
  }
}

struct Outline: View {
  @EnvironmentObject private var study: Study
  var close: () -> Void
  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      Text("目录").font(.system(size: 16, weight: .semibold)).padding(.horizontal, 13).padding(.bottom, 22)
      Text("求解").font(.system(size: 11)).foregroundStyle(.secondary).padding(.horizontal, 13).padding(.bottom, 9)
      Button(action: close) { HStack(spacing: 7) { Circle().fill(study.green).frame(width: 5, height: 5); Text("solve(x^2 − 5*x + 6…)").lineLimit(1).font(.system(size: 11, design: .monospaced)) }.frame(maxWidth: .infinity, alignment: .leading).padding(12).background(study.green.opacity(0.10), in: RoundedRectangle(cornerRadius: 8)) }.buttonStyle(.plain)
      Text("图形").font(.system(size: 11)).foregroundStyle(.secondary).padding(.horizontal, 13).padding(.top, 25).padding(.bottom, 10)
      Button(action: close) { Label("plot(x^2 − 5*x + 6…)", systemImage: "chart.xyaxis.line").font(.system(size: 11)).lineLimit(1).padding(12) }.buttonStyle(.plain)
      Button(action: close) { Label("观察与结论", systemImage: "text.alignleft").font(.system(size: 13)).padding(12) }.buttonStyle(.plain)
      Spacer()
      Divider().padding(.bottom, 15)
      Button { study.hint("此处将打开系统文稿浏览器。") } label: { Label("所有文稿", systemImage: "folder").font(.system(size: 12)).padding(.horizontal, 10).padding(.vertical, 12) }.buttonStyle(.plain)
      Text("5 个单元格 · Modern").font(.system(size: 10)).foregroundStyle(.secondary).padding(.horizontal, 10).padding(.top, 15)
    }.padding(.horizontal, 12).padding(.top, 23).padding(.bottom, 33)
  }
}
struct Inspector: View {
  @EnvironmentObject private var study: Study
  @Binding var panel: Panel
  @Binding var draft: String
  var phone: Bool
  var close: () -> Void
  @Namespace private var selection
  var body: some View {
    VStack(spacing: 0) {
      HStack(spacing: 5) {
        GlassEffectContainer(spacing: 8) {
          HStack(spacing: 2) {
            ForEach(Panel.allCases) { item in
              if panel == item {
                tab(item).glassEffect(study.glass, in: .capsule).glassEffectID("selection", in: selection)
              } else {
                tab(item)
              }
            }
          }
        }
        if phone { GlassIcon(symbol: "xmark", label: "关闭检查面板", action: close).scaleEffect(0.86).frame(width: 42) }
      }.padding(.horizontal, phone ? 17 : 16).padding(.top, phone ? 0 : 13).padding(.bottom, 16)
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          switch panel {
          case .steps: StepsContent(phone: phone)
          case .assistant: AssistantContent(phone: phone)
          case .variables: VariableContent()
          }
        }.padding(.horizontal, phone ? 25 : 23).padding(.top, 8).padding(.bottom, 24).frame(maxWidth: .infinity, alignment: .leading)
      }.scrollIndicators(.hidden)
      if panel == .assistant { Composer(draft: $draft).padding(.horizontal, 13).padding(.bottom, phone ? 33 : 25) }
    }.frame(maxHeight: .infinity)
  }
  private func tab(_ item: Panel) -> some View {
    Button { withAnimation(.spring(response: 0.34, dampingFraction: 0.8)) { panel = item } } label: {
      Text(item.rawValue).font(.system(size: 13, weight: panel == item ? .medium : .regular))
        .foregroundStyle(panel == item ? Color.primary : Color.secondary).frame(maxWidth: .infinity).frame(height: 41)
    }.buttonStyle(.plain)
  }
}
struct StepsContent: View {
  @EnvironmentObject private var study: Study
  var phone: Bool
  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      Text("单元格 2 · 求解记录").font(.system(size: 11)).foregroundStyle(.secondary).padding(.bottom, 10)
      Text("二次方程的根").font(.system(size: phone ? 21 : 18, weight: .semibold)).padding(.bottom, 24)
      Text("𝑥² − 5𝑥 + 6 = 0").font(.system(size: 24, design: .serif)).padding(.bottom, 24)
      Divider().padding(.bottom, 24)
      step(1, "分解因式", "(𝑥 − 2)(𝑥 − 3) = 0", "寻找乘积为 6、和为 5 的两个数。")
      step(2, "分别令因式为零", "𝑥 − 2 = 0\n𝑥 − 3 = 0", "")
      step(3, "得到精确解", "𝑥 = 2，𝑥 = 3", "")
      Divider().padding(.top, 2).padding(.bottom, 20)
      Label("已代回原式验证", systemImage: "checkmark.seal").font(.system(size: 11)).foregroundStyle(study.green).padding(.bottom, 12)
      Text("2² − 5 × 2 + 6 = 0\n3² − 5 × 3 + 6 = 0").font(.system(size: 17, design: .serif)).lineSpacing(8)
      Text("预置推导示例").font(.system(size: 9)).foregroundStyle(.tertiary).padding(.top, 26)
    }
  }
  private func step(_ number: Int, _ title: String, _ math: String, _ note: String) -> some View {
    HStack(alignment: .top, spacing: 11) {
      Text("\(number)").font(.system(size: 10)).foregroundStyle(.secondary).frame(width: 21, height: 21).overlay(Circle().strokeBorder(study.line, lineWidth: 1))
      VStack(alignment: .leading, spacing: 10) {
        Text(title).font(.system(size: phone ? 14 : 13, weight: .medium))
        Text(math).font(.system(size: phone ? 24 : 21, design: .serif)).lineSpacing(9)
        if !note.isEmpty { Text(note).font(.system(size: 11)).foregroundStyle(.secondary).lineSpacing(5) }
      }
    }.padding(.bottom, 27)
  }
}
struct AssistantContent: View {
  var phone: Bool
  var body: some View {
    VStack(alignment: .leading, spacing: 18) {
      Text("当前笔记本").font(.system(size: 11)).foregroundStyle(.secondary)
      Text("理解这两个根").font(.system(size: 19, weight: .semibold))
      Text("这两个解和图形上的交点，有什么关系？").font(.system(size: 14)).lineSpacing(6)
        .padding(14).background(.primary.opacity(0.045), in: RoundedRectangle(cornerRadius: 17)).padding(.leading, 25)
      Label("助手  ·  讨论示例", systemImage: "bubble.left").font(.system(size: 11)).foregroundStyle(agentPurple)
      Text("它们是同一件事的两种表达。令函数值为 0，就是在找曲线与 x 轴的交点。")
        .font(.system(size: 14)).lineSpacing(7)
      VStack(alignment: .leading, spacing: 15) {
        Label("读取单元格 2、4     ✓", systemImage: "book")
        Label("查看求根与图形结果   ✓", systemImage: "chart.xyaxis.line")
      }.font(.system(size: 11)).foregroundStyle(.secondary).padding(.leading, 12).overlay(alignment: .leading) { Rectangle().fill(agentPurple.opacity(0.16)).frame(width: 2) }
      Text("𝑓(2) = 0，𝑓(3) = 0").font(.system(size: 24, design: .serif))
      Text("两个精确解，对应图形上的 (2, 0) 和 (3, 0)。").font(.system(size: 14)).lineSpacing(7)
    }
  }
}
struct VariableContent: View {
  var body: some View {
    VStack(alignment: .leading, spacing: 22) {
      Text("变量与定义").font(.system(size: 21, weight: .semibold))
      HStack { Text("x").font(.system(size: 16, design: .monospaced)); Spacer(); Text("求解自变量").foregroundStyle(.secondary) }.font(.system(size: 13))
      Divider()
      Text("当前文稿没有全局赋值。求解得到的是解集，不会覆盖变量。").font(.system(size: 14)).foregroundStyle(.secondary).lineSpacing(7)
    }
  }
}
struct Composer: View {
  @EnvironmentObject private var study: Study
  @Binding var draft: String
  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Label("当前笔记本", systemImage: "book").font(.system(size: 10)).foregroundStyle(.secondary)
      TextField("继续提问，或引用单元格…", text: $draft, axis: .vertical).font(.system(size: 13)).textFieldStyle(.plain).lineLimit(2...3)
      HStack(spacing: 7) {
        Button { study.hint("此处将打开附件选择器。") } label: { Image(systemName: "plus").frame(width: 25, height: 33) }
        Text("讨论 ⌄").font(.system(size: 11)); Spacer()
        Text("示例模型 ⌄").font(.system(size: 10))
        Button { study.hint("已保留输入；草稿不会发送模型请求。") } label: { Image(systemName: "arrow.up").font(.system(size: 15, weight: .medium)).frame(width: 34, height: 34) }
          .foregroundStyle(study.green).glassEffect(study.glass.tint(study.green.opacity(0.18)).interactive(), in: .circle)
      }.buttonStyle(.plain).foregroundStyle(.secondary)
    }.padding(14).background(study.paper, in: RoundedRectangle(cornerRadius: 21))
      .overlay { RoundedRectangle(cornerRadius: 21).strokeBorder(study.line, lineWidth: 1) }
  }
}
struct KeyboardStudy: View {
  @EnvironmentObject private var study: Study
  var body: some View {
    VStack(spacing: 0) {
      HStack(spacing: 0) {
        ForEach(["𝑥", "²", "√", "(", ")", "=", "π"], id: \.self) { key in Text(key).font(.system(size: 21, design: .serif)).frame(maxWidth: .infinity).frame(height: 46) }
        Button("完成") { study.editing = false }.font(.system(size: 13, weight: .semibold)).buttonStyle(.plain).foregroundStyle(study.green).padding(.horizontal, 12)
      }.background(.regularMaterial)
      VStack(spacing: 9) {
        HStack { Text("数学源码"); Spacer(); Text("Modern"); Spacer(); Text("⌘ ↵ 运行") }.font(.system(size: 10)).foregroundStyle(.secondary).padding(.horizontal, 18).padding(.vertical, 3)
        ForEach(["qwertyuiop", "asdfghjkl", "zxcvbnm"], id: \.self) { row in
          HStack(spacing: 5) { ForEach(Array(row), id: \.self) { key in Text(String(key)).font(.system(size: 20)).frame(maxWidth: .infinity).frame(height: 38).background(study.paper, in: RoundedRectangle(cornerRadius: 5)).shadow(color: .black.opacity(0.16), radius: 0, y: 1) } }.padding(.horizontal, row.count == 10 ? 6 : row.count == 9 ? 24 : 38)
        }
        HStack(spacing: 6) {
          Text("123").frame(width: 45); Text("空格").frame(maxWidth: .infinity).frame(height: 38).background(study.paper, in: RoundedRectangle(cornerRadius: 5)); Text("运行").foregroundStyle(.white).frame(width: 77, height: 38).background(accent, in: RoundedRectangle(cornerRadius: 5))
        }.font(.system(size: 13)).padding(.horizontal, 6)
      }.padding(.top, 7).padding(.bottom, 33).background(study.dark ? Color(white: 0.16) : Color(red: 0.83, green: 0.85, blue: 0.87))
    }
  }
}
