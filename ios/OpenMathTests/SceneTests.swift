import XCTest
@testable import OpenMath

final class SceneTests: XCTestCase {
  @MainActor func testMobilePreservesActual3DSourceAndExplicitFallbackWithoutGeneratingMeshes() async throws {
    let client = try KernelClient()
    defer { client.close() }
    for source in ["plot(sin(x)*cos(y),x:-pi..pi,y:-pi..pi)", "parametric_plot([cos(t),sin(t),t],t:0..2*pi)", "implicit_plot(x^2+y^2+z^2=1,x:-2..2,y:-2..2,z:-2..2)"] {
      let packet = try await client.request(.object(["type": .string("evaluate"), "cell_id": .string("scene"), "source": .string(source), "dialect": .string("Modern")]))
      let output = packet.response.body["output"], item = output["items"][0]
      XCTAssertTrue(output["messages"].array.isEmpty, output.pretty)
      XCTAssertEqual(item["type"].string, "scene3_d", output.pretty)
      XCTAssertTrue(item["data"].isNull)
      XCTAssertTrue(item["unavailable"].string.contains("尚未适配"))
      XCTAssertFalse(item["request"]["expressions"].array.isEmpty)
    }
  }
}
