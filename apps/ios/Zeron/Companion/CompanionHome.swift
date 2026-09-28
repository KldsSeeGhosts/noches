import SwiftUI

/// The phone's control plane: Needs you / Running / Recent sections of T3
/// thread cards. Colors and sizes follow docs/design/mobile.md; every glyph
/// and hue comes from the shared control-plane primitives.
struct CompanionDashboard: View {
    let model: CompanionModel
    let newSession: (String) -> Void
    let pair: () -> Void
    let opened: (HostChat) -> Void
    @State private var project = ""
    @State private var query = ""
    @State private var scope: CompanionScope = .all
    @FocusState private var searchFocused: Bool

    private var visible: [HostChat] { model.visibleChats(project: project, query: query, scope: scope) }
    private var needsYou: [HostChat] { visible.filter { model.state($0).needsYou } }
    private var running: [HostChat] { visible.filter { model.state($0).running } }
    private var recent: [HostChat] {
        visible.filter { let state = model.state($0); return !state.needsYou && !state.running }
    }
    private var projectTitle: String { model.localSpaces.first { $0.id == project }?.displayName ?? "All projects" }

    var body: some View {
        List {
            connectionHeader
                .listRowInsets(EdgeInsets(top: 2, leading: 20, bottom: 6, trailing: 20))
                .listRowSeparator(.hidden).listRowBackground(Color.clear)
            if !model.online { offlineRow }
            listContent
        }
        .listStyle(.plain).scrollContentBackground(.hidden)
        .scrollDismissesKeyboard(.interactively)
        .background(Theme.bg).foregroundStyle(Theme.text)
        .motionAnimation(Motion.resort, value: visible.map(\.id))
        .safeAreaInset(edge: .bottom, spacing: 0) { bottomBar }
        .onAppear { searchFocused = false }
        .onChange(of: model.selectedID) { _, _ in project = ""; query = ""; scope = .all }
    }

    // MARK: - Sections

    @ViewBuilder
    private var listContent: some View {
        if visible.isEmpty {
            emptyState
                .listRowInsets(EdgeInsets(top: 24, leading: 20, bottom: 24, trailing: 20))
                .listRowSeparator(.hidden).listRowBackground(Color.clear)
        } else if scope == .archived {
            rows(visible)
        } else {
            if !needsYou.isEmpty {
                sectionHeader("Needs you", dot: SessionState.awaitingInput.color, count: needsYou.count)
                rows(needsYou)
            }
            if !running.isEmpty {
                sectionHeader("Running", dot: SessionState.working.color, count: running.count)
                rows(running)
            }
            if !recent.isEmpty {
                if needsYou.isEmpty && running.isEmpty {
                    rows(recent)
                } else {
                    sectionHeader("Recent", dot: nil, count: recent.count)
                    rows(recent)
                }
            }
        }
    }

    private func sectionHeader(_ title: String, dot: Color?, count: Int) -> some View {
        SectionHeader(title: title, dotColor: dot, count: count)
            .padding(.top, 22).padding(.bottom, 4).padding(.horizontal, 20)
            .listRowInsets(EdgeInsets())
            .listRowSeparator(.hidden).listRowBackground(Color.clear)
            .accessibilityAddTraits(.isHeader)
    }

    private func rows(_ chats: [HostChat]) -> some View {
        ForEach(Array(chats.enumerated()), id: \.element.id) { index, chat in
            CompanionSessionRow(model: model, chat: chat, showsSeparator: index < chats.count - 1) {
                searchFocused = false
                UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
                opened(chat)
            }
            .listRowInsets(EdgeInsets(top: 0, leading: 8, bottom: 0, trailing: 0))
            .listRowSeparator(.hidden).listRowBackground(Color.clear)
        }
    }

    // MARK: - Header

