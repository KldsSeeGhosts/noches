import XCTest
@testable import Zeron

final class CompanionTranscriptTests: XCTestCase {
    private func hostFrame(_ text: String) throws -> HostTranscriptFrame {
        try JSONDecoder().decode(HostTranscriptFrame.self, from: Data(text.utf8))
    }

    private func hostMessage(_ text: String) throws -> HostMessage {
        try JSONDecoder().decode(HostMessage.self, from: Data(text.utf8))
    }

    private func call(_ tag: String, _ fields: [String: AnyHashable] = [:]) -> RenderToolCall {
        RenderToolCall(tag: tag, fields: fields)
    }

    func testToolPartDecodesResolutionOutputAndSubagent() throws {
        let message = try hostMessage(#"{"id":"m","role":"assistant","parts":[{"id":"t","kind":"tool","call":{"kind":"unknown","name":"Agent: scan repo","input":{"description":"scan repo","model":"haiku"}},"isError":true,"resolved":true,"output":"done","outputBytes":42,"diffStats":[{"path":"/w/a.rs","additions":3,"deletions":1}],"subagentRef":"doc-9","subagentStatus":"failed","subagentTail":"last line"}]}"#)
        let part = message.parts[0]
        XCTAssertEqual(part.resolved, true)
        XCTAssertEqual(part.isError, true)
        XCTAssertEqual(part.output, "done")
        XCTAssertEqual(part.outputBytes, 42)
        XCTAssertEqual(part.diffStats?.first?.additions, 3)
        XCTAssertEqual(part.subagentRef, "doc-9")
        XCTAssertEqual(part.subagentStatus, "failed")
        XCTAssertEqual(part.subagentTail, "last line")

        guard case .tool(_, let rendered, let isError, let resolved) = message.renderedEntry.parts[0] else {
            return XCTFail("expected a tool part")
        }
        XCTAssertTrue(isError)
        XCTAssertTrue(resolved)
        XCTAssertTrue(rendered.isSubagentSpawn)
        XCTAssertEqual(rendered.subagentTitle, "scan repo")
        XCTAssertEqual(rendered.subagentModel, "haiku")
        XCTAssertEqual(rendered.subagentPhase(isError: true, resolved: true, streaming: false), .failed)
    }

    func testResolvedUsesTheRealFieldNotIsErrorPresence() throws {
        let message = try hostMessage(#"{"id":"m","role":"assistant","parts":[{"id":"t","kind":"tool","call":{"kind":"exec","command":"ls"},"isError":false,"resolved":false}]}"#)
        guard case .tool(_, _, let isError, let resolved) = message.renderedEntry.parts[0] else {
            return XCTFail("expected a tool part")
        }
        XCTAssertFalse(isError)
        XCTAssertFalse(resolved)
    }

    func testReasoningPartRendersAsThinkingRow() throws {
        let message = try hostMessage(#"{"id":"m","role":"assistant","parts":[{"id":"r","kind":"reasoning","text":"weighing the options"}]}"#)
        guard case .tool(_, let rendered, false, true) = message.renderedEntry.parts[0] else {
            return XCTFail("expected the synthesized Thinking tool part")
        }
        XCTAssertTrue(rendered.isThought)
        XCTAssertEqual(rendered.chipLabel, "Thinking")
        XCTAssertEqual(rendered.expandedDetail, "weighing the options")
    }

    func testImagePartBuildsGeneratedImageReference() throws {
        let message = try hostMessage(#"{"id":"m","role":"assistant","parts":[{"id":"i","kind":"image","path":"/uploads/a.png","name":"a.png","mimeType":"image/png"}]}"#)
        guard case .image(_, let reference) = message.renderedEntry.parts[0] else {
            return XCTFail("expected an image part")
        }
        XCTAssertEqual(reference.path, "/uploads/a.png")
        XCTAssertEqual(reference.name, "a.png")
        XCTAssertEqual(reference.mimeType, "image/png")
        XCTAssertTrue(reference.isValid)

        let invalid = try hostMessage(#"{"id":"m","role":"assistant","parts":[{"id":"i","kind":"image","path":"","name":"","mimeType":"image/png"}]}"#)
        guard case .error(_, let text) = invalid.renderedEntry.parts[0] else {
            return XCTFail("expected the invalid image to degrade to an error chip")
        }
        XCTAssertEqual(text, "Generated image unavailable")
    }

    func testContextUsageRidesFramesWithAndWithoutRowChanges() throws {
        let opening = try hostFrame(#"{"reset":[{"id":"a","role":"assistant","parts":[{"id":"p","kind":"text","text":"hi"}]}],"contextUsage":{"tokens":100,"window":200,"session":{"input":1,"output":2,"cacheRead":3}}}"#)
        let applied = try opening.apply(to: [])
        XCTAssertEqual(applied.messages.map(\.id), ["a"])
        XCTAssertEqual(applied.contextUsage?.fraction, 0.5)
        XCTAssertEqual(applied.contextUsage?.session?.cacheRead, 3)

        // A usage-only frame carries no rows and must apply untouched.
        let usageOnly = try hostFrame(#"{"contextUsage":{"tokens":null,"window":200}}"#)
        let unchanged = try usageOnly.apply(to: applied.messages)
        XCTAssertEqual(unchanged.messages.map(\.id), ["a"])
        XCTAssertNil(unchanged.contextUsage?.tokens)
        XCTAssertNotNil(unchanged.contextUsage?.window)

        // A count that disagrees with the held rows is still a desync.
        XCTAssertThrowsError(try hostFrame(#"{"count":9}"#).apply(to: applied.messages))
    }

    func testChipLabelsMirrorDesktopView() {
        let long = String(repeating: "x", count: 90)
        let cases: [(RenderToolCall, String, String)] = [
            (call("exec", ["command": "cargo test"]), "Run", "cargo test"),
            (call("readFile", ["path": "/w/a.rs"]), "Read", "/w/a.rs"),
            (call("writeFile", ["path": "/w/b.rs"]), "Write", "/w/b.rs"),
            (call("editFile", ["path": "/w/c.rs"]), "Edit", "/w/c.rs"),
            (call("applyPatch", ["path": "/w/d.rs"]), "Patch", "/w/d.rs"),
            (call("applyPatch"), "Patch", "workspace"),
            (call("search", ["pattern": "needle"]), "Search", "needle"),
            (call("search", ["pattern": "needle", "path": "/w"]), "Search", "needle in /w"),
            (call("glob", ["pattern": "*.swift"]), "Glob", "*.swift"),
            (call("webFetch", ["url": "https://x.test"]), "Fetch", "https://x.test"),
            (call("webSearch", ["query": "swift concurrency"]), "Web", "swift concurrency"),
            (call("todo", ["items": #"[{"text":"a","done":true},{"text":"b","done":false}]"#]), "Todo", "1/2 done"),
            (call("mcp", ["server": "fs", "tool": "read"]), "MCP", "fs · read"),
            (call("unknown", ["name": "Agent: scan repo"]), "Agent", "scan repo"),
            (call("unknown", ["name": "Agent"]), "Agent", ""),
            (call("unknown", ["name": "codex-research", "input": #"{"query":"pi acp tool kinds"}"#]), "Codex research", "pi acp tool kinds"),
            (call("unknown", ["name": "ask_user_question"]), "Ask user question", ""),
            (call("unknown", ["name": "mcp__github__create_issue", "input": #"{"title":"Bug"}"#]), "Github · create issue", "Bug"),
            (call("unknown", ["name": "other"]), "Tool", ""),
            (call("unknown", ["name": "custom", "input": #"{"description":"\#(long)"}"#]), "Custom", String(repeating: "x", count: 80) + "…"),
        ]
        for (tool, label, detail) in cases {
            XCTAssertEqual(tool.chipLabel, label, tool.tag)
            XCTAssertEqual(tool.chipDetail, detail, tool.tag)
        }
        // The cloud adapter hands over the per-item Swift descriptions.
        let cloudTodo = call("todo", ["items": [#"["text": "a", "done": true]"#, #"["text": "b", "done": false]"#]])
        XCTAssertEqual(cloudTodo.chipDetail, "1/2 done")
    }

    func testToolFamiliesMatchDesktopPalette() {
        XCTAssertEqual(ToolFamily.of(call("readFile", ["path": "x"])), .explore)
        XCTAssertEqual(ToolFamily.of(call("search", ["pattern": "x"])), .explore)
        XCTAssertEqual(ToolFamily.of(call("glob", ["pattern": "x"])), .explore)
        XCTAssertEqual(ToolFamily.of(call("webFetch", ["url": "x"])), .explore)
        XCTAssertEqual(ToolFamily.of(call("webSearch", ["query": "x"])), .explore)
        XCTAssertEqual(ToolFamily.of(call("editFile", ["path": "x"])), .change)
        XCTAssertEqual(ToolFamily.of(call("writeFile", ["path": "x"])), .change)
        XCTAssertEqual(ToolFamily.of(call("applyPatch", ["path": "x"])), .change)
        XCTAssertEqual(ToolFamily.of(call("unknown", ["name": "Agent: x"])), .delegate)
        XCTAssertEqual(ToolFamily.of(call("mcp", ["server": "s", "tool": "Agent"])), .delegate)
        XCTAssertEqual(ToolFamily.of(call("unknown", ["name": "Wait for agents"])), .delegate)
        XCTAssertEqual(ToolFamily.of(call("exec", ["command": "ls"])), .neutral)
        XCTAssertEqual(ToolFamily.of(call("todo", ["items": "[]"])), .neutral)
        XCTAssertEqual(ToolFamily.of(call("thinking", ["text": "hmm"])), .neutral)
    }

    func testSubagentDerivationAndOrdering() throws {
        let messages = [
            try hostMessage(#"{"id":"u1","role":"user","parts":[]}"#),
            try hostMessage(#"{"id":"m1","role":"assistant","status":"complete","createdAt":1000,"parts":[{"id":"a","kind":"tool","call":{"kind":"unknown","name":"Agent: scout"},"isError":false,"resolved":true,"output":"all green","subagentRef":"doc-a","subagentStatus":"done"},{"id":"b","kind":"tool","call":{"kind":"unknown","name":"Agent: probe","input":{"description":"probe","model":"haiku"}},"isError":false,"resolved":false,"subagentRef":"doc-b","subagentStatus":"running"}]}"#),
            try hostMessage(#"{"id":"u2","role":"user","parts":[]}"#),
            try hostMessage(#"{"id":"m2","role":"assistant","status":"streaming","createdAt":2000,"parts":[{"id":"s","kind":"tool","call":{"kind":"unknown","name":"Agent: bg","input":{"run_in_background":true}},"isError":false,"resolved":true}]}"#),
        ]
        let subagents = companionSubagents(messages)
        XCTAssertEqual(subagents.map(\.id), ["doc-b", "s", "doc-a"])
        XCTAssertEqual(subagents.map(\.phase), [.running, .started, .done])
        XCTAssertEqual(subagents[0].title, "probe")
        XCTAssertEqual(subagents[0].model, "haiku")
        XCTAssertEqual(subagents[2].tail, "all green")
        XCTAssertNotNil(subagents[0].startedAt)
        XCTAssertNil(subagents[2].finishedAt)
        XCTAssertFalse(subagents[0].latestTurn)
        XCTAssertTrue(subagents[1].latestTurn)
        XCTAssertFalse(subagents[2].latestTurn)
        XCTAssertEqual(companionTraySubagents(subagents).map(\.id), ["doc-b", "s"])
    }

    func testSubagentPhasesFollowTheToolCallLifecycle() {
        let spawn = call("unknown", ["name": "Agent: pi"])
        XCTAssertEqual(spawn.subagentPhase(isError: false, resolved: false, streaming: true), .running)
        XCTAssertEqual(spawn.subagentPhase(isError: false, resolved: false, streaming: false), .failed)
        XCTAssertEqual(spawn.subagentPhase(isError: false, resolved: true, streaming: true), .started)
        XCTAssertEqual(spawn.subagentPhase(isError: false, resolved: true, streaming: false), .done)
        let background = call("unknown", ["name": "Agent: bg", "input": #"{"run_in_background":true}"#])
        XCTAssertEqual(background.subagentPhase(isError: false, resolved: true, streaming: false), .started)
        let reported = call("unknown", ["name": "Agent: live", "subagentStatus": "running"])
        XCTAssertEqual(reported.subagentPhase(isError: false, resolved: true, streaming: false), .running)
        XCTAssertNil(call("exec", ["command": "ls"]).subagentPhase(isError: false, resolved: true, streaming: false))
    }
}
