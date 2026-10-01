import SwiftUI

/// The session screen's single header menu. Workspace destinations (changes,
/// files, terminal) and the host project's actions come first, then the
/// session itself (rename, archive). One glass target instead of a workspace
/// pill plus an overflow pill.
struct CompanionSessionActions: View {
    let model: CompanionModel
    let chat: HostChat
    let projectActions: [HostProjectAction]
    let runningAction: String?
    let open: (CompanionInspector) -> Void
    let run: (HostProjectAction) -> Void
    var archived: () -> Void = {}

    @State private var renaming = false
    @State private var title = ""
    @State private var error: String?
    @State private var busy = false

    var body: some View {
        Menu {
            Section("Workspace") {
                Button("Review changes", systemImage: "arrow.triangle.branch") { open(.changes) }
                Button("Browse files", systemImage: "folder") { open(.files) }
                Button("Terminal", systemImage: "terminal") { open(.terminal) }
            }
            if !projectActions.isEmpty {
                Section("Project actions") {
                    ForEach(projectActions) { action in
                        Button("Run: \(action.name)", systemImage: "play") { run(action) }
                            .disabled(runningAction != nil)
                    }
                }
            }
            Section("Session") {
                Button("Rename session", systemImage: "pencil") { title = chat.displayTitle; renaming = true }
                Button(chat.archived ? "Restore session" : "Archive session",
                       systemImage: chat.archived ? "tray.and.arrow.up" : "archivebox") {
                    perform { try await model.archive(chat, archived: !chat.archived); archived() }
                }
            }
        } label: {
            Image(systemName: "ellipsis")
                .font(.system(size: 16))
                .frame(width: 44, height: 44)
                .contentShape(Rectangle())
        }
        .disabled(!model.online || busy)
        .accessibilityLabel("Session actions for \(chat.displayTitle)")
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