    /// One quiet line under the wordmark: which computer (with its link
    /// state) on the left, which project on the right.
    private var connectionHeader: some View {
        HStack(spacing: 10) {
            Menu {
                ForEach(model.profiles) { host in
                    Button { model.select(host.id) } label: {
                        if host.id == model.selectedID { Label(host.name, systemImage: "checkmark") }
                        else { Text(host.name) }
                    }
                }
                Divider()
                Button("Pair a computer", systemImage: "plus", action: pair)
            } label: {
                HStack(spacing: 8) {
                    Circle().fill(model.online ? Theme.statusCompleted : Theme.warning).frame(width: 6, height: 6)
                    Text(model.selected?.name ?? "Computer").font(Theme.sans(14, weight: .medium))
                        .foregroundStyle(Theme.text).lineLimit(1)
                    Text(model.connectionMessage).font(Theme.mono(12))
                        .foregroundStyle(model.online ? Theme.textFaint : Theme.warning).lineLimit(1)
                    Image(systemName: "chevron.down").font(.system(size: 9, weight: .semibold)).foregroundStyle(Theme.textFaint)
                }
                .frame(minHeight: 44).contentShape(Rectangle())
            }
            .accessibilityHint("Switch computer")
            Spacer(minLength: 8)
            Menu {
                Button { project = "" } label: {
                    if project.isEmpty { Label("All projects", systemImage: "checkmark") }
                    else { Text("All projects") }
                }
                ForEach(model.localSpaces) { space in
                    Button { project = space.id } label: {
                        HStack(spacing: 8) {
                            CompanionProjectBadge(model: model, space: space, name: space.displayName,
                                                  seed: space.path, size: 16)
                            Text(space.displayName)
                            if project == space.id {
                                Spacer(minLength: 16)
                                Image(systemName: "checkmark")
                            }
                        }
                    }
                }
            } label: {
                HStack(spacing: 7) {
                    if let space = model.localSpaces.first(where: { $0.id == project }) {
                        CompanionProjectBadge(model: model, space: space, name: space.displayName, seed: space.path, size: 16)
                    } else {
                        LineIconView(.folder, size: 13, color: Theme.textMuted)
                    }
                    Text(projectTitle).font(Theme.sans(14)).foregroundStyle(Theme.textMuted).lineLimit(1)
                    Image(systemName: "chevron.down").font(.system(size: 9, weight: .semibold)).foregroundStyle(Theme.textFaint)
                }
                .frame(minHeight: 44).contentShape(Rectangle())
            }
            .accessibilityLabel("Filter by project, \(projectTitle)")
        }
    }

    private var offlineRow: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Circle().fill(Theme.warning).frame(width: 6, height: 6)
            Text(model.error ?? "Connect Tailscale and keep Noches running on your computer.")
                .font(Theme.sans(13)).foregroundStyle(Theme.textMuted)
                .lineLimit(2).fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 0)
        }
        .listRowInsets(EdgeInsets(top: 0, leading: 20, bottom: 12, trailing: 20))
        .listRowSeparator(.hidden).listRowBackground(Color.clear)
    }

    // MARK: - Bottom bar

    private var bottomBar: some View {
        HStack(spacing: 8) {
            Menu {
                ForEach(CompanionScope.allCases, id: \.self) { item in
                    Button { scope = item } label: {
                        if item == scope { Label(item.title, systemImage: "checkmark") } else { Text(item.title) }
                    }.accessibilityIdentifier("scope-\(item.title)")
                }
            } label: {
                Image(systemName: scope == .all ? "line.3.horizontal.decrease" : "line.3.horizontal.decrease.circle.fill")
                    .font(.system(size: 18, weight: .regular)).frame(width: 46, height: 46)
                    .nochesGlass(in: Circle())
            }.accessibilityLabel("Filter sessions")
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass").font(.system(size: 14)).foregroundStyle(Theme.textMuted)
                TextField("Search", text: $query).font(Theme.sans(15))
                    .autocorrectionDisabled().textInputAutocapitalization(.never)
                    .accessibilityIdentifier("session-search").focused($searchFocused)
                    .submitLabel(.search).onSubmit { searchFocused = false }
                if !query.isEmpty {
                    Button { query = "" } label: { Image(systemName: "xmark.circle.fill").font(.system(size: 15)).foregroundStyle(Theme.textMuted) }
                        .frame(width: 30, height: 44).accessibilityLabel("Clear search")
                }
            }.padding(.horizontal, 15).frame(height: 46).nochesGlass(in: Capsule())
            Button { searchFocused = false; newSession(project) } label: {
                Image(systemName: "square.and.pencil").font(.system(size: 18)).frame(width: 46, height: 46)
                    .nochesGlass(in: Circle())
            }.disabled(!model.online).accessibilityLabel("New session").accessibilityIdentifier("new-session")
        }
        .padding(.horizontal, 16).padding(.top, 10).padding(.bottom, 8)
        .background {
            LinearGradient(colors: [Theme.bg.opacity(0), Theme.bg.opacity(0.94), Theme.bg], startPoint: .top, endPoint: .bottom)
                .ignoresSafeArea(edges: .bottom)
        }
    }

    private var emptyState: some View {
        VStack(alignment: .leading, spacing: 10) {
            NochesMark(size: 22).opacity(0.5).padding(.bottom, 6)
            Text(!query.isEmpty ? "No matching sessions" : scope == .attention ? "Nothing needs you"
                 : scope == .working ? "No agents working" : scope == .archived ? "No archived sessions" : "Start a session")
                .font(Theme.sans(20, weight: .medium)).tracking(-0.4)
            Text(!query.isEmpty ? "Search by title, branch, or project."
                 : scope == .archived ? "Archived sessions can be restored here."
                 : "Your sessions stay in sync with your computer.")
                .font(Theme.sans(15)).foregroundStyle(Theme.textMuted)
        }.padding(.vertical, 40)
    }
}

