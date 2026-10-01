import SwiftUI

/// The projects drawer: every project on the host with its thread count and
/// state dots. Tapping a row filters Home to it; the context menu opens the
/// project's settings; the last row adds a project by browsing the host.
struct CompanionProjectList: View {
    let model: CompanionModel
    @Binding var selection: String
    /// Starts a new session in a project ("" is no project). The caller
    /// dismisses this drawer first and presents the sheet afterwards.
    let newSession: (String) -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var settingsSpace: HostSpace?
    @State private var adding = false

    private var groups: [CompanionProjectGroup] { model.projectGroups(chats: model.localChats, keepEmpty: true) }

    var body: some View {
        NavigationStack {
            List {
                allRow
                ForEach(groups) { group in row(group) }
                Button { adding = true } label: {
                    HStack(spacing: 12) {
                        Image(systemName: "plus").font(.system(size: 14, weight: .medium))
                            .foregroundStyle(Theme.textMuted).frame(width: 22)
                        Text("Add project…").font(Theme.sans(16)).foregroundStyle(Theme.text)
                        Spacer(minLength: 0)
                    }
                    .frame(minHeight: 48).contentShape(Rectangle())
                }
                .buttonStyle(.plain).disabled(!model.online)
                .accessibilityIdentifier("add-project")
                .listRowBackground(Color.clear)
            }
            .listStyle(.plain).scrollContentBackground(.hidden).background(Theme.bg)
            .navigationTitle("Projects").navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
            .sheet(item: $settingsSpace) { space in CompanionProjectSettingsSheet(model: model, space: space) }
            .sheet(isPresented: $adding) { CompanionAddProjectSheet(model: model) { _ in adding = false } }
        }
        .tint(Theme.text)
        .presentationDetents([.medium, .large]).presentationDragIndicator(.visible)
    }

    private var allRow: some View {
        Button { selection = ""; dismiss() } label: {
            HStack(spacing: 12) {
                LineIconView(.folder, size: 15, color: Theme.textMuted).frame(width: 22)
                Text("All projects").font(Theme.sans(16)).foregroundStyle(Theme.text)
                Spacer(minLength: 8)
                Text("\(model.localChats.count)").font(Theme.mono(12)).foregroundStyle(Theme.textFaint)
                check(selection.isEmpty)
            }
            .frame(minHeight: 48).contentShape(Rectangle())
        }
        .buttonStyle(.plain).accessibilityIdentifier("project-all")
        .listRowBackground(Color.clear)
    }

    private func row(_ group: CompanionProjectGroup) -> some View {
        Button { selection = group.filterID; dismiss() } label: {
            HStack(spacing: 12) {
                CompanionProjectBadge(model: model, space: group.space, name: group.name,
                                      seed: group.space?.path ?? "home", size: 22)
                VStack(alignment: .leading, spacing: 2) {
                    Text(group.name).font(Theme.sans(16)).foregroundStyle(Theme.text).lineLimit(1)
                    Text(group.hint).font(Theme.mono(12)).foregroundStyle(Theme.textFaint)
                        .lineLimit(1).truncationMode(.head)
                }
                Spacer(minLength: 8)
                CompanionStateDots(needsYou: group.needsYou, running: group.running)
                Text("\(group.chats.count)").font(Theme.mono(12)).foregroundStyle(Theme.textFaint)
                check(selection == group.filterID)
            }
            .frame(minHeight: 52).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("project-\(group.id)")
        .accessibilityLabel("\(group.name), \(group.chats.count) sessions")
        .listRowBackground(Color.clear)
        .contextMenu {
            Button("New session", systemImage: "square.and.pencil") { newSession(group.projectID) }
                .disabled(!model.online)
            if let space = group.space {
                Button("Project settings…", systemImage: "gearshape") { settingsSpace = space }
            }
        }
    }

    private func check(_ on: Bool) -> some View {
        Image(systemName: "checkmark").font(.system(size: 12, weight: .semibold))
            .foregroundStyle(Theme.textMuted).opacity(on ? 1 : 0).frame(width: 14)
    }
}

/// Indigo (needs you) and sky (running) dots: the only color a project row
/// carries besides its badge.
struct CompanionStateDots: View {
    let needsYou: Int
    let running: Int
    var body: some View {
        HStack(spacing: 5) {
            if needsYou > 0 { Circle().fill(SessionState.awaitingInput.color ?? Theme.textMuted).frame(width: 6, height: 6) }
            if running > 0 { Circle().fill(SessionState.working.color ?? Theme.textMuted).frame(width: 6, height: 6) }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel([needsYou > 0 ? "\(needsYou) need you" : nil, running > 0 ? "\(running) running" : nil]
            .compactMap { $0 }.joined(separator: ", "))
    }
}
