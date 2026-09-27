import SwiftUI

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
    @State private var busy = false
    @State private var error: String?
    var body: some View {
        NavigationStack {
            Form {
                Section("Run on") { Label(model.selected?.name ?? "Computer", systemImage: "desktopcomputer") }
                Section {
                    Picker("Project", selection: $space) {
                        Text("No project · home folder").tag("")
                        ForEach(model.localSpaces) { Text($0.displayName).tag($0.id) }
                    }
                    Picker("Agent", selection: $harness) {
                        ForEach(model.harnesses) { Text($0.label).tag($0.id) }
                    }
                }
                Section("Model") {
                    Picker("Model", selection: $agentModel) {
                        Text("Agent default").tag("")
                        ForEach(models) { Text($0.label).tag($0.id) }
                    }.disabled(loadingModels)
                    if let chosen = models.first(where: { $0.id == agentModel }), !chosen.reasoningLevels.isEmpty {
                        Picker("Reasoning", selection: $reasoning) {
                            Text("Default").tag("")
                            ForEach(chosen.reasoningLevels, id: \.self) { Text($0.capitalized).tag($0) }
                        }
                    }
                    if loadingModels { ProgressView("Loading models from your computer…") }
                    if let catalogError { Text(catalogError).font(Theme.sans(12)).foregroundStyle(Theme.textMuted) }
                }
                Section {
                    Button(busy ? "Creating…" : "Create session") {
                        busy = true
                        Task {
                            do {
                                let chat = try await model.create(spaceID: space.isEmpty ? nil : space, harness: harness, model: agentModel.isEmpty ? nil : agentModel, reasoning: reasoning.isEmpty ? nil : reasoning)
                                dismiss(); opened(chat)
                            } catch { self.error = error.localizedDescription }
                            busy = false
                        }
                    }.disabled(busy || harness.isEmpty || !model.online)
                    if let error { Text(error).foregroundStyle(Theme.danger) }
                } footer: { Text("New sessions allow workspace writes and ask for approval when required.") }
            }.scrollContentBackground(.hidden).background(Theme.bg)
                .navigationTitle("New session").navigationBarTitleDisplayMode(.inline)
                .onAppear { harness = model.harnesses.first?.id ?? ""; space = initialProject }
                .task(id: harness) {
                    agentModel = ""; reasoning = ""; models = []; catalogError = nil
                    guard !harness.isEmpty else { return }
                    loadingModels = true
                    do {
                        let catalog = try await model.models(for: harness)
                        guard !Task.isCancelled else { return }
                        models = catalog
                    } catch {
                        guard !Task.isCancelled else { return }
                        catalogError = "Couldn't load models. You can still use the agent's default."
                    }
                    loadingModels = false
                }
                .onChange(of: agentModel) { _, _ in reasoning = "" }
                .onChange(of: model.harnesses.map(\.id)) { _, ids in if harness.isEmpty { harness = ids.first ?? "" } }
                .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } } }
        }
    }
}
