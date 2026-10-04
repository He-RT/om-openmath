import SwiftUI
import UIKit

extension Color {
  /// Existing OpenMath semantic accent: #286650 light, #8ac9a8 dark.
  static let openMathAccent = Color(
    UIColor { traits in
      traits.userInterfaceStyle == .dark
        ? UIColor(red: 138 / 255, green: 201 / 255, blue: 168 / 255, alpha: 1)
        : UIColor(red: 40 / 255, green: 102 / 255, blue: 80 / 255, alpha: 1)
    })
}
