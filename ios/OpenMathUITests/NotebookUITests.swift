import XCTest

final class NotebookUITests: XCTestCase {
  @MainActor private func launch() -> XCUIApplication {
    let app = XCUIApplication()
    app.launchArguments = [
      "-OpenMathUITesting", "-AppleLanguages", "(zh-Hans)", "-AppleLocale", "zh_CN",
    ]
    XCUIDevice.shared.orientation = .portrait
    app.launch()
    return app
  }
  @MainActor private func screenshot(_ name: String, _ app: XCUIApplication) {
    let image = XCTAttachment(screenshot: app.screenshot())
    image.name = name
    image.lifetime = .keepAlways
    add(image)
  }
  @MainActor func testRealQuadraticStepsRotationAndSettings() {
    let app = launch()
    XCTAssertTrue(app.buttons["example.quadratic"].waitForExistence(timeout: 30))
    app.buttons["example.quadratic"].tap()
    XCTAssertTrue(app.staticTexts["solution.-3"].waitForExistence(timeout: 15))
    XCTAssertTrue(app.staticTexts["solution.1"].exists)
    screenshot("quadratic-portrait", app)
    XCUIDevice.shared.orientation = .landscapeLeft
    XCTAssertTrue(app.staticTexts["solution.-3"].exists)
    app.buttons["inspector.steps"].tap()
    XCTAssertTrue(app.staticTexts["steps.title"].waitForExistence(timeout: 5))
    screenshot("steps-landscape", app)
    XCUIDevice.shared.orientation = .portrait
    // A compact sheet owns the inspector on phone; dismiss it before opening settings.
    if app.buttons["inspector.close"].waitForExistence(timeout: 2) {
      app.buttons["inspector.close"].tap()
    }
    app.buttons["file.menu"].tap()
    let settings = app.descendants(matching: .any)["settings.open"].firstMatch
    XCTAssertTrue(settings.waitForExistence(timeout: 5))
    settings.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
    XCTAssertTrue(app.buttons["保存"].waitForExistence(timeout: 5))
    screenshot("settings", app)
  }
  @MainActor func testReactiveNotebookHasActualResultsAndEditableSources() {
    let app = launch()
    XCTAssertTrue(app.buttons["example.reactive"].waitForExistence(timeout: 30))
    app.buttons["example.reactive"].tap()
    XCTAssertTrue(
      app.otherElements["expression.3"].waitForExistence(timeout: 15) || app.staticTexts["3"].exists
    )
    XCTAssertEqual(app.textViews.count, 2)
    screenshot("reactive-notebook", app)
    app.buttons["file.menu"].tap()
    app.buttons["file.save"].tap()
    XCTAssertTrue(app.staticTexts["笔记本已保存"].waitForExistence(timeout: 5))
    XCUIDevice.shared.orientation = .landscapeLeft
    XCTAssertEqual(app.textViews.count, 2)
    screenshot("reactive-landscape", app)
  }
  @MainActor func testActualStructuredTableKeepsHeadersTextAndRotation() {
    let app=launch()
    XCTAssertTrue(app.buttons["example.table"].waitForExistence(timeout:30))
    app.buttons["example.table"].tap()
    XCTAssertTrue(app.otherElements["value.output"].waitForExistence(timeout:15))
    XCTAssertTrue(app.staticTexts["表格"].exists)
    XCTAssertTrue(app.staticTexts["x"].exists)
    XCTAssertTrue(app.staticTexts["\"中文\""].exists)
    screenshot("structured-table-portrait",app)
    XCUIDevice.shared.orientation = .landscapeLeft
    XCTAssertTrue(app.staticTexts["\"emoji🙂\""].waitForExistence(timeout:5))
    screenshot("structured-table-landscape",app)
  }
  @MainActor func testActualDataPlotExposesKernelSamplesAndKeepsThemAfterRotation() {
    let app = launch()
    XCTAssertTrue(app.buttons["example.plot"].waitForExistence(timeout: 30))
    app.buttons["example.plot"].tap()
    let plot = app.otherElements["plot.canvas"]
    XCTAssertTrue(plot.waitForExistence(timeout: 15))
    XCTAssertTrue(String(describing: plot.value).contains("0/0/0/3"))
    app.buttons["采样数据"].tap()
    let samples = app.staticTexts["plot.samples"]
    XCTAssertTrue(samples.waitForExistence(timeout: 5))
    XCTAssertTrue(samples.label.contains("points"))
    screenshot("data-plot-portrait", app)
    XCUIDevice.shared.orientation = .landscapeLeft
    XCTAssertTrue(samples.waitForExistence(timeout: 5))
    screenshot("data-plot-landscape", app)
  }

}
