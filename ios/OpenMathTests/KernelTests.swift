import XCTest

@testable import OpenMath

final class KernelTests: XCTestCase {
  func testActualKernelJSONContract() async throws {
    let client = try KernelClient()
    defer { client.close() }
    let packet = try await client.request(
      .object([
        "type": .string("evaluate"), "cell_id": .string("a"), "source": .string("solve(x^2==4,x)"),
        "dialect": .string("Modern"),
      ]))
    XCTAssertEqual(packet.response.body["type"].string, "evaluated")
    XCTAssertEqual(packet.response.body["output"]["items"][0]["view"]["solutions"].array.count, 2)
    let stored = try await client.request(.object(["type": .string("save_notebook")]))
    XCTAssertTrue(stored.response.body["file"]["config"].isNull)
  }
  func testNotebookRejectsDuplicateIDsAndPreservesUnicode() throws {
    let cell = CellInput(id: "one", source: "solve(α^2==4,α)")
    let file = NotebookFile(title: "Unicode", cells: [cell])
    try file.validate()
    XCTAssertEqual(
      try JSONDecoder().decode(NotebookFile.self, from: JSONEncoder().encode(file)), file)
    XCTAssertThrowsError(try NotebookFile(cells: [cell, cell]).validate())
  }
}
