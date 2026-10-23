import Foundation

enum MobileSceneGeometry {
  static func validate(_ data: JSONValue) throws {
    func finite(_ value: JSONValue) -> Bool { if case .number(let n) = value { return n.isFinite }; return false }
    let bounds = data["bounds"].array
    guard bounds.count == 2, bounds.allSatisfy({ $0.array.count == 3 && $0.array.allSatisfy(finite) }) else { throw KernelError.message("无效三维范围") }
    for axis in 0..<3 {
      let width = bounds[1][axis].double - bounds[0][axis].double
      guard width.isFinite, width >= 0 else { throw KernelError.message("无效三维范围") }
    }
    func position(_ point: JSONValue) -> Bool {
      guard point.array.count == 3 else { return false }
      return (0..<3).allSatisfy { axis in
        guard case .number(let coordinate) = point[axis], coordinate.isFinite else { return false }
        let low = bounds[0][axis].double, high = bounds[1][axis].double
        let tolerance = max(1, high - low) * 1e-9
        return coordinate >= low - tolerance && coordinate <= high + tolerance
      }
    }
    func color(_ value: JSONValue) -> Bool { value.array.count == 4 && value.array.allSatisfy { if case .number(let n) = $0 { return n.isFinite && (0...1).contains(n) }; return false } }
    var vertices = 0
    guard data["meshes"].array.count + data["lines"].array.count <= 512 else { throw KernelError.message("三维对象超过显示预算") }
    for mesh in data["meshes"].array {
      let points = mesh["positions"].array
      vertices += points.count
      guard points.count == mesh["normals"].array.count, points.count == mesh["colors"].array.count,
        points.allSatisfy(position),
        mesh["normals"].array.allSatisfy({ $0.array.count == 3 && $0.array.allSatisfy(finite) }),
        mesh["colors"].array.allSatisfy(color),
        mesh["triangles"].array.allSatisfy({ $0.array.count == 3 && $0.array.allSatisfy({ finite($0) && $0.double >= 0 && $0.double.rounded() == $0.double && $0.double < Double(points.count) }) })
      else { throw KernelError.message("无效三维网格") }
    }
    for line in data["lines"].array {
      vertices += line["positions"].array.count
      guard line["positions"].array.count == line["colors"].array.count,
        line["positions"].array.allSatisfy(position), line["colors"].array.allSatisfy(color)
      else { throw KernelError.message("无效三维曲线") }
    }
    guard data["points"].array.allSatisfy({ position($0["position"]) && color($0["color"]) }),
      data["labels"].array.allSatisfy({ position($0["position"]) && color($0["color"]) && $0["text"].string.count <= 10000 }) else { throw KernelError.message("无效三维标注") }
    guard vertices <= 500_000, data["points"].array.count <= 10_000, data["labels"].array.count <= 256 else { throw KernelError.message("三维几何超过显示预算") }
  }
}
