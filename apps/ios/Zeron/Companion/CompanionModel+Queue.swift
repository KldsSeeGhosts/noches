import Foundation

/// One unsent message in a session's queue (`crates/doc/src/queue.rs`
/// `QueuedMessage`).
struct HostQueueItem: Decodable, Identifiable, Hashable {
    var id: String
    var text: String
    var attachments: [String]?
    /// Epoch milliseconds of the last text edit.
    var editedAt: Int64?
    var attachmentCount: Int { attachments?.count ?? 0 }
    var edited: Date? { HostDate.millis(editedAt) }
}

extension CompanionModel {
    /// The lease identity this phone writes into a queue edit. The host only
    /// records it (Locked replies name the owner); any stable, distinct value
    /// keeps two phones from fighting over a row.
    static let queueEditorDeviceId = "ios-companion"
    static let queueEditorInstanceId = UUID().uuidString.lowercased()

    private struct QueueEditLease: Decodable {
        let outcome: String
        let leaseId: String?
        let baseTextHash: String?
    }

    func removeQueued(_ chat: HostChat, _ item: HostQueueItem) async throws {
        try await queued("RemoveQueuedMessage", chat, item)
    }

    func sendQueuedNow(_ chat: HostChat, _ item: HostQueueItem) async throws {
        try await queued("SendQueuedMessageNow", chat, item)
    }

    func steerQueuedNow(_ chat: HostChat, _ item: HostQueueItem) async throws {
        try await queued("SteerQueuedMessageNow", chat, item)
    }

    /// Edit through the host's lease: Begin, then Finish with the lease's
    /// base-text hash so a concurrent edit loses instead of clobbering. A
    /// failed commit releases the row.
    func editQueued(_ chat: HostChat, _ item: HostQueueItem, text: String) async throws {
        guard online, chat.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        let reply = try await connection.call("BeginQueuedMessageEdit", ["chatId": chat.id, "id": item.id,
            "editorDeviceId": Self.queueEditorDeviceId, "editorInstanceId": Self.queueEditorInstanceId])
        let lease = try Self.decode(QueueEditLease.self, reply)
        guard lease.outcome == "acquired", let leaseId = lease.leaseId, let baseTextHash = lease.baseTextHash else {
            throw RelayError.rpc(lease.outcome == "locked"
                ? "Another device is editing this message." : "This queued message is already gone.")
        }
        do {
            _ = try await connection.call("FinishQueuedMessageEdit", ["chatId": chat.id, "id": item.id,
                "leaseId": leaseId, "action": "commit", "text": text, "expectedTextHash": baseTextHash])
        } catch {
            _ = try? await connection.call("FinishQueuedMessageEdit", ["chatId": chat.id, "id": item.id,
                "leaseId": leaseId, "action": "cancel"])
            throw error
        }
    }

    private func queued(_ method: String, _ chat: HostChat, _ item: HostQueueItem) async throws {
        guard online, chat.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        _ = try await connection.call(method, ["chatId": chat.id, "id": item.id])
    }
}
