import SwiftUI

/// The phone's control plane: sessions as T3 thread cards, either sectioned
/// by state (Needs you / Running / Recent) or grouped under their projects.
/// Colors and sizes follow docs/design/mobile.md; every glyph and hue comes
/// from the shared control-plane primitives.
struct CompanionDashboard: View {
    let model: CompanionModel
    let newSession: (String) -> Void
    let pair: () -> Void
    let opened: (HostChat) -> Void
    /// The session showing in the iPad detail column, highlighted in the list.
    var selectedChatID: String? = nil
    @State private var project = ""
    @State private var query = ""
    @State private var scope: CompanionScope = .all
    @State private var sheet: HomeSheet?
    /// Runs once the drawer has finished dismissing; swapping sheets in one tap races.
    @State private var afterSheet: (() -> Void)?
    @AppStorage("companion.home.grouping") private var groupingRaw = CompanionGrouping.status.rawValue
    @AppStorage("companion.home.collapsed") private var collapsedRaw = ""
    @FocusState private var searchFocused: Bool

    private enum HomeSheet: Identifiable {
        case projects, addProject
        var id: Int { self == .projects ? 0 : 1 }
    }

    private var grouping: CompanionGrouping { CompanionGrouping(rawValue: groupingRaw) ?? .status }
    private var collapsed: Set<String> { CompanionCollapsed.ids(collapsedRaw) }
    private var visible: [HostChat] { model.visibleChats(project: project, query: query, scope: scope) }
    private var needsYou: [HostChat] { visible.filter { model.state($0).needsYou } }
    private var running: [HostChat] { visible.filter { model.state($0).running } }
    private var recent: [HostChat] {
        visible.filter { let state = model.state($0); return !state.needsYou && !state.running }
    }
    /// Empty projects still get a header in the project view, so a fresh
    /// project has somewhere to start its first thread. Filters hide them.
    private var groups: [CompanionProjectGroup] {
        let unfiltered = project.isEmpty && query.isEmpty && scope == .all
        let all = model.projectGroups(chats: visible, keepEmpty: unfiltered)
        return project.isEmpty ? all : all.filter { $0.filterID == project }
    }
    private var projectTitle: String {
        project == CompanionModel.noProjectFilter ? "No project"
            : model.localSpaces.first { $0.id == project }?.displayName ?? "All projects"
    }
    private var hasNoProjects: Bool { model.online && model.localSpaces.isEmpty }
    private var hasActiveFilters: Bool { !project.isEmpty || scope != .all }

    var body: some View {
        List {
            connectionHeader
                .listRowInsets(EdgeInsets(top: 0, leading: 16, bottom: 0, trailing: 3))
                .listRowSeparator(.hidden).listRowBackground(Color.clear)
            if hasActiveFilters {
                activeFilters
                    .listRowInsets(EdgeInsets(top: 0, leading: 16, bottom: 4, trailing: 16))
                    .listRowSeparator(.hidden).listRowBackground(Color.clear)
            }
            if !model.online { offlineRow }
            listContent
        }
        .listStyle(.plain).scrollContentBackground(.hidden)
        .scrollDismissesKeyboard(.interactively)
        .refreshable { await refresh() }
        .background(Theme.bg).foregroundStyle(Theme.text)
        .motionAnimation(Motion.resort, value: visible.map(\.id))
        .motionAnimation(Motion.collapse, value: collapsedRaw)
        .motionAnimation(Motion.collapse, value: groupingRaw)
        .scrollEdgeEffectStyle(.hard, for: .bottom)
        .safeAreaBar(edge: .bottom, spacing: 0) { bottomBar }
        .onAppear { searchFocused = false }
        .onChange(of: model.selectedID) { _, _ in project = ""; query = ""; scope = .all }
        .onChange(of: model.localSpaces.map(\.id)) { _, ids in
            if !project.isEmpty, project != CompanionModel.noProjectFilter, model.online, !ids.contains(project) { project = "" }
        }
        .sheet(item: $sheet, onDismiss: { afterSheet?(); afterSheet = nil }) { item in
            switch item {
            case .projects:
                CompanionProjectList(model: model, selection: $project) { id in
                    afterSheet = { newSession(id) }
                    sheet = nil
                }
            case .addProject:
                CompanionAddProjectSheet(model: model) { space in
                    collapsedRaw = CompanionCollapsed.ids(collapsedRaw).subtracting([space.id]).sorted().joined(separator: "\n")
                    sheet = nil
                }
            }
        }
    }

