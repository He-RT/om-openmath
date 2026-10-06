import XCTest
import UIKit
@testable import OpenMath

final class ArtifactTests: XCTestCase {
  @MainActor func testActualRetainedDataAndSharedPNGPersistAndReadBack() async throws {
    let client = try KernelClient()
    defer { client.close() }
    let table = try await client.request(.object(["type": .string("evaluate"), "cell_id": .string("data"), "dialect": .string("Modern"), "source": .string("[[1,2],[3,4]]")])).response.body["output"]["items"][0]
    let query: JSONValue = .object(["cell_id": .string("data"), "out_index": table["out_index"], "view_id": table["presentation"]["view_id"], "path": .array([]), "offset": .number(32), "limit": .number(32), "column_offset": .number(8), "column_limit": .number(8), "include_source": .bool(false)])
    let csv = try await client.request(.object(["type": .string("export_value"), "format": .string("csv"), "query": query])).response.body["artifact"]
    let csvBytes = try XCTUnwrap(Data(base64Encoded: csv["base64"].string))
    XCTAssertEqual(String(decoding: csvBytes, as: UTF8.self), "1,2\r\n3,4\r\n")
    let plot = try await client.request(.object(["type": .string("evaluate"), "cell_id": .string("plot"), "dialect": .string("Modern"), "source": .string("plot(x^2,x:-2..2)")])).response.body["output"]["items"][0]
    let figure: JSONValue = .object(["data": plot["data"], "axis_x": .string("α"), "axis_y": .string("y"), "title": .string("地月 L2"), "color": .null, "region": .bool(false), "parameters": .object([:]), "width": .number(1000), "height": .number(600)])
    let png = try await client.request(.object(["type": .string("export_plot"), "format": .string("png"), "figure": figure])).response.body["artifact"]
    let data = try XCTUnwrap(Data(base64Encoded: png["base64"].string))
    XCTAssertEqual(data.count, Int(png["byte_len"].double))
    let image = try XCTUnwrap(UIImage(data: data)); XCTAssertEqual(image.size.width, 1000); XCTAssertEqual(image.size.height, 600)
    let defaults = UserDefaults(suiteName: "ArtifactTests-"+UUID().uuidString)!
    let controller = NotebookController(defaults: defaults)
    try controller.shareArtifact(png, name: "OpenMath-test")
    let url = try XCTUnwrap(controller.shareURL)
    XCTAssertEqual(try Data(contentsOf: url), data)
    try FileManager.default.removeItem(at: url)
  }
}
