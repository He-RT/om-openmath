import SwiftUI
import UIKit

extension Color {
  /// Workbench v2 semantic accent: #1D7550 light, #56C891 dark.
  static let openMathAccent = Color(
    UIColor { traits in
      traits.userInterfaceStyle == .dark
        ? UIColor(red: 86 / 255, green: 200 / 255, blue: 145 / 255, alpha: 1)
        : UIColor(red: 29 / 255, green: 117 / 255, blue: 80 / 255, alpha: 1)
    })
}
