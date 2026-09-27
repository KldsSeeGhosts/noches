import SwiftUI

struct CompanionSessionView: View {
    let model: CompanionModel
    let chat: HostChat
    @State private var messages: [HostMessage] = []
    @Environment(\.dismiss) private var dismiss
    private var draft: Binding<String> {
        Binding(get: { model.drafts[model.draftKey(for: chat)] ?? "" },
                set: { model.drafts[model.draftKey(for: chat)] = $0 })
    }
    @State private var error: String?
    @State private var busy = false
    @State private var loaded = false
    @State private var queue: [JSONValue] = []
    @State private var scroll = ScrollState()
    @State private var submittedID: String?
    @State private var inspector: CompanionInspector?
    @FocusState private var composerFocused: Bool
    private var currentChat: HostChat { model.chats.first { $0.id == chat.id } ?? chat }

    var body: some View {
        VStack(spacing: 0) {
            ZStack {
                CompanionTranscript(messages: messages, scroll: scroll, online: model.online, busy: busy, submittedID: submittedID) { id, answers in
                    perform { try await model.respond(requestID: id, answers: answers, chat: currentChat) }
                }
                if !loaded { ProgressView("Loading session…").font(Theme.sans(13)) }
                if loaded && messages.isEmpty {
                    VStack(spacing: 14) {
                        CompanionAvatar(status: "idle", seed: chat.id).scaleEffect(1.3)
                        Text("What are we working on?").font(Theme.sans(21, weight: .medium)).tracking(-0.5)
                        Text(currentChat.cwd ?? "Home folder").font(Theme.mono(11)).foregroundStyle(Theme.textMuted)
                    }.padding(30)
                }
            }
            composer
        }
        .background(Theme.bg).foregroundStyle(Theme.text)
        .navigationTitle(currentChat.displayTitle).navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItemGroup(placement: .topBarTrailing) {
                Button { inspector = .files } label: { Image(systemName: "folder") }.accessibilityLabel("Browse files")
                Button { inspector = .changes } label: { Image(systemName: "arrow.triangle.branch") }.accessibilityLabel("Review changes")
                CompanionSessionMenu(model: model, chat: currentChat, archived: { dismiss() })
            }
            ToolbarItemGroup(placement: .keyboard) {
                Spacer()
                Button("Done") { composerFocused = false }
            }
        }
        .sheet(item: $inspector) { CompanionWorkspaceSheet(model: model, chat: currentChat, tab: $0) }
        .task(id: "\(model.generation)-\(model.online)") { await watchTranscript() }
        .task(id: "queue-\(model.generation)-\(model.online)") {
            guard model.online else { return }
            let connection = model.connection
            do {
                for try await value in try await connection.watch("WatchQueue", ["chatId": chat.id]) {
                    guard !Task.isCancelled, connection === model.connection else { return }
                    queue = value.objectValue?["items"]?.arrayValue ?? []
                }
            } catch { if !Task.isCancelled { self.error = error.localizedDescription } }
        }
    }

    private var composer: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let error { Text(error).font(Theme.sans(12)).foregroundStyle(Theme.danger).textSelection(.enabled).padding(.horizontal, 8) }
            if !model.online { Text("Reconnecting to your computer…").font(Theme.sans(12)).foregroundStyle(Theme.warning).padding(.horizontal, 8) }
            if !queue.isEmpty {
                DisclosureGroup("Queued · \(queue.count)") {
                    ForEach(Array(queue.enumerated()), id: \.offset) { _, row in
                        Text(row.objectValue?["text"]?.stringValue ?? "Queued message").font(Theme.sans(13))
                            .foregroundStyle(Theme.textMuted).frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 6)
                    }
                }.font(Theme.sans(12)).padding(.horizontal, 12)
            }
            HStack(spacing: 6) {
                if let harness = currentChat.config?.harness {
                    BrandMarkShape(mark: .forHarness(harness)).fill(BrandMark.tint(for: harness))
                        .frame(width: 11, height: 11)
                    Text(currentChat.config?.model ?? HarnessCatalog.label(for: harness)).font(Theme.sans(11)).lineLimit(1)
                }
                Spacer()
                if ["working", "awaitingInput"].contains(model.status(chat.id)) {
                    Text(statusLabel(model.status(chat.id))).font(Theme.sans(11)).foregroundStyle(companionStatusColor(model.status(chat.id)))
                    Button { perform { try await model.stop(currentChat) } } label: {
                        Image(systemName: "stop.fill").font(.system(size: 10)).frame(width: 32, height: 32)
                    }.accessibilityLabel("Stop session").disabled(busy || !model.online)
                } else {
                    Text(currentChat.branch ?? "").font(Theme.mono(10)).lineLimit(1)
                }
            }.foregroundStyle(Theme.textMuted).padding(.horizontal, 14).frame(minHeight: 26)
            HStack(alignment: .bottom, spacing: 10) {
                TextField(["working", "awaitingInput"].contains(model.status(chat.id)) ? "Queue a follow-up…" : "Do anything…", text: draft, axis: .vertical).lineLimit(1...6)
                    .font(Theme.sans(16)).focused($composerFocused).accessibilityIdentifier("companion-composer")
                    .padding(.vertical, 11).padding(.leading, 8)
                Button {
                    let text = draft.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines)
                    let id = UUID().uuidString.lowercased()
                    let queued = ["working", "awaitingInput"].contains(model.status(chat.id))
                    scroll.arm()
                    perform {
                        try await model.send(text, chat: currentChat, messageID: id)
                        if !queued { submittedID = id }
                        if draft.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines) == text { draft.wrappedValue = "" }
                    }
                } label: {
                    Image(systemName: "arrow.up").font(.system(size: 16, weight: .medium)).frame(width: 38, height: 38)
                        .background(Theme.text.opacity(canSend ? 1 : 0.12), in: Circle()).foregroundStyle(canSend ? Theme.bg : Theme.textMuted)
                        .frame(width: 44, height: 44)
                }.accessibilityLabel("Send message").disabled(!canSend)
            }
            .padding(6).padding(.leading, 4).nochesGlass(in: RoundedRectangle(cornerRadius: 28))
        }.padding(.horizontal, 12).padding(.top, 6).padding(.bottom, 8)
            .background(Theme.bg)
    }
    private var canSend: Bool { !busy && loaded && model.online && !draft.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

    private func perform(_ action: @escaping () async throws -> Void) {
        busy = true; error = nil
        Task {
            do { try await action() } catch { self.error = error.localizedDescription }
            busy = false
        }
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
                    messages = try frame.applying(to: messages)
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
}
