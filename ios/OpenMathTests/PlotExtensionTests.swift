import XCTest
@testable import OpenMath

final class PlotExtensionTests: XCTestCase {
  @MainActor func testActualExtendedGeometryAndViewportSurviveJSONTransport() async throws {
    let client = try KernelClient()
    defer { client.close() }
    for (source, kind, countKey, count) in [
      ("parametric_plot([cos(t),sin(t)],t:0..2*pi)", "Parametric", "curves", 1),
      ("field_plot([-y,x],x:-2..2,y:-2..2)", "Field", "arrows", 400),
      ("plot(x*y,x:-2..2,y:-2..2,view:\"density\")", "Density", "tiles", 9216),
      ("data_plot([[0,0],[1,2]])", "Data", "points", 2),
      ("histogram([1,1,2,3,3],bins:3)", "Histogram", "tiles", 3),
    ] {
      let packet = try await client.request(.object([
        "type": .string("evaluate"), "cell_id": .string("plot"), "source": .string(source),
        "dialect": .string("Modern"),
      ]))
      let output = packet.response.body["output"]
      XCTAssertTrue(output["messages"].array.isEmpty, output.pretty)
      let item = output["items"][0]
      XCTAssertEqual(item["type"].string, "plot", output.pretty)
      XCTAssertEqual(item["request"]["kind"].string, kind)
      let data = item["data"]
      XCTAssertEqual(countKey == "curves" ? data[countKey].array.count : data["geometry"][countKey].array.count, count)
      if kind == "Histogram" {
        XCTAssertEqual(data["geometry"]["tiles"].array.reduce(0) { $0 + $1["value"].double }, 5)
        var request = item["request"]
        request["y_range"] = .array([.number(1), .number(10)])
        let pan = try await client.request(.object(["type": .string("sample_plot"), "request": request]))
        XCTAssertEqual(pan.response.body["data"]["y_range"], request["y_range"])
      }
      if kind == "Field" {
        for a in data["geometry"]["arrows"].array {
          XCTAssertEqual(a["value"][0].double, -a["start"][1].double)
          XCTAssertEqual(a["value"][1].double, a["start"][0].double)
        }
      }
    }
    let packet = try await client.request(.object([
      "type": .string("evaluate"), "cell_id": .string("log"), "dialect": .string("Modern"),
      "source": .string("plot(x^2,x:1..1000,scale:\"log_log\")"),
    ]))
    XCTAssertEqual(packet.response.body["output"]["items"][0]["data"]["scale"].string, "log_log")
  }
}
