import Foundation

/// A session's user-facing state. This mirrors `crates/ui/src/status_palette.rs`
/// `SessionState`. It is the only source of status meaning on the phone. Its
/// colors and glyphs live in `Theme/ControlPlaneStyle.swift`.
enum SessionState: String, CaseIterable, Hashable {
    case failed, awaitingInput, working, queued, completed, idle

    var needsYou: Bool { self == .failed || self == .awaitingInput }
    var running: Bool { self == .working || self == .queued }

    /// Desktop labels. Idle has no label.
    var label: String? {
        switch self {
        case .failed: "Failed"
        case .awaitingInput: "Awaiting input"
        case .working: "Working"
        case .queued: "Queued"
        case .completed: "Completed"
        case .idle: nil
        }
    }

    /// Attention order for lists: lower values are more urgent. This mirrors
    /// `view.rs` `attention_rank`, with queued ranked beside working.
    var attentionRank: Int {
        switch self {
        case .awaitingInput: 0
        case .failed: 1
        case .working, .queued: 2
        case .completed: 3
        case .idle: 4
        }
    }
}

/// Host-owned context-window snapshot (`crates/proto/src/agent.rs` `ContextUsage`).
/// It arrives as `contextUsage` on `WatchDocMessages` frames.
struct ContextUsage: Decodable, Equatable {
    struct Totals: Decodable, Equatable {
        var input: UInt64
        var output: UInt64
        var cacheRead: UInt64
    }
    var tokens: UInt64?
    var window: UInt64?
    /// The token count at which the harness auto-compacts. This is present only
    /// when the harness reports it.
    var compactAt: UInt64?
    var session: Totals?

    /// The fraction of the window in use, from 0 to 1. This is nil when unknown.
    /// After compaction `tokens == nil` means "waiting", and never 0%.
    var fraction: Double? {
        guard let tokens, let window, window > 0 else { return nil }
        return min(1, Double(tokens) / Double(window))
    }
}

/// The phase of a subagent spawned by a tool part (`subagents.rs`
/// `SubagentSummary`). `started` is the honest state for background spawns
/// whose own tool result landed before a lifecycle status arrived.
enum SubagentPhase: String, Hashable {
    case running, started, done, failed
}
