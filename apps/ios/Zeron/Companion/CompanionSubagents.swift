import SwiftUI

// Companion subagent inventory: a pure selector over a chat's spawn tool
// parts (crates/ui/src/subagents.rs `subagents_for`) plus the read-only
// pushed screen for one subagent.

struct CompanionSubagent: Identifiable, Hashable {
    /// The subagent doc ref when the host stamped one, else the spawn part id.
    var id: String
    var partID: String
    var docRef: String?
    var title: String
    var agentType: String?
    var model: String?
    var phase: SubagentPhase
    var startedAt: Date?
    var finishedAt: Date?
    /// One-line result (or live tail).
    var tail: String?
    /// Sits in the chat's latest turn (after the last user entry).
    var latestTurn: Bool
}

/// The chat's spawn calls as display summaries. Ordering mirrors desktop:
/// active (Running/Started) oldest first, then finished newest first.
func companionSubagents(_ messages: [HostMessage]) -> [CompanionSubagent] {
    guard !messages.isEmpty else { return [] }
    let lastUser = messages.lastIndex { $0.role == "user" } ?? 0
    var out: [CompanionSubagent] = []
    for (index, entry) in messages.enumerated() where entry.role != "user" {
        let streaming = entry.status == "streaming"
        for part in entry.parts where part.kind == "tool" {
            let call = part.renderedCall()
            guard let phase = call.subagentPhase(isError: part.isError ?? false,
                                                 resolved: part.resolved ?? false,
                                                 streaming: streaming) else { continue }
            out.append(CompanionSubagent(
                id: part.subagentRef ?? part.id,
                partID: part.id,
                docRef: part.subagentRef,
                title: call.subagentTitle,
                agentType: call.subagentType,
                model: call.subagentModel,
                phase: phase,
                startedAt: millisDate(entry.createdAt),
                finishedAt: nil,
                tail: call.subagentTail,
                latestTurn: index >= lastUser))
        }
    }
    out.sort { a, b in
        if a.phase.active != b.phase.active { return a.phase.active }
        if a.phase.active { return (a.startedAt ?? .distantPast) < (b.startedAt ?? .distantPast) }
        return (b.finishedAt ?? b.startedAt ?? .distantPast) > (a.finishedAt ?? a.startedAt ?? .distantPast)
    }
    return out
}

/// The agents tray subset (`subagents.rs` `strip_visible`): the latest turn's
/// agents plus anything still Running from earlier turns.
func companionTraySubagents(_ subagents: [CompanionSubagent]) -> [CompanionSubagent] {
    subagents.filter { $0.latestTurn || $0.phase == .running }
}

private func millisDate(_ ms: Int64?) -> Date? {
    guard let ms, ms > 0 else { return nil }
    return Date(timeIntervalSince1970: Double(ms) / 1000)
}

/// Read-only pushed subagent transcript. Real sub docs stream through
/// `WatchDocMessages {chatId: subagentRef}`; a doc-less spawn (Pi) or one
/// whose doc fails to open shows its frozen tail/result as a single message,
/// never an empty screen.
struct CompanionSubagentView: View {
    let model: CompanionModel
    let subagent: CompanionSubagent
    @State private var messages: [HostMessage] = []
    @State private var scroll = ScrollState()
    @State private var failed = false

    private var usingFrozen: Bool { subagent.docRef == nil || failed }

    private var frozen: [HostMessage] {
        let text = subagent.tail ?? (subagent.phase == .failed
            ? "This subagent failed before reporting a result."
            : "This subagent's transcript stays on your computer.")
        return [HostMessage(id: "\(subagent.id)#frozen", role: "assistant", status: "complete",
                            parts: [HostPart(id: "frozen", kind: "text", text: text)])]
    }

    var body: some View {
        ZStack {
            CompanionTranscript(messages: usingFrozen ? frozen : messages,
                                scroll: scroll, online: model.online, busy: false, submittedID: nil) { _, _ in }
            if !usingFrozen && messages.isEmpty {
                ProgressView().tint(Theme.textFaint)
            }
        }
        .background(Theme.bg)
        .navigationTitle(subagent.title)
        .navigationBarTitleDisplayMode(.inline)
        .task(id: subagent.docRef) {
            guard let ref = subagent.docRef else { return }
            let connection = model.connection
            do {
                for try await value in try await connection.watch("WatchDocMessages", ["chatId": ref]) {
                    guard !Task.isCancelled, model.connection === connection else { return }
                    let frame = try CompanionModel.decode(HostTranscriptFrame.self, value)
                    messages = try frame.applying(to: messages)
                    // A doc that opened with no entries still has the spawn's
                    // result to show; never leave the screen empty.
                    if messages.isEmpty, subagent.tail != nil { failed = true; return }
                }
            } catch {
                guard !Task.isCancelled else { return }
                failed = true
            }
        }
    }
}
