import Foundation

extension CompanionModel {
    /// Clear the completed-unseen badge. `WatchChats` owns local state, so a
    /// successful write only arrives through the watch. Nothing to do when the
    /// chat is already seen.
    func markSeen(_ chat: HostChat) async throws {
        guard chat.unseen else { return }
        try await mutate(chat, values: ["op": "markChatSeen"])
    }

    /// Full-config replace (rpc.rs `SetChatConfig`, pickers.rs `setChatConfig`).
    /// Nil keeps the chat's current pick; harness changes still send the whole
    /// config so `modelOptions` and sandbox survive.
    func setConfig(_ chat: HostChat, harness: String? = nil, model: String? = nil, reasoning: String? = nil) async throws {
        guard let harness = harness ?? chat.config?.harness else {
            throw RelayError.rpc("Choose an agent for this session first.")
        }
        var config: [String: Any] = ["harness": harness,
            "sandbox": chat.config?.sandbox ?? "workspace-write",
            "modelOptions": try JSONSerialization.jsonObject(with: JSONEncoder().encode(chat.config?.modelOptions ?? [:]))]
        if let model = model ?? chat.config?.model { config["model"] = model }
        if let reasoning = reasoning ?? chat.config?.reasoning { config["reasoning"] = reasoning }
        try await mutate(chat, values: ["op": "setChatConfig", "config": config])
    }
}
