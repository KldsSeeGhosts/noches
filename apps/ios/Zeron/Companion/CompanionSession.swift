import SwiftUI

/// The session screen (docs/design/mobile.md "Session"): a two-line header,
/// the shared native transcript, the live activity line, the agents and queue
/// trays, and the composer pill - all on the same shell backdrop.
///
/// The bottom cluster is a SIBLING of the transcript, never an overlay: the
/// transcript's bottom clearance tracks the composer + trays height exactly,
/// and the cluster animates in and out with `Motion.fadeQuick`.
struct CompanionSessionView: View {
    let model: CompanionModel
    let chat: HostChat
    @Environment(\.dismiss) private var dismiss
    @State private var messages: [HostMessage] = []
    @State private var contextUsage: ContextUsage?
    @State private var queue: [HostQueueItem] = []
    @State private var error: String?
    @State private var responding = false
    @State private var loaded = false
    @State private var scroll = ScrollState()
    @State private var submittedID: String?
    @State private var inspector: CompanionInspector?
    @State private var changes = CompanionChangesStore()
    @State private var projectActions: [HostProjectAction] = []
    @State private var runningAction: String?
    @State private var badge: UIImage?
    @State private var seen: Task<Void, Never>?
    @FocusState private var composerFocused: Bool

    private var draft: Binding<String> {
        Binding(get: { model.drafts[model.draftKey(for: chat)] ?? "" },
                set: { model.drafts[model.draftKey(for: chat)] = $0 })
    }
    private var currentChat: HostChat { model.chats.first { $0.id == chat.id } ?? chat }
    private var state: SessionState { model.state(currentChat) }
    private var harness: String { currentChat.config?.harness ?? "claude-code" }
    private var mark: BrandMark { .forHarness(harness) }
    private var agents: [CompanionSubagent] { companionTraySubagents(companionSubagents(messages)) }
    /// The harness steers mid-turn (`HarnessDescriptor::steers_mid_turn`).
    private var steers: Bool { model.harnesses.first { $0.id == harness }?.steersMidTurn ?? false }
    /// Drives the tray-stack animation: a change inserts or removes a surface.
    private var stackKey: String { "\(agents.count)|\(queue.count)|\(state.rawValue)" }

    var body: some View {
        VStack(spacing: 0) {
            transcript
            bottom
        }
        .background(Theme.bg)
        .foregroundStyle(Theme.text)
        .navigationTitle(currentChat.displayTitle)
        .navigationBarTitleDisplayMode(.inline)
        .motionAnimation(Motion.fadeQuick, value: stackKey)
        .toolbar {
            ToolbarItem(placement: .principal) { header }
            ToolbarItem(placement: .topBarTrailing) { actions }
            ToolbarItemGroup(placement: .keyboard) {
                Spacer()
                Button("Done") { composerFocused = false }
                    .buttonStyle(.plain)
            }
        }
        .sheet(item: $inspector) { CompanionWorkspaceSheet(model: model, chat: currentChat, tab: $0, changes: changes) }
        .task(id: "changes-\(currentChat.cwd ?? "")-\(model.generation)-\(model.online)") {
            await changes.run(model: model, chat: currentChat)
        }
        .task(id: "actions-\(currentChat.spaceId ?? "")-\(model.generation)") { await loadActions() }
        .task(id: "\(model.generation)-\(model.online)") { await watchTranscript() }
        .task(id: "queue-\(model.generation)-\(model.online)") { await watchQueue() }
        .task(id: "badge-\(currentChat.spaceId ?? "")-\(model.generation)") { await loadBadge() }
        .task { try? await model.markSeen(currentChat) }
        .onChange(of: messages) { _, _ in scheduleSeen() }
        .onChange(of: state) { _, _ in scheduleSeen() }
        .onDisappear { seen?.cancel() }
    }

    // MARK: Header

    /// One menu: Workspace (changes, files, terminal, project actions), then
    /// the session itself (rename, archive). One 44pt glass target instead of
    /// two pills competing with the title for width.
    private var actions: some View {
        CompanionSessionActions(model: model, chat: currentChat, projectActions: projectActions,
                                runningAction: runningAction,
                                open: { inspector = $0 }, run: run, archived: { dismiss() })
    }

