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
    XCTAssertTrue(app.descendants(matching: .any)["solution.-3"].firstMatch.waitForExistence(timeout: 15))
    XCTAssertTrue(app.descendants(matching: .any)["solution.1"].firstMatch.exists)
    screenshot("quadratic-portrait", app)
    XCUIDevice.shared.orientation = .landscapeLeft
    XCTAssertTrue(app.descendants(matching: .any)["solution.-3"].firstMatch.exists)
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

  @MainActor func testExplorationRunsInIsolatedScopeAndKeepsSliderAfterRotation() {
    let app = launch()
    XCTAssertTrue(app.buttons["example.explore"].waitForExistence(timeout: 30))
    app.buttons["example.explore"].tap()
    let slider = app.sliders["explore.slider.a"]
    XCTAssertTrue(slider.waitForExistence(timeout: 15))
    slider.adjust(toNormalizedSliderPosition: 1)
    // XCUITest can stop just inside the endpoint on the wider iPad track (actual a=3.96).
    // Finish the real thumb drag past the track, then require a=4 and the original exact 16.
    if let text=slider.value as? String,let value=Double(text),value>=3.8 && value<4 {
      let frame=slider.frame,inset=min(frame.height/2,frame.width/4)
      let position=(inset+(frame.width-2*inset)*(value/4))/frame.width
      slider.coordinate(withNormalizedOffset:.init(dx:position,dy:0.5)).press(forDuration:0.1,
        thenDragTo:slider.coordinate(withNormalizedOffset:.init(dx:1.15,dy:0.5)))
    }
    let atMaximum=NSPredicate {_,_ in (slider.value as? String).flatMap(Double.init)==4}
    XCTAssertTrue(XCTWaiter.wait(for:[XCTNSPredicateExpectation(predicate:atMaximum,object:nil)],timeout:5) == .completed)
    XCTAssertTrue(app.descendants(matching: .any)["expression.16."].firstMatch.waitForExistence(timeout: 15))
    screenshot("explore-portrait", app)
    XCUIDevice.shared.orientation = .landscapeLeft
    XCTAssertTrue(slider.waitForExistence(timeout: 5))
    XCTAssertTrue(app.descendants(matching: .any)["expression.16."].firstMatch.exists)
    screenshot("explore-landscape", app)
  }

  @MainActor func testActual3DSceneAndAssistantDraftSurviveRotation() {
    let app = launch()
    XCTAssertTrue(app.buttons["example.scene"].waitForExistence(timeout: 30))
    app.buttons["example.scene"].tap()
    let canvas = app.descendants(matching: .any)["scene.canvas"].firstMatch
    XCTAssertTrue(canvas.waitForExistence(timeout: 20))
    // Drag the document gutter: a drag inside the scene belongs to its camera.
    let document = app.scrollViews.firstMatch
    document.coordinate(withNormalizedOffset: .init(dx: 0.04, dy: 0.8)).press(forDuration: 0.1,
      thenDragTo: document.coordinate(withNormalizedOffset: .init(dx: 0.04, dy: 0.25)))
    app.buttons["图形信息"].tap()
    XCTAssertTrue(app.staticTexts["scene.geometry"].waitForExistence(timeout: 5))
    screenshot("scene-portrait", app)
    XCUIDevice.shared.orientation = .landscapeLeft
    XCTAssertTrue(canvas.waitForExistence(timeout: 5))
    screenshot("scene-landscape", app)
    let inspector = app.buttons["inspector.toggle"].firstMatch
    XCTAssertTrue(inspector.waitForExistence(timeout: 5)); inspector.tap()
    let input = app.textViews["assistant.input"]
    XCTAssertTrue(input.waitForExistence(timeout: 5))
    input.tap(); input.typeText("Duo draft 123")
    XCUIDevice.shared.orientation = .portrait
    XCTAssertTrue(input.waitForExistence(timeout: 5))
    XCTAssertEqual(input.value as? String, "Duo draft 123")
    screenshot("assistant-draft-portrait", app)
  }

}
