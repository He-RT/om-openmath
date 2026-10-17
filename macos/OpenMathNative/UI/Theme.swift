import AppKit
import SwiftUI

/// Workbench v2 (§4). System surfaces and text follow the user's macOS appearance.
enum Theme {
  enum Color {
    static let paper = SwiftUI.Color(nsColor: .textBackgroundColor)
    static let chrome = SwiftUI.Color(nsColor: .windowBackgroundColor)
    static let ink = SwiftUI.Color(nsColor: .labelColor)
    static let ink2 = SwiftUI.Color(nsColor: .secondaryLabelColor)
    static let ink3 = SwiftUI.Color(nsColor: .tertiaryLabelColor)
    static let line = SwiftUI.Color(nsColor: .separatorColor)
    static let red = SwiftUI.Color(nsColor: .systemRed)
    static let well = SwiftUI.Color("Well")
    static let lineStrong = SwiftUI.Color("LineStrong")
    static let verified = SwiftUI.Color("AccentColor")
    static let agent = SwiftUI.Color("Agent")
    static let conditional = SwiftUI.Color("Conditional")
    static let codeBackground = SwiftUI.Color("CodeBackground")
    static let plot1 = SwiftUI.Color("Plot1")
    static let plot2 = SwiftUI.Color("Plot2")
    static let sceneBackground = SwiftUI.Color("SceneBackground")
  }

  @MainActor enum Font {
    static let ui = SwiftUI.Font.system(size: 13)
    static let conversation = SwiftUI.Font.system(size: 13.5)
    static let paperTitle = SwiftUI.Font(nsPaperTitle)
    static let paperH2 = SwiftUI.Font(nsPaperH2)
    static let paperBody = SwiftUI.Font(nsPaperBody)
    static let source = SwiftUI.Font(nsSource)
    static let code = SwiftUI.Font.system(size: 12.5, design: .monospaced)

    static let nsPaperTitle = paper(size: 30, weight: .semibold)
    static let nsPaperH2 = paper(size: 21, weight: .regular)
    static let nsPaperBody = paper(size: 16.5, weight: .regular)
    static let nsSource = NSFont.monospacedSystemFont(ofSize: 13.5, weight: .regular)

    private static func paper(size: CGFloat, weight: NSFont.Weight) -> NSFont {
      let system = NSFont.systemFont(ofSize: size, weight: weight)
      let serif = system.fontDescriptor.withDesign(.serif) ?? system.fontDescriptor
      let descriptor = serif.addingAttributes([
        .cascadeList: [NSFontDescriptor(fontAttributes: [.family: "Songti SC"])],
      ])
      return NSFont(descriptor: descriptor, size: size) ?? system
    }
  }

  enum Space {
    static let xxs: CGFloat = 4
    static let xs: CGFloat = 8
    static let sm: CGFloat = 12
    static let md: CGFloat = 16
    static let lg: CGFloat = 20
    static let xl: CGFloat = 24
    static let xxl: CGFloat = 32
  }

  enum Radius {
    static let well: CGFloat = 8
    static let code: CGFloat = 6
    static let bubble: CGFloat = 14
    static let composer: CGFloat = 14
    static let menu: CGFloat = 11
  }

  enum Symbol {
    static let outline = "sidebar.left"
    static let inspector = "sidebar.right"
    static let run = "play.fill"
    static let stop = "stop.fill"
    static let add = "plus"
    static let more = "ellipsis"
    static let verified = "checkmark.seal"
    static let success = "checkmark"
    static let conditional = "exclamationmark.circle"
    static let error = "exclamationmark.triangle"
    static let read = "book"
    static let lookup = "magnifyingglass"
    static let preview = "eye"
    static let edit = "pencil"
    static let inspect = "cube"
    static let locate = "scope"
    static let undo = "arrow.uturn.backward"
    static let copy = "doc.on.doc"
    static let send = "arrow.up"
    static let history = "clock.arrow.circlepath"
    static let newConversation = "square.and.pencil"
    static let attachment = "plus"
  }
}
