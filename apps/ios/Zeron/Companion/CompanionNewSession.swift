import SwiftUI

/// The new-session sheet: project, agent, model and reasoning, then a first
/// message. "Start session" creates the chat and sends the message in the
/// same step, then pushes the session.
struct NewHostSessionSheet: View {
    @Environment(\.dismiss) private var dismiss
    let model: CompanionModel
    var initialProject = ""
    let opened: (HostChat) -> Void
    @State private var space = ""
    @State private var harness = ""
    @State private var agentModel = ""
    @State private var reasoning = ""
    @State private var models: [HostAgentModel] = []
    @State private var loadingModels = false
    @State private var catalogError: String?
    @State private var draft = ""
    @State private var busy = false
    @State private var error: String?
    @State private var addingProject = false
    @State private var browsingFolder = false
    @State private var branches: [String] = []
    @State private var useWorktree = false
    @State private var baseBranch = ""

    private var offered: [HostHarness] { model.harnesses.filter(\.isOffered) }
    private var levels: [String] {
        if agentModel.isEmpty { return offered.first { $0.id == harness }?.reasoningLevels ?? [] }
        return models.first { $0.id == agentModel }?.reasoningLevels ?? []
    }
    private var harnessLabel: String { offered.first { $0.id == harness }?.label ?? "your agent" }
    private var modelLabel: String {
        guard !agentModel.isEmpty else { return "Agent default" }
        return models.first { $0.id == agentModel }?.label ?? agentModel
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    projectSection
                    agentSection
                    modelSection
                    messageSection
                }
                .padding(.top, 12).padding(.bottom, 16)
                .frame(maxWidth: 620)
                .frame(maxWidth: .infinity)
            }
            .background(Theme.bg).foregroundStyle(Theme.text)
            .scrollDismissesKeyboard(.interactively)
            .safeAreaInset(edge: .bottom, spacing: 0) {
                SheetPinnedBar {
                    if let error {
                        Text(error).font(Theme.sans(13)).foregroundStyle(Theme.danger)
                            .fixedSize(horizontal: false, vertical: true)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    startButton
                }
            }
            .navigationTitle("New session").navigationBarTitleDisplayMode(.inline)
            .navigationSubtitle(model.selected?.name ?? "")
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } } }
            .onAppear {
                space = initialProject
                harness = offered.first?.id ?? ""
                reasoning = defaultLevel(offered.first?.reasoningLevels ?? [])
            }
            .task(id: harness) {
                agentModel = ""; models = []; catalogError = nil
                guard !harness.isEmpty else { return }
                loadingModels = true
                do {
                    let catalog = try await model.models(for: harness)
                    guard !Task.isCancelled else { return }
                    models = catalog
                    reasoning = defaultLevel(levels)
                } catch {
                    guard !Task.isCancelled else { return }
                    catalogError = "Couldn't load models. The agent default still works."
                }
                loadingModels = false
            }
            .task(id: space) {
                branches = []; useWorktree = false; baseBranch = ""
                guard let project = model.localSpaces.first(where: { $0.id == space }) else { return }
                // Non-git folders fail `ListBranches`; the branch section stays hidden.
                guard let found = try? await model.branches(for: project), !Task.isCancelled else { return }
                branches = found
                baseBranch = found.first ?? ""
            }
            .sheet(isPresented: $addingProject) {
                CompanionAddProjectSheet(model: model) { added in space = added.id }
            }
            .sheet(isPresented: $browsingFolder) { browseFolderSheet }
            .onChange(of: agentModel) { _, _ in reasoning = defaultLevel(levels) }
            .onChange(of: harness) { _, id in
                agentModel = ""
                reasoning = defaultLevel(offered.first { $0.id == id }?.reasoningLevels ?? [])
            }
            .onChange(of: model.harnesses.map(\.id)) { _, _ in
                if !offered.contains(where: { $0.id == harness }) { harness = offered.first?.id ?? "" }
            }
        }
    }

    // MARK: - Sections

    private var projectSection: some View {
        VStack(alignment: .leading, spacing: 22) {
            SheetGroup(title: "Project", separatorInset: SheetMetrics.margin + 20 + 12) {
                projectRow(title: "No project · home folder", name: "Home", seed: "home", project: nil, id: "")
                ForEach(model.localSpaces) { item in
                    projectRow(title: item.displayName, name: item.displayName, seed: item.path, project: item, id: item.id)
                }
                actionRow("Add project…", icon: "plus", id: "add-project-row") { addingProject = true }
                actionRow("Browse folder…", icon: "folder", id: "browse-folder-row") { browsingFolder = true }
            }
            if !branches.isEmpty { branchControls }
        }
    }

    private func actionRow(_ title: String, icon: String, id: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: icon).font(.system(size: 14, weight: .regular)).foregroundStyle(Theme.textMuted)
                    .frame(width: 20, height: 20)
                Text(title).font(Theme.sans(15)).foregroundStyle(Theme.textMuted)
                Spacer(minLength: 8)
            }
            .padding(.horizontal, SheetMetrics.margin).frame(minHeight: SheetMetrics.rowHeight)
            .contentShape(Rectangle())
        }
        .buttonStyle(SheetPressStyle())
        .disabled(!model.online)
        .accessibilityIdentifier(id)
    }

    /// Browse any host folder and add it as a project without a name step.
    private var browseFolderSheet: some View {
        NavigationStack {
            CompanionFolderBrowser(model: model, title: "Browse folder") { path, isRepo in
                Task {
                    do {
                        let added = try await model.addProject(path: path, name: nil, isRepo: isRepo)
                        space = added.id; browsingFolder = false
                    } catch { self.error = error.localizedDescription; browsingFolder = false }
                }
            }
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { browsingFolder = false } } }
        }
    }

    /// Git projects: stay on the current branch, or start in a fresh worktree
    /// (`zeron/<name>` off the chosen base branch).
    private var branchControls: some View {
        VStack(alignment: .leading, spacing: 10) {
            SheetSectionLabel("Branch").padding(.horizontal, SheetMetrics.margin)
            SheetSegmented(options: [
                .init(value: false, title: "Current branch"),
                .init(value: true, title: "New worktree"),
            ], selection: $useWorktree)
            .padding(.horizontal, SheetMetrics.margin)
            if useWorktree {
                SheetGroup(footer: "Creates an isolated checkout on a new branch so this session can't touch your working tree.") {
                    Menu {
                        ForEach(branches, id: \.self) { name in
                            Button { baseBranch = name } label: {
                                if baseBranch == name { Label(name, systemImage: "checkmark") } else { Text(name) }
                            }
                        }
                    } label: {
                        HStack(spacing: 12) {
                            LineIconView(.gitBranch, size: 14, color: Theme.textMuted).frame(width: 20)
                            Text("Based on").font(Theme.sans(15)).foregroundStyle(Theme.text)
                            Spacer(minLength: 8)
                            Text(baseBranch).font(Theme.mono(12)).foregroundStyle(Theme.textMuted).lineLimit(1)
                            Image(systemName: "chevron.up.chevron.down").font(.system(size: 11, weight: .medium))
                                .foregroundStyle(Theme.textFaint)
                        }
                        .padding(.horizontal, SheetMetrics.margin).frame(minHeight: SheetMetrics.rowHeight)
                        .contentShape(Rectangle())
                    }
                }
            }
        }
    }

    private func projectRow(title: String, name: String, seed: String, project: HostSpace?, id: String) -> some View {
        Button { space = id } label: {
            HStack(spacing: 12) {
                CompanionProjectBadge(model: model, space: project, name: name, seed: seed, size: 20)
                Text(title).font(Theme.sans(15)).foregroundStyle(Theme.text).lineLimit(1)
                Spacer(minLength: 8)
                if space == id {
                    Image(systemName: "checkmark").font(.system(size: 13, weight: .semibold)).foregroundStyle(Theme.text)
                }
            }
            .padding(.horizontal, SheetMetrics.margin).frame(minHeight: SheetMetrics.rowHeight)
            .contentShape(Rectangle())
        }
        .buttonStyle(SheetPressStyle())
        .accessibilityAddTraits(space == id ? .isSelected : [])
    }

    private var agentSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            SheetSectionLabel("Agent").padding(.horizontal, SheetMetrics.margin)
            SheetSegmented(options: offered.map { item in
                .init(value: item.id, title: item.label,
                      icon: AnyView(BrandMarkShape(mark: .forHarness(item.id))
                        .fill(BrandMark.tint(for: item.id), style: FillStyle(eoFill: BrandMark.forHarness(item.id).evenOddFill))
                        .frame(width: 14, height: 14)),
                      identifier: "harness-\(item.id)")
            }, selection: $harness, height: 44)
            .padding(.horizontal, SheetMetrics.margin)
        }
    }

    private var modelSection: some View {
        VStack(alignment: .leading, spacing: 16) {
            SheetGroup(title: "Model") {
                Menu {
                    Button { agentModel = "" } label: {
                        if agentModel.isEmpty { Label("Agent default", systemImage: "checkmark") } else { Text("Agent default") }
                    }
                    ForEach(models) { item in
                        Button { agentModel = item.id } label: {
                            if agentModel == item.id { Label(item.label, systemImage: "checkmark") } else { Text(item.label) }
                        }
                    }
                } label: {
                    HStack(spacing: 8) {
                        Text(modelLabel).font(Theme.sans(15)).foregroundStyle(Theme.text).lineLimit(1)
                        Spacer(minLength: 8)
                        if loadingModels { ProgressView().controlSize(.small) }
                        else { Image(systemName: "chevron.up.chevron.down").font(.system(size: 11, weight: .medium)).foregroundStyle(Theme.textFaint) }
                    }
                    .padding(.horizontal, SheetMetrics.margin).frame(minHeight: SheetMetrics.rowHeight)
                    .contentShape(Rectangle())
                }
                .disabled(loadingModels)
            }
            if !levels.isEmpty {
                VStack(alignment: .leading, spacing: 10) {
                    SheetSectionLabel("Reasoning").padding(.horizontal, SheetMetrics.margin)
                    SheetSegmented(options: levels.map { level in
                        .init(value: level, title: HarnessCatalog.reasoningLabel(level), identifier: "reasoning-\(level)")
                    }, selection: $reasoning)
                    .padding(.horizontal, SheetMetrics.margin)
                }
            }
            if let catalogError {
                Text(catalogError).font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
                    .padding(.horizontal, SheetMetrics.margin)
            }
        }
    }

    private var messageSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            SheetSectionLabel(title: "First message") {
                Text("Optional").font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
            }
            .padding(.horizontal, SheetMetrics.margin)
            TextField("Message \(harnessLabel)…", text: $draft, axis: .vertical)
                .font(Theme.sans(16))
                .lineLimit(2...8)
                .accessibilityIdentifier("first-message")
                .padding(14)
                .frame(minHeight: 72, alignment: .topLeading)
                .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).stroke(Theme.border, lineWidth: 1))
                .padding(.horizontal, SheetMetrics.margin)
        }
    }

    private var startButton: some View {
        Button {
            busy = true; error = nil
            Task {
                do {
                    let picked = agentModel.isEmpty ? nil : agentModel
                    let effort = reasoning.isEmpty ? nil : reasoning
                    let chat: HostChat
                    if useWorktree, !baseBranch.isEmpty, let project = model.localSpaces.first(where: { $0.id == space }) {
                        let tree = try await model.createWorktree(for: project, base: baseBranch)
                        chat = try await model.create(space: project, harness: harness, model: picked, reasoning: effort,
                                                      branch: tree.branch, cwd: tree.path)
                    } else {
                        chat = try await model.create(spaceID: space.isEmpty ? nil : space, harness: harness,
                                                      model: picked, reasoning: effort)
                    }
                    let message = draft.trimmingCharacters(in: .whitespacesAndNewlines)
                    if !message.isEmpty {
                        do { try await model.send(message, chat: chat) }
                        catch { model.drafts[model.draftKey(for: chat)] = message }
                    }
                    dismiss(); opened(chat)
                } catch { self.error = error.localizedDescription }
                busy = false
            }
        } label: {
            Text(busy ? "Starting…" : "Start session")
        }
        .buttonStyle(SheetPrimaryButtonStyle(enabled: !(busy || harness.isEmpty || !model.online)))
        .disabled(busy || harness.isEmpty || !model.online)
        .accessibilityIdentifier("start-session")
    }

    private func defaultLevel(_ levels: [String]) -> String {
        if levels.contains("high") { return "high" }
        if levels.contains("medium") { return "medium" }
        return levels.first ?? ""
    }
}
