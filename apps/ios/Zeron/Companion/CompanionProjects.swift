import SwiftUI

/// A folder picked in the browser, waiting for an optional name.
private struct PickedFolder: Hashable {
    let path: String
    let isRepo: Bool
}

/// Add a host folder as a project: browse, confirm (optional display name),
/// then `onAdded` with the new (or already existing) project.
struct CompanionAddProjectSheet: View {
    @Environment(\.dismiss) private var dismiss
    let model: CompanionModel
    let onAdded: (HostSpace) -> Void
    @State private var picked: PickedFolder?

    init(model: CompanionModel, onAdded: @escaping (HostSpace) -> Void) {
        self.model = model
        self.onAdded = onAdded
    }

    var body: some View {
        NavigationStack {
            CompanionFolderBrowser(model: model, title: "Add project") { path, isRepo in
                picked = PickedFolder(path: path, isRepo: isRepo)
            }
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } } }
            .navigationDestination(item: $picked) { folder in
                CompanionAddProjectConfirm(model: model, folder: folder) { space in
                    onAdded(space); dismiss()
                }
            }
        }
    }
}

private struct CompanionAddProjectConfirm: View {
    let model: CompanionModel
    let folder: PickedFolder
    let added: (HostSpace) -> Void
    @State private var name = ""
    @State private var busy = false
    @State private var error: String?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 22) {
                VStack(alignment: .leading, spacing: 10) {
                    SectionHeader(title: "Folder", dotColor: nil, count: nil)
                    HStack(spacing: 10) {
                        if folder.isRepo { LineIconView(.gitBranch, size: 14, color: Theme.textMuted) }
                        Text(folder.path).font(Theme.mono(12)).foregroundStyle(Theme.textMuted)
                            .textSelection(.enabled)
                    }
                }
                VStack(alignment: .leading, spacing: 10) {
                    SectionHeader(title: "Name", dotColor: nil, count: nil)
                    TextField((folder.path as NSString).lastPathComponent, text: $name)
                        .font(Theme.sans(16)).textInputAutocapitalization(.words)
                        .padding(.horizontal, 14).frame(minHeight: 48)
                        .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                        .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).stroke(Theme.border, lineWidth: 1))
                        .accessibilityIdentifier("project-name")
                    Text("Optional. Defaults to the folder name.").font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
                }
                Button {
                    busy = true; error = nil
                    Task {
                        do { added(try await model.addProject(path: folder.path, name: name, isRepo: folder.isRepo)) }
                        catch { self.error = error.localizedDescription }
                        busy = false
                    }
                } label: {
                    Text(busy ? "Adding…" : "Add project").font(Theme.sans(16, weight: .medium))
                        .frame(maxWidth: .infinity, minHeight: 50)
                        .background(Theme.text, in: Capsule()).foregroundStyle(Theme.bg)
                        .opacity(busy || !model.online ? 0.4 : 1)
                }
                .disabled(busy || !model.online)
                .accessibilityIdentifier("add-project")
                if let error {
                    Text(error).font(Theme.sans(13)).foregroundStyle(Theme.danger)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .padding(20).frame(maxWidth: 620).frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(Theme.bg).foregroundStyle(Theme.text)
        .scrollDismissesKeyboard(.interactively)
        .navigationTitle("Add project").navigationBarTitleDisplayMode(.inline)
    }
}

/// Rename or remove a project. Removal deletes the project's sessions on the host.
struct CompanionProjectSettingsSheet: View {
    @Environment(\.dismiss) private var dismiss
    let model: CompanionModel
    let space: HostSpace
    @State private var name: String
    @State private var busy = false
    @State private var confirmRemove = false
    @State private var error: String?

    init(model: CompanionModel, space: HostSpace) {
        self.model = model
        self.space = space
        _name = State(initialValue: space.name ?? "")
    }

    private var sessionCount: Int { model.chats.filter { $0.spaceId == space.id }.count }
    private var changed: Bool { name.trimmingCharacters(in: .whitespacesAndNewlines) != (space.name ?? "") }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 26) {
                    VStack(alignment: .leading, spacing: 10) {
                        SectionHeader(title: "Name", dotColor: nil, count: nil)
                        HStack(spacing: 8) {
                            TextField(space.displayName, text: $name)
                                .font(Theme.sans(16)).textInputAutocapitalization(.words)
                                .submitLabel(.done).onSubmit(save)
                            if changed {
                                Button("Save", action: save).font(Theme.sans(15, weight: .medium))
                                    .frame(minHeight: 44).disabled(busy || !model.online)
                            }
                        }
                        .padding(.horizontal, 14).frame(minHeight: 48)
                        .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                        .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).stroke(Theme.border, lineWidth: 1))
                        Text("Leave empty to use the folder name.").font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
                    }
                    VStack(alignment: .leading, spacing: 10) {
                        SectionHeader(title: "Location", dotColor: nil, count: nil)
                        VStack(alignment: .leading, spacing: 6) {
                            Text(space.path).font(Theme.mono(12)).foregroundStyle(Theme.textMuted)
                            HStack(spacing: 6) {
                                Image(systemName: "desktopcomputer").font(.system(size: 11))
                                Text(model.selected?.name ?? space.deviceId).font(Theme.mono(12))
                            }.foregroundStyle(Theme.textFaint)
                        }.textSelection(.enabled)
                    }
                    VStack(alignment: .leading, spacing: 10) {
                        Button(role: .destructive) { confirmRemove = true } label: {
                            Text("Remove project").font(Theme.sans(16, weight: .medium))
                                .frame(maxWidth: .infinity, minHeight: 50)
                                .overlay(Capsule().stroke(Theme.danger.opacity(0.5), lineWidth: 1))
                                .foregroundStyle(Theme.danger)
                        }
                        .disabled(busy || !model.online)
                        .accessibilityIdentifier("remove-project")
                        Text("Removing deletes this project and its sessions on the host. The folder and its files stay untouched.")
                            .font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    if let error {
                        Text(error).font(Theme.sans(13)).foregroundStyle(Theme.danger)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
                .padding(20).frame(maxWidth: 620).frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(Theme.bg).foregroundStyle(Theme.text)
            .scrollDismissesKeyboard(.interactively)
            .navigationTitle(space.displayName).navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
            .confirmationDialog("Remove \(space.displayName)?", isPresented: $confirmRemove, titleVisibility: .visible) {
                Button("Remove project and \(sessionCount) session\(sessionCount == 1 ? "" : "s")", role: .destructive, action: remove)
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("This deletes the project's sessions on the host. Files in the folder are not deleted.")
            }
        }
    }

    private func save() {
        guard changed else { return }
        busy = true; error = nil
        Task {
            do { try await model.renameProject(space, name: name); dismiss() }
            catch { self.error = error.localizedDescription }
            busy = false
        }
    }

    private func remove() {
        busy = true; error = nil
        Task {
            do { try await model.removeProject(space); dismiss() }
            catch { self.error = error.localizedDescription }
            busy = false
        }
    }
}
