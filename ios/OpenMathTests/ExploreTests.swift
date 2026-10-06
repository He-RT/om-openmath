import XCTest
@testable import OpenMath

final class ExploreTests: XCTestCase {
  @MainActor func testActualReadonlyContextChangesParametersWithoutMutatingNotebook() async throws {
    let main = try KernelClient(), task = try KernelClient()
    defer { main.close(); task.close() }
    func evaluate(_ source: String, _ id: String) async throws -> JSONValue {
      try await main.request(.object(["type": .string("evaluate"), "cell_id": .string(id), "source": .string(source), "dialect": .string("Modern")])).response.body["output"]
    }
    _ = try await evaluate("let a=99;let rate=2;let f(x)=rate*x", "definitions")
    let output = try await evaluate("explore([[a,f(a)],[rate,a^2]],controls:{a:0..4})", "e")
    let item = output["items"][0]
    XCTAssertEqual(item["type"].string, "explore", output.pretty)
    let query: JSONValue = .object(["cell_id": .string("e"), "out_index": item["out_index"], "view_id": item["view_id"]])
    let context = try await main.request(.object(["type": .string("get_explore_context"), "query": query])).response.body["context"]
    _ = try await evaluate("let a=100;let rate=20;let f(x)=rate*x", "definitions")
    let answer = try await task.request(.object(["type": .string("run_explore_context"), "context": context, "values": .object(["a": .number(3)]), "revision": .number(7)])).response.body
    XCTAssertEqual(answer["result"]["item"]["input_form"].string, "{{3., 6.}, {2, 9.}}")
    XCTAssertEqual(answer["result"]["revision"].double, 7)
    let probe = try await evaluate("[a,rate]", "probe")
    XCTAssertEqual(probe["items"][0]["input_form"].string, "{100, 20}")
    let saved = try await task.request(.object(["type": .string("save_notebook")]))
    XCTAssertTrue(saved.response.body["file"]["cells"].array.isEmpty)
    do {
      _ = try await main.request(.object(["type": .string("get_explore_context"), "query": query]))
      XCTFail("Obsolete snapshots must fail")
    } catch let error as KernelError { XCTAssertTrue(error.localizedDescription.contains("已")) }
  }
}
