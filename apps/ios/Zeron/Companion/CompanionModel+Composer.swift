import Foundation

/// How a run treats tool approvals. Maps to `autoApprove` on the run command.
enum CompanionAccess: String, CaseIterable {
    case ask, auto
    var label: String { self == .ask ? "Ask" : "Auto-approve" }
    var icon: String { self == .ask ? "hand.raised" : "bolt.fill" }
}

/// Sandbox levels (`SandboxLevel`, kebab-case on the wire).
enum CompanionSandbox: String, CaseIterable {
    case readOnly = "read-only", workspaceWrite = "workspace-write", full = "danger-full-access"
    var label: String {
        switch self {
        case .readOnly: "Read-only"
        case .workspaceWrite: "Workspace write"
        case .full: "Full access"
        }
    }
}

extension CompanionModel {
    private func accessKey(_ chat: HostChat) -> String { "companion.access.\(draftKey(for: chat))" }

    /// Per host+chat, persisted in UserDefaults. Defaults to Ask.
    func access(for chat: HostChat) -> CompanionAccess {
        UserDefaults.standard.string(forKey: accessKey(chat)).flatMap(CompanionAccess.init) ?? .ask
    }

    func setAccess(_ access: CompanionAccess, for chat: HostChat) {
        UserDefaults.standard.set(access.rawValue, forKey: accessKey(chat))
    }

    /// Persist the sandbox on the chat (`setChatConfig`, whole config).
    func setSandbox(_ chat: HostChat, _ sandbox: CompanionSandbox) async throws {
        guard let config = chat.config else { throw RelayError.rpc("Choose an agent for this session first.") }
        let encoded = try JSONSerialization.jsonObject(with: JSONEncoder().encode(config.modelOptions))
        var values: [String: Any] = ["harness": config.harness, "sandbox": sandbox.rawValue, "modelOptions": encoded]
        if let model = config.model { values["model"] = model }
        if let reasoning = config.reasoning { values["reasoning"] = reasoning }
        try await mutate(chat, values: ["op": "setChatConfig", "config": values])
    }

    /// `SearchFiles {query, chatId}` -> `[{path, isDir}]`, scoped to the chat's checkout.
    func searchFiles(_ chat: HostChat, query: String) async throws -> [HostFileMatch] {
        let reply = try await read("SearchFiles", chat: chat, params: Self.searchFilesParams(chatID: chat.id, query: query))
        return try Self.decode([HostFileMatch].self, reply)
    }

    static func searchFilesParams(chatID: String, query: String) -> [String: Any] {
        ["chatId": chatID, "query": String(query.prefix(256))]
    }

    /// `ListCommands {harness}` -> `[{name, description, inputHint?}]`.
    func slashCommands(for harness: String) async throws -> [HostSlashCommand] {
        guard online else { throw RelayError.notConnected }
        return try Self.decode([HostSlashCommand].self, await connection.call("ListCommands", ["harness": harness]))
    }
}
