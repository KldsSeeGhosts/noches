import XCTest

@MainActor
final class CompanionUITests: XCTestCase {
    override func setUp() {
        continueAfterFailure = false
        XCUIDevice.shared.orientation = .portrait
    }
    private func fixtureCode() throws -> String {
        let profile: [String: String] = ["id":"mobile-fixture", "name":"Studio fixture", "endpoint":"ws://127.0.0.1:28777",
                                       "token":String(repeating:"a",count:64), "deviceId":"fixture-mac"]
        let data = try JSONSerialization.data(withJSONObject: profile)
        return "noches-connect:" + data.base64EncodedString().replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: "=", with: "")
    }
    private func launchFixture(mode: String = "dark", chat: String? = nil) async throws -> XCUIApplication {
        var reset = URLRequest(url: URL(string: "http://127.0.0.1:28777/reset")!)
        reset.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: reset)
        let app = XCUIApplication()
        app.launchArguments = ["-appearance.mode", mode, "-appearance.\(mode)", "zeron-\(mode)",
                               "-companion-pair", try fixtureCode()]
        if let chat { app.launchArguments += ["-companion-open", chat] }
        app.launch()
        if chat != nil {
            XCTAssertTrue(app.descendants(matching: .any)["companion-composer"].firstMatch.waitForExistence(timeout: 15))
            let online = XCTNSPredicateExpectation(predicate: NSPredicate(format: "enabled == true"),
                object: app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Session actions for'")).firstMatch)
            XCTAssertEqual(XCTWaiter.wait(for: [online], timeout: 15), .completed)
        } else {
            XCTAssertTrue(connectedHost(app).waitForExistence(timeout: 15))
        }
        return app
    }
    private func workspace(_ app: XCUIApplication, _ action: String) {
        app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Session actions for'")).firstMatch.tap()
        XCTAssertTrue(app.buttons[action].waitForExistence(timeout: 5))
        app.buttons[action].tap()
    }
    private func scrollTo(_ element: XCUIElement, in app: XCUIApplication) {
        for _ in 0..<8 where !element.isHittable { app.swipeUp() }
        XCTAssertTrue(element.isHittable)
    }
    /// The home host switcher reads "<name>, Connected" once the link is up.
    private func connectedHost(_ app: XCUIApplication) -> XCUIElement {
        app.buttons.matching(NSPredicate(format: "identifier == 'host-switcher' AND label CONTAINS 'Connected'")).firstMatch
    }
    /// The filter menu's project row reads "Project: <name>".
    private func projectFilterItem(_ app: XCUIApplication) -> XCUIElement {
        app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Project:'")).firstMatch
    }
    private func capture(_ name: String) {
        let screenshot = XCUIScreen.main.screenshot()
        if let directory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first {
            try? screenshot.pngRepresentation.write(to: directory.appendingPathComponent("\(name).png"))
        }
        let attachment = XCTAttachment(screenshot: screenshot)
        attachment.name = name; attachment.lifetime = .keepAlways; add(attachment)
    }
    override func tearDown() {
        capture("final-\(name.replacingOccurrences(of: "/", with: "-"))")
        print(XCUIApplication().debugDescription)
        super.tearDown()
    }
    private func openSession(_ app: XCUIApplication, id: String = "fixture-chat") {
        let link = app.buttons["open-session-\(id)"]
        for _ in 0..<6 {
            let frame = link.frame
            if link.isHittable && frame.minY > 120 && frame.maxY < app.frame.maxY - 150 { break }
            if frame.minY < 120 { app.swipeDown() } else { app.swipeUp() }
        }
        XCTAssertTrue(link.waitForExistence(timeout: 5))
        link.tap()
        let opened = app.descendants(matching: .any)["companion-composer"].firstMatch.waitForExistence(timeout: 5)
        if !opened { print(app.debugDescription); capture("navigation-failure") }
        XCTAssertTrue(opened)
    }
    func testPairReadAndSendOnFixtureHost() async throws {
        var reset = URLRequest(url: URL(string: "http://127.0.0.1:28777/reset")!)
        reset.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: reset)
        let app = XCUIApplication()
        app.launchArguments = ["-appearance.mode", "dark", "-appearance.dark", "zeron-dark", "-appearance.surface", "frosted"]
        app.launch()
        if app.buttons["pair-computer"].waitForExistence(timeout: 3) {
            app.buttons["pair-computer"].tap()
        } else {
            app.buttons["Appearance and computers"].tap()
            app.buttons["Pair a computer"].tap()
        }
        let field = app.secureTextFields["connection-code"]
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        let profile: [String: String] = ["id":"mobile-fixture", "name":"Studio fixture", "endpoint":"ws://127.0.0.1:28777", "token":String(repeating:"a",count:64), "deviceId":"fixture-mac"]
        let data = try JSONSerialization.data(withJSONObject: profile)
        let code = "noches-connect:" + data.base64EncodedString().replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: "=", with: "")
        focus(field); field.typeText(code)
        app.buttons["Connect to computer"].tap()
        XCTAssertTrue(connectedHost(app).waitForExistence(timeout: 15))
        app.buttons["Filter sessions"].tap(); app.buttons["By status"].tap()
        XCTAssertTrue(app.staticTexts["Needs you"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Running"].exists)
        capture("companion-connected-dark")
        let session = app.staticTexts["Build the mobile companion"]
        XCTAssertTrue(session.waitForExistence(timeout: 5)); openSession(app)
        workspace(app, "Browse files")
        XCTAssertTrue(app.staticTexts["Sources"].waitForExistence(timeout: 5))
        app.staticTexts["Sources"].tap()
        XCTAssertTrue(app.staticTexts["Client.swift"].waitForExistence(timeout: 5))
        app.staticTexts["Client.swift"].tap()
        XCTAssertTrue(app.staticTexts["    let connected = true"].waitForExistence(timeout: 5))
        capture("companion-file-preview")
        app.buttons["Done"].tap()
        workspace(app, "Review changes")
        XCTAssertTrue(app.staticTexts["Client.swift"].waitForExistence(timeout: 5))
        app.staticTexts["Client.swift"].tap()
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'let connected = true'")).firstMatch.waitForExistence(timeout: 5))
        capture("companion-changes")
        app.buttons["Done"].tap()
        let composer = app.descendants(matching: .any)["companion-composer"].firstMatch
        XCTAssertTrue(composer.waitForExistence(timeout: 5))
        composer.tap(); composer.typeText("Check the mobile companion fixture.")
        app.buttons["Send message"].tap()
        XCTAssertTrue(app.staticTexts["Received on the fixture host. No real agent was started."].waitForExistence(timeout: 8))
        app.buttons["Done"].tap()
        let replies = app.staticTexts.matching(NSPredicate(format: "label == %@", "Received on the fixture host. No real agent was started."))
        XCTAssertTrue(replies.allElementsBoundByIndex.last?.isHittable == true)
        capture("companion-transcript-dark")
        app.navigationBars.buttons.firstMatch.tap()
        app.buttons["New session"].tap()
        XCTAssertTrue(app.buttons["Start session"].waitForExistence(timeout: 5))
        let editor = app.descendants(matching: .any)["first-message"].firstMatch
        if editor.exists { editor.tap(); editor.typeText("Kick off the mobile work.") }
        if !app.buttons["Start session"].isHittable { app.swipeUp() }
        app.buttons["Start session"].tap()
        XCTAssertTrue(app.navigationBars["New mobile session"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.descendants(matching: .any)["companion-composer"].firstMatch.exists)
        XCTAssertTrue(app.staticTexts["Received on the fixture host. No real agent was started."].waitForExistence(timeout: 8))
        capture("companion-new-session")
    }

    func testSessionManagementAndDraftRetention() async throws {
        var reset = URLRequest(url: URL(string: "http://127.0.0.1:28777/reset")!)
        reset.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: reset)
        let app = XCUIApplication()
        app.launchArguments = ["-appearance.mode", "dark", "-appearance.dark", "zeron-dark", "-appearance.surface", "frosted"]
        app.launch()
        // The pairing test establishes the fixture profile, or pair on a clean device.
        if app.buttons["pair-computer"].waitForExistence(timeout: 2) {
            app.buttons["pair-computer"].tap()
            let profile: [String: String] = ["id":"mobile-fixture", "name":"Studio fixture", "endpoint":"ws://127.0.0.1:28777", "token":String(repeating:"a",count:64), "deviceId":"fixture-mac"]
            let data = try JSONSerialization.data(withJSONObject: profile)
            let code = "noches-connect:" + data.base64EncodedString().replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: "=", with: "")
            let field = app.secureTextFields["connection-code"]
            focus(field); field.typeText(code)
            app.buttons["Connect to computer"].tap()
        }
        XCTAssertTrue(connectedHost(app).waitForExistence(timeout: 15))
        app.buttons["Filter sessions"].tap()
        app.buttons["Needs you"].tap()
        XCTAssertTrue(app.staticTexts["Review the authentication flow"].exists)
        XCTAssertFalse(app.staticTexts["Build the mobile companion"].exists)
        capture("companion-needs-you")
        app.buttons["Filter sessions"].tap()
        app.buttons["All"].tap()
        let search = app.textFields["session-search"]
        search.tap(); search.typeText("companion")
        let session = app.staticTexts["Build the mobile companion"]
        XCTAssertTrue(session.waitForExistence(timeout: 5)); openSession(app)
        let composer = app.descendants(matching: .any)["companion-composer"].firstMatch
        XCTAssertTrue(composer.waitForExistence(timeout: 5))
        composer.tap(); composer.typeText("Keep my unsent draft")
        app.buttons["Done"].tap()
        app.navigationBars.buttons.firstMatch.tap()
        openSession(app)
        XCTAssertEqual(composer.value as? String, "Keep my unsent draft")
        app.buttons["Session actions for Build the mobile companion"].tap()
        app.buttons["Rename session"].tap()
        let title = app.alerts.textFields.firstMatch
        title.tap(withNumberOfTaps: 3, numberOfTouches: 1)
        title.typeText("Pocket control")
        XCTAssertEqual(title.value as? String, "Pocket control")
        app.alerts.buttons["Save"].tap()
        XCTAssertTrue(app.navigationBars["Pocket control"].waitForExistence(timeout: 5))
        app.buttons["Session actions for Pocket control"].tap()
        app.buttons["Archive session"].tap()
        XCTAssertTrue(app.buttons["Filter sessions"].waitForExistence(timeout: 5))
        if app.buttons["Clear search"].exists { app.buttons["Clear search"].tap() }
        app.buttons["Filter sessions"].tap()
        app.buttons["Archived"].tap()
        XCTAssertTrue(app.staticTexts["Pocket control"].waitForExistence(timeout: 5))
        app.buttons["open-session-fixture-chat"].press(forDuration: 1)
        app.buttons["Restore session"].tap()
        XCTAssertTrue(app.staticTexts["Pocket control"].waitForNonExistence(timeout: 5))
        app.buttons["Filter sessions"].tap()
        app.buttons["All"].tap()
        XCTAssertTrue(app.staticTexts["Pocket control"].waitForExistence(timeout: 5))
        capture("companion-restored-session")
    }

    func testApprovalQueueAndStop() async throws {
        var reset = URLRequest(url: URL(string: "http://127.0.0.1:28777/reset")!)
        reset.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: reset)
        let app = XCUIApplication()
        app.launchArguments = ["-appearance.mode", "dark", "-appearance.dark", "zeron-dark"]
        app.launch()
        // Pair with this test's fixture host whether the app is empty or holds
        // a profile from another host, so the assertions can never read the
        // wrong fixture's /commands list.
        if app.buttons["pair-computer"].waitForExistence(timeout: 2) {
            app.buttons["pair-computer"].tap()
        } else {
            app.buttons["Appearance and computers"].tap()
            app.buttons["Pair a computer"].tap()
        }
        do {
            let profile: [String: String] = ["id":"mobile-fixture", "name":"Studio fixture", "endpoint":"ws://127.0.0.1:28777", "token":String(repeating:"a",count:64), "deviceId":"fixture-mac"]
            let data = try JSONSerialization.data(withJSONObject: profile)
            let code = "noches-connect:" + data.base64EncodedString().replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: "=", with: "")
            let field = app.secureTextFields["connection-code"]
            focus(field); field.typeText(code)
            app.buttons["Connect to computer"].tap()
        }
        XCTAssertTrue(connectedHost(app).waitForExistence(timeout: 15))
        openSession(app, id: "fixture-approval")
        let allow = app.buttons.containing(.staticText, identifier: "Allow").firstMatch
        XCTAssertTrue(allow.waitForExistence(timeout: 5))
        capture("companion-approval")
        allow.tap()
        XCTAssertTrue(allow.waitForNonExistence(timeout: 5))
        app.navigationBars.buttons.firstMatch.tap()
        openSession(app, id: "fixture-working")
        let composer = app.descendants(matching: .any)["companion-composer"].firstMatch
        composer.tap(); composer.typeText("Follow up after this turn")
        app.buttons["Send message"].tap()
        app.buttons["Done"].tap()
        let queued = app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Queued · '")).firstMatch
        XCTAssertTrue(queued.waitForExistence(timeout: 5))
        // The queue tray starts expanded; its header collapses and restores it.
        XCTAssertTrue(app.staticTexts["Follow up after this turn"].waitForExistence(timeout: 5))
        queued.tap()
        XCTAssertTrue(app.staticTexts["Follow up after this turn"].waitForNonExistence(timeout: 5))
        capture("companion-queued-collapsed")
        queued.tap()
        XCTAssertTrue(app.staticTexts["Follow up after this turn"].waitForExistence(timeout: 5))
        capture("companion-queued")
        app.buttons["Stop session"].tap()
        XCTAssertTrue(app.buttons["Stop session"].waitForNonExistence(timeout: 5))
        let (data, _) = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:28777/commands")!)
        let commands = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [[String: Any]])
        let params = commands.compactMap { $0["params"] as? [String: Any] }
        let answer = try XCTUnwrap(params.compactMap { $0["command"] as? [String: Any] }.first { $0["kind"] as? String == "respondInput" })
        XCTAssertEqual(answer["requestId"] as? String, "approval-request")
        XCTAssertEqual((answer["answers"] as? [[String: Any]])?.first?["labels"] as? [String], ["Allow"])
        XCTAssertTrue(params.contains { $0["text"] as? String == "Follow up after this turn" && $0["holdForTurnEnd"] as? Bool == true })
    }

    func testThemeScreensAndAppearanceSettings() {
        let app = XCUIApplication()
        for (mode, theme) in [("light", "zeron-light"), ("dark", "catppuccin-mocha"), ("dark", "tokyo-night")] {
            app.launchArguments = ["-appearance.mode", mode, "-appearance.\(mode)", theme, "-appearance.surface", "opaque"]
            app.launch()
            XCTAssertTrue(app.buttons["Appearance and computers"].waitForExistence(timeout: 5))
            capture("companion-\(theme)")
            app.buttons["Appearance and computers"].tap()
            app.buttons["Appearance"].tap()
            XCTAssertTrue(app.staticTexts["Light theme"].waitForExistence(timeout: 5))
            XCTAssertTrue(app.staticTexts["Dark theme"].exists)
            capture("appearance-\(theme)")
        }
    }

    func testProjectsFoldersGroupingAndWorktree() async throws {
        let app = try await launchFixture()
        app.buttons["Filter sessions"].tap()
        app.buttons["By project"].tap()
        let group = app.buttons["project-group-fixture-space"]
        XCTAssertTrue(group.waitForExistence(timeout: 5))
        group.tap()
        XCTAssertFalse(app.buttons["open-session-fixture-approval"].exists)
        group.tap()
        XCTAssertTrue(app.buttons["open-session-fixture-approval"].exists)
        capture("overhaul-project-groups")
        app.buttons["Filter sessions"].tap(); projectFilterItem(app).tap()
        app.buttons["project-home"].tap()
        XCTAssertTrue(app.staticTexts["Scratch notes"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["open-session-fixture-approval"].exists)
        app.buttons["Filter sessions"].tap(); projectFilterItem(app).tap()
        app.buttons["project-all"].tap()
        app.buttons["Filter sessions"].tap(); projectFilterItem(app).tap()
        app.buttons["add-project"].tap()
        let folder = app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'mobile-qa-project'")).firstMatch
        XCTAssertTrue(folder.waitForExistence(timeout: 5)); folder.tap()
        app.buttons["Use this folder"].tap()
        let name = app.textFields["project-name"]
        XCTAssertTrue(name.waitForExistence(timeout: 5))
        name.tap(); name.typeText("Mobile QA")
        app.buttons["add-project"].tap()
        XCTAssertTrue(app.buttons.matching(NSPredicate(format: "label CONTAINS 'Mobile QA'")).firstMatch.waitForExistence(timeout: 5))
        capture("overhaul-added-project")
        app.buttons["Done"].tap()
        app.buttons["New session"].tap()
        let project = app.buttons.containing(.staticText, identifier: "Mobile QA").firstMatch
        XCTAssertTrue(project.waitForExistence(timeout: 5)); project.tap()
        XCTAssertTrue(app.buttons["New worktree"].waitForExistence(timeout: 5))
        app.buttons["New worktree"].tap()
        capture("overhaul-new-worktree")
        let start = app.buttons["start-session"]
        scrollTo(start, in: app); start.tap()
        XCTAssertTrue(app.descendants(matching: .any)["companion-composer"].firstMatch.waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'zeron/qa-fox'")).firstMatch.exists)
        capture("overhaul-worktree-session")
        let (data, _) = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:28777/commands")!)
        let commands = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [[String: Any]])
        XCTAssertTrue(commands.contains { $0["method"] as? String == "CreateWorktree" })
        let params = commands.compactMap { $0["params"] as? [String: Any] }
        XCTAssertTrue(params.contains { $0["op"] as? String == "createSpace" && $0["path"] as? String == "/tmp/mobile-qa-project" })
        XCTAssertTrue(params.contains { $0["op"] as? String == "createChat" && $0["cwd"] as? String == "/tmp/fixture-worktrees/qa-fox" })
    }

    func testWorkspaceEditingSearchTerminalAndSuggestions() async throws {
        let app = try await launchFixture(mode: "light", chat: "fixture-chat")
        workspace(app, "Browse files")
        XCTAssertTrue(app.staticTexts["Sources"].waitForExistence(timeout: 5))
        app.staticTexts["Sources"].tap()
        app.staticTexts["Client.swift"].tap()
        XCTAssertTrue(app.buttons["Edit"].waitForExistence(timeout: 5))
        app.buttons["Edit"].tap()
        let editor = app.textViews.firstMatch
        XCTAssertTrue(editor.waitForExistence(timeout: 5))
        focus(editor); editor.typeText("\n// Mobile QA edit")
        app.buttons["Save"].tap()
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'Mobile QA edit'")).firstMatch.waitForExistence(timeout: 5))
        capture("overhaul-file-edited-light")
        app.buttons["Done"].tap()
        workspace(app, "Review changes")
        XCTAssertTrue(app.staticTexts["Client.swift"].waitForExistence(timeout: 5))
        app.staticTexts["Client.swift"].tap()
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'let connected = true'")).firstMatch.waitForExistence(timeout: 5))
        capture("overhaul-file-diff-light")
        app.buttons["Done"].tap()
        workspace(app, "Terminal")
        XCTAssertTrue(app.staticTexts["Fixture shell ready"].waitForExistence(timeout: 8))
        let command = app.textFields["Command"]
        command.tap(); command.typeText("printf QA_TERMINAL_OK")
        app.buttons["terminal-send"].tap()
        XCTAssertTrue(app.staticTexts["QA_TERMINAL_OK"].waitForExistence(timeout: 5))
        capture("overhaul-terminal-light")
        app.buttons["Done"].tap()
        workspace(app, "Run: Fixture tests")
        XCTAssertTrue(app.staticTexts["QA_ACTION_OK"].waitForExistence(timeout: 8))
        app.buttons["Done"].tap()
        let composer = app.descendants(matching: .any)["companion-composer"].firstMatch
        composer.tap(); composer.typeText("@Cl")
        let suggestion = app.buttons.matching(NSPredicate(format: "label CONTAINS 'Client.swift'")).firstMatch
        XCTAssertTrue(suggestion.waitForExistence(timeout: 5)); suggestion.tap()
        XCTAssertEqual(composer.value as? String, "@Sources/Client.swift ")
        composer.typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: 22))
        composer.typeText("/rev")
        let review = app.buttons.matching(NSPredicate(format: "label BEGINSWITH '/review'")).firstMatch
        XCTAssertTrue(review.waitForExistence(timeout: 5)); review.tap()
        XCTAssertEqual(composer.value as? String, "/review ")
        capture("overhaul-suggestions-light")
        app.buttons["Done"].tap()
    }

    func testQueueReorderEditAndAccessControls() async throws {
        let app = try await launchFixture(chat: "fixture-working")
        XCTAssertTrue(app.buttons["Stop session"].waitForExistence(timeout: 8))
        let composer = app.descendants(matching: .any)["companion-composer"].firstMatch
        composer.tap(); composer.typeText("Third queued item")
        app.buttons["Send message"].tap()
        app.buttons["Done"].tap()
        XCTAssertTrue(app.staticTexts["Third queued item"].waitForExistence(timeout: 5))
        let menus = app.buttons.matching(identifier: "Queued message actions")
        XCTAssertEqual(menus.count, 3)
        menus.element(boundBy: 2).tap()
        app.buttons["Move to top"].tap()
        let moved = XCTNSPredicateExpectation(predicate: NSPredicate { _, _ in
            app.staticTexts["Third queued item"].frame.minY < app.staticTexts["Also check the compact status row at 320pt wide."].frame.minY
        }, object: nil)
        XCTAssertEqual(XCTWaiter.wait(for: [moved], timeout: 5), .completed)
        menus.element(boundBy: 0).tap()
        app.buttons["Edit"].tap()
        let edit = app.textFields["queue-edit"]
        XCTAssertTrue(edit.waitForExistence(timeout: 5))
        focus(edit); edit.typeText(" amended")
        try await Task.sleep(for: .seconds(21))
        let editedText = try XCTUnwrap(edit.value as? String)
        app.buttons["Save"].tap()
        XCTAssertTrue(app.staticTexts[editedText.trimmingCharacters(in: .whitespacesAndNewlines)].waitForExistence(timeout: 5))
        capture("overhaul-queue-reordered-edited")
        let access = app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'Access:'")).firstMatch
        access.tap()
        app.buttons["Auto-approve"].coordinate(withNormalizedOffset: CGVector(dx: 0.2, dy: 0.5)).tap()
        XCTAssertTrue(app.buttons["Access: Auto-approve, Workspace write"].waitForExistence(timeout: 5))
        access.tap(); app.buttons["Read-only"].tap()
        XCTAssertTrue(app.buttons["Access: Auto-approve, Read-only"].waitForExistence(timeout: 5))
        capture("overhaul-access-read-only")
        app.buttons["Stop session"].tap()
        XCTAssertTrue(app.buttons["Stop session"].waitForNonExistence(timeout: 5))
        composer.tap(); composer.typeText("Verify access dispatch")
        app.buttons["Send message"].tap()
        XCTAssertTrue(app.staticTexts["Received on the fixture host. No real agent was started."].waitForExistence(timeout: 5))
        let (data, _) = try await URLSession.shared.data(from: URL(string: "http://127.0.0.1:28777/commands")!)
        let commands = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [[String: Any]])
        XCTAssertTrue(commands.contains { $0["method"] as? String == "MoveQueuedMessage" })
        XCTAssertTrue(commands.contains { $0["method"] as? String == "FinishQueuedMessageEdit" })
        XCTAssertTrue(commands.contains { $0["method"] as? String == "RenewQueuedMessageEdit" })
        let runs = commands.compactMap { ($0["params"] as? [String: Any])?["command"] as? [String: Any] }
        let run = try XCTUnwrap(runs.last { $0["kind"] as? String == "run" })
        let request = try XCTUnwrap(run["request"] as? [String: Any])
        XCTAssertEqual(request["autoApprove"] as? Bool, true)
        XCTAssertEqual(request["sandbox"] as? String, "read-only")
    }

    func testFileSearchConflictAndDiscardConfirmation() async throws {
        let app = try await launchFixture(chat: "fixture-chat")
        workspace(app, "Browse files")
        let search = app.searchFields.firstMatch
        if !search.isHittable { app.swipeDown() }
        XCTAssertTrue(search.waitForExistence(timeout: 5))
        search.tap(); search.typeText("Client")
        let file = app.staticTexts["Client.swift"]
        XCTAssertTrue(file.waitForExistence(timeout: 5)); file.tap()
        XCTAssertTrue(app.buttons["Edit"].waitForExistence(timeout: 5)); app.buttons["Edit"].tap()
        let editor = app.textViews.firstMatch
        focus(editor); editor.typeText("// local edit\n")
        var change = URLRequest(url: URL(string: "http://127.0.0.1:28777/change-file")!)
        change.httpMethod = "POST"
        _ = try await URLSession.shared.data(for: change)
        app.buttons["Save"].tap()
        XCTAssertTrue(app.alerts["File changed on your computer"].waitForExistence(timeout: 5))
        capture("overhaul-conflict-protected")
        app.alerts.buttons["Reload and discard my edits"].tap()
        XCTAssertTrue(app.staticTexts["// Changed on host"].waitForExistence(timeout: 5))
        app.buttons["Done"].tap()
        workspace(app, "Review changes")
        XCTAssertTrue(app.buttons["Discard all changes"].waitForExistence(timeout: 5))
        app.buttons["Discard all changes"].tap()
        XCTAssertTrue(app.sheets["Discard all changes?"].waitForExistence(timeout: 5))
        // iOS 27 presents this confirmation as a popover without a Cancel row.
        // A tap outside dismisses it without dispatching the destructive RPC.
        app.coordinate(withNormalizedOffset: CGVector(dx: 0.95, dy: 0.15)).tap()
        XCTAssertTrue(app.staticTexts["Client.swift"].exists)
        app.buttons["Discard all changes"].tap()
        app.buttons.matching(identifier: "Discard all changes").allElementsBoundByIndex.last!.tap()
        XCTAssertTrue(app.staticTexts["No uncommitted changes"].waitForExistence(timeout: 5))
        capture("overhaul-discard-confirmed")
    }

    /// Sheets can still be animating in when their field first exists; retry
    /// until the tap actually lands keyboard focus.
    private func focus(_ field: XCUIElement) {
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        for _ in 0..<5 {
            if field.isHittable {
                if field.elementType == .textView {
                    field.coordinate(withNormalizedOffset: CGVector(dx: 0.35, dy: 0.2)).tap()
                } else { field.tap() }
            }
            if (field.value(forKey: "hasKeyboardFocus") as? Bool) == true { return }
            Thread.sleep(forTimeInterval: 0.3)
        }
    }
}
