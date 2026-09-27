import SwiftUI
import VisionKit

struct CompanionView: View {
    @Environment(AppModel.self) private var app
    @Environment(\.scenePhase) private var scenePhase
    @State private var model = CompanionModel()
    @State private var pairing = false
    @State private var settings = false
    @State private var newSession = false
    @State private var cloud = false
    @State private var path: [HostChat] = []
    @State private var initialProject = ""
    @State private var images: CompanionImageLoader?

    var body: some View {
        NavigationStack(path: $path) {
            Group {
                if model.selected != nil {
                    CompanionDashboard(model: model, newSession: { initialProject = $0; newSession = true }, pair: { pairing = true }, opened: { path.append($0) })
                } else {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 28) { heading; welcome }
                            .padding(24).frame(maxWidth: 620).frame(maxWidth: .infinity)
                    }
                }
            }
            .background(Theme.bg.ignoresSafeArea())
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    HStack(spacing: 9) {
                        Image("zeron-logo").resizable().scaledToFit().frame(width: 16, height: 19)
                        Text("noches").font(Theme.sans(20, weight: .semibold)).tracking(-0.7)
                    }.foregroundStyle(Theme.text).fixedSize()
                }.sharedBackgroundVisibility(.hidden)
                ToolbarItem(placement: .topBarTrailing) {
                    Button { settings = true } label: { Image(systemName: "ellipsis") }
                        .accessibilityLabel("Appearance and computers")
                }
            }
            .navigationDestination(for: HostChat.self) { chat in
                CompanionSessionView(model: model, chat: chat)
            }
            .sheet(isPresented: $pairing) { PairComputerSheet(model: model) }
            .sheet(isPresented: $newSession) {
                NewHostSessionSheet(model: model, initialProject: initialProject) { path.append($0) }
            }
            .sheet(isPresented: $cloud) { SignInView() }
            .sheet(isPresented: $settings) {
                NavigationStack {
                    List {
                        NavigationLink("Appearance") { AppearanceSettingsView() }
                        Section("Computers") {
                            ForEach(model.profiles) { host in
                                Button { model.select(host.id); settings = false } label: {
                                    HStack {
                                        Label(host.name, systemImage: "desktopcomputer")
                                        Spacer()
                                        if model.selectedID == host.id { Image(systemName: "checkmark") }
                                    }
                                }
                                .swipeActions {
                                    Button("Forget", role: .destructive) {
                                        do { try model.forget(host) } catch { model.error = error.localizedDescription }
                                    }
                                }
                            }
                            Button("Pair a computer") { settings = false; pairing = true }
                        }
                        Section {
                            Button("Connect a cloud account") { settings = false; cloud = true }
                            Button("Explore demo sessions") { settings = false; app.enterDemoMode() }
                        }
                        Text("Forgetting a computer removes its key from this phone. Revoke the key on the host to disable it everywhere.")
                            .font(Theme.sans(13)).foregroundStyle(Theme.textMuted)
                    }
                    .scrollContentBackground(.hidden).background(Theme.bg)
                    .navigationTitle("Settings")
                    .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { settings = false } } }
                }
            }
        }
        .tint(Theme.text)
        .environment(\.companionImageLoader, images)
        .onAppear { if images == nil { images = CompanionImageLoader(model: model) } }
        .task(id: "\(model.selectedID ?? "")-\(model.connectionRevision)-\(scenePhase == .active)") {
            guard scenePhase == .active else { model.connection.close(); model.online = false; return }
            await model.maintainConnection()
        }
        .onAppear {
            if ProcessInfo.processInfo.arguments.contains("-companion-settings") { settings = true }
        }
        .onChange(of: model.selectedID) { _, _ in path = [] }
    }

    private var heading: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(model.selected == nil ? "Your computer.\nWithin reach." : "Your computers")
                .font(Theme.sans(34, weight: .semibold)).tracking(-1.4)
                .foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
            Text(model.selected == nil ? "Pair your Mac or Linux host to pick up a session and put your agents to work." : "The work stays on your computer. You stay in control.")
                .font(Theme.sans(15)).foregroundStyle(Theme.textMuted).lineSpacing(4)
        }.padding(.top, 16)
    }

    private var welcome: some View {
        VStack(alignment: .leading, spacing: 24) {
            HStack(spacing: 20) {
                Image(systemName: "desktopcomputer").font(.system(size: 54, weight: .ultraLight))
                Image(systemName: "link").font(.system(size: 17)).foregroundStyle(Theme.accent)
                Image(systemName: "iphone").font(.system(size: 44, weight: .ultraLight))
            }.foregroundStyle(Theme.text).frame(maxWidth: .infinity).padding(.vertical, 28).accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 8) {
                Text("Start with a computer").font(Theme.sans(21, weight: .medium))
                Text("Connect to a running Noches instance. Your files, agent accounts, and sessions stay on the host.")
                    .font(Theme.sans(15)).foregroundStyle(Theme.textMuted).lineSpacing(4)
            }
            Button { pairing = true } label: {
                HStack { Text("Pair a computer"); Spacer(); Image(systemName: "arrow.right") }
                    .font(Theme.sans(16, weight: .medium)).padding(18)
                    .background(Theme.text, in: Capsule()).foregroundStyle(Theme.bg)
            }.accessibilityIdentifier("pair-computer")
            Text("macOS & Linux · Private connection").font(Theme.mono(11)).foregroundStyle(Theme.textFaint)
                .frame(maxWidth: .infinity)
            if let error = model.error { Text(error).font(Theme.sans(13)).foregroundStyle(Theme.danger) }
        }
        .foregroundStyle(Theme.text).padding(24).modifier(CompanionPanel())
    }

}

func statusLabel(_ status: String) -> String {
    switch status {
    case "working": return "Working"
    case "awaitingInput": return "Needs your input"
    case "errored": return "Needs attention"
    case "completed": return "Done"
    default: return "Ready"
    }
}