private extension CompanionScope {
    /// The phone calls the working scope "Running", matching its section.
    var title: String { self == .working ? "Running" : rawValue }
}

// MARK: - Cards

/// One thread card: the title owns the first line with the status slot on the
/// right, a needs-you preview under it, then one metadata line - project
/// badge, project, branch, and the harness mark. The title leads because it
/// is what people scan for; where it runs is supporting detail.
struct CompanionSessionRow: View {
    let model: CompanionModel
    let chat: HostChat
    var showsSeparator = true
    let opened: () -> Void
    @State private var rename = false
    @State private var name = ""
    @State private var error: String?
    @State private var busy = false

    private var state: SessionState { model.state(chat) }
    private var space: HostSpace? { model.localSpaces.first { $0.id == chat.spaceId } }
    private var project: String { model.projectName(for: chat) }
    private var branch: String { chat.branch ?? (chat.cwd as NSString?)?.lastPathComponent ?? "home" }

    var body: some View {
        Button(action: opened) {
            HStack(spacing: 0) {
                if state.needsYou {
                    RoundedRectangle(cornerRadius: 1.5, style: .continuous)
                        .fill(state.color ?? Theme.warning)
                        .frame(width: 3)
                        .frame(maxHeight: .infinity)
                }
                content
                    .padding(.leading, state.needsYou ? 9 : 12)
                    .padding(.trailing, 20)
                    .padding(.vertical, 14)
            }
            .overlay(alignment: .bottom) {
                if showsSeparator {
                    Rectangle().fill(Theme.border.opacity(0.6)).frame(height: 0.5).padding(.leading, 12)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(CompanionPressStyle())
        .accessibilityIdentifier("open-session-\(chat.id)")
        .contextMenu {
            Button("Rename session", systemImage: "pencil") { name = chat.displayTitle; rename = true }
            Button(chat.archived ? "Restore session" : "Archive session",
                   systemImage: chat.archived ? "tray.and.arrow.up" : "archivebox") { archive() }
            if chat.unseen {
                Button("Mark as read", systemImage: "checkmark.circle") { perform { try await model.markSeen(chat) } }
            }
        }
        .swipeActions(edge: .trailing, allowsFullSwipe: false) {
            Button(chat.archived ? "Restore" : "Archive", systemImage: chat.archived ? "tray.and.arrow.up" : "archivebox") { archive() }
                .tint(Theme.textMuted)
        }
        .disabled(busy)
        .alert("Rename session", isPresented: $rename) {
            TextField("Session name", text: $name)
            Button("Cancel", role: .cancel) {}
            Button("Save") { perform { try await model.rename(chat, title: name) } }
                .disabled(!model.online || name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        }
        .alert("Could not update session", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
            Button("OK", role: .cancel) { error = nil }
        } message: { Text(error ?? "") }
    }

    private var content: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Text(chat.displayTitle)
                    .font(Theme.sans(16, weight: state.needsYou ? .medium : .regular))
                    .tracking(-0.2)
                    .foregroundStyle(state.needsYou ? Theme.text : Theme.text.opacity(0.92))
                    .lineLimit(2).multilineTextAlignment(.leading)
                    .fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: 8)
                trailing
            }
            if state.needsYou, let preview = chat.lastMessagePreview, !preview.isEmpty {
                Text(preview)
                    .font(Theme.sans(14)).foregroundStyle(Theme.textMuted)
                    .lineLimit(2).multilineTextAlignment(.leading)
                    .fixedSize(horizontal: false, vertical: true)
            }
            HStack(spacing: 6) {
                CompanionProjectBadge(model: model, space: space, name: project, seed: model.monogramSeed(for: chat), size: 16)
                Text(project).font(Theme.sans(13)).foregroundStyle(Theme.textMuted).lineLimit(1).layoutPriority(1)
                LineIconView(.gitBranch, size: 11, color: Theme.textFaint).padding(.leading, 4)
                Text(branch).font(Theme.mono(12)).foregroundStyle(Theme.textFaint)
                    .lineLimit(1).truncationMode(.middle)
                Spacer(minLength: 8)
                if let harness = chat.config?.harness {
                    BrandMarkShape(mark: .forHarness(harness))
                        .fill(BrandMark.tint(for: harness), style: FillStyle(eoFill: BrandMark.forHarness(harness).evenOddFill))
                        .frame(width: 13, height: 13)
                }
            }
        }
    }

