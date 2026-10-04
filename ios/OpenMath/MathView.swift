import SwiftMath
import SwiftUI

/// Math is parsed before display. Unsupported syntax stays visible as source.
struct MathView: View {
  var latex: String
  var source: String = ""
  var fontSize: CGFloat = 21
  var inline = false
  var body: some View {
    if inline {
      ViewThatFits(in: .horizontal) {
        NativeMathLabel(latex: latex, source: source, fontSize: fontSize, inline: true).fixedSize()
        ScrollView(.horizontal) {
          NativeMathLabel(latex: latex, source: source, fontSize: fontSize, inline: true)
            .fixedSize()
        }.scrollIndicators(.hidden)
      }
    } else {
      ScrollView(.horizontal) {
        NativeMathLabel(latex: latex, source: source, fontSize: fontSize, inline: false).fixedSize()
      }.scrollIndicators(.hidden)
    }
  }
}
struct NativeMathLabel: UIViewRepresentable {
  var latex: String
  var source: String
  var fontSize: CGFloat
  var inline: Bool
  @Environment(\.colorScheme) private var scheme
  @Environment(\.dynamicTypeSize) private var typeSize
  func makeUIView(context: Context) -> MathContainer { MathContainer() }
  func updateUIView(_ view: MathContainer, context: Context) {
    _ = typeSize
    view.fallback.adjustsFontForContentSizeCategory = true
    view.math.latex = MathTypesetting.prepare(latex)
    view.math.font = MTFontManager().latinModernFont(
      withSize: UIFontMetrics(forTextStyle: .body).scaledValue(
        for: fontSize, compatibleWith: view.traitCollection))
    view.math.labelMode = inline ? .text : .display
    view.math.textColor = .label
    view.math.textAlignment = .left
    view.math.displayErrorInline = false
    view.fallback.text = source.isEmpty ? latex : source
    view.fallback.font = UIFont.preferredFont(forTextStyle: .body).withDesign(.monospaced)
    view.fallback.textColor = .label
    view.math.isHidden = view.math.error != nil
    view.fallback.isHidden = view.math.error == nil
    view.isAccessibilityElement = true
    view.accessibilityLabel = source.isEmpty ? latex : source
    view.accessibilityTraits = .staticText
    view.invalidateIntrinsicContentSize()
  }
  func sizeThatFits(_ proposal: ProposedViewSize, uiView: MathContainer, context: Context)
    -> CGSize?
  { uiView.intrinsicContentSize }
}
final class MathContainer: UIView {
  let math = MTMathUILabel()
  let fallback = UILabel()
  init() {
    super.init(frame: .zero)
    addSubview(math)
    addSubview(fallback)
    fallback.numberOfLines = 1
  }
  required init?(coder: NSCoder) { fatalError("init(coder:) is unavailable") }
  override var intrinsicContentSize: CGSize {
    let size = math.isHidden ? fallback.intrinsicContentSize : math.intrinsicContentSize
    return CGSize(width: max(1, size.width), height: max(22, size.height))
  }
  override func layoutSubviews() {
    super.layoutSubviews()
    math.frame = bounds
    fallback.frame = bounds
  }
}

/// SwiftMath supports upright names via mathrm, but has no operatorname parser.
/// Only flat ASCII names are converted; source/clipboard LaTeX remains original.
enum MathTypesetting {
  static func prepare(_ latex: String) -> String {
    guard
      let pattern = try? NSRegularExpression(pattern: #"\\operatorname\{([A-Za-z][A-Za-z0-9 ]*)\}"#)
    else { return latex }
    return pattern.stringByReplacingMatches(
      in: latex, range: NSRange(location: 0, length: latex.utf16.count),
      withTemplate: #"\\mathrm{$1}"#)
  }
}
