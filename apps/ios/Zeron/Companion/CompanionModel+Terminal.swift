import Foundation

/// `TerminalSession`: an open PTY on the host.
struct HostTerminalSession: Decodable, Equatable {
    let id: String
    let cwd: String
    let shell: String
}

/// One `SubscribeTerminal` item.
enum HostTerminalEvent: Equatable {
    case data(seq: UInt64, bytes: Data)
    case exit(seq: UInt64, code: Int, signal: String?)

    /// Decodes `{type:"data",seq,data:base64}` and `{type:"exit",seq,exitCode,signal?}`.
    init?(_ value: JSONValue) {
        guard let object = value.objectValue, let seq = object["seq"]?.int64Value.map(UInt64.init(clamping:)) else { return nil }
        switch object["type"]?.stringValue {
        case "data":
            guard let text = object["data"]?.stringValue, let bytes = Data(base64Encoded: text) else { return nil }
            self = .data(seq: seq, bytes: bytes)
        case "exit":
            self = .exit(seq: seq, code: Int(object["exitCode"]?.int64Value ?? 0), signal: object["signal"]?.stringValue)
        default: return nil
        }
    }
}

/// `ProjectAction` (`ListProjectActions` snapshot entry).
struct HostProjectAction: Decodable, Identifiable, Hashable {
    let id: String
    let name: String
    let command: String
    let icon: String
}

private struct HostProjectActionsSnapshot: Decodable { let actions: [HostProjectAction] }

/// `ProjectActionRun` (`RunProjectAction` reply).
struct HostProjectActionRun: Decodable {
    let actionId: String
    let actionName: String
    let terminal: HostTerminalSession
}

extension CompanionModel {
    /// `OpenTerminal {chatId, cols, rows}`; the host resolves the chat's cwd.
    func openTerminal(for chat: HostChat, cols: Int, rows: Int) async throws -> HostTerminalSession {
        try Self.decode(HostTerminalSession.self, await read("OpenTerminal", chat: chat,
            params: ["chatId": chat.id, "cols": cols, "rows": rows]))
    }

    /// `SubscribeTerminal {terminalId, afterSeq?}`: replays the host's bounded window
    /// (resuming after `afterSeq`), then tails live output until the shell exits.
    func subscribeTerminal(_ id: String, afterSeq: UInt64?) async throws -> AsyncThrowingStream<HostTerminalEvent, Error> {
        guard online else { throw RelayError.notConnected }
        var params: [String: Any] = ["terminalId": id]
        if let afterSeq { params["afterSeq"] = afterSeq }
        let source = try await connection.watch("SubscribeTerminal", params)
        return AsyncThrowingStream { continuation in
            let task = Task {
                do {
                    for try await value in source { if let event = HostTerminalEvent(value) { continuation.yield(event) } }
                    continuation.finish()
                } catch { continuation.finish(throwing: error) }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    /// `WriteTerminal {terminalId, data}` with base64 bytes. Never replayed.
    func writeTerminal(_ id: String, chat: HostChat, bytes: Data) async throws {
        _ = try await read("WriteTerminal", chat: chat, params: ["terminalId": id, "data": bytes.base64EncodedString()])
    }

    /// `ResizeTerminal {terminalId, cols, rows}`.
    func resizeTerminal(_ id: String, chat: HostChat, cols: Int, rows: Int) async throws {
        _ = try await read("ResizeTerminal", chat: chat, params: ["terminalId": id, "cols": cols, "rows": rows])
    }

    /// `CloseTerminal {terminalId}`.
    func closeTerminal(_ id: String, chat: HostChat) async throws {
        _ = try await read("CloseTerminal", chat: chat, params: ["terminalId": id])
    }

    /// `ListProjectActions {spaceId}`. Empty when the chat has no project.
    func projectActions(for chat: HostChat) async throws -> [HostProjectAction] {
        guard let spaceId = chat.spaceId else { return [] }
        return try Self.decode(HostProjectActionsSnapshot.self, await read("ListProjectActions", chat: chat,
            params: ["spaceId": spaceId])).actions
    }

    /// `RunProjectAction {spaceId, chatId, actionId, cols, rows}`. The returned
    /// terminal is registered as the chat's terminal so the Terminal view shows it.
    func runProjectAction(_ action: HostProjectAction, chat: HostChat, cols: Int = 80, rows: Int = 24) async throws -> HostTerminalSession {
        guard let spaceId = chat.spaceId else { throw RelayError.rpc("This session has no project.") }
        let run = try Self.decode(HostProjectActionRun.self, await read("RunProjectAction", chat: chat,
            params: ["spaceId": spaceId, "chatId": chat.id, "actionId": action.id, "cols": cols, "rows": rows]))
        CompanionTerminalRegistry.shared.terminals[terminalKey(for: chat)] = run.terminal.id
        return run.terminal
    }

    func terminalKey(for chat: HostChat) -> String { "\(chat.deviceId)/\(chat.id)" }
}