    @ViewBuilder
    private var trailing: some View {
        if state == .idle {
            TimelineView(.periodic(from: .now, by: 60)) { context in
                Text(Elapsed.relative(chat.activityDate, now: context.date))
                    .font(Theme.mono(12)).foregroundStyle(Theme.textFaint)
            }
        } else {
            StatusSlot(state: state, since: model.workingSince(chat))
        }
    }

    private func archive() { perform { try await model.archive(chat, archived: !chat.archived) } }

    private func perform(_ action: @escaping () async throws -> Void) {
        busy = true
        Task { do { try await action() } catch { self.error = error.localizedDescription }; busy = false }
    }
}

/// Shared pressed wash for companion rows and cards.
struct CompanionPressStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(configuration.isPressed ? Theme.elementHover : Color.clear)
    }
}

/// A project badge that resolves the repo favicon through the model. The
/// model caches the fetch for its TTL; the view keeps the decoded image so
/// list recycling never flashes the monogram back in.
struct CompanionProjectBadge: View {
    let model: CompanionModel
    let space: HostSpace?
    let name: String
    let seed: String
    var size: CGFloat = 18
    @State private var image: UIImage?

    var body: some View {
        ProjectBadge(name: name, seed: seed, image: image, size: size)
            .task(id: "\(space?.id ?? seed)-\(model.online)-\(model.generation)") {
                guard let space else { image = nil; return }
                image = await model.projectIcon(for: space)
            }
    }
}

// MARK: - Session actions


struct CompanionSessionMenu: View {
    let model: CompanionModel
    let chat: HostChat
    var archived: () -> Void = {}
    var files: (() -> Void)? = nil
    @State private var renaming = false
    @State private var title = ""
    @State private var error: String?
    @State private var busy = false
    var body: some View {
        Menu {
            if let files { Button("Browse files", systemImage: "folder", action: files); Divider() }
            Button("Rename session", systemImage: "pencil") { title = chat.displayTitle; renaming = true }
            Button(chat.archived ? "Restore session" : "Archive session", systemImage: chat.archived ? "tray.and.arrow.up" : "archivebox") {
                perform { try await model.archive(chat, archived: !chat.archived); archived() }
            }
        } label: {
            Image(systemName: "ellipsis").font(.system(size: 16)).foregroundStyle(Theme.textMuted).frame(width: 44, height: 44)
        }
        .accessibilityLabel("Session actions for \(chat.displayTitle)")
        .disabled(!model.online || busy)
        .alert("Rename session", isPresented: $renaming) {
            TextField("Session name", text: $title)
            Button("Cancel", role: .cancel) {}
            Button("Save") { perform { try await model.rename(chat, title: title) } }
                .disabled(title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        }
        .alert("Could not update session", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
            Button("OK", role: .cancel) { error = nil }
        } message: { Text(error ?? "") }
    }
    private func perform(_ action: @escaping () async throws -> Void) {
        busy = true
        Task {
            do { try await action() } catch { self.error = error.localizedDescription }
            busy = false
        }
    }
}