    /// Line one: harness mark and title. Line two: project, branch, and the
    /// working-tree diff, all 11pt mono metadata. Nothing is joined with
    /// punctuation; the branch carries its own glyph.
    private var header: some View {
        VStack(spacing: 3) {
            HStack(spacing: 6) {
                BrandMarkShape(mark: mark)
                    .fill(BrandMark.tint(for: harness), style: FillStyle(eoFill: mark.evenOddFill))
                    .frame(width: 12, height: 12)
                Text(currentChat.displayTitle)
                    .font(Theme.sans(15, weight: .semibold))
                    .lineLimit(1)
                    .truncationMode(.tail)
            }
            .frame(maxWidth: .infinity)
            HStack(spacing: 6) {
                ProjectBadge(name: model.projectName(for: currentChat),
                             seed: model.monogramSeed(for: currentChat), image: badge, size: 12)
                Text(model.projectName(for: currentChat))
                    .lineLimit(1)
                    .truncationMode(.tail)
                    .layoutPriority(1)
                if let branch = currentChat.branch, !branch.isEmpty {
                    HStack(spacing: 3) {
                        Image(systemName: "arrow.triangle.branch").font(.system(size: 9, weight: .medium))
                        Text(branch).lineLimit(1).truncationMode(.middle)
                    }
                }
                if let diff = changes.diff, !diff.files.isEmpty {
                    Button { inspector = .changes } label: {
                        DiffStat(additions: diff.additions, deletions: diff.deletions)
                            .frame(minHeight: 44)
                            .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .fixedSize()
                    .accessibilityLabel("Review changes, \(diff.additions) additions, \(diff.deletions) deletions")
                }
            }
            .font(Theme.mono(11))
            .foregroundStyle(Theme.textFaint)
            .frame(maxWidth: .infinity, maxHeight: 22)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("\(currentChat.displayTitle), \(contextLine)")
    }

    /// Spoken form of the second line: project, then branch.
    private var contextLine: String {
        let project = model.projectName(for: currentChat)
        if let branch = currentChat.branch, !branch.isEmpty { return "\(project), branch \(branch)" }
        return project
    }

    // MARK: Transcript

    private var transcript: some View {
        ZStack {
            CompanionTranscript(messages: messages, scroll: scroll, online: model.online,
                                busy: responding, submittedID: submittedID) { id, answers in
                perform { try await model.respond(requestID: id, answers: answers, chat: currentChat) }
            }
            if !loaded { ProgressView().accessibilityLabel("Loading session") }
            if loaded && messages.isEmpty { emptyState }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 10) {
            BrandMarkShape(mark: mark)
                .fill(BrandMark.tint(for: harness), style: FillStyle(eoFill: mark.evenOddFill))
                .frame(width: 24, height: 24)
            Text("What are we working on?")
                .font(Theme.sans(20, weight: .medium))
                .tracking(-0.4)
            Text(currentChat.cwd ?? "Home folder")
                .font(Theme.mono(11))
                .foregroundStyle(Theme.textFaint)
                .lineLimit(1)
                .truncationMode(.head)
        }
        .padding(.horizontal, Theme.spaceLG)
        .accessibilityElement(children: .combine)
    }

    // MARK: Bottom cluster

    /// Top to bottom: connection and error notes, the live activity line, the
    /// flush tray sections, then the composer pill. Every row shares the same
    /// 16pt side margin.
    private var bottom: some View {
        VStack(alignment: .leading, spacing: 0) {
            if !model.online {
                note("Reconnecting to your computer…", color: Theme.warning)
            }
            if let error {
                note(error, color: Theme.danger).textSelection(.enabled).lineLimit(3)
            }
            if state == .working || state == .awaitingInput || state == .failed {
                CompanionActivityLine(state: state, since: model.workingSince(currentChat))
                    .padding(.horizontal, Theme.spaceLG)
                    .padding(.bottom, 10)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
            if !agents.isEmpty && !composerFocused {
                CompanionAgentsTray(model: model, agents: agents)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
            if !queue.isEmpty {
                CompanionQueueTray(model: model, chat: currentChat, items: queue,
                                   steers: steers, composing: composerFocused, error: $error)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
            CompanionComposer(model: model, chat: currentChat, draft: draft,
                              usage: contextUsage, state: state,
                              send: send, stop: { try await model.stop(currentChat) },
                              error: $error, focused: $composerFocused)
                .padding(.horizontal, Theme.spaceLG)
                .padding(.top, 8)
        }
        .padding(.top, 8)
        .padding(.bottom, 8 + (composerFocused ? 48 : 0))
        .background(Theme.bg)
        .motionAnimation(Motion.fadeQuick, value: composerFocused)
    }

    private func note(_ text: String, color: Color) -> some View {
        Text(text)
            .font(Theme.sans(13))
            .foregroundStyle(color)
            .padding(.horizontal, Theme.spaceLG)
            .padding(.bottom, 8)
    }

    // MARK: Wiring

    private func loadActions() async {
        guard model.online else { return }
        projectActions = (try? await model.projectActions(for: currentChat)) ?? []
    }

    /// Runs a project action on the host and opens the terminal it returns.
    private func run(_ action: HostProjectAction) {
        runningAction = action.id
        error = nil
        Task {
            do {
                _ = try await model.runProjectAction(action, chat: currentChat)
                inspector = .terminal
            } catch { self.error = error.localizedDescription }
            runningAction = nil
        }
    }

    private func send(_ text: String, _ attachments: [HostAttachment], _ messageID: String) async throws {
        let live = model.state(currentChat)
        let queued = live == .working || live == .awaitingInput
        scroll.arm()
        try await model.send(text, chat: currentChat, attachments: attachments, messageID: messageID)
        if !queued { submittedID = messageID }
    }

    private func perform(_ action: @escaping () async throws -> Void) {
        responding = true
        error = nil
        Task {
            do { try await action() } catch { self.error = error.localizedDescription }
            responding = false
        }
    }

    /// Opening the session marks it seen; later messages and the transition
    /// to a settled state do too, debounced so a streaming turn does not
    /// write per frame.
    private func scheduleSeen() {
        seen?.cancel()
        seen = Task {
            try? await Task.sleep(for: .seconds(1))
            guard !Task.isCancelled else { return }
            let live = model.state(currentChat)
            guard live != .working, live != .awaitingInput else { return }
            try? await model.markSeen(currentChat)
        }
    }

    private func loadBadge() async {
        guard let space = model.localSpaces.first(where: { $0.id == currentChat.spaceId }) else {
            badge = nil
            return
        }
        badge = await model.projectIcon(for: space)
    }

    private func watchTranscript() async {
        guard model.online else { return }
        loaded = false
        // A new watch's first frame owns the snapshot; a stale ring from the
        // previous connection must not outlive it.
        contextUsage = nil
        let connection = model.connection
        while !Task.isCancelled && model.online {
            do {
                for try await value in try await connection.watch("WatchDocMessages", ["chatId": chat.id]) {
                    guard !Task.isCancelled, model.connection === connection else { return }
                    let frame = try CompanionModel.decode(HostTranscriptFrame.self, value)
                    let update = try frame.apply(to: messages)
                    messages = update.messages
                    // The host sends `contextUsage` on every emitted frame;
                    // null clears it, absent (older host) leaves it alone.
                    if update.carriesContextUsage { contextUsage = update.contextUsage }
                    loaded = true
                }
                return
            } catch {
                if Task.isCancelled { return }
                self.error = error.localizedDescription
                do { try await Task.sleep(for: .seconds(2)) } catch { return }
            }
        }
    }

    private func watchQueue() async {
        guard model.online else { return }
        let connection = model.connection
        do {
            for try await value in try await connection.watch("WatchQueue", ["chatId": chat.id]) {
                guard !Task.isCancelled, connection === model.connection else { return }
                queue = try CompanionModel.decode([HostQueueItem].self,
                                                  value.objectValue?["items"] ?? .array([]))
            }
        } catch {
            if !Task.isCancelled { self.error = error.localizedDescription }
        }
    }
}
