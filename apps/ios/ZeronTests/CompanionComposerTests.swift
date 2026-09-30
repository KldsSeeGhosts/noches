import XCTest
@testable import Zeron

@MainActor
final class CompanionComposerTests: XCTestCase {
    func testMentionTriggerAtEndOfWord() throws {
        let text = "look at @src/ma"
        let trigger = try XCTUnwrap(ComposerTrigger.active(in: text))
        XCTAssertEqual(trigger.kind, .mention)
        XCTAssertEqual(trigger.query, "src/ma")
        XCTAssertEqual(trigger.inserting("src/main.rs", into: text), "look at @src/main.rs ")
    }

    func testMentionNeedsTrailingTokenAndSigilOnlyAtWordStart() {
        XCTAssertNil(ComposerTrigger.active(in: "look at @src "))
        XCTAssertNil(ComposerTrigger.active(in: "mail me@host"))
        XCTAssertEqual(ComposerTrigger.active(in: "@")?.query, "")
        XCTAssertEqual(ComposerTrigger.active(in: "a\n@b")?.query, "b")
    }

    func testSlashOnlyAtDraftStart() throws {
        let trigger = try XCTUnwrap(ComposerTrigger.active(in: "/com"))
        XCTAssertEqual(trigger.kind, .command)
        XCTAssertEqual(trigger.inserting("compact", into: "/com"), "/compact ")
        XCTAssertNil(ComposerTrigger.active(in: "run /compact"))
        XCTAssertNil(ComposerTrigger.active(in: "/usr/bin"))
    }

    func testCommandFilterPrefixFirst() {
        let json = #"[{"name":"review"},{"name":"init","description":"x"},{"name":"preview","inputHint":"h"}]"#
        let all = try! JSONDecoder().decode([HostSlashCommand].self, from: Data(json.utf8))
        XCTAssertEqual(filterSlashCommands(all, query: "re").map(\.name), ["review", "preview"])
        XCTAssertEqual(filterSlashCommands(all, query: "").count, 3)
    }

    func testFileMatchDecoding() throws {
        let m = try JSONDecoder().decode([HostFileMatch].self, from: Data(#"[{"path":"src/a.rs","isDir":false},{"path":"src"}]"#.utf8))
        XCTAssertEqual(m[0].name, "a.rs")
        XCTAssertEqual(m[0].parent, "src")
        XCTAssertTrue(!m[1].isDir)
    }

    func testRpcParamShapes() {
        let move = CompanionModel.moveQueuedParams(chatID: "c", id: "r", toIndex: -3)
        XCTAssertEqual(move["chatId"] as? String, "c")
        XCTAssertEqual(move["id"] as? String, "r")
        XCTAssertEqual(move["toIndex"] as? Int, 0)
        let lease = CompanionQueuedEditLease(chatId: "c", deviceId: "d", rowId: "r", leaseId: "l", text: "t", baseTextHash: "h")
        let renew = CompanionModel.renewQueuedParams(lease)
        XCTAssertEqual(renew as? [String: String], ["chatId": "c", "id": "r", "leaseId": "l"])
        let search = CompanionModel.searchFilesParams(chatID: "c", query: String(repeating: "a", count: 300))
        XCTAssertEqual((search["query"] as? String)?.count, 256)
    }

    func testAccessAndSandboxWireValues() {
        XCTAssertEqual(CompanionSandbox.readOnly.rawValue, "read-only")
        XCTAssertEqual(CompanionSandbox.full.rawValue, "danger-full-access")
        XCTAssertEqual(CompanionAccess(rawValue: "auto"), .auto)
    }
}
