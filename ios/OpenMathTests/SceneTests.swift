import XCTest
@testable import OpenMath

final class SceneTests: XCTestCase {
  @MainActor func testMobilePreservesActual3DSourceAndExplicitFallbackWithoutGeneratingMeshes() async throws {
    let client = try KernelClient()
    defer { client.close() }
    for source in ["plot(sin(x)*cos(y),x:-pi..pi,y:-pi..pi)", "parametric_plot([cos(t),sin(t),t],t:0..2*pi)", "implicit_plot(x^2+y^2+z^2=1,x:-2..2,y:-2..2,z:-2..2)", "scene([sphere([0,0,0],1),translate(ellipsoid([0,0,0],[1,2,1]),[3,0,0])],dimensions:3)"] {
      let packet = try await client.request(.object(["type": .string("evaluate"), "cell_id": .string("scene"), "source": .string(source), "dialect": .string("Modern")]))
      let output = packet.response.body["output"], item = output["items"][0]
      XCTAssertTrue(output["messages"].array.isEmpty, output.pretty)
      XCTAssertEqual(item["type"].string, "scene3_d", output.pretty)
      XCTAssertTrue(item["data"].isNull)
      XCTAssertTrue(item["unavailable"].string.contains("尚未适配"))
      XCTAssertFalse(item["request"]["expressions"].array.isEmpty)
    }
  }
  @MainActor func testNativeComposedXYSceneHasRealUnicodeLabelsMarkersAndTransformedPaths() async throws {
    let client = try KernelClient()
    defer { client.close() }
    let source = "let earth_x = 2;scene([point([earth_x,0],color:\"red\"),label(\"地球🙂\",[earth_x,0]),translate(rotate(line([[0,0],[1,0]]),pi/2),[2,3]),disk([0,0],1)])"
    let packet = try await client.request(.object(["type": .string("evaluate"), "cell_id": .string("scene"), "source": .string(source), "dialect": .string("Modern")]))
    let output = packet.response.body["output"], item = output["items"].array.last ?? .null
    XCTAssertTrue(output["messages"].array.isEmpty, output.pretty)
    XCTAssertEqual(item["type"].string,"plot",output.pretty)
    let g = item["data"]["geometry"]
    XCTAssertEqual(g["markers"][0]["position"][0].double,2,accuracy:1e-12)
    XCTAssertEqual(g["labels"][0]["text"].string,"地球🙂")
    XCTAssertEqual(g["paths"][0]["points"][1][0].double,2,accuracy:1e-12)
    XCTAssertEqual(g["paths"][0]["points"][1][1].double,4,accuracy:1e-12)
    XCTAssertFalse(g["polygons"].array.isEmpty)
  }
}
