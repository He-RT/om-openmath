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
    app.descendants(matching: .any)["settings.open"].firstMatch.tap()
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
}
