import XCTest

@testable import OpenMath

final class ValuePresentationTests: XCTestCase {
  @MainActor func testRealRetainedTablePageAndStaleSnapshotAreCheckedByKernel() async throws {
    let client = try KernelClient()
    defer { client.close() }
    let result = try await client.request(.object([
      "type": .string("evaluate"), "cell_id": .string("table"), "dialect": .string("Modern"),
      "source": .string("parse_csv(\"x,说明\\n1,中文\\n2,emoji🙂\")"),
    ]))
    let item = result.response.body["output"]["items"][0]
    let page = item["presentation"]
    XCTAssertEqual(page["kind"].string, "table")
    XCTAssertEqual(page["row_count"].double, 2)
    XCTAssertEqual(page["columns"][1].string, "说明")
    let query = JSONValue.object([
      "cell_id": .string("table"), "out_index": item["out_index"], "view_id": page["view_id"],
      "path": .array([]), "offset": .number(1), "limit": .number(32), "column_offset": .number(1),
      "column_limit": .number(8), "include_source": .bool(false),
    ])
    let reply = try await client.request(.object(["type": .string("inspect_value"), "query": query]))
    XCTAssertEqual(reply.response.body["type"].string, "value_page")
    XCTAssertEqual(reply.response.body["page"]["rows"][0]["id"], page["rows"][1]["id"])
    XCTAssertEqual(reply.response.body["page"]["rows"][0]["cells"][0]["source"]["input_form"].string, "\"emoji🙂\"")
    _ = try await client.request(.object([
      "type": .string("evaluate"), "cell_id": .string("table"), "dialect": .string("Modern"),
      "source": .string("[4,5,6]"),
    ]))
    do {
      _ = try await client.request(.object(["type": .string("inspect_value"), "query": query]))
      XCTFail("An obsolete snapshot must be rejected by the kernel")
    } catch let error as KernelError {
      XCTAssertEqual(error.localizedDescription, "结果已更新、已过期或分页路径无效")
    }
  }
  @MainActor func testAuthoredRecordCannotAcquireFreshScientificProvenance() async throws {
    let client = try KernelClient()
    defer { client.close() }
    let genuine = try await client.request(.object([
      "type": .string("evaluate"), "cell_id": .string("actual"), "dialect": .string("Modern"),
      "source": .string("optimize((x-2)^2,x,scope:\"global\")"),
    ]))
    XCTAssertEqual(genuine.response.body["output"]["items"][0]["presentation"]["origin"]["function_id"].string, "fn_000179")
    let authored = try await client.request(.object([
      "type": .string("evaluate"), "cell_id": .string("authored"), "dialect": .string("Modern"),
      "source": .string("{converged:true,guarantee:\"certified_global\",point:[99]}"),
    ]))
    XCTAssertTrue(authored.response.body["output"]["items"][0]["presentation"]["origin"].isNull)
  }
}
