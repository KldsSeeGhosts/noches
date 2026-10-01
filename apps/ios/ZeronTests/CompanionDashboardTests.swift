import XCTest
@testable import Zeron

@MainActor
final class CompanionDashboardTests: XCTestCase {
    private func iso(_ offset: TimeInterval = 0) -> String {
        ISO8601DateFormatter().string(from: Date().addingTimeInterval(offset))
    }
    private func model() -> CompanionModel {
        let model = CompanionModel()
        model.profiles = [ConnectionProfile(id: "test-host", name: "Studio", endpoint: "ws://127.0.0.1:28777", token: String(repeating: "a", count: 64), deviceId: "mac")]
        model.selectedID = "test-host"
        model.spaces = [HostSpace(id: "project", deviceId: "mac", path: "/code/noches", name: "Noches")]
        model.chats = [
            HostChat(id: "idle", deviceId: "mac", title: "Newest session", archived: false, branch: "fix/keychain", createdAt: "2026-09-22"),
            HostChat(id: "working", deviceId: "mac", title: "Desktop sidebar", archived: false, spaceId: "project", createdAt: "2026-09-21"),
            HostChat(id: "attention", deviceId: "mac", title: "Approve migration", archived: false, spaceId: "project", createdAt: "2026-09-20"),
            HostChat(id: "archive", deviceId: "mac", title: "Old work", archived: true, createdAt: "2026-09-19"),
            HostChat(id: "remote", deviceId: "other", title: "Other host", archived: false, createdAt: "2026-09-23")
        ]
        model.sessions = [
            HostSession(chatId: "working", status: "working", startedAt: iso(-120), updatedAt: iso(-5)),
            HostSession(chatId: "attention", status: "awaitingInput", startedAt: iso(-30), updatedAt: iso(-5))
        ]
        return model
    }
    /// Chats covering every indicator plus a session per live state.
    private func stateModel() -> CompanionModel {
        let model = CompanionModel()
        model.profiles = [ConnectionProfile(id: "test-host", name: "Studio", endpoint: "ws://127.0.0.1:28777", token: String(repeating: "a", count: 64), deviceId: "mac")]
        model.selectedID = "test-host"
        func chat(_ id: String, message: TimeInterval?, seen: TimeInterval?) -> HostChat {
            var chat = HostChat(id: id, deviceId: "mac", archived: false, createdAt: iso(-86_400))
            chat.lastMessageAt = message.map { iso($0) }
            chat.lastSeenAt = seen.map { iso($0) }
            return chat
        }
        model.chats = [
            chat("working", message: -3600, seen: -7200),
            chat("stale", message: -3600, seen: -60),
            chat("attention", message: -60, seen: nil),
            chat("failed", message: -600, seen: nil),
            chat("seen-failed", message: -600, seen: -60),
            chat("done", message: -3600, seen: -7200),
            chat("never-seen", message: -3600, seen: nil),
            chat("idle", message: -7200, seen: -60),
            chat("empty", message: nil, seen: nil)
        ]
        model.sessions = [
            HostSession(chatId: "working", status: "working", startedAt: iso(-120), updatedAt: iso(-5)),
            HostSession(chatId: "stale", status: "working", startedAt: iso(-120), updatedAt: iso(-60)),
            HostSession(chatId: "attention", status: "awaitingInput", startedAt: iso(-30), updatedAt: iso(-5)),
            HostSession(chatId: "failed", status: "errored", startedAt: iso(-900), updatedAt: iso(-600)),
            HostSession(chatId: "seen-failed", status: "errored", startedAt: iso(-900), updatedAt: iso(-600))
        ]
        return model
    }
    func testAttentionSortingAndHostIsolation() {
        let model = model()
        XCTAssertEqual(model.visibleChats().map(\.id), ["attention", "working", "idle"])
        XCTAssertEqual(model.visibleChats(scope: .attention).map(\.id), ["attention"])
        XCTAssertEqual(model.visibleChats(scope: .working).map(\.id), ["working"])
        XCTAssertEqual(model.visibleChats(scope: .archived).map(\.id), ["archive"])
    }
    func testSearchCombinesProjectScopeAndMetadata() {
        let model = model()
        XCTAssertEqual(model.visibleChats(query: " KEYCHAIN ").map(\.id), ["idle"])
        XCTAssertEqual(model.visibleChats(query: "noches").map(\.id), ["attention", "working"])
        XCTAssertTrue(model.visibleChats(project: "project", query: "keychain").isEmpty)
        XCTAssertEqual(model.visibleChats(project: "project", scope: .working).map(\.id), ["working"])
    }
    func testStateDerivationGatesWorkingOnStaleness() {
        let model = stateModel()
        XCTAssertEqual(model.state(model.chats[0]), .working)
        XCTAssertEqual(model.workingSince(model.chats[0]), model.sessions[0].started)
        XCTAssertEqual(model.state(model.chats[1]), .idle, "a working row past the 45s window is dead")
        XCTAssertNil(model.workingSince(model.chats[1]))
        XCTAssertEqual(model.state(model.chats[2]), .awaitingInput)
        XCTAssertEqual(model.state(model.chats[3]), .failed)
        XCTAssertEqual(model.state(model.chats[4]), .idle, "an errored chat that has been seen settles")
        XCTAssertEqual(model.state(model.chats[5]), .completed)
        XCTAssertEqual(model.state(model.chats[6]), .completed, "a message without a seen marker is unseen")
        XCTAssertEqual(model.state(model.chats[7]), .idle)
        XCTAssertEqual(model.state(model.chats[8]), .idle)
        XCTAssertEqual(model.visibleChats(scope: .attention).map(\.id), ["attention", "failed"])
        XCTAssertEqual(model.visibleChats(scope: .working).map(\.id), ["working"])
    }
    func testProjectIdentityFallsBackToHome() {
        let model = model()
        let project = model.chats.first { $0.id == "working" }!
        XCTAssertEqual(model.projectName(for: project), "Noches")
        XCTAssertEqual(model.monogramSeed(for: project), "/code/noches")
        let home = model.chats.first { $0.id == "idle" }!
        XCTAssertEqual(model.projectName(for: home), "Home")
        XCTAssertEqual(model.monogramSeed(for: home), "home")
    }
    func testNoProjectFilterIsDistinctFromAllProjects() {
        let model = model()
        let home = model.visibleChats(project: CompanionModel.noProjectFilter)
        XCTAssertFalse(home.isEmpty)
        XCTAssertTrue(home.allSatisfy { $0.spaceId == nil })
        XCTAssertGreaterThan(model.visibleChats().count, home.count)
        XCTAssertNotEqual(CompanionModel.noProjectFilter, "")
    }
    func testHostDateParsesChronoPrecision() {
        let base = Date(timeIntervalSince1970: 1_790_078_400)
        XCTAssertEqual(HostDate.parse("2026-09-22T12:00:00Z"), base)
        XCTAssertEqual(HostDate.parse("2026-09-22T12:00:00.123456789Z")?.timeIntervalSince1970 ?? 0, base.timeIntervalSince1970 + 0.123, accuracy: 0.001)
        XCTAssertEqual(HostDate.parse("2026-09-22T14:00:00.25+02:00")?.timeIntervalSince1970 ?? 0, base.timeIntervalSince1970 + 0.25, accuracy: 0.001)
        XCTAssertEqual(HostDate.parse("2026-09-22T12:00:00.123456789123Z")?.timeIntervalSince1970 ?? 0, base.timeIntervalSince1970 + 0.123, accuracy: 0.002)
        XCTAssertNil(HostDate.parse("2026-09-22"))
        XCTAssertNil(HostDate.parse(nil))
        XCTAssertEqual(HostDate.millis(1_790_078_400_500)?.timeIntervalSince1970 ?? 0, base.timeIntervalSince1970 + 0.5, accuracy: 0.001)
    }
    func testUnseenMatchesTheSyncedSeenMarker() {
        var chat = HostChat(id: "c", deviceId: "mac", archived: false, createdAt: "2026-09-22T12:00:00Z")
        XCTAssertFalse(chat.unseen, "no message is never unseen")
        chat.lastMessageAt = "2026-09-22T12:00:00.500Z"
        XCTAssertTrue(chat.unseen, "a message without a seen marker is unseen")
        chat.lastSeenAt = "2026-09-22T12:00:00Z"
        XCTAssertTrue(chat.unseen)
        chat.lastSeenAt = "2026-09-22T12:00:01Z"
        XCTAssertFalse(chat.unseen)
    }
    func testHarnessCatalogMirrorsDescriptorEnabled() throws {
        func harness(_ json: String) throws -> HostHarness { try JSONDecoder().decode(HostHarness.self, from: Data(json.utf8)) }
        let claude = try harness(#"{"id":"claude-code","name":"Claude Code","installed":true,"reasoningLevels":["low","high"]}"#)
        XCTAssertTrue(claude.isOffered)
        XCTAssertEqual(claude.reasoningLevels, ["low", "high"])
        XCTAssertFalse(claude.steersMidTurn)
        let codex = try harness(#"{"id":"codex","name":"Codex","installed":true,"supportsSteering":true,"steeringMode":"step-boundary"}"#)
        XCTAssertTrue(codex.steersMidTurn)
        XCTAssertFalse(try harness(#"{"id":"mock","name":"Mock","installed":true}"#).isOffered)
        XCTAssertFalse(try harness(#"{"id":"antigravity","name":"Antigravity","installed":true}"#).isOffered)
        XCTAssertFalse(try harness(#"{"id":"pi","name":"Pi","installed":true,"enabled":false}"#).isOffered)
        XCTAssertFalse(try harness(#"{"id":"pi","name":"Pi","installed":false,"enabled":true}"#).isOffered)
    }
    func testMarkSeenSkipsSeenChatsAndStillGuardsTheHost() async {
        let model = model()
        var seen = model.chats[0]
        seen.lastMessageAt = "2026-09-22T12:00:00Z"
        seen.lastSeenAt = "2026-09-22T12:00:01Z"
        do { try await model.markSeen(seen) } catch { XCTFail("A seen chat is a no-op, not an error: \(error)") }
        var unseen = seen
        unseen.lastSeenAt = nil
        do { try await model.markSeen(unseen); XCTFail("Offline and unseen must not pretend to clear the badge") } catch {}
    }
    func testQueuedItemDecodesTheDocShapeAndAttachmentPromptTransport() throws {
        let item = try JSONDecoder().decode(HostQueueItem.self, from: Data(#"{"id":"q1","text":"follow up","attachments":["/u/a.png"],"editedAt":1790078400500}"#.utf8))
        XCTAssertEqual(item.attachmentCount, 1)
        XCTAssertEqual(item.edited?.timeIntervalSince1970 ?? 0, 1_790_078_400.5, accuracy: 0.001)
        XCTAssertEqual(HostAttachment.composed("", paths: []), "")
        let body = HostAttachment.composed("fix this", paths: ["/u/a.png"])
        XCTAssertTrue(body.hasPrefix("fix this\n\nAttached images"))
        XCTAssertTrue(body.contains("- /u/a.png"))
        XCTAssertTrue(HostAttachment.composed("", paths: ["/u/a.png"]).hasPrefix("See the attached image(s)."))
    }
    func testSelectingSameHostRestartsConnectionAndDraftsStayScoped() {
        let model = model()
        let chat = model.chats[0]
        var other = chat; other.deviceId = "other"
        model.drafts[model.draftKey(for: chat)] = "Keep this draft"
        let revision = model.connectionRevision
        model.select(model.selectedID)
        XCTAssertEqual(model.connectionRevision, revision + 1)
        XCTAssertEqual(model.drafts[model.draftKey(for: chat)], "Keep this draft")
        XCTAssertNil(model.drafts[model.draftKey(for: other)])
        XCTAssertFalse(model.online)
    }
    func testTranscriptAdapterPreservesToolResolutionAndApprovalIdentity() throws {
        let json = #"{"id":"message","role":"assistant","status":"streaming","parts":[{"id":"tool","kind":"tool","call":{"kind":"exec","command":"swift test"},"isError":false,"resolved":true},{"id":"question","kind":"input","requestId":"approval-17","questions":[{"id":"q","header":"Approve","question":"Run tests?","options":["Allow"]}],"resolved":false}]}"#
        let message = try JSONDecoder().decode(HostMessage.self, from: Data(json.utf8))
        let entry = message.renderedEntry
        XCTAssertEqual(entry.status, .streaming)
        guard case .tool(_, let call, let isError, let resolved) = entry.parts[0] else { return XCTFail("Missing tool") }
        XCTAssertEqual(call.string("command"), "swift test")
        XCTAssertFalse(isError); XCTAssertTrue(resolved)
        guard case .input(_, let request, let questions, let answered) = entry.parts[1] else { return XCTFail("Missing approval") }
        XCTAssertEqual(request, "approval-17")
        XCTAssertEqual(questions.first?.options, ["Allow"])
        XCTAssertFalse(answered)
    }

    func testOfflineAndWrongHostCannotMutate() async {
        let model = model()
        do { try await model.rename(model.chats[0], title: "Rename"); XCTFail("Offline") } catch {}
        model.online = true
        do { try await model.archive(model.chats.last!, archived: true); XCTFail("Wrong host") } catch {}
        do { try await model.rename(model.chats[0], title: " \n "); XCTFail("Blank title") } catch {}
    }
}
