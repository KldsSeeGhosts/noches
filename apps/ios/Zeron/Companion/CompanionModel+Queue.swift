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

/// An acquired host edit lease (`doc_host.rs` `BeginQueueEditOutcome::Acquired`).
struct CompanionQueuedEditLease: Equatable, Sendable {
    let chatId: String
    let deviceId: String
    let rowId: String
    let leaseId: String
    /// The row's text at lease time; the editor opens on this, not a stale list.
    let text: String
    let baseTextHash: String
}

extension CompanionModel {
    /// The lease identity this phone writes into a queue edit. The host only
    /// records it (Locked replies name the owner); any stable, distinct value
    /// keeps two phones from fighting over a row.
    static let queueEditorDeviceId = "ios-companion"
    static let queueEditorInstanceId = UUID().uuidString.lowercased()

    private struct QueueEditBeginReply: Decodable {
        let outcome: String
        let leaseId: String?
        let text: String?
        let baseTextHash: String?
    }

    private struct QueueEditFinishReply: Decodable {
        let outcome: String
        let currentText: String?
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

    /// Take the host edit lease for a row. It stays gated on every other
    /// device until `commitQueuedEdit` or `cancelQueuedEdit` resolves it.
    func beginQueuedEdit(_ chat: HostChat, _ item: HostQueueItem) async throws -> CompanionQueuedEditLease {
        guard online, chat.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        let reply = try await connection.call("BeginQueuedMessageEdit", ["chatId": chat.id, "id": item.id,
            "editorDeviceId": Self.queueEditorDeviceId, "editorInstanceId": Self.queueEditorInstanceId])
        let lease = try Self.decode(QueueEditBeginReply.self, reply)
        switch lease.outcome {
        case "acquired":
            guard let leaseId = lease.leaseId, let baseTextHash = lease.baseTextHash else {
                throw RelayError.rpc("The host returned an incomplete edit lease.")
            }
            return CompanionQueuedEditLease(chatId: chat.id, deviceId: chat.deviceId, rowId: item.id,
                                            leaseId: leaseId, text: lease.text ?? item.text,
                                            baseTextHash: baseTextHash)
        case "locked":
            throw RelayError.rpc("Another device is editing this message.")
        default:
            throw RelayError.rpc("This queued message is already gone.")
        }
    }

    /// Commit through the lease's base-text hash so a concurrent edit loses
    /// instead of being clobbered. Only `committed` is success; every other
    /// outcome keeps the draft in the editor as an error.
    func commitQueuedEdit(_ lease: CompanionQueuedEditLease, text: String) async throws {
        guard online, lease.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        let reply = try await connection.call("FinishQueuedMessageEdit", ["chatId": lease.chatId, "id": lease.rowId,
            "leaseId": lease.leaseId, "action": "commit", "text": text, "expectedTextHash": lease.baseTextHash])
        let finish = try Self.decode(QueueEditFinishReply.self, reply)
        switch finish.outcome {
        case "committed":
            return
        case "conflict":
            throw RelayError.rpc("This message changed on another device. Your edit was not saved.")
        case "missing":
            throw RelayError.rpc("This queued message is no longer available.")
        default:
            throw RelayError.rpc("The edit lease expired. Reopen the message to try again.")
        }
    }

    /// Release a lease without changing the row. Best effort: an unlanded
    /// release also expires on the host.
    func cancelQueuedEdit(_ lease: CompanionQueuedEditLease) async {
        guard online, lease.deviceId == selected?.deviceId else { return }
        _ = try? await connection.call("FinishQueuedMessageEdit", ["chatId": lease.chatId, "id": lease.rowId,
            "leaseId": lease.leaseId, "action": "cancel"])
    }

    /// `MoveQueuedMessage {chatId, id, toIndex}`; `toIndex` is the row's
    /// final position in the list.
    func moveQueued(_ chat: HostChat, _ item: HostQueueItem, to index: Int) async throws {
        guard online, chat.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        _ = try await connection.call("MoveQueuedMessage", Self.moveQueuedParams(chatID: chat.id, id: item.id, toIndex: index))
    }

    /// `RenewQueuedMessageEdit {chatId, id, leaseId}` keeps a long edit alive.
    func renewQueuedEdit(_ lease: CompanionQueuedEditLease) async {
        guard online, lease.deviceId == selected?.deviceId else { return }
        _ = try? await connection.call("RenewQueuedMessageEdit", Self.renewQueuedParams(lease))
    }

    static func moveQueuedParams(chatID: String, id: String, toIndex: Int) -> [String: Any] {
        ["chatId": chatID, "id": id, "toIndex": max(0, toIndex)]
    }

    static func renewQueuedParams(_ lease: CompanionQueuedEditLease) -> [String: Any] {
        ["chatId": lease.chatId, "id": lease.rowId, "leaseId": lease.leaseId]
    }

    private func queued(_ method: String, _ chat: HostChat, _ item: HostQueueItem) async throws {
        guard online, chat.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        _ = try await connection.call(method, ["chatId": chat.id, "id": item.id])
    }
}
