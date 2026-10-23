import Foundation
import JavaScriptCore

/// URL resolution for data-only schema references. It performs no IO.
@MainActor enum MobileJSHost {
  static func install(in context: JSContext) {
    let resolve: @convention(block) (String, String) -> String = { source, base in
      let url = URL(string: source, relativeTo: base.isEmpty ? nil : URL(string: base))?.absoluteURL
      guard let url, let parts = URLComponents(url: url, resolvingAgainstBaseURL: true) else { return "{}" }
      var origin = (parts.scheme ?? "") + "://" + (parts.host ?? "")
      if let port = parts.port { origin += ":\(port)" }
      let result = ["href": url.absoluteString, "hash": parts.fragment.map { "#" + $0 } ?? "", "origin": origin,
        "protocol": (parts.scheme ?? "") + ":", "pathname": parts.path]
      return String(decoding: (try? JSONEncoder().encode(result)) ?? Data("{}".utf8), as: UTF8.self)
    }
    context.setObject(resolve, forKeyedSubscript: "nativeURL" as NSString)
  }
}
