import SwiftUI

// The session tray stack (docs/design/mobile.md "Tray stack"): the live
// activity line, the agents tray, and the queue tray, stacked over the
// composer pill. Trays sit flush on the shell backdrop like the desktop
// panes: no cards, just a 1px hairline across the top of each section.

/// One stacked tray section: full-bleed hairline on top, 16pt side margins.
struct TraySurface<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        content
            .padding(.horizontal, Theme.spaceLG)
            .frame(maxWidth: .infinity, alignment: .leading)
            .overlay(alignment: .top) {
                Rectangle().fill(Theme.border).frame(height: 1)
            }
    }
}

// MARK: - Activity line

/// The single 13pt row above the tray stack: Working with mono elapsed time,
/// Awaiting input, or Failed. Settled sessions render nothing. The status hue
/// comes from `SessionState`; the glyph and label share it.
struct CompanionActivityLine: View {
    let state: SessionState
    let since: Date?

    var body: some View {
        switch state {
        case .working:
            if let since {
                TimelineView(.periodic(from: .now, by: 1)) { context in
                    row(elapsed: Elapsed.working(Int(context.date.timeIntervalSince(since))))
                }
            } else {
                row(elapsed: nil)
            }
        case .awaitingInput, .failed:
            row(elapsed: nil)
        default:
            EmptyView()
        }
    }

    private func row(elapsed: String?) -> some View {
        let tint = state.color ?? Theme.textMuted
        return HStack(spacing: 5) {
            StatusGlyph(state: state)
            if let label = state.label {
                Text(label)
                    .font(Theme.sans(13, weight: .medium))
                    .foregroundStyle(tint)
            }
            if let elapsed {
                Text(elapsed)
                    .font(Theme.mono(11))
                    .foregroundStyle(Theme.textFaint)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .combine)
    }
}

// MARK: - Agents tray

/// The latest turn's agents plus anything still running, as a horizontally
/// scrolling row of 32pt pills. Tapping one pushes its read-only transcript.
struct CompanionAgentsTray: View {
    let model: CompanionModel
    let agents: [CompanionSubagent]

    private var done: Int { agents.filter { $0.phase == .done }.count }

    var body: some View {
        TraySurface {
            HStack(spacing: 10) {
                HStack(spacing: 5) {
                    Text("Agents")
                        .font(Theme.sans(12, weight: .medium))
                    Text("\(done)/\(agents.count)")
                        .font(Theme.mono(11))
                }
                .foregroundStyle(Theme.textFaint)
                .fixedSize()
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(agents) { agent in
                            NavigationLink {
                                CompanionSubagentView(model: model, subagent: agent)
                            } label: {
                                TrayAgentPill(agent: agent)
                            }
                            .buttonStyle(.plain)
                            .accessibilityLabel("\(agent.title), \(agent.phase.rawValue)")
                        }
                    }
                    .padding(.vertical, 6)
                }
                .scrollClipDisabled()
            }
        }
    }
}

private struct TrayAgentPill: View {
    let agent: CompanionSubagent

    var body: some View {
        if agent.phase == .running, agent.startedAt != nil {
            TimelineView(.periodic(from: .now, by: 1)) { context in
                AgentPill(phase: agent.phase, title: agent.title, elapsed: elapsed(now: context.date))
            }
        } else {
            AgentPill(phase: agent.phase, title: agent.title, elapsed: nil)
        }
    }

    private func elapsed(now: Date) -> String? {
        guard agent.phase == .running, let started = agent.startedAt else { return nil }
        return Elapsed.working(Int(now.timeIntervalSince(started)))
    }
}

// MARK: - Queue tray

/// Messages waiting for the running turn, with Send now / Steer now, Edit,
/// and Remove. The header collapses the rows.
struct CompanionQueueTray: View {
    let model: CompanionModel
    let chat: HostChat
    let items: [HostQueueItem]
    /// The chat's harness steers mid-turn (`HarnessDescriptor::steers_mid_turn`).
    let steers: Bool
    var composing = false
    @Binding var error: String?
    @State private var expanded = true
    @State private var editing: HostQueueItem?
    @State private var busy = false
    private var rowsVisible: Bool { expanded && !composing }

    var body: some View {
        TraySurface {
            VStack(alignment: .leading, spacing: 0) {
                header
                if rowsVisible {
                    ScrollView {
                        VStack(spacing: 0) {
                            ForEach(items) { item in row(item).frame(minHeight: 44) }
                        }
                    }
                    .frame(height: min(CGFloat(items.count) * 44, 176))
                    .transition(.opacity)
                }
            }
        }
        .motionAnimation(Motion.collapse, value: expanded)
        .sheet(item: $editing) { item in
            CompanionQueueEditSheet(model: model, chat: chat, item: item)
        }
    }

