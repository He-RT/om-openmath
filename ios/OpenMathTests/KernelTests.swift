import XCTest

@testable import OpenMath

final class KernelTests: XCTestCase {
  func testPacketDecodingRejectsBridgeErrorsAndPreservesNestedData() throws {
    let decoder = JSONDecoder()
    XCTAssertThrowsError(
      try decoder.decode(KernelPacket.self, from: Data(#"{"bridge_error":"Invalid session"}"#.utf8))
    ) { error in
      XCTAssertEqual((error as? KernelError)?.errorDescription, "Invalid session")
    }
    let source = Data(
      #"{"response":{"id":7,"body":{"type":"ok","payload":{"中文":[true,false,null,2.5,"🍉"]}}},"events":[]}"#
        .utf8)
    let packet = try decoder.decode(KernelPacket.self, from: source)
    XCTAssertEqual(packet.response.id, 7)
    XCTAssertEqual(packet.response.body["payload"]["中文"][4].string, "🍉")
    XCTAssertEqual(
      try decoder.decode(JSONValue.self, from: JSONEncoder().encode(packet)),
      try decoder.decode(JSONValue.self, from: source))
  }
  func testActualKernelJSONContract() async throws {
    let client = try KernelClient()
    defer { client.close() }
    let packet = try await client.request(
      .object([
        "type": .string("evaluate"), "cell_id": .string("a"), "source": .string("solve(x^2==4,x)"),
        "dialect": .string("Modern"),
      ]))
    XCTAssertEqual(packet.response.body["type"].string, "evaluated")
    let timing = try XCTUnwrap(packet.transportTiming)
    XCTAssertGreaterThanOrEqual(timing.queuedMS, 0)
    XCTAssertGreaterThanOrEqual(timing.ffiMS, 0)
    XCTAssertGreaterThanOrEqual(timing.decodeMS, 0)
    XCTAssertGreaterThanOrEqual(timing.resumeMS, 0)
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
