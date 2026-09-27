import SwiftUI

// The session tray stack (docs/design/mobile.md "Tray stack"): the live
// activity line, the agents tray, and the queue tray, stacked over the
// composer pill. Trays use `surfaceRaised` with 16pt top corners and a
// `border` hairline, and tuck 14pt behind the element below them, so Agents,
// Queue, and the pill read as one continuous surface.

/// How far a tray's bottom edge hides behind the element below it. One hair
/// more than the 16pt corner radius so the corner arcs never expose a notch.
let trayTuck: CGFloat = 18

/// One stacked tray surface: `surfaceRaised`, 16pt top corners, a hairline
/// border, and enough bottom padding that the tuck hides surface, not content.
struct TraySurface<Content: View>: View {
    @ViewBuilder var content: Content

    private var shape: UnevenRoundedRectangle {
        UnevenRoundedRectangle(topLeadingRadius: 16, bottomLeadingRadius: 0,
                               bottomTrailingRadius: 0, topTrailingRadius: 16,
                               style: .continuous)
    }

    var body: some View {
        content
            .padding(.horizontal, 12)
            .padding(.bottom, trayTuck + 8)
            .background(Theme.surfaceRaised, in: shape)
            .overlay(TrayEdge())
    }
}

/// The tray's outline: top corners and both sides, with no bottom edge. The
/// bottom edge lives behind the next surface, and drawing it would leave a
/// hairline across the sliver the next surface's rounded corner does not
/// cover.
private struct TrayEdge: View {
    var body: some View {
        GeometryReader { geometry in
            Path { path in
                let width = geometry.size.width
                let bottom = geometry.size.height + 24
                path.move(to: CGPoint(x: 0, y: bottom))
                path.addLine(to: CGPoint(x: 0, y: 16))
                path.addQuadCurve(to: CGPoint(x: 16, y: 0), control: .zero)
                path.addLine(to: CGPoint(x: width - 16, y: 0))
                path.addQuadCurve(to: CGPoint(x: width, y: 16), control: CGPoint(x: width, y: 0))
                path.addLine(to: CGPoint(x: width, y: bottom))
            }
            .stroke(Theme.border, lineWidth: 1)
        }
        .allowsHitTesting(false)
    }
}

// MARK: - Activity line

/// The single 13pt row above the tray stack: Working with mono elapsed time,
/// Awaiting input, or Failed. Settled sessions render nothing.
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
                    .font(Theme.mono(12))
                    .foregroundStyle(tint)
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
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    Text("Agents")
                        .font(Theme.sans(12, weight: .medium))
                        .foregroundStyle(Theme.textFaint)
                    Text("\(done)/\(agents.count)")
                        .font(Theme.mono(12))
                        .foregroundStyle(Theme.textFaint)
                    Spacer(minLength: 0)
                }
                .frame(height: 40)
                .padding(.leading, 4)
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
                    .padding(.vertical, 2)
                }
            }
        }
    }
}

private struct TrayAgentPill: View {
    let agent: CompanionSubagent

    var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { context in
            AgentPill(phase: agent.phase, title: agent.title, elapsed: elapsed(now: context.date))
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
    @Binding var error: String?
    @State private var expanded = true
    @State private var editing: HostQueueItem?
    @State private var busy = false

    var body: some View {
        TraySurface {
            VStack(alignment: .leading, spacing: 0) {
                header
                if expanded {
                    ForEach(items) { item in
                        row(item)
                    }
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
                Text("Queued · \(items.count)")
                    .font(Theme.sans(13, weight: .medium))
                    .foregroundStyle(Theme.textMuted)
                Spacer(minLength: 8)
                Image(systemName: "chevron.down")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(Theme.textFaint)
                    .rotationEffect(.degrees(expanded ? 0 : -90))
            }
            .padding(.horizontal, 4)
            .frame(minHeight: 32)
            .padding(.vertical, 6)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Queued · \(items.count)")
        .accessibilityHint(expanded ? "Collapse queued messages" : "Expand queued messages")
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
            .padding(.leading, 4)
            Menu {
                Button(actionLabel(item), systemImage: actionIcon(item)) { advance(item) }
                Button("Edit", systemImage: "pencil") { editing = item }
                Divider()
                Button("Remove", systemImage: "trash", role: .destructive) { remove(item) }
            } label: {
                Image(systemName: "ellipsis")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(Theme.textMuted)
                    .frame(width: 44, height: 44)
                    .contentShape(Rectangle())
            }
            .accessibilityLabel("Queued message actions")
            .disabled(busy || !model.online)
        }
        .padding(.vertical, 1)
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

/// Retypes one queued message through the host's edit lease. Conflicts and
/// failures stay in the sheet so the draft is never lost silently.
struct CompanionQueueEditSheet: View {
    let model: CompanionModel
    let chat: HostChat
    let item: HostQueueItem
    @Environment(\.dismiss) private var dismiss
    @State private var text = ""
    @State private var error: String?
    @State private var busy = false

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
        .onAppear { text = MessageQueue.visibleText(item.text, attachments: item.attachments ?? []) }
    }

    private func save() {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let body = MessageQueue.editedText(trimmed, hasAttachments: item.attachmentCount > 0) ?? trimmed
        busy = true
        error = nil
        Task {
            do {
                try await model.editQueued(chat, item, text: body)
                dismiss()
            } catch {
                self.error = error.localizedDescription
            }
            busy = false
        }
    }
}
