import XCTest
@testable import Zeron

@MainActor
final class CompanionGroupingTests: XCTestCase {
    private let spaces = [
        HostSpace(id: "a", deviceId: "mac", path: "/code/alpha", name: "Alpha"),
        HostSpace(id: "b", deviceId: "mac", path: "/code/beta", name: "Beta"),
        HostSpace(id: "c", deviceId: "mac", path: "/code/gamma", name: "Gamma")
    ]
    private func chat(_ id: String, space: String?, created: String = "2026-09-20T10:00:00Z") -> HostChat {
        HostChat(id: id, deviceId: "mac", archived: false, spaceId: space, createdAt: created)
    }
    private func groups(_ chats: [HostChat], keepEmpty: Bool = false,
                        states: [String: SessionState] = [:]) -> [CompanionProjectGroup] {
        CompanionModel.projectGroups(chats: chats, spaces: spaces, keepEmpty: keepEmpty) { states[$0.id] ?? .idle }
    }

    func testGroupsByProjectAndFoldsUnknownIntoHome() {
        let result = groups([chat("1", space: "a"), chat("2", space: nil), chat("3", space: "gone")])
        XCTAssertEqual(Set(result.map(\.id)), ["a", "home"])
        XCTAssertEqual(result.first { $0.id == "home" }?.chats.map(\.id).sorted(), ["2", "3"])
        XCTAssertTrue(result.first { $0.id == "home" }?.isHome ?? false)
        XCTAssertEqual(result.first { $0.id == "home" }?.projectID, "")
    }

    func testEmptyProjectsOnlyWhenKept() {
        XCTAssertEqual(groups([chat("1", space: "a")]).map(\.id), ["a"])
        XCTAssertEqual(groups([chat("1", space: "a")], keepEmpty: true).map(\.id), ["a", "b", "c"])
    }

    func testUrgentProjectsLeadAndCountsAreExact() {
        let chats = [chat("1", space: "a", created: "2026-09-22T10:00:00Z"), chat("2", space: "b"), chat("3", space: "b")]
        let result = groups(chats, states: ["2": .awaitingInput, "3": .working])
        XCTAssertEqual(result.map(\.id), ["b", "a"])
        XCTAssertEqual(result[0].needsYou, 1)
        XCTAssertEqual(result[0].running, 1)
        XCTAssertEqual(result[1].needsYou + result[1].running, 0)
    }

    func testRecencyThenNameBreakTies() {
        let result = groups([chat("1", space: "a", created: "2026-09-20T10:00:00Z"), chat("2", space: "b", created: "2026-09-22T10:00:00Z")])
        XCTAssertEqual(result.map(\.id), ["b", "a"])
    }

    func testChatOrderInsideGroupIsPreserved() {
        let result = groups([chat("z", space: "a"), chat("y", space: "a"), chat("x", space: "a")])
        XCTAssertEqual(result[0].chats.map(\.id), ["z", "y", "x"])
    }

    func testPathHintAndCollapsedPersistence() {
        XCTAssertEqual(CompanionProjectGroup.pathHint("/Users/me/code/noches"), "…/code/noches")
        XCTAssertEqual(CompanionProjectGroup.pathHint("/code"), "/code")
        var raw = ""
        raw = CompanionCollapsed.toggled(raw, id: "a")
        raw = CompanionCollapsed.toggled(raw, id: "b")
        XCTAssertEqual(CompanionCollapsed.ids(raw), ["a", "b"])
        raw = CompanionCollapsed.toggled(raw, id: "a")
        XCTAssertEqual(CompanionCollapsed.ids(raw), ["b"])
    }
}
