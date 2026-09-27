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
        .sheet(item: $inspector) { CompanionWorkspaceSheet(model: model, chat: currentChat, tab: $0) }
        .task(id: "\(model.generation)-\(model.online)") { await watchTranscript() }
        .task(id: "queue-\(model.generation)-\(model.online)") { await watchQueue() }
        .task(id: "badge-\(currentChat.spaceId ?? "")-\(model.generation)") { await loadBadge() }
        .task { try? await model.markSeen(currentChat) }
        .onChange(of: messages) { _, _ in scheduleSeen() }
        .onDisappear { seen?.cancel() }
    }

    // MARK: Header

    /// One glass group for Changes, Files, and More. Splitting them into three
    /// toolbar items costs ~60pt each on iOS 26, which crushes the two-line
    /// title; the compact slots buy the title back without shrinking any hit
    /// target below 44pt (the visual glyph stays 16pt).
    private var actions: some View {
        HStack(spacing: 0) {
            Button { inspector = .changes } label: {
                Image(systemName: "arrow.triangle.branch")
                    .frame(width: 38, height: 44)
                    .contentShape(Rectangle())
            }
            .accessibilityLabel("Review changes")
            Button { inspector = .files } label: {
                Image(systemName: "folder")
                    .frame(width: 38, height: 44)
                    .contentShape(Rectangle())
            }
            .accessibilityLabel("Browse files")
            CompanionSessionMenu(model: model, chat: currentChat, archived: { dismiss() })
                .frame(width: 38, height: 44)
        }
        .frame(height: 44)
    }

    private var header: some View {
        VStack(spacing: 2) {
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
            HStack(spacing: 5) {
                ProjectBadge(name: model.projectName(for: currentChat),
                             seed: model.monogramSeed(for: currentChat), image: badge, size: 12)
                Text(contextLine)
                    .font(Theme.mono(12))
                    .foregroundStyle(Theme.textFaint)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }
            .frame(maxWidth: .infinity)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(currentChat.displayTitle), \(contextLine)")
    }

    /// `{project}:{branch}`, just the project when there is no branch.
    private var contextLine: String {
        let project = model.projectName(for: currentChat)
        if let branch = currentChat.branch, !branch.isEmpty { return "\(project):\(branch)" }
        return project
    }

    // MARK: Transcript

    private var transcript: some View {
        ZStack {
            CompanionTranscript(messages: messages, scroll: scroll, online: model.online,
                                busy: responding, submittedID: submittedID) { id, answers in
                perform { try await model.respond(requestID: id, answers: answers, chat: currentChat) }
            }
            if !loaded { ProgressView("Loading session…").font(Theme.sans(13)) }
            if loaded && messages.isEmpty { emptyState }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 12) {
            BrandMarkShape(mark: mark)
                .fill(BrandMark.tint(for: harness), style: FillStyle(eoFill: mark.evenOddFill))
                .frame(width: 28, height: 28)
            Text("What are we working on?")
                .font(Theme.sans(21, weight: .medium))
                .tracking(-0.5)
            Text(currentChat.cwd ?? "Home folder")
                .font(Theme.mono(12))
                .foregroundStyle(Theme.textFaint)
        }
        .padding(30)
    }

    // MARK: Bottom cluster

    private var bottom: some View {
        VStack(alignment: .leading, spacing: 0) {
            if !model.online {
                Text("Reconnecting to your computer…")
                    .font(Theme.sans(13))
                    .foregroundStyle(Theme.warning)
                    .padding(.bottom, 6)
            }
            if let error {
                Text(error)
                    .font(Theme.sans(13))
                    .foregroundStyle(Theme.danger)
                    .textSelection(.enabled)
                    .lineLimit(3)
                    .padding(.bottom, 6)
            }
            if state == .working || state == .awaitingInput || state == .failed {
                CompanionActivityLine(state: state, since: model.workingSince(currentChat))
                    .padding(.leading, 4)
                    .padding(.bottom, 8)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
            VStack(spacing: -trayTuck) {
                if !agents.isEmpty {
                    CompanionAgentsTray(model: model, agents: agents)
                        .zIndex(0)
                        .transition(.move(edge: .bottom).combined(with: .opacity))
                }
                if !queue.isEmpty {
                    CompanionQueueTray(model: model, chat: currentChat, items: queue,
                                       steers: steers, error: $error)
                        .zIndex(1)
                        .transition(.move(edge: .bottom).combined(with: .opacity))
                }
                CompanionComposer(model: model, chat: currentChat, draft: draft,
                                  usage: contextUsage, state: state,
                                  send: send, stop: { try await model.stop(currentChat) },
                                  error: $error, focused: $composerFocused)
                    .zIndex(2)
            }
        }
        .padding(.horizontal, 12)
        .padding(.top, 6)
        .padding(.bottom, 8 + (composerFocused ? 48 : 0))
        .background(Theme.bg)
        .motionAnimation(Motion.fadeQuick, value: composerFocused)
    }

    // MARK: Wiring

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

    /// Opening the session marks it seen; later messages do too once the turn
    /// has settled, debounced so a streaming turn does not write per frame.
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
        let connection = model.connection
        while !Task.isCancelled && model.online {
            do {
                for try await value in try await connection.watch("WatchDocMessages", ["chatId": chat.id]) {
                    guard !Task.isCancelled, model.connection === connection else { return }
                    let frame = try CompanionModel.decode(HostTranscriptFrame.self, value)
                    let update = try frame.apply(to: messages)
                    messages = update.messages
                    contextUsage = update.contextUsage ?? contextUsage
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