    /// Sessions, projects and agents arrive over live watches, so refreshing
    /// an online host just drops cached badge art. An offline host retries now
    /// instead of waiting out the four-second backoff.
    private func refresh() async {
        if model.online {
            model.projectIcons = [:]
            model.generation += 1
            return
        }
        guard model.selected != nil else { return }
        model.connectionRevision += 1
        for _ in 0..<20 where !model.online {
            try? await Task.sleep(for: .milliseconds(250))
        }
    }

    // MARK: - Sections

    @ViewBuilder
    private var listContent: some View {
        if visible.isEmpty && (grouping == .status || groups.isEmpty) {
            emptyState
                .listRowInsets(EdgeInsets(top: 24, leading: 16, bottom: 24, trailing: 16))
                .listRowSeparator(.hidden).listRowBackground(Color.clear)
        } else if grouping == .project {
            projectGroups
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
            .padding(.top, 20).padding(.bottom, 4).padding(.horizontal, 16)
            .listRowInsets(EdgeInsets())
            .listRowSeparator(.hidden).listRowBackground(Color.clear)
            .accessibilityAddTraits(.isHeader)
    }

    private func rows(_ chats: [HostChat], showsProject: Bool = true) -> some View {
        ForEach(Array(chats.enumerated()), id: \.element.id) { index, chat in
            CompanionSessionRow(model: model, chat: chat, showsSeparator: index < chats.count - 1,
                                isSelected: chat.id == selectedChatID, showsProject: showsProject) {
                searchFocused = false
                UIApplication.shared.sendAction(#selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
                opened(chat)
            }
            .listRowInsets(EdgeInsets())
            .listRowSeparator(.hidden).listRowBackground(Color.clear)
        }
    }

    // MARK: - Project groups

    @ViewBuilder
    private var projectGroups: some View {
        ForEach(groups) { group in
            // A search should reveal matches, so it overrides collapsing.
            let open = !collapsed.contains(group.id) || !query.isEmpty
            groupHeader(group, open: open)
            if open {
                if group.chats.isEmpty {
                    Text("No sessions yet").font(Theme.sans(13)).foregroundStyle(Theme.textFaint)
                        .padding(.leading, 16).padding(.vertical, 8)
                        .listRowInsets(EdgeInsets()).listRowSeparator(.hidden).listRowBackground(Color.clear)
                } else {
                    rows(group.chats, showsProject: false)
                }
            }
        }
        if project.isEmpty {
            Button { sheet = .addProject } label: {
                HStack(spacing: 10) {
                    Image(systemName: "plus").font(.system(size: 13, weight: .medium)).frame(width: 20)
                    Text("Add project").font(Theme.sans(14))
                    Spacer(minLength: 0)
                }
                .foregroundStyle(Theme.textMuted).padding(.horizontal, 16).frame(minHeight: 48)
                .contentShape(Rectangle())
            }
            .buttonStyle(CompanionPressStyle()).disabled(!model.online)
            .accessibilityIdentifier("add-project-row")
            .listRowInsets(EdgeInsets()).listRowSeparator(.hidden).listRowBackground(Color.clear)
        }
    }

    /// Badge, name, state dots, count, and a "+" that starts a thread in this
    /// project. The name side toggles collapsing; no path, no chevron column.
    private func groupHeader(_ group: CompanionProjectGroup, open: Bool) -> some View {
        HStack(spacing: 0) {
            Button {
                collapsedRaw = CompanionCollapsed.toggled(collapsedRaw, id: group.id)
            } label: {
                HStack(spacing: 8) {
                    CompanionProjectBadge(model: model, space: group.space, name: group.name,
                                          seed: group.space?.path ?? "home", size: 20)
                    Text(group.name).font(Theme.sans(15, weight: .medium)).foregroundStyle(Theme.text).lineLimit(1)
                        .layoutPriority(1)
                    Image(systemName: "chevron.right").font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(Theme.textFaint).rotationEffect(.degrees(open ? 90 : 0))
                    Spacer(minLength: 8)
                    CompanionStateDots(needsYou: group.needsYou, running: group.running)
                    Text("\(group.chats.count)").font(Theme.mono(11)).foregroundStyle(Theme.textFaint)
                }
                .padding(.leading, 16).frame(minHeight: 44).contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("project-group-\(group.id)")
            .accessibilityLabel("\(group.name), \(group.chats.count) sessions")
            .accessibilityValue(open ? "Expanded" : "Collapsed")
            .accessibilityHint("Shows or hides this project's sessions")
            Button { searchFocused = false; newSession(group.projectID) } label: {
                Image(systemName: "plus").font(.system(size: 14, weight: .regular))
                    .foregroundStyle(Theme.textMuted).frame(width: 44, height: 44).contentShape(Rectangle())
            }
            .buttonStyle(.plain).disabled(!model.online)
            .accessibilityLabel("New session in \(group.name)")
            .accessibilityIdentifier("project-new-\(group.id)")
        }
        .padding(.top, 12)
        .listRowInsets(EdgeInsets()).listRowSeparator(.hidden).listRowBackground(Color.clear)
        .accessibilityAddTraits(.isHeader)
    }

    // MARK: - Header

    /// One quiet line under the wordmark: the computer on the left (a dot and
    /// its short name; the full link state lives in its menu) and the single
    /// filter control on the right.
    private var connectionHeader: some View {
        HStack(spacing: 8) {
            Menu {
                Label(model.connectionMessage, systemImage: model.online ? "checkmark.circle" : "exclamationmark.circle")
                    .disabled(true)
                Divider()
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
                    CompanionConnectionDot(online: model.online, message: model.connectionMessage)
                    Text(model.selected?.name ?? "Computer").font(Theme.sans(14, weight: .medium))
                        .foregroundStyle(Theme.text).lineLimit(1)
                    if !model.online {
                        Text(model.connectionMessage).font(Theme.sans(13)).foregroundStyle(Theme.warning).lineLimit(1)
                            .layoutPriority(-1)
                    }
                    Image(systemName: "chevron.down").font(.system(size: 9, weight: .semibold)).foregroundStyle(Theme.textFaint)
                }
                .frame(minHeight: 44).contentShape(Rectangle())
            }
            .accessibilityLabel("\(model.selected?.name ?? "Computer"), \(model.connectionMessage)")
            .accessibilityHint("Switch computer")
            .accessibilityIdentifier("host-switcher")
            Spacer(minLength: 8)
            filterMenu
        }
    }

    /// The one control for how the list is cut: grouping, scope and project.
    private var filterMenu: some View {
        Menu {
            Section("Group") {
                Picker("Group sessions", selection: $groupingRaw) {
                    ForEach(CompanionGrouping.allCases, id: \.rawValue) { Text($0.title).tag($0.rawValue) }
                }
                .pickerStyle(.inline)
            }
            Section("Show") {
                ForEach(CompanionScope.allCases, id: \.self) { item in
                    Button { scope = item } label: {
                        if item == scope { Label(item.title, systemImage: "checkmark") } else { Text(item.title) }
                    }.accessibilityIdentifier("scope-\(item.title)")
                }
            }
            Section {
                Button { searchFocused = false; sheet = .projects } label: {
                    Label("Project: \(projectTitle)", systemImage: "folder")
                }.accessibilityIdentifier("project-filter")
            }
        } label: {
            Image(systemName: hasActiveFilters ? "line.3.horizontal.decrease.circle.fill" : "line.3.horizontal.decrease")
                .font(.system(size: 17)).foregroundStyle(hasActiveFilters ? Theme.text : Theme.textMuted)
                .frame(width: 44, height: 44).contentShape(Rectangle())
        }
        .accessibilityLabel("Filter sessions")
        .accessibilityValue("\(grouping.title), \(scope.title), \(projectTitle)")
        .accessibilityIdentifier("filter-sessions")
    }

    /// Active project and scope filters, each dismissable, so a narrowed list
    /// never looks like a missing one.
    private var activeFilters: some View {
        HStack(spacing: 8) {
            if !project.isEmpty { filterChip(projectTitle, id: "project") { project = "" } }
            if scope != .all { filterChip(scope.title, id: "scope") { scope = .all } }
            Spacer(minLength: 0)
        }
    }

    private func filterChip(_ title: String, id: String, clear: @escaping () -> Void) -> some View {
        Button(action: clear) {
            HStack(spacing: 6) {
                Text(title).font(Theme.sans(12, weight: .medium)).lineLimit(1)
                Image(systemName: "xmark").font(.system(size: 8, weight: .bold))
            }
            .foregroundStyle(Theme.textMuted).padding(.horizontal, 10).frame(height: 28)
            .background(Theme.text.opacity(0.06), in: Capsule())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Clear filter, \(title)")
        .accessibilityIdentifier("filter-chip-\(id)")
    }

    private var offlineRow: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Circle().fill(Theme.warning).frame(width: 6, height: 6)
            Text(model.error ?? "Connect Tailscale and keep Noches running on your computer.")
                .font(Theme.sans(13)).foregroundStyle(Theme.textMuted)
                .lineLimit(2).fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 0)
        }
        .listRowInsets(EdgeInsets(top: 4, leading: 16, bottom: 12, trailing: 16))
        .listRowSeparator(.hidden).listRowBackground(Color.clear)
    }

