import SwiftUI
import VisionKit

/// Pairing: one headline, the scan and paste inputs, and the host checklist.
/// Connect is pinned to the bottom so it is always in reach. Behavior is
/// unchanged: only layout and copy.
struct PairComputerSheet: View {
    @Environment(\.dismiss) private var dismiss
    let model: CompanionModel
    @State private var code = ""
    @State private var scanning = false
    @State private var error: String?
    private var trimmedCode: String { code.trimmingCharacters(in: .whitespacesAndNewlines) }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 28) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Bring your computer along.")
                            .font(Theme.sans(24, weight: .semibold)).tracking(-0.6)
                        Text("On your computer, open Noches, go to Settings → Connections, and choose Pair a phone. Scan the code or paste it below.")
                            .font(Theme.sans(15)).foregroundStyle(Theme.textMuted).lineSpacing(3)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .padding(.horizontal, SheetMetrics.margin)

                    VStack(alignment: .leading, spacing: 10) {
                        SheetSectionLabel("Connection code").padding(.horizontal, SheetMetrics.margin)
                        VStack(spacing: 10) {
                            if DataScannerViewController.isSupported {
                                Button { scanning = true } label: {
                                    Label("Scan connection code", systemImage: "qrcode.viewfinder")
                                        .font(Theme.sans(15, weight: .medium)).foregroundStyle(Theme.text)
                                        .frame(maxWidth: .infinity, minHeight: 48)
                                        .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous)
                                            .stroke(Theme.border, lineWidth: 1))
                                        .contentShape(Rectangle())
                                }
                                .buttonStyle(.plain)
                            }
                            SecureField("noches-connect:…", text: $code)
                                .textInputAutocapitalization(.never).autocorrectionDisabled()
                                .font(Theme.mono(13)).padding(.horizontal, 14).frame(minHeight: 48)
                                .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous)
                                    .stroke(Theme.border, lineWidth: 1))
                                .accessibilityIdentifier("connection-code")
                            Label("The code grants access to this computer. It is stored in your phone's Keychain.", systemImage: "lock")
                                .font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
                                .labelStyle(SheetAlignedLabelStyle())
                                .fixedSize(horizontal: false, vertical: true)
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                        .padding(.horizontal, SheetMetrics.margin)
                    }

                    SheetGroup(title: "Before you connect", separatorInset: SheetMetrics.margin + 24 + 12) {
                        checklistRow("network", "Connect Tailscale on both devices")
                        checklistRow("desktopcomputer", "Keep Noches running on your computer")
                        checklistRow("antenna.radiowaves.left.and.right", "Connect from Wi-Fi or cellular")
                    }
                    Text("The phone cannot wake or start a stopped host.")
                        .font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
                        .padding(.horizontal, SheetMetrics.margin).padding(.top, -20)
                }
                .padding(.vertical, 16)
                .frame(maxWidth: 640).frame(maxWidth: .infinity)
            }
            .scrollDismissesKeyboard(.interactively)
            .background(Theme.bg).foregroundStyle(Theme.text)
            .safeAreaInset(edge: .bottom, spacing: 0) {
                SheetPinnedBar {
                    if let error {
                        Text(error).font(Theme.sans(13)).foregroundStyle(Theme.danger)
                            .fixedSize(horizontal: false, vertical: true)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    Button {
                        do { try model.pair(code); code = ""; dismiss() } catch { self.error = error.localizedDescription }
                    } label: {
                        Text("Connect to computer")
                    }
                    .buttonStyle(SheetPrimaryButtonStyle(enabled: !trimmedCode.isEmpty))
                    .disabled(trimmedCode.isEmpty)
                }
            }
            .sheet(isPresented: $scanning) {
                NavigationStack {
                    CompanionScanner(scanned: { code = $0; scanning = false }, failed: { error = $0; scanning = false })
                        .ignoresSafeArea(edges: .bottom)
                        .overlay(alignment: .bottom) {
                            Text("Point the camera at the code in Noches → Settings → Connections.")
                                .font(Theme.sans(13)).foregroundStyle(.white)
                                .multilineTextAlignment(.center)
                                .padding(.horizontal, 16).padding(.vertical, 10)
                                .background(.black.opacity(0.55), in: Capsule())
                                .padding(.bottom, 32).padding(.horizontal, 24)
                        }
                        .navigationTitle("Scan connection code").navigationBarTitleDisplayMode(.inline)
                        .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { scanning = false } } }
                }
            }
            .navigationTitle("Pair a computer").navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } } }
        }
    }

    private func checklistRow(_ icon: String, _ text: String) -> some View {
        HStack(spacing: 12) {
            Image(systemName: icon).font(.system(size: 15)).foregroundStyle(Theme.textMuted).frame(width: 24)
            Text(text).font(Theme.sans(15)).foregroundStyle(Theme.text)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, SheetMetrics.margin).frame(minHeight: 44)
        .accessibilityElement(children: .combine)
    }
}

/// Icon on a fixed leading column, text top-aligned beside it.
private struct SheetAlignedLabelStyle: LabelStyle {
    func makeBody(configuration: Configuration) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            configuration.icon
            configuration.title
        }
    }
}
