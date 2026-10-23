import SceneKit
import SwiftUI

/// Kernel-generated geometry only. SceneKit supplies native Metal rendering and camera gestures.
struct SceneView: View {
  let item: JSONValue
  let controller: NotebookController
  @State private var data: JSONValue = .null
  @State private var failure: String?
  @State private var reset = 0
  @State private var details = false
  var sample: ((JSONValue) async throws -> KernelPacket)? = nil
  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      if !data.isNull {
        NativeSceneCanvas(data: data, reset: reset).frame(height: 300)
          .clipShape(RoundedRectangle(cornerRadius: 10)).accessibilityIdentifier("scene.canvas")
        HStack {
          Button(controller.text("复位视角", "Reset camera")) { reset += 1 }
          Button(controller.text("图形信息", "Scene information")) { details.toggle() }
          Spacer()
          ExportArtifactButton(title: "OBJ", name: "OpenMath-scene", version: data["sampled"].pretty,
            enabled: true, controller: controller) {
              .object(["type": .string("export_scene3_d"), "data": data, "title": .string(controller.title)])
            }
        }.font(.caption).buttonStyle(.borderless)
        if details {
          let triangles = data["meshes"].array.reduce(0) { $0 + $1["triangles"].array.count }
          Text(controller.text("\(triangles) 个三角形 · \(data["lines"].array.count) 条曲线", "\(triangles) triangles · \(data["lines"].array.count) curves"))
            .font(.caption).foregroundStyle(.secondary).accessibilityIdentifier("scene.geometry")
          Text(controller.text("拖动旋转，双指平移或缩放。", "Drag to orbit; use two fingers to pan or zoom."))
            .font(.caption).foregroundStyle(.secondary)
        }
      } else if let failure {
        Label(failure, systemImage: "exclamationmark.triangle").foregroundStyle(.secondary)
      } else { ProgressView(controller.text("准备三维图形…", "Preparing 3D scene…")) }
    }.task {
      do {
        if !item["data"].isNull { data = item["data"] }
        else {
          let request = JSONValue.object(["type": .string("sample_scene3_d"), "request": item["request"]])
          let packet = try await (sample ?? controller.call)(request)
          try Task.checkCancellation()
          data = packet.response.body["data"]
          guard !data.isNull else { throw KernelError.message("三维采样未返回几何数据") }
        }
        try MobileSceneGeometry.validate(data)
      } catch { data = .null; failure = error.localizedDescription }
    }
  }
}

struct NativeSceneCanvas: UIViewRepresentable {
  let data: JSONValue
  let reset: Int
  func makeCoordinator() -> Coordinator { Coordinator() }
  final class Coordinator { var data: JSONValue = .null; var reset = -1 }
  func makeUIView(context: Context) -> SCNView {
    let view = SCNView(frame: .zero, options: [SCNView.Option.preferredRenderingAPI.rawValue: SCNRenderingAPI.metal.rawValue])
    view.allowsCameraControl = true; view.autoenablesDefaultLighting = true
    view.rendersContinuously = false; view.isPlaying = false; view.preferredFramesPerSecond = 60
    view.antialiasingMode = .multisampling4X; view.backgroundColor = .secondarySystemBackground
    view.accessibilityLabel = "三维图形，可旋转、平移、缩放 / Interactive 3D scene"
    view.defaultCameraController.inertiaEnabled = false
    return view
  }
  func updateUIView(_ view: SCNView, context: Context) {
    guard data != context.coordinator.data || reset != context.coordinator.reset else { return }
    context.coordinator.data = data; context.coordinator.reset = reset
    guard (try? MobileSceneGeometry.validate(data)) != nil else { return }
    let scene = SCNScene(), bounds = data["bounds"].array
    let spans = (0..<3).map { bounds[1][$0].double - bounds[0][$0].double }
    let scale = max(spans.max() ?? 1, 1e-12) / 2
    func position(_ value: JSONValue) -> SCNVector3 {
      let coordinates = (0..<3).map { Float((value[$0].double - bounds[0][$0].double - spans[$0] / 2) / scale) }
      return SCNVector3(coordinates[0], coordinates[1], coordinates[2])
    }
    func color(_ v: JSONValue) -> UIColor { UIColor(red: v[0].double, green: v[1].double, blue: v[2].double, alpha: v[3].isNull ? 1 : v[3].double) }
    func colored(_ points: [JSONValue], _ colors: [JSONValue], _ indices: [UInt32], _ kind: SCNGeometryPrimitiveType, normals: [JSONValue] = []) -> SCNNode {
      let vertices = points.map(position), rgba = colors.flatMap { $0.array.map { Float($0.double) } }
      var sources = [SCNGeometrySource(vertices: vertices)]
      if !normals.isEmpty { sources.append(SCNGeometrySource(normals: normals.map { SCNVector3($0[0].double, $0[1].double, $0[2].double) })) }
      if rgba.count == vertices.count * 4 {
        let bytes = rgba.withUnsafeBytes { Data($0) }
        sources.append(SCNGeometrySource(data: bytes, semantic: .color, vectorCount: vertices.count, usesFloatComponents: true, componentsPerVector: 4, bytesPerComponent: 4, dataOffset: 0, dataStride: 16))
      }
      let indexBytes = indices.withUnsafeBytes { Data($0) }
      let element = SCNGeometryElement(data: indexBytes, primitiveType: kind, primitiveCount: indices.count / (kind == .triangles ? 3 : kind == .line ? 2 : 1), bytesPerIndex: 4)
      let geometry = SCNGeometry(sources: sources, elements: [element]), material = SCNMaterial()
      material.isDoubleSided = true; material.lightingModel = normals.isEmpty ? .constant : .blinn
      material.diffuse.contents = UIColor.white; geometry.materials = [material]
      return SCNNode(geometry: geometry)
    }
    for mesh in data["meshes"].array {
      scene.rootNode.addChildNode(colored(mesh["positions"].array, mesh["colors"].array,
        mesh["triangles"].array.flatMap { $0.array.map { UInt32($0.double) } }, .triangles, normals: mesh["normals"].array))
    }
    for line in data["lines"].array where line["positions"].array.count > 1 {
      let count = line["positions"].array.count
      scene.rootNode.addChildNode(colored(line["positions"].array, line["colors"].array,
        (0..<(count - 1)).flatMap { [UInt32($0), UInt32($0 + 1)] }, .line))
    }
    for point in data["points"].array {
      let sphere = SCNSphere(radius: 0.015); sphere.firstMaterial?.diffuse.contents = color(point["color"])
      let node = SCNNode(geometry: sphere); node.position = position(point["position"]); scene.rootNode.addChildNode(node)
    }
    for label in data["labels"].array {
      let text = SCNText(string: label["text"].string, extrusionDepth: 0); text.font = .systemFont(ofSize: 10)
      text.firstMaterial?.diffuse.contents = color(label["color"])
      let node = SCNNode(geometry: text); node.scale = SCNVector3(0.008, 0.008, 0.008); node.position = position(label["position"])
      node.constraints = [SCNBillboardConstraint()]; scene.rootNode.addChildNode(node)
    }
    let camera = SCNNode(); camera.camera = SCNCamera(); camera.camera?.zNear = 0.01; camera.camera?.zFar = 100
    camera.position = SCNVector3(3, 2, 4); camera.look(at: SCNVector3Zero)
    scene.rootNode.addChildNode(camera); view.scene = scene; view.pointOfView = camera
    view.defaultCameraController.target = SCNVector3Zero
  }
}