    // MARK: - Bottom bar

    /// Search and new session, both glass, floating over a soft scroll edge
    /// (`safeAreaBar` insets the list so the last row clears them).
    private var bottomBar: some View {
        HStack(spacing: 8) {
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
            }.padding(.horizontal, 16).frame(height: 48).nochesGlass(in: Capsule())
            Button { searchFocused = false; newSession(project == CompanionModel.noProjectFilter ? "" : project) } label: {
                Image(systemName: "square.and.pencil").font(.system(size: 18)).frame(width: 48, height: 48)
                    .nochesGlass(in: Circle())
            }.disabled(!model.online).accessibilityLabel("New session").accessibilityIdentifier("new-session")
        }
        .padding(.horizontal, 16).padding(.vertical, 8)
    }

    @ViewBuilder
    private var emptyState: some View {
        if hasNoProjects && query.isEmpty && scope == .all {
            VStack(alignment: .leading, spacing: 10) {
                NochesMark(size: 22).opacity(0.5).padding(.bottom, 6)
                Text("No projects yet").font(Theme.sans(20, weight: .medium)).tracking(-0.4)
                Text("Add a folder from your computer to group its sessions by project.")
                    .font(Theme.sans(15)).foregroundStyle(Theme.textMuted)
                Button { sheet = .addProject } label: {
                    HStack(spacing: 8) {
                        Image(systemName: "plus").font(.system(size: 14, weight: .medium))
                        Text("Add a project").font(Theme.sans(16, weight: .medium))
                    }
                    .padding(.horizontal, 22).frame(minHeight: 48)
                    .background(Theme.text, in: Capsule()).foregroundStyle(Theme.bg)
                }
                .padding(.top, 10).accessibilityIdentifier("add-project-empty")
            }.padding(.vertical, 24)
        } else {
            VStack(alignment: .leading, spacing: 10) {
                NochesMark(size: 22).opacity(0.5).padding(.bottom, 6)
                Text(!query.isEmpty ? "No matching sessions" : scope == .attention ? "Nothing needs you"
                     : scope == .working ? "No agents working" : scope == .archived ? "No archived sessions" : "Start a session")
                    .font(Theme.sans(20, weight: .medium)).tracking(-0.4)
                Text(!query.isEmpty ? "Search by title, branch, or project."
                     : scope == .archived ? "Archived sessions can be restored here."
                     : "Your sessions stay in sync with your computer.")
                    .font(Theme.sans(15)).foregroundStyle(Theme.textMuted)
            }.padding(.vertical, 24)
        }
    }
}

