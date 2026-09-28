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
                VStack(alignment: .leading, spacing: 26) {
                    HStack(spacing: 6) {
                        Image(systemName: "desktopcomputer").font(.system(size: 11))
                        Text("Run on \(model.selected?.name ?? "your computer")").font(Theme.mono(11))
                    }.foregroundStyle(Theme.textFaint)
                    projectSection
                    agentSection
                    modelSection
                    messageSection
                    startButton
                    if let error {
                        Text(error).font(Theme.sans(13)).foregroundStyle(Theme.danger)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
                .padding(20)
                .frame(maxWidth: 620)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(Theme.bg).foregroundStyle(Theme.text)
            .scrollDismissesKeyboard(.interactively)
            .navigationTitle("New session").navigationBarTitleDisplayMode(.inline)
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
        VStack(alignment: .leading, spacing: 10) {
            SectionHeader(title: "Project", dotColor: nil, count: nil)
            VStack(spacing: 0) {
                projectRow(title: "No project · home folder", name: "Home", seed: "home", project: nil, id: "")
                ForEach(Array(model.localSpaces.enumerated()), id: \.element.id) { index, item in
                    if index > 0 { rowSeparator }
                    projectRow(title: item.displayName, name: item.displayName, seed: item.path, project: item, id: item.id)
                }
            }
            .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).stroke(Theme.border, lineWidth: 1))
        }
    }

    private func projectRow(title: String, name: String, seed: String, project: HostSpace?, id: String) -> some View {
        Button { space = id } label: {
            HStack(spacing: 12) {
                CompanionProjectBadge(model: model, space: project, name: name, seed: seed, size: 20)
                Text(title).font(Theme.sans(15)).foregroundStyle(Theme.text).lineLimit(1)
                Spacer(minLength: 8)
                if space == id {
                    Image(systemName: "checkmark").font(.system(size: 13, weight: .semibold)).foregroundStyle(Theme.textMuted)
                }
            }
            .padding(.horizontal, 14).frame(minHeight: 48)
            .contentShape(Rectangle())
        }
        .buttonStyle(CompanionPressStyle())
    }

    private var rowSeparator: some View {
        Rectangle().fill(Theme.border).frame(height: 0.5).padding(.leading, 46)
    }

    private var agentSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionHeader(title: "Agent", dotColor: nil, count: nil)
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(offered) { item in
                        Button { harness = item.id } label: {
                            VStack(spacing: 9) {
                                BrandMarkShape(mark: .forHarness(item.id))
                                    .fill(BrandMark.tint(for: item.id), style: FillStyle(eoFill: BrandMark.forHarness(item.id).evenOddFill))
                                    .frame(width: 20, height: 20)
                                Text(item.label).font(Theme.sans(13))
                                    .foregroundStyle(harness == item.id ? Theme.text : Theme.textMuted)
                                    .lineLimit(1)
                            }
                            .frame(width: 100, height: 74)
                            .background(harness == item.id ? Theme.surfaceRaised : Theme.text.opacity(0.04),
                                        in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                            .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous)
                                .stroke(harness == item.id ? Theme.text.opacity(0.5) : Theme.border, lineWidth: 1))
                        }
                        .buttonStyle(.plain)
                        .accessibilityIdentifier("harness-\(item.id)")
                    }
                }.padding(.vertical, 2)
            }
        }
    }

    private var modelSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionHeader(title: "Model", dotColor: nil, count: nil)
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
                .padding(.horizontal, 14).frame(minHeight: 48)
                .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).stroke(Theme.border, lineWidth: 1))
            }
            .disabled(loadingModels)
            if !levels.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 6) {
                        ForEach(levels, id: \.self) { level in
                            Button { reasoning = level } label: {
                                Text(HarnessCatalog.reasoningLabel(level))
                                    .font(Theme.mono(12))
                                    .foregroundStyle(reasoning == level ? Theme.bg : Theme.textMuted)
                                    .padding(.horizontal, 12).frame(height: 32)
                                    .background(reasoning == level ? Theme.text : Theme.text.opacity(0.06), in: Capsule())
                            }
                            .buttonStyle(.plain)
                            .accessibilityIdentifier("reasoning-\(level)")
                        }
                    }.padding(.vertical, 2)
                }
            }
            if let catalogError {
                Text(catalogError).font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
            }
        }
    }

    private var messageSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionHeader(title: "First message", dotColor: nil, count: nil)
            TextField("Message \(harnessLabel)…", text: $draft, axis: .vertical)
                .font(Theme.sans(16))
                .lineLimit(4...8)
                .padding(14)
                .frame(minHeight: 96, alignment: .topLeading)
                .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).stroke(Theme.border, lineWidth: 1))
                .accessibilityIdentifier("first-message")
        }
    }

    private var startButton: some View {
        Button {
            busy = true; error = nil
            Task {
                do {
                    let chat = try await model.create(spaceID: space.isEmpty ? nil : space, harness: harness,
                                                      model: agentModel.isEmpty ? nil : agentModel,
                                                      reasoning: reasoning.isEmpty ? nil : reasoning)
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
                .font(Theme.sans(16, weight: .medium))
                .frame(maxWidth: .infinity, minHeight: 50)
                .background(Theme.text, in: Capsule()).foregroundStyle(Theme.bg)
                .opacity(busy || harness.isEmpty || !model.online ? 0.4 : 1)
        }
        .disabled(busy || harness.isEmpty || !model.online)
        .accessibilityIdentifier("start-session")
    }

    private func defaultLevel(_ levels: [String]) -> String {
        if levels.contains("high") { return "high" }
        if levels.contains("medium") { return "medium" }
        return levels.first ?? ""
    }
}
