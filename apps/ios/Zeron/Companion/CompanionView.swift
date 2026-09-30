import SwiftUI
import VisionKit

struct CompanionView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.scenePhase) private var scenePhase
    @State private var model = CompanionModel()
    @State private var pairing = false
    @State private var settings = false
    /// Presented once the settings sheet finishes dismissing; swapping sheets in one tap races.
    @State private var afterSettings: (() -> Void)?
    @State private var newSession: NewSessionRequest?
    @State private var cloud = false
    @State private var path: [HostChat] = []
    @State private var images: CompanionImageLoader?

    @Environment(\.horizontalSizeClass) private var sizeClass
    @State private var columns = NavigationSplitViewVisibility.all

    /// Regular width (iPad) puts the project/thread list beside the session.
    private var split: Bool { sizeClass == .regular && model.selected != nil }

    private var dashboard: some View {
        CompanionDashboard(model: model, newSession: { newSession = NewSessionRequest(project: $0) },
                           pair: { pairing = true },
                           opened: { split ? (path = [$0]) : path.append($0) },
                           selectedChatID: split ? path.last?.id : nil)
    }

    @ToolbarContentBuilder
    private var homeToolbar: some ToolbarContent {
        ToolbarItem(placement: .topBarLeading) {
            NochesWordmark()
        }.sharedBackgroundVisibility(.hidden)
        ToolbarItem(placement: .topBarTrailing) {
            Button { settings = true } label: { Image(systemName: "ellipsis") }
                .accessibilityLabel("Appearance and computers")
        }
    }

    @ViewBuilder
    private var layout: some View {
        if split {
            NavigationSplitView(columnVisibility: $columns) {
                dashboard
                    .background(Theme.bg.ignoresSafeArea())
                    .navigationBarTitleDisplayMode(.inline)
                    .toolbar { homeToolbar }
                    .navigationSplitViewColumnWidth(min: 340, ideal: 400, max: 460)
            } detail: {
                NavigationStack {
                    if let chat = path.last {
                        CompanionSessionView(model: model, chat: chat).id(chat.id)
                    } else {
                        VStack(spacing: 12) {
                            NochesMark(size: 26).opacity(0.5)
                            Text("Select a session").font(Theme.sans(20, weight: .medium)).tracking(-0.4)
                            Text("Or start a new one from the list.")
                                .font(Theme.sans(15)).foregroundStyle(Theme.textMuted)
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity).background(Theme.bg.ignoresSafeArea())
                    }
                }
            }
            .navigationSplitViewStyle(.balanced)
        } else {
            NavigationStack(path: $path) {
                Group {
                    if model.selected != nil {
                        dashboard
                    } else {
                        ScrollView {
                            VStack(alignment: .leading, spacing: 0) { welcome }
                                .padding(24).frame(maxWidth: 620).frame(maxWidth: .infinity)
                        }
                    }
                }
                .background(Theme.bg.ignoresSafeArea())
                .navigationBarTitleDisplayMode(.inline)
                .toolbar { homeToolbar }
                .navigationDestination(for: HostChat.self) { chat in
                    CompanionSessionView(model: model, chat: chat)
                }
            }
        }
    }

    var body: some View {
        layout
            .sheet(isPresented: $pairing) { PairComputerSheet(model: model) }
            .sheet(item: $newSession) { request in
                NewHostSessionSheet(model: model, initialProject: request.project) { path = split ? [$0] : path + [$0] }
            }
            .sheet(isPresented: $cloud) { SignInView() }
            .sheet(isPresented: $settings, onDismiss: { afterSettings?(); afterSettings = nil }) { settingsSheet }
        .tint(Theme.text)
        .environment(\.companionImageLoader, images)
        .onAppear { if images == nil { images = CompanionImageLoader(model: model) } }
        .task(id: "\(model.selectedID ?? "")-\(model.connectionRevision)-\(scenePhase == .active)") {
            guard scenePhase == .active else { model.connection.close(); model.online = false; return }
            await model.maintainConnection()
        }
        .onAppear {
            let arguments = ProcessInfo.processInfo.arguments
            if arguments.contains("-companion-settings") { settings = true }
            #if DEBUG
            // Simulator rigs (scripts/companion-rig.sh): pair from a launch argument.
            if let index = arguments.firstIndex(of: "-companion-pair"), arguments.indices.contains(index + 1) {
                try? model.pair(arguments[index + 1])
            }
            #endif
        }
        .onChange(of: model.chats.count) { _, _ in
            #if DEBUG
            let arguments = ProcessInfo.processInfo.arguments
            if path.isEmpty, let index = arguments.firstIndex(of: "-companion-open"), arguments.indices.contains(index + 1),
               let chat = model.chats.first(where: { $0.id == arguments[index + 1] }) { path.append(chat) }
            #endif
        }
        .onChange(of: model.selectedID) { _, _ in path = [] }
    }

    // MARK: - Settings

    private var settingsSheet: some View {
        NavigationStack {
            List {
                Section("Computers") {
                    ForEach(model.profiles) { host in
                        Button { model.select(host.id); settings = false } label: {
                            HStack(spacing: 12) {
                                Image(systemName: "desktopcomputer")
                                    .font(.system(size: 15)).foregroundStyle(Theme.textMuted)
                                    .frame(width: 22)
                                Text(host.name).font(Theme.sans(15)).foregroundStyle(Theme.text).lineLimit(1)
                                Spacer(minLength: 8)
                                if model.selectedID == host.id {
                                    Circle()
                                        .fill(model.online ? Theme.statusCompleted : Theme.warning)
                                        .frame(width: 6, height: 6)
                                    Image(systemName: "checkmark")
                                        .font(.system(size: 12, weight: .semibold))
                                        .foregroundStyle(Theme.textMuted)
                                }
                            }
                            .frame(minHeight: 44)
                        }
                        .buttonStyle(.plain)
                        .swipeActions {
                            Button("Forget", role: .destructive) {
                                do { try model.forget(host) } catch { model.error = error.localizedDescription }
                            }
                        }
                    }
                    Button { afterSettings = { pairing = true }; settings = false } label: {
                        Label("Pair a computer", systemImage: "plus")
                            .font(Theme.sans(15)).foregroundStyle(Theme.text)
                            .frame(minHeight: 44)
                    }
                }
                Section {
                    NavigationLink("Appearance") { AppearanceSettingsView() }
                }
                Section {
                    Button { afterSettings = { cloud = true }; settings = false } label: {
                        Label("Connect a cloud account", systemImage: "cloud")
                            .font(Theme.sans(15)).foregroundStyle(Theme.textMuted)
                    }
                    Button { settings = false; app.enterDemoMode() } label: {
                        Label("Explore demo sessions", systemImage: "sparkles")
                            .font(Theme.sans(15)).foregroundStyle(Theme.textMuted)
                    }
                } footer: {
                    Text("Forgetting a computer removes its key from this phone. Revoke the key on the host to disable it everywhere.")
                }
            }
            .scrollContentBackground(.hidden).background(Theme.bg)
            .navigationTitle("Settings")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { settings = false } } }
        }
    }

    // MARK: - Signed out

    private var welcome: some View {
        VStack(alignment: .leading, spacing: 22) {
            NochesNightHero().frame(height: 260).padding(.top, 4)
            VStack(alignment: .leading, spacing: 12) {
                Eyebrow(text: "Companion")
                Text("Your agents,\nwithin reach.")
                    .font(Theme.sans(34, weight: .medium)).tracking(-1.2).lineSpacing(0)
                    .fixedSize(horizontal: false, vertical: true)
                Text("Pair a Mac or Linux host running Noches. Files, agent accounts, and sessions stay on the host; this phone steers.")
                    .font(Theme.sans(15)).foregroundStyle(Theme.textMuted).lineSpacing(4)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Button { pairing = true } label: {
                HStack { Text("Pair a computer"); Spacer(); Image(systemName: "arrow.right") }
                    .font(Theme.sans(16, weight: .medium)).padding(.horizontal, 22).frame(minHeight: 54)
                    .background(Theme.text, in: Capsule()).foregroundStyle(Theme.bg)
            }.accessibilityIdentifier("pair-computer").padding(.top, 6)
            Text("macOS & Linux · Private over Tailscale").font(Theme.mono(11)).foregroundStyle(Theme.textFaint)
                .frame(maxWidth: .infinity)
            if let error = model.error { Text(error).font(Theme.sans(13)).foregroundStyle(Theme.danger) }
        }
        .foregroundStyle(Theme.text)
    }
}

/// The compose action's request: carrying the filtered project in an item
/// avoids the stale-state race of setting a project and a boolean together.
struct NewSessionRequest: Identifiable {
    let id = UUID()
    let project: String
}