    private var header: some View {
        Button {
            expanded.toggle()
        } label: {
            HStack(spacing: 6) {
                Text("Queued")
                    .font(Theme.sans(12, weight: .medium))
                    .foregroundStyle(Theme.textFaint)
                Text("\(items.count)")
                    .font(Theme.mono(11))
                    .foregroundStyle(Theme.textFaint)
                Spacer(minLength: 8)
                Image(systemName: "chevron.down")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(Theme.textFaint)
                    .rotationEffect(.degrees(rowsVisible ? 0 : -90))
            }
            .frame(minHeight: 44)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(composing)
        .accessibilityLabel("Queued · \(items.count)")
        .accessibilityHint(rowsVisible ? "Collapse queued messages" : "Expand queued messages")
    }

    private func row(_ item: HostQueueItem) -> some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text(MessageQueue.visibleText(item.text, attachments: item.attachments ?? []))
                    .font(Theme.sans(14))
                    .foregroundStyle(Theme.textMuted)
                    .lineLimit(2)
                    .frame(maxWidth: .infinity, alignment: .leading)
                if item.attachmentCount > 0 {
                    Text(item.attachmentCount == 1 ? "1 image" : "\(item.attachmentCount) images")
                        .font(Theme.mono(11))
                        .foregroundStyle(Theme.textFaint)
                }
            }
            Menu {
                Button(actionLabel(item), systemImage: actionIcon(item)) { advance(item) }
                Button("Edit", systemImage: "pencil") { editing = item }
                if let index = items.firstIndex(of: item), items.count > 1 {
                    if index > 0 {
                        Button("Move up", systemImage: "arrow.up") { move(item, to: index - 1) }
                        if index > 1 { Button("Move to top", systemImage: "arrow.up.to.line") { move(item, to: 0) } }
                    }
                    if index < items.count - 1 {
                        Button("Move down", systemImage: "arrow.down") { move(item, to: index + 1) }
                    }
                }
                Divider()
                Button("Remove", systemImage: "trash", role: .destructive) { remove(item) }
            } label: {
                Image(systemName: "ellipsis")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(Theme.textMuted)
                    .frame(width: 44, height: 44, alignment: .trailing)
                    .contentShape(Rectangle())
            }
            .accessibilityLabel("Queued message actions")
            .disabled(busy || !model.online)
        }
    }

    private func actionLabel(_ item: HostQueueItem) -> String {
        steers && item.attachmentCount == 0 ? "Steer now" : "Send now"
    }

    private func actionIcon(_ item: HostQueueItem) -> String {
        steers && item.attachmentCount == 0 ? "arrow.turn.down.right" : "arrow.right"
    }

    private func advance(_ item: HostQueueItem) {
        busy = true
        error = nil
        let steers = self.steers && item.attachmentCount == 0
        Task {
            do {
                if steers { try await model.steerQueuedNow(chat, item) }
                else { try await model.sendQueuedNow(chat, item) }
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }

    private func move(_ item: HostQueueItem, to index: Int) {
        busy = true
        error = nil
        UISelectionFeedbackGenerator().selectionChanged()
        Task {
            do { try await model.moveQueued(chat, item, to: index) }
            catch { self.error = error.localizedDescription }
            busy = false
        }
    }

    private func remove(_ item: HostQueueItem) {
        busy = true
        error = nil
        Task {
            do { try await model.removeQueued(chat, item) }
            catch { self.error = error.localizedDescription }
            busy = false
        }
    }
}

// MARK: - Queue edit sheet

/// Retypes one queued message through the host's edit lease. The lease is
/// taken when the sheet opens and populated from the host's current text;
/// it is released if the sheet closes without saving. Conflicts and failures
/// stay in the sheet so the draft is never lost silently.
struct CompanionQueueEditSheet: View {
    let model: CompanionModel
    let chat: HostChat
    let item: HostQueueItem
    @Environment(\.dismiss) private var dismiss
    @State private var lease: CompanionQueuedEditLease?
    @State private var text = ""
    @State private var error: String?
    @State private var busy = true
    @State private var committed = false
    @State private var renewal: Task<Void, Never>?

    var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 12) {
                TextField("Queued message", text: $text, axis: .vertical)
                    .lineLimit(3...12)
                    .font(Theme.sans(16))
                    .foregroundStyle(Theme.text)
                    .padding(10)
                    .background(Theme.wash(0.04), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                    .accessibilityIdentifier("queue-edit")
                if let error {
                    Text(error)
                        .font(Theme.sans(13))
                        .foregroundStyle(Theme.danger)
                }
                Spacer(minLength: 0)
            }
            .padding(20)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(Theme.bg)
            .navigationTitle("Edit queued message")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") { save() }
                        .disabled(busy || text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
        }
        .tint(Theme.text)
        .presentationDetents([.medium, .large])
        .task { await begin() }
        .onDisappear {
            // Swipe-away or Cancel closes the row to other devices; a save
            // that succeeded already committed the lease.
            renewal?.cancel()
            guard !committed, let lease else { return }
            Task { await model.cancelQueuedEdit(lease) }
        }
    }

    /// Acquire the lease and open the editor on the host's current text, not
    /// the possibly stale queue snapshot.
    private func begin() async {
        guard lease == nil else { return }
        do {
            let started = try await model.beginQueuedEdit(chat, item)
            // Dismissal can win the race with the host's ACK; release the
            // lease instead of leaving the row gated for other devices.
            guard !Task.isCancelled else {
                await model.cancelQueuedEdit(started)
                return
            }
            lease = started
            renewal = Task { [model] in
                // Leases expire on the host; renew while the sheet stays open.
                while !Task.isCancelled {
                    try? await Task.sleep(for: .seconds(20))
                    if Task.isCancelled { return }
                    await model.renewQueuedEdit(started)
                }
            }
            text = MessageQueue.visibleText(started.text, attachments: item.attachments ?? [])
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
        busy = false
    }

    private func save() {
        guard let lease else { return }
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let body = MessageQueue.editedText(trimmed, hasAttachments: item.attachmentCount > 0) ?? trimmed
        busy = true
        error = nil
        Task {
            do {
                try await model.commitQueuedEdit(lease, text: body)
                committed = true
                dismiss()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}
