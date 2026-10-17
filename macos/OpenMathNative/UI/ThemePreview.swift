#if DEBUG
import SwiftUI

/// A local appearance override leaves the user's system settings untouched.
struct ThemePreview: View {
  @State private var dark = false
  private let swatches: [(String, SwiftUI.Color)] = [
    ("Well", Theme.Color.well), ("LineStrong", Theme.Color.lineStrong),
    ("AccentColor", Theme.Color.verified), ("Agent", Theme.Color.agent),
    ("Conditional", Theme.Color.conditional), ("CodeBackground", Theme.Color.codeBackground),
    ("Plot1", Theme.Color.plot1), ("Plot2", Theme.Color.plot2),
    ("SceneBackground", Theme.Color.sceneBackground),
  ]

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: Theme.Space.xl) {
        HStack {
          Text("工作台 v2 · 令牌预览").font(.headline)
          Spacer()
          Toggle("深色", isOn: $dark).toggleStyle(.switch)
            .accessibilityIdentifier("theme-preview-dark")
        }
        LazyVGrid(columns: Array(repeating: GridItem(.flexible(), alignment: .leading), count: 3), spacing: Theme.Space.md) {
          ForEach(swatches, id: \.0) { name, color in
            VStack(alignment: .leading, spacing: Theme.Space.xs) {
              RoundedRectangle(cornerRadius: Theme.Radius.well).fill(color)
                .overlay(RoundedRectangle(cornerRadius: Theme.Radius.well).stroke(Theme.Color.line))
                .frame(height: 48)
              Text(name).font(.caption.monospaced())
            }
          }
        }
        Divider()
        VStack(alignment: .leading, spacing: Theme.Space.sm) {
          Text("二次方程与图形 · Mathematics").font(Theme.Font.paperTitle)
          Text("求解与推导 · Results").font(Theme.Font.paperH2)
          Text("先求二次方程的根，再看图形。输入与结果各有自己的位置。")
            .font(Theme.Font.paperBody).lineSpacing(7)
          Text("solve(x^2 - 5x + 6 = 0, x)").font(Theme.Font.source)
            .padding(Theme.Space.sm).frame(maxWidth: .infinity, alignment: .leading)
            .background(Theme.Color.well, in: RoundedRectangle(cornerRadius: Theme.Radius.well))
          Label("精确解，已代入验证", systemImage: Theme.Symbol.verified)
            .font(Theme.Font.ui).foregroundStyle(Theme.Color.verified)
          Label("助手 · gpt-6.1-sol，执行", systemImage: Theme.Symbol.edit)
            .font(Theme.Font.conversation).foregroundStyle(Theme.Color.agent)
        }
      }.padding(Theme.Space.xxl)
    }
    .foregroundStyle(Theme.Color.ink).background(Theme.Color.paper)
    .tint(Theme.Color.verified).preferredColorScheme(dark ? .dark : .light)
    .frame(minWidth: 720, minHeight: 660)
  }
}
#endif
