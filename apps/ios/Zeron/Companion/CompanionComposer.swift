import PhotosUI
import SwiftUI
import UIKit

/// The companion session's composer pill (docs/design/mobile.md "Composer
/// pill"): attachment thumbnails over the editor, then the control row -
/// attach, the model/reasoning chip, the context ring, and the 36pt action
/// button that morphs from Send to Stop when an empty box faces a running turn.
struct CompanionComposer: View {
    let model: CompanionModel
    let chat: HostChat
    @Binding var draft: String
    let usage: ContextUsage?
    let state: SessionState
    /// Sends `text` with uploaded attachments; the caller owns the runway.
    let send: (_ text: String, _ attachments: [HostAttachment], _ messageID: String) async throws -> Void
    let stop: () async throws -> Void
    @Binding var error: String?
    @FocusState.Binding var focused: Bool

    @State private var staged: [StagedAttachment] = []
    @State private var pickerItems: [PhotosPickerItem] = []
    @State private var pickerPresented = false
    @State private var catalog: [HostAgentModel]?
    @State private var busy = false
    @State private var saving = false
    @State private var uploading: (done: Int, total: Int)?
    @State private var showContext = false

    private var harness: String { chat.config?.harness ?? "claude-code" }
    private var mark: BrandMark { .forHarness(harness) }
    private var harnessLabel: String {
        model.harnesses.first { $0.id == harness }?.label ?? HarnessCatalog.label(for: harness)
    }
    private var running: Bool { state == .working || state == .awaitingInput }

    private var models: [ModelInfo] {
        guard let catalog, !catalog.isEmpty else { return HarnessCatalog.models(for: harness) }
        return catalog.map { ModelInfo(id: $0.id, label: $0.label, description: nil, reasoningLevels: $0.reasoningLevels) }
    }
    private var currentModel: ModelInfo {
        HarnessCatalog.resolve(modelId: chat.config?.model, in: models, harness: harness)
    }
    private var currentReasoning: String? {
        guard !currentModel.reasoningLevels.isEmpty else { return nil }
        if let reasoning = chat.config?.reasoning, currentModel.reasoningLevels.contains(reasoning) { return reasoning }
        return HarnessCatalog.defaultReasoning(for: currentModel)
    }
    /// `pickers.rs chip_model_label`: only the last `/` segment survives.
    private var modelName: String {
        currentModel.label.components(separatedBy: "/").last ?? currentModel.label
    }

