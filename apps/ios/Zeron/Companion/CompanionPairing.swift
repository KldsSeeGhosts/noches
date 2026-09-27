import SwiftUI
import VisionKit

/// Pairing, in the control-plane language: a link hero, the connection code,
/// and the host checklist. Behavior is unchanged - this is spacing and type.
struct PairComputerSheet: View {
    @Environment(\.dismiss) private var dismiss
    let model: CompanionModel
    @State private var code = ""
    @State private var scanning = false
    @State private var error: String?
    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    Image(systemName: "link").font(.system(size: 32, weight: .light)).foregroundStyle(Theme.accent)
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Bring your computer along.").font(Theme.sans(26, weight: .semibold)).tracking(-0.7)
                        Text("Paste the private connection code created by the Noches connection gateway on your Mac or Linux host.")
                            .font(Theme.sans(15)).foregroundStyle(Theme.textMuted).lineSpacing(3)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    if DataScannerViewController.isSupported {
                        Button { scanning = true } label: {
                            Label("Scan connection code", systemImage: "qrcode.viewfinder")
                                .font(Theme.sans(15, weight: .medium)).foregroundStyle(Theme.text)
                                .frame(maxWidth: .infinity, minHeight: 50)
                                .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                                .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).stroke(Theme.border, lineWidth: 1))
                        }
                    }
                    SecureField("noches-connect:…", text: $code)
                        .textInputAutocapitalization(.never).autocorrectionDisabled()
                        .font(Theme.mono(13)).padding(.horizontal, 16).frame(minHeight: 50)
                        .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                        .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous).stroke(Theme.border, lineWidth: 1))
                        .accessibilityIdentifier("connection-code")
                    Button {
                        do { try model.pair(code); code = ""; dismiss() } catch { self.error = error.localizedDescription }
                    } label: {
                        Text("Connect to computer").font(Theme.sans(16, weight: .medium))
                            .frame(maxWidth: .infinity, minHeight: 50)
                            .background(Theme.text, in: Capsule()).foregroundStyle(Theme.bg)
                            .opacity(code.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? 0.4 : 1)
                    }.disabled(code.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    if let error { Text(error).foregroundStyle(Theme.danger).font(Theme.sans(13)) }
                    Label("The code grants access to this computer. It is stored securely in your phone's Keychain.", systemImage: "lock")
                        .font(Theme.sans(13)).foregroundStyle(Theme.textMuted)
                        .fixedSize(horizontal: false, vertical: true)
                    VStack(alignment: .leading, spacing: 10) {
                        Label("Connect Tailscale on both devices", systemImage: "network")
                        Label("Keep Noches running on your computer", systemImage: "desktopcomputer")
                        Label("Connect from Wi-Fi or cellular", systemImage: "antenna.radiowaves.left.and.right")
                    }.font(Theme.sans(13)).foregroundStyle(Theme.textMuted).padding(.vertical, 6)
                    DisclosureGroup("Set up your host") {
                        Text("Run Noches on your computer, then use noches-connect pair to create a phone key and noches-connect serve to enable the connection. Use your host's Tailscale IP address and keep Tailscale connected on both devices. The phone cannot wake or start a stopped host service.")
                            .font(Theme.sans(14)).foregroundStyle(Theme.textMuted).padding(.top, 10)
                            .fixedSize(horizontal: false, vertical: true)
                    }.font(Theme.sans(14))
                }.padding(24).frame(maxWidth: 640)
            }.background(Theme.bg).foregroundStyle(Theme.text)
                .sheet(isPresented: $scanning) {
                    NavigationStack {
                        CompanionScanner(scanned: { code = $0; scanning = false }, failed: { error = $0; scanning = false })
                            .ignoresSafeArea(edges: .bottom)
                            .navigationTitle("Scan connection code").navigationBarTitleDisplayMode(.inline)
                            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { scanning = false } } }
                    }
                }
                .navigationTitle("Pair a computer").navigationBarTitleDisplayMode(.inline)
                .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } } }
        }
    }
}
