import SwiftUI

// MARK: - Host transcript model
//
// The relay's JSON shape of `crates/doc/src/parts.rs` / `transcript_delta.rs`.
// These live with the companion transcript adapter (not CompanionModel) so the
// rendering rules and the wire types they consume stay in one file.

/// Per-file diff stats (`parts.rs` `ToolDiffStat`).
struct HostDiffStat: Codable, Equatable {
    var path: String
    var additions: UInt64
    var deletions: UInt64
}

struct HostPart: Codable, Identifiable, Equatable {
    var id: String
    var kind: String
    var text: String? = nil
    var message: String? = nil
    var call: JSONValue? = nil
    var isError: Bool? = nil
    var requestId: String? = nil
    var questions: [UserInputQuestion]? = nil
    /// True once a ToolResult arrived (`parts.rs` Tool.resolved).
    var resolved: Bool? = nil
    /// One-line tool output summary.
    var output: String? = nil
    var outputBytes: UInt64? = nil
    var diffStats: [HostDiffStat]? = nil
    /// The SUBAGENT doc this spawn chip indexes.
    var subagentRef: String? = nil
    /// "running" / "done" / "failed" (`parts.rs` SubagentStatus).
    var subagentStatus: String? = nil
    var subagentTail: String? = nil
    /// Generated image fields.
    var path: String? = nil
    var name: String? = nil
    var mimeType: String? = nil
}

struct HostMessage: Codable, Identifiable, Equatable {
    var id: String
    var role: String
    var status: String?
    var parts: [HostPart]
    var createdAt: Int64? = nil
    var deviceId: String? = nil
}

struct HostTranscriptFrame: Decodable {
    struct Upsert: Decodable { let after: String?; let entry: HostMessage }
    struct Append: Decodable { let entry: String; let part: String; let text: String; let len: Int }
    var reset: [HostMessage]?
    var upsert: [Upsert]?
    var append: [Append]?
    var remove: [String]?
    var count: Int?
    /// Host-owned context snapshot (`transcript_delta.rs` TranscriptUpdate).
    /// It may ride a frame with no row changes at all.
    var contextUsage: ContextUsage?
    /// True when the frame carried `contextUsage` at all. The host sends the
    /// key on every frame it emits, so a null clears the snapshot; an older
    /// host omits the key, which leaves the last snapshot alone
    /// (`TranscriptUpdate`'s `#[serde(default)]`).
    var carriesContextUsage = false

    private enum CodingKeys: String, CodingKey {
        case reset, upsert, append, remove, count, contextUsage
    }

    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        reset = try values.decodeIfPresent([HostMessage].self, forKey: .reset)
        upsert = try values.decodeIfPresent([Upsert].self, forKey: .upsert)
        append = try values.decodeIfPresent([Append].self, forKey: .append)
        remove = try values.decodeIfPresent([String].self, forKey: .remove)
        count = try values.decodeIfPresent(Int.self, forKey: .count)
        carriesContextUsage = values.contains(.contextUsage)
        contextUsage = try values.decodeIfPresent(ContextUsage.self, forKey: .contextUsage)
    }

    struct Update {
        var messages: [HostMessage]
        var contextUsage: ContextUsage?
        /// True when the frame carried the `contextUsage` key at all.
        var carriesContextUsage: Bool
    }

    func applying(to source: [HostMessage]) throws -> [HostMessage] {
        try apply(to: source).messages
    }

    /// Apply the row changes and carry the frame's context snapshot out.
    /// `count` is a desync tripwire only when the host sent it.
    func apply(to source: [HostMessage]) throws -> Update {
        if let reset { return Update(messages: reset, contextUsage: contextUsage, carriesContextUsage: carriesContextUsage) }
        var rows = source
        rows.removeAll { (remove ?? []).contains($0.id) }
        for change in upsert ?? [] {
            rows.removeAll { $0.id == change.entry.id }
            if let anchor = change.after {
                guard let index = rows.firstIndex(where: { $0.id == anchor }) else { throw RelayError.rpc("Transcript needs a refresh.") }
                rows.insert(change.entry, at: index + 1)
            } else { rows.insert(change.entry, at: 0) }
        }
        for change in append ?? [] {
            guard let row = rows.firstIndex(where: { $0.id == change.entry }),
                  let part = rows[row].parts.firstIndex(where: { $0.id == change.part }) else { throw RelayError.rpc("Transcript needs a refresh.") }
            let text = (rows[row].parts[part].text ?? "") + change.text
            guard text.utf8.count == change.len else { throw RelayError.rpc("Transcript needs a refresh.") }
            rows[row].parts[part].text = text
        }
        if let count, count != rows.count { throw RelayError.rpc("Transcript needs a refresh.") }
        return Update(messages: rows, contextUsage: contextUsage, carriesContextUsage: carriesContextUsage)
    }
}

// MARK: - Companion adapter

extension HostPart {
    /// The shared render call: `tag = call.kind`, `fields` = the call payload
    /// plus the part's sibling render state (output / subagent lifecycle) so
    /// the shared chip and spawn-row code reads the same keys as the desktop
    /// row model. Arrays and objects (Todo items, spawn input) ride as JSON
    /// strings, matching what the chip helpers parse.
    func renderedCall() -> RenderToolCall {
        let object = call?.objectValue ?? [:]
        var fields: [String: AnyHashable] = [:]
        for (key, value) in object where key != "kind" {
            if let string = value.stringValue { fields[key] = string }
            else if let data = try? JSONEncoder().encode(value) { fields[key] = String(decoding: data, as: UTF8.self) }
        }
        if let output { fields["output"] = output }
        if let subagentRef { fields["subagentRef"] = subagentRef }
        if let subagentStatus { fields["subagentStatus"] = subagentStatus }
        if let subagentTail { fields["subagentTail"] = subagentTail }
        return RenderToolCall(tag: object["kind"]?.stringValue ?? "unknown", fields: fields)
    }

