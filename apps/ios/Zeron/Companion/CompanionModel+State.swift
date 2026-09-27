import Foundation

/// Chat display state, mirroring `crates/proto/src/view.rs` (`display_status` /
/// `effective_indicator`) and `crates/proto/src/entities.rs` (`chat_indicator`).
extension CompanionModel {
    /// A working/awaiting-input session older than this is dead (`SESSION_STALE_MS`).
    static let sessionStaleAfter: TimeInterval = 45

    /// The chat's session row while its liveness heartbeat is still fresh.
    /// Errored and idle rows never expire; a crashed backend must not show an
    /// eternal "Working".
    func liveSession(_ chat: HostChat) -> HostSession? {
        guard let session = sessions.first(where: { $0.chatId == chat.id }) else { return nil }
        switch session.status {
        case "working", "awaitingInput":
            guard let updated = session.updated, Date().timeIntervalSince(updated) <= Self.sessionStaleAfter else { return nil }
        default: break
        }
        return session
    }

    func state(_ chat: HostChat) -> SessionState {
        switch liveSession(chat)?.status {
        case "working": return .working
        case "awaitingInput": return .awaitingInput
        case "errored" where chat.unseen: return .failed
        default: return chat.unseen ? .completed : .idle
        }
    }

    /// When the live working session started, for the row's elapsed time.
    func workingSince(_ chat: HostChat) -> Date? {
        guard state(chat) == .working else { return nil }
        return sessions.first { $0.chatId == chat.id }?.started
    }
}