/// The computer's link state: emerald when connected, a slow warning pulse
/// while connecting or reconnecting (steady under Reduce Motion).
struct CompanionConnectionDot: View {
    let online: Bool
    let message: String
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var dim = false

    private var pulsing: Bool { !online && (message == "Connecting" || message == "Reconnecting" || message == "Loading sessions") }

    var body: some View {
        Circle().fill(online ? Theme.statusCompleted : Theme.warning).frame(width: 6, height: 6)
            .opacity(pulsing && !reduceMotion && dim ? 0.35 : 1)
            .onAppear { update() }
            .onChange(of: pulsing) { _, _ in update() }
    }

    private func update() {
        if pulsing && !reduceMotion {
            withAnimation(.easeInOut(duration: 0.9).repeatForever(autoreverses: true)) { dim = true }
        } else {
            withAnimation(Motion.fadeQuick) { dim = false }
        }
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
    var isSelected = false
    /// False under a project header, where repeating the project is noise.
    var showsProject = true
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
            content
                .padding(.horizontal, 16).padding(.vertical, 12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .overlay(alignment: .bottom) {
                    if showsSeparator {
                        Rectangle().fill(Theme.border.opacity(0.6)).frame(height: 0.5).padding(.leading, 16)
                    }
                }
                .background(isSelected ? Theme.elementHover : Color.clear)
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

    /// Two columns that share one trailing edge: the title and metadata line
    /// on the left; on the right, state (or time) over the harness mark.
    private var content: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
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
                    .font(Theme.sans(13)).foregroundStyle(Theme.textMuted)
                    .lineLimit(2).multilineTextAlignment(.leading)
                    .fixedSize(horizontal: false, vertical: true)
            }
            HStack(spacing: 6) {
                if showsProject {
                    CompanionProjectBadge(model: model, space: space, name: project, seed: model.monogramSeed(for: chat), size: 14)
                    Text(project).font(Theme.sans(12)).foregroundStyle(Theme.textMuted).lineLimit(1).layoutPriority(1)
                }
                LineIconView(.gitBranch, size: 11, color: Theme.textFaint).padding(.leading, showsProject ? 4 : 0)
                Text(branch).font(Theme.mono(11)).foregroundStyle(Theme.textFaint)
                    .lineLimit(1).truncationMode(.middle)
                Spacer(minLength: 8)
                if let harness = chat.config?.harness {
                    BrandMarkShape(mark: .forHarness(harness))
                        .fill(BrandMark.tint(for: harness), style: FillStyle(eoFill: BrandMark.forHarness(harness).evenOddFill))
                        .frame(width: 13, height: 13)
                }
            }
            .padding(.top, 2)
        }
    }

    @ViewBuilder
    private var trailing: some View {
        if state == .idle {
            TimelineView(.periodic(from: .now, by: 60)) { context in
                Text(Elapsed.relative(chat.activityDate, now: context.date))
                    .font(Theme.mono(11)).foregroundStyle(Theme.textFaint)
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