    private var placeholder: String {
        running ? "Queue a follow-up…" : "Message \(harnessLabel)…"
    }
    private var hasContent: Bool {
        !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || !staged.isEmpty
    }
    private var showsStop: Bool { running && !hasContent }
    private var canSend: Bool {
        guard model.online, !busy else { return false }
        return showsStop || hasContent
    }
    private var showsRing: Bool {
        guard let usage, let window = usage.window else { return false }
        return window > 0
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let uploading {
                Text("Uploading \(uploading.done + 1) of \(uploading.total)…")
                    .font(Theme.mono(11))
                    .foregroundStyle(Theme.textFaint)
                    .padding(.leading, 4)
            }
            pill
        }
        .photosPicker(isPresented: $pickerPresented, selection: $pickerItems,
                      maxSelectionCount: 6, matching: .images)
        .onChange(of: pickerItems) { _, items in stage(items) }
        .task(id: "\(harness)-\(model.generation)") {
            guard model.online else { return }
            if let loaded = try? await model.models(for: harness), !loaded.isEmpty { catalog = loaded }
        }
    }

    // MARK: Pill

    private var pill: some View {
        VStack(alignment: .leading, spacing: 6) {
            if !staged.isEmpty { attachmentRow }
            TextField(placeholder, text: $draft, axis: .vertical)
                .lineLimit(1...8)
                .font(Theme.sans(17))
                .foregroundStyle(Theme.text)
                .focused($focused)
                .accessibilityIdentifier("companion-composer")
                .onKeyPress(.return, phases: .down) { press in
                    guard press.modifiers.contains(.command), canSend else { return .ignored }
                    submit()
                    return .handled
                }
                .padding(.horizontal, 2)
                .padding(.top, 2)
            controlRow
        }
        .padding(.horizontal, 12)
        .padding(.top, 10)
        .padding(.bottom, 6)
        .background(Theme.wash(0.04), in: RoundedRectangle(cornerRadius: 26, style: .continuous))
        .nochesGlass(.regular.interactive(), in: RoundedRectangle(cornerRadius: 26, style: .continuous))
    }

    private var attachmentRow: some View {
        HStack(spacing: 8) {
            ForEach(staged) { item in
                attachmentThumb(item)
            }
        }
    }

    private func attachmentThumb(_ item: StagedAttachment) -> some View {
        Image(uiImage: item.image)
            .resizable()
            .scaledToFill()
            .frame(width: 56, height: 56)
            .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(alignment: .topTrailing) {
                Button { staged.removeAll { $0.id == item.id } } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 9, weight: .bold))
                        .foregroundStyle(Theme.bg)
                        .frame(width: 18, height: 18)
                        .background(Theme.text.opacity(0.85), in: Circle())
                        .frame(width: 40, height: 40, alignment: .topTrailing)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .offset(x: 6, y: -6)
                .accessibilityLabel("Remove attachment")
            }
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Attached image")
    }

    private var controlRow: some View {
        HStack(spacing: 6) {
            attachButton
            modelChip
            Spacer(minLength: 4)
            if showsRing {
                contextButton
            }
            actionButton
        }
    }

    private var attachButton: some View {
        Button { pickerPresented = true } label: {
            Image(systemName: "plus")
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(Theme.textMuted)
                .frame(width: 32, height: 32)
                .background(Theme.wash(0.06), in: Circle())
                .frame(width: 44, height: 44)
                .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .disabled(busy || !model.online || staged.count >= 6)
        .accessibilityLabel("Attach photos")
    }

    private var modelChip: some View {
        Menu {
            ForEach(models) { info in
                Button { choose(model: info) } label: {
                    if info.id == currentModel.id { Label(chipLabel(info.label), systemImage: "checkmark") }
                    else { Text(chipLabel(info.label)) }
                }
            }
            if !currentModel.reasoningLevels.isEmpty {
                Divider()
                Menu("Reasoning") {
                    ForEach(currentModel.reasoningLevels, id: \.self) { level in
                        Button { write(model: currentModel.id, reasoning: level) } label: {
                            if level == currentReasoning { Label(HarnessCatalog.reasoningLabel(level), systemImage: "checkmark") }
                            else { Text(HarnessCatalog.reasoningLabel(level)) }
                        }
                    }
                }
            }
        } label: {
            HStack(spacing: 6) {
                BrandMarkShape(mark: mark)
                    .fill(BrandMark.tint(for: harness), style: FillStyle(eoFill: mark.evenOddFill))
                    .frame(width: 12, height: 12)
                Text(modelName)
                    .font(Theme.sans(13))
                    .foregroundStyle(Theme.textMuted)
                    .lineLimit(1)
                if let reasoning = currentReasoning {
                    Text(HarnessCatalog.reasoningLabel(reasoning))
                        .font(Theme.sans(12))
                        .foregroundStyle(Theme.textFaint)
                        .lineLimit(1)
                }
            }
            .padding(.horizontal, 8)
            .frame(minHeight: 32)
            .contentShape(Rectangle())
        }
        .disabled(busy || saving || !model.online)
        .accessibilityLabel("Model: \(modelName)\(currentReasoning.map { ", reasoning \(HarnessCatalog.reasoningLabel($0))" } ?? "")")
    }

    private var contextButton: some View {
        Button { showContext = true } label: {
            HStack(spacing: 5) {
                ContextRing(usage: usage, size: 16)
                if let fraction = usage?.fraction {
                    Text("\(Int((fraction * 100).rounded()))%")
                        .font(Theme.mono(12))
                        .foregroundStyle(fraction >= 0.75 ? ContextFill(fraction: fraction).color : Theme.textFaint)
                        .monospacedDigit()
                }
            }
            .padding(.horizontal, 6)
            .frame(minWidth: 44, minHeight: 44)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Context window")
        .popover(isPresented: $showContext) {
            ContextCard(usage: usage)
                .frame(width: 236)
                .background(Theme.surfaceRaised)
                .presentationCompactAdaptation(.popover)
        }
    }

    private var actionButton: some View {
        Button {
            submit()
        } label: {
            Group {
                if busy {
                    ProgressView().controlSize(.small).tint(Theme.bg)
                } else if showsStop {
                    RoundedRectangle(cornerRadius: 2.5)
                        .fill(canSend ? Theme.bg : Theme.textFaint)
                        .frame(width: 12, height: 12)
                } else {
                    Image(systemName: "arrow.up")
                        .font(.system(size: 16, weight: .semibold))
                        .foregroundStyle(canSend ? Theme.bg : Theme.textFaint)
                }
            }
            .frame(width: 36, height: 36)
            .background(canSend ? AnyShapeStyle(Theme.text) : AnyShapeStyle(Theme.wash(0.10)), in: Circle())
            .frame(width: 44, height: 44)
            .contentShape(Circle())
        }
        .buttonStyle(.plain)
        .disabled(!canSend)
        .keyboardShortcut(.return, modifiers: .command)
        .motionAnimation(Motion.fadeQuick, value: showsStop)
        .accessibilityLabel(showsStop ? "Stop session" : "Send message")
    }

    // MARK: Actions

    private func submit() {
        guard model.online, !busy else { return }
        if showsStop {
            UIImpactFeedbackGenerator(style: .medium).impactOccurred()
            busy = true
            error = nil
            Task {
                do { try await stop() } catch { self.error = error.localizedDescription }
                busy = false
            }
            return
        }
        let raw = draft
        let text = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty || !staged.isEmpty else { return }
        let picked = staged
        let messageID = UUID().uuidString.lowercased()
        UIImpactFeedbackGenerator(style: .light).impactOccurred()
        busy = true
        error = nil
        Task {
            do {
                let attachments = try await upload(picked)
                try await send(text, attachments, messageID)
                if draft == raw { draft = "" }
                staged = []
            } catch {
                self.error = error.localizedDescription
            }
            uploading = nil
            busy = false
        }
    }

    private func upload(_ items: [StagedAttachment]) async throws -> [HostAttachment] {
        guard !items.isEmpty else { return [] }
        var out: [HostAttachment] = []
        for (index, item) in items.enumerated() {
            uploading = (index, items.count)
            out.append(try await model.uploadImage(data: item.data, name: item.name,
                                                   mimeType: mimeType(item.name)))
        }
        return out
    }

    private func stage(_ items: [PhotosPickerItem]) {
        guard !items.isEmpty else { return }
        Task {
            var failed = 0
            for item in items where staged.count < 6 {
                guard let data = try? await item.loadTransferable(type: Data.self),
                      let attachment = StagedAttachment.stage(data: data) else {
                    failed += 1
                    continue
                }
                staged.append(attachment)
            }
            pickerItems = []
            if failed > 0 {
                error = failed == 1
                    ? "One image couldn't be attached (unsupported or over 24 MB)."
                    : "\(failed) images couldn't be attached (unsupported or over 24 MB)."
            }
        }
    }

    private func choose(model info: ModelInfo) {
        var reasoning = currentReasoning
        if !info.reasoningLevels.isEmpty, !info.reasoningLevels.contains(reasoning ?? "") {
            reasoning = HarnessCatalog.defaultReasoning(for: info)
        }
        write(model: info.id, reasoning: info.reasoningLevels.isEmpty ? nil : reasoning)
    }

    private func write(model newModel: String?, reasoning: String?) {
        saving = true
        error = nil
        Task {
            do { try await model.setConfig(chat, model: newModel, reasoning: reasoning) }
            catch { self.error = error.localizedDescription }
            saving = false
        }
    }

    /// The stored first `/` segment goes to the host; the chip shows the last.
    private func chipLabel(_ label: String) -> String {
        label.components(separatedBy: "/").last ?? label
    }

    private func mimeType(_ name: String) -> String {
        switch (name as NSString).pathExtension.lowercased() {
        case "png": return "image/png"
        case "gif": return "image/gif"
        case "webp": return "image/webp"
        default: return "image/jpeg"
        }
    }
}