    func renderedPart() -> MessagePart? {
        switch kind {
        case "text": return .text(id: id, text: text ?? "")
        case "reasoning":
            // A neutral Thinking row that participates in tool grouping.
            return .tool(id: id, call: RenderToolCall(tag: RenderToolCall.thinkingTag, fields: ["text": text ?? ""]),
                         isError: false, resolved: true)
        case "tool":
            return .tool(id: id, call: renderedCall(), isError: isError ?? false, resolved: resolved ?? false)
        case "input":
            return .input(id: id, requestId: requestId ?? id, questions: questions ?? [], resolved: resolved ?? false)
        case "error": return .error(id: id, message: message ?? "The host reported an error.")
        case "image":
            let reference = GeneratedImageReference(path: path ?? "", name: name ?? "", mimeType: mimeType ?? "")
            guard reference.isValid else { return .error(id: id, message: "Generated image unavailable") }
            return .image(id: id, reference: reference)
        default: return nil
        }
    }
}

extension HostMessage {
    var renderedEntry: MessageEntry {
        MessageEntry(id: id, role: MessageRole(rawValue: role) ?? .system,
            parts: parts.compactMap { $0.renderedPart() },
            createdAt: createdAt ?? 0, deviceId: deviceId ?? "",
            status: status.flatMap(MessageStatus.init(rawValue:)), continuationOf: nil)
    }
}

/// Reuses Noches' tested native transcript, incremental Markdown parser,
/// tool disclosures, text folding, and gesture-owned scroll state.
struct CompanionTranscript: View {
    let messages: [HostMessage]
    let scroll: ScrollState
    let online: Bool
    let busy: Bool
    let submittedID: String?
    let respond: (String, [UserInputAnswer]) -> Void
    @State private var rows: [TranscriptRow] = []
    @State private var cache = TranscriptBuilderCache()
    @State private var revision: UInt64 = 0
    @State private var veils = VeilStore()
    @State private var expanded = Set<String>()
    @State private var openTools = Set<String>()
    @State private var expansionHeights: [String: CGFloat] = [:]
    @Environment(\.companionImageLoader) private var imageLoader
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    var body: some View {
        NativeTranscriptTable(rows: rows, scroll: scroll, runwayID: submittedID,
            expansionHeight: submittedID.flatMap { expansionHeights[$0] } ?? 0,
            bottomSpacing: 18, reduceMotion: reduceMotion,
            configurationID: expanded.hashValue ^ openTools.hashValue ^ dynamicTypeSize.hashValue ^ online.hashValue ^ busy.hashValue) { row in
                AnyView(content(row).padding(.top, row.topGap).padding(.horizontal, Theme.spaceLG)
                    .frame(maxWidth: TranscriptView.maxContentWidth).frame(maxWidth: .infinity)
                    .environment(\.dynamicTypeSize, dynamicTypeSize)
                    .environment(\.companionImageLoader, imageLoader))
            }
            // The shared table renders beyond its viewport for keyboard motion.
            // Keep that overscan out of the companion navigation/status bars.
            .clipped()
            .background(Theme.bg)
            .onChange(of: messages, initial: true) { _, value in
                revision &+= 1
                rows = cache.rows(revision: revision, entries: value.map(\.renderedEntry), pendingSends: [])
            }
            .overlay(alignment: .bottomTrailing) {
                if scroll.showJump {
                    Button { scroll.arm(); scroll.jumpToLatest?(!reduceMotion) } label: {
                        Image(systemName: "arrow.down").font(.system(size: 16, weight: .medium)).frame(width: 44, height: 44)
                    }.nochesGlass(.regular.interactive(), in: Circle())
                        .accessibilityLabel("Jump to latest").padding(12)
                }
            }
    }

    @ViewBuilder private func content(_ row: TranscriptRow) -> some View {
        switch row.kind {
        case .user(let text):
            UserBubble(text: text, pending: false, deviceId: "", expanded: expanded.contains(row.entryId), onToggle: {
                scroll.pinned = false
                if !expanded.insert(row.entryId).inserted { expanded.remove(row.entryId) }
                scroll.refreshLayout?()
            }, onExpansionHeightChanged: { height in expansionHeights[row.entryId] = height; scroll.refreshLayout?() })
        case .markdown(let block, let streaming):
            MarkdownRowView(row: row, block: block, streaming: streaming, veils: veils)
        case .toolGroup(let tools, _):
            ToolGroupView(tools: tools, open: openTools.contains(row.id), userToggled: true, toggle: {
                if !openTools.insert(row.id).inserted { openTools.remove(row.id) }
                scroll.refreshLayout?()
            }, onDetailChanged: { scroll.refreshLayout?() })
        case .inputChip(let header, let resolved):
            if !resolved, let part = messages.first(where: { $0.id == row.entryId })?.parts.first(where: { "\(row.entryId)#\($0.id)" == row.id }) {
                QuestionPanel(requestId: part.requestId ?? part.id, questions: part.questions ?? [], respond: respond)
                    .disabled(!online || busy)
            } else { InputChipView(header: header, resolved: resolved) }
        case .errorChip(let message): ErrorChipView(message: message)
        case .generatedImage(_, let reference):
            CompanionGeneratedImageView(reference: reference, onResize: { scroll.refreshLayout?() })
        }
    }
}
