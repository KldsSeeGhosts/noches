// Transcript row model — a port of crates/ui/src/shell/transcript.rs
// rows_for_entry. One row = one markdown top-level block / tool group / chip,
// never one message: streamed tokens re-render one row, and SwiftUI's lazy
// stack only re-measures what changed.
//
// Stable ids: markdown rows are "{entryId}#{partId}.{blockIx}", tool groups
// "{entryId}#g{groupIx}", chips "{entryId}#{partId}". Live and completed parts
// split identically, so the live→complete handoff never changes row identity.

import Foundation
import SwiftUI

enum RowKind {
    case generatedImage(owner: String, reference: GeneratedImageReference)
    case user(text: String)
    case markdown(block: MDBlock, streaming: Bool)
    case toolGroup(tools: [ToolItem], autoOpen: Bool)
    case inputChip(header: String, resolved: Bool)
    case errorChip(message: String)
}

struct ToolItem: Hashable {
    var call: RenderToolCall
    var isError: Bool
    var resolved: Bool
    /// Spawn lifecycle when the call is a subagent spawn (`subagents.rs`).
    var subagentPhase: SubagentPhase?
    /// One-line result/live tail shown under a spawn row.
    var subagentTail: String?
}

struct TranscriptRow: Identifiable {
    var id: String
    /// Content fingerprint — SwiftUI diff key; a changed version re-renders
    /// exactly one row.
    var version: UInt64
    var turnStart: Bool
    var kind: RowKind
    var entryId: String
    var timestamp: Int64?
    /// "{entryId}#{partId}" for markdown rows, nil otherwise — two adjacent
    /// rows sharing it are blocks of the same part (the tighter gap).
    var partKey: String?
    /// Leading gap, resolved at build time. It depends on the PREVIOUS row, so
    /// deriving it in the view body forced an `enumerated()` copy of the whole
    /// row array on every frame; now the body just reads it.
    var topGap: CGFloat = 0
}

/// A settled part's parse, keyed by content so a completed block is parsed
/// once rather than on every rebuild.
struct CompletedParse {
    var source: String
    var blocks: [TopBlock]
}

enum TranscriptRowBuilder {
    /// Split entries into rows. `parsers` caches one incremental parser per
    /// "{entryId}#{partId}" so the streaming tail re-parses O(delta + tail);
    /// `completed` memoizes settled parts so they parse exactly once.
    static func rows(entries: [MessageEntry],
                     pendingSends: [PendingSend],
                     parsers: inout [String: IncrementalMarkdownParser],
                     completed: inout [String: CompletedParse]) -> [TranscriptRow] {
        var rows: [TranscriptRow] = []
        var live = Set<String>()
        for entry in entries {
            rowsForEntry(entry, into: &rows, parsers: &parsers,
                         completed: &completed, live: &live)
        }
        // Optimistic echo: pending sends share their client-minted id, so the
        // host's real entry replaces them without a flicker.
        let ids = Set(entries.map(\.id))
        for pending in pendingSends where !ids.contains(pending.messageId) {
            rows.append(TranscriptRow(id: pending.messageId,
                                      version: fnv1a(pending.text) | 1,
                                      turnStart: true,
                                      kind: .user(text: pending.text),
                                      entryId: pending.messageId,
                                      timestamp: nil,
                                      partKey: nil))
        }
        // Drop memos for parts that no longer exist. The count guard keeps the
        // common (append-only) rebuild from copying the dict every token.
        if completed.count > live.count {
            completed = completed.filter { live.contains($0.key) }
        }
        for ix in rows.indices {
            rows[ix].topGap = gap(for: rows[ix],
                                  previous: ix > 0 ? rows[ix - 1] : nil,
                                  isFirst: ix == 0)
        }
        return rows
    }

    private static func gap(for row: TranscriptRow,
                            previous: TranscriptRow?,
                            isFirst: Bool) -> CGFloat {
        if isFirst { return Theme.spaceLG + 10 }
        if row.turnStart { return Theme.spaceLG }
        // Same part ⇒ these are sibling markdown blocks, not a new turn.
        if let key = row.partKey, key == previous?.partKey { return MD.blockGap }
        // Tool stacks are visually denser than prose, so use one larger
        // global spacing step on both boundaries, matching desktop.
        if isToolGroup(row) || isToolGroup(previous) { return Theme.spaceMD }
        return Theme.spaceSM
    }

    private static func isToolGroup(_ row: TranscriptRow?) -> Bool {
        guard let row else { return false }
        if case .toolGroup = row.kind { return true }
        return false
    }

    private static func rowsForEntry(_ entry: MessageEntry,
                                     into rows: inout [TranscriptRow],
                                     parsers: inout [String: IncrementalMarkdownParser],
                                     completed: inout [String: CompletedParse],
                                     live: inout Set<String>) {
        let streaming = entry.status == .streaming
        let settled = entry.status != nil && !streaming

        if entry.role == .user {
            // One bubble row per user message.
            let text = entry.parts.compactMap { part -> String? in
                if case .text(_, let t) = part { return t }
                return nil
            }.joined(separator: "\n")
            guard !text.isEmpty else { return }
            rows.append(TranscriptRow(id: entry.id, version: fnv1a(text),
                                      turnStart: true, kind: .user(text: text),
                                      entryId: entry.id, timestamp: entry.createdAt,
                                      partKey: nil))
            return
        }

        var first = true
        var pendingTools: [ToolItem] = []
        var groupIx = 0
        let lastPartIx = entry.parts.indices.last

        func flushTools(lastIx: Int?) {
            guard !pendingTools.isEmpty else { return }
            let autoOpen = streaming && lastIx == lastPartIx
            let id = "\(entry.id)#g\(groupIx)"
            var version = toolFingerprint(pendingTools)
            if autoOpen { version ^= 1 }
            rows.append(TranscriptRow(id: id, version: version, turnStart: first,
                                      kind: .toolGroup(tools: pendingTools, autoOpen: autoOpen),
                                      entryId: entry.id, timestamp: nil, partKey: nil))
            first = false
            pendingTools = []
            groupIx += 1
        }

        for (ix, part) in entry.parts.enumerated() {
            switch part {
            case .tool(_, let call, let isError, let resolved):
                pendingTools.append(ToolItem(call: call, isError: isError, resolved: resolved,
                    subagentPhase: call.subagentPhase(isError: isError, resolved: resolved, streaming: streaming),
                    subagentTail: call.subagentTail))
                if ix == lastPartIx { flushTools(lastIx: ix) }

            case .text(let partId, let text):
                flushTools(lastIx: ix - 1)
                guard !text.isEmpty else { continue }
                let key = "\(entry.id)#\(partId)"
                live.insert(key)
                let isLiveTail = streaming && ix == lastPartIx
                let blocks = parse(text: text, key: key, streaming: isLiveTail,
                                   parsers: &parsers, completed: &completed)
                for (blockIx, top) in blocks.enumerated() {
                    var version = (top.fingerprint << 1) | (isLiveTail && blockIx == blocks.count - 1 ? 1 : 0)
                    if settled, ix == lastPartIx, blockIx == blocks.count - 1 {
                        version ^= 1 << 62  // timestamp attach keeps the diff key honest
                    }
                    rows.append(TranscriptRow(
                        id: "\(key).\(blockIx)", version: version, turnStart: first,
                        kind: .markdown(block: top.block,
                                        streaming: isLiveTail && blockIx == blocks.count - 1),
                        entryId: entry.id,
                        timestamp: settled && ix == lastPartIx && blockIx == blocks.count - 1
                            ? entry.createdAt : nil,
                        partKey: key))
                    first = false
                }

            case .image(let partId, let reference):
                flushTools(lastIx: ix - 1)
                var version = fnv1a("\(entry.deviceId)\0\(reference.path)\0\(reference.name)\0\(reference.mimeType)")
                if settled, ix == lastPartIx { version ^= 1 << 62 }
                rows.append(TranscriptRow(id: "\(entry.id)#\(partId)", version: version,
                                          turnStart: first,
                                          kind: .generatedImage(owner: entry.deviceId, reference: reference),
                                          entryId: entry.id,
                                          timestamp: settled && ix == lastPartIx ? entry.createdAt : nil,
                                          partKey: nil))
                first = false

            case .input(let partId, _, let questions, let resolved):
                flushTools(lastIx: ix - 1)
                let header = questions.first?.header ?? "Question"
                rows.append(TranscriptRow(id: "\(entry.id)#\(partId)",
                                          version: fnv1a(header) | (resolved ? 1 : 0),
                                          turnStart: first,
                                          kind: .inputChip(header: header, resolved: resolved),
                                          entryId: entry.id, timestamp: nil, partKey: nil))
                first = false

            case .error(let partId, let message):
                flushTools(lastIx: ix - 1)
                rows.append(TranscriptRow(id: "\(entry.id)#\(partId)", version: fnv1a(message),
                                          turnStart: first,
                                          kind: .errorChip(message: message),
                                          entryId: entry.id, timestamp: nil, partKey: nil))
                first = false
            }
        }
        flushTools(lastIx: lastPartIx)
    }

    private static func parse(text: String, key: String, streaming: Bool,
                              parsers: inout [String: IncrementalMarkdownParser],
                              completed: inout [String: CompletedParse]) -> [TopBlock] {
        if streaming {
            let parser = parsers[key] ?? IncrementalMarkdownParser()
            parser.setText(text)
            parsers[key] = parser
            return parser.blocks
        }
        // Completed. Drop the live parser either way, then serve from the memo:
        // rows are rebuilt on every doc update, and re-parsing every settled
        // part each time made a rebuild O(whole transcript) — the dominant cost
        // of opening a long cached session, and paid again per streamed token.
        let handoff = parsers.removeValue(forKey: key)
        if let hit = completed[key], hit.source == text {
            return hit.blocks
        }
        // Adopt the live parser's tree on the live→complete flip, else parse.
        let blocks = handoff?.source == text
            ? (handoff?.blocks ?? MarkdownParser.parse(text))
            : MarkdownParser.parse(text)
        completed[key] = CompletedParse(source: text, blocks: blocks)
        return blocks
    }

    private static func toolFingerprint(_ tools: [ToolItem]) -> UInt64 {
        var hash: UInt64 = 0xcbf29ce484222325
        for tool in tools {
            for byte in tool.call.tag.utf8 {
                hash ^= UInt64(byte)
                hash = hash &* 0x100000001b3
            }
            hash ^= UInt64(tool.call.fields.count) &+ (tool.isError ? 2 : 0) &+ (tool.resolved ? 4 : 0)
            for text in [tool.subagentPhase?.rawValue ?? "", tool.subagentTail ?? ""] {
                for byte in text.utf8 {
                    hash ^= UInt64(byte)
                    hash = hash &* 0x100000001b3
                }
                hash = hash &* 0x100000001b3
            }
            hash = hash &* 0x100000001b3
            for (k, v) in tool.call.fields.sorted(by: { $0.key < $1.key }) {
                for byte in "\(k)=\(v)".utf8 {
                    hash ^= UInt64(byte)
                    hash = hash &* 0x100000001b3
                }
            }
        }
        return hash << 3
    }

    static func fnv1a(_ text: String) -> UInt64 {
        var hash: UInt64 = 0xcbf29ce484222325
        for byte in text.utf8 {
            hash ^= UInt64(byte)
            hash = hash &* 0x100000001b3
        }
        return hash << 1
    }
}

// MARK: - Tool chip content (view.rs tool_chip_content_raw)
//
// Mirrors `crates/proto/src/view.rs` exactly: the cloud and companion
// adapters both feed these fields, so a tool is named identically on every
// surface. Unknown names are humanized; spawn calls read as "Agent".

extension RenderToolCall {
    static let thinkingTag = "thinking"

    /// A reasoning part rendered as the neutral Thinking row.
    var isThought: Bool { tag == Self.thinkingTag }

    /// `ToolCall::is_subagent_spawn`: Unknown/Mcp named "Agent[: x]".
    var isSubagentSpawn: Bool {
        switch tag {
        case "unknown": return Self.isAgentName(string("name") ?? "")
        case "mcp": return Self.isAgentName(string("tool") ?? "")
        default: return false
        }
    }

    private static func isAgentName(_ name: String) -> Bool {
        name == "Agent" || name.hasPrefix("Agent: ")
    }

    /// Flattened spawn input. The companion adapter carries it as a JSON
    /// string; a future adapter may hand over native scalars.
    var subagentInput: [String: AnyHashable]? {
        if let input = fields["input"] as? [String: AnyHashable] { return input }
        guard let json = fields["input"] as? String,
              let data = json.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
        var out: [String: AnyHashable] = [:]
        for (key, value) in object {
            if let text = value as? String { out[key] = text }
            else if let flag = value as? Bool { out[key] = flag }
            else if let number = value as? NSNumber { out[key] = number.intValue }
        }
        return out
    }

    /// The model a spawn named, when it named one (`SUBAGENT_MODEL_KEYS`).
    var subagentModel: String? {
        spawnInputString(keys: ["model", "modelId", "model_id", "subagent_model"])
    }

    var subagentType: String? { spawnInputString(keys: ["subagent_type"]) }

    private func spawnInputString(keys: [String]) -> String? {
        guard isSubagentSpawn, let input = subagentInput else { return nil }
        for key in keys {
            if let value = input[key] as? String {
                let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
                if !trimmed.isEmpty { return trimmed }
            }
        }
        return nil
    }

    /// `run_in_background`, tolerating a stringly "true".
    var isBackgroundSubagent: Bool {
        guard let value = subagentInput?["run_in_background"] else { return false }
        if let flag = value as? Bool { return flag }
        if let flag = value as? String { return flag.trimmingCharacters(in: .whitespaces).lowercased() == "true" }
        return false
    }

    /// `subagents.rs` `SubagentPhase` derivation. `Started` is the honest
    /// state for background spawns and the eager-done window.
    func subagentPhase(isError: Bool, resolved: Bool, streaming: Bool) -> SubagentPhase? {
        guard isSubagentSpawn else { return nil }
        if isError { return .failed }
        switch string("subagentStatus") {
        case "running": return .running
        case "done": return .done
        case "failed": return .failed
        default: break
        }
        if isBackgroundSubagent { return .started }
        guard resolved else {
            // A turn that ended without the spawn resolving died with the run.
            return streaming ? .running : .failed
        }
        if string("output") != nil { return .done }
        return streaming ? .started : .done
    }

    /// One-line result (or live tail), promoting `output` over `subagentTail`.
    var subagentTail: String? {
        for key in ["output", "subagentTail"] {
            if let text = string(key), let line = subagentOneLine(text) { return line }
        }
        return nil
    }

    /// `subagents.rs` `spawn_title`: strip the Agent:/Task: genus, prefer the
    /// spawn name, then the description; cap 40 chars; else "Agent".
    var subagentTitle: String {
        let name = (tag == "mcp" ? string("tool") : string("name")) ?? "Agent"
        let bare: String
        if name.hasPrefix("Agent: ") { bare = String(name.dropFirst("Agent: ".count)) }
        else if name.hasPrefix("Task: ") { bare = String(name.dropFirst("Task: ".count)) }
        else { bare = name }
        for candidate in [bare, subagentInput?["description"] as? String ?? ""] {
            let line = singleLine(candidate)
            if !line.isEmpty && line.lowercased() != "agent" { return String(line.prefix(40)) }
        }
        return "Agent"
    }

    /// Per-kind label + one-line detail (`view.rs` tool_chip_content_raw).
    private var chipContent: (label: String, detail: String) {
        switch tag {
        case "exec": return ("Run", string("command") ?? "")
        case "readFile": return ("Read", string("path") ?? "")
        case "writeFile": return ("Write", string("path") ?? "")
        case "editFile": return ("Edit", string("path") ?? "")
        case "applyPatch": return ("Patch", string("path") ?? "workspace")
        case "search":
            let pattern = string("pattern") ?? ""
            guard let path = string("path") else { return ("Search", pattern) }
            return ("Search", "\(pattern) in \(path)")
        case "glob": return ("Glob", string("pattern") ?? "")
        case "webFetch": return ("Fetch", string("url") ?? "")
        case "webSearch": return ("Web", string("query") ?? "")
        case "todo":
            let (done, total) = todoCounts
            return ("Todo", "\(done)/\(total) done")
        case "mcp": return ("MCP", "\(string("server") ?? "") · \(string("tool") ?? "")")
        case Self.thinkingTag: return ("Thinking", "")
        case "unknown":
            let name = string("name") ?? ""
            if name.hasPrefix("Agent: ") { return ("Agent", String(name.dropFirst("Agent: ".count))) }
            if name == "Agent" { return ("Agent", "") }
            return (Self.humanizeToolName(name), unknownDetail)
        default: return ("Tool", "")
        }
    }

    var chipLabel: String { chipContent.label }

    var chipDetail: String { chipContent.detail }

    /// Preserve the full invocation in the expanded row and on the clipboard.
    var expandedDetail: String {
        if isThought { return string("text") ?? "" }
        switch tag {
        case "readFile", "writeFile", "editFile": return string("path") ?? ""
        default: return chipDetail
        }
    }

    var chipSymbol: String {
        switch tag {
        case "exec": return "terminal"
        case "readFile", "applyPatch": return "doc.text"
        case "writeFile": return "doc.badge.plus"
        case "editFile": return "pencil"
        case "search": return "magnifyingglass"
        case "glob": return "folder"
        case "webFetch", "webSearch": return "globe"
        case "todo": return "checklist"
        case Self.thinkingTag: return "brain"
        default: return "square.grid.2x2"
        }
    }

    /// The most informative scalar arg of an Unknown call, capped at 80
    /// chars (`view.rs` unknown_detail).
    private var unknownDetail: String {
        guard let input = subagentInput else { return "" }
        for key in ["description", "query", "q", "pattern", "path", "url", "command", "subject", "action", "title"] {
            if let value = input[key] as? String, !value.isEmpty { return Self.truncate(value, 80) }
        }
        return ""
    }

    /// `view.rs` humanize_tool_name: snake/kebab and `mcp__server__tool`
    /// names become Title Case words ("codex-research" -> "Codex research",
    /// "mcp__server__tool" -> "server · tool").
    static func humanizeToolName(_ name: String) -> String {
        if name.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() == "other" { return "Tool" }
        var parts = name.components(separatedBy: "__").map { part in
            part.split(whereSeparator: { $0 == "_" || $0 == "-" })
                .filter { !$0.isEmpty }.joined(separator: " ")
        }.filter { !$0.isEmpty }
        if parts.first == "mcp" { parts.removeFirst() }
        guard !parts.isEmpty else { return "Tool" }
        let joined = parts.joined(separator: " · ")
        return String(joined.prefix(1)).uppercased() + String(joined.dropFirst())
    }

    private static func truncate(_ text: String, _ max: Int) -> String {
        text.count <= max ? text : String(text.prefix(max)) + "…"
    }

    /// Todo "n done" counts. The cloud adapter hands over the per-item
    /// Swift descriptions; the companion one a JSON array string.
    private var todoCounts: (done: Int, total: Int) {
        if let items = fields["items"] as? [String] {
            return (items.filter(Self.todoItemDone).count, items.count)
        }
        if let json = fields["items"] as? String,
           let data = json.data(using: .utf8),
           let items = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] {
            return (items.filter { $0["done"] as? Bool ?? false }.count, items.count)
        }
        return (0, 0)
    }

    private static func todoItemDone(_ item: String) -> Bool {
        if let data = item.data(using: .utf8),
           let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
            return object["done"] as? Bool ?? false
        }
        return item.range(of: #"done["\s:]*true"#, options: [.regularExpression, .caseInsensitive]) != nil
    }
}

/// "Ran 3 commands · edited 2 files · 1 failed" (`transcript.rs`
/// tool_group_summary, which delegates the pair rule to view.rs).
func toolGroupSummary(_ tools: [ToolItem]) -> String {
    let pairs = tools.filter { !$0.call.isThought }
    let base = viewPortToolSummary(pairs)
    let thoughts = tools.count - pairs.count
    switch (base.isEmpty, thoughts) {
    case (_, 0): return base
    case (true, 1): return "Thought process"
    case (true, let count): return "Thought \(count) times"
    case (false, 1): return "Thought · \(base)"
    default: return "Thought \(thoughts) times · \(base)"
    }
}

/// `view.rs` tool_group_summary over non-thought chips.
private func viewPortToolSummary(_ tools: [ToolItem]) -> String {
    var commands = 0, reads = 0, searches = 0, fetches = 0, todos = 0, other = 0, failed = 0
    var edited: [String] = []
    for tool in tools {
        if tool.isError { failed += 1 }
        switch tool.call.tag {
        case "exec": commands += 1
        case "writeFile", "editFile", "applyPatch":
            let path = tool.call.string("path") ?? "patch"
            if !edited.contains(path) { edited.append(path) }
        case "readFile": reads += 1
        case "search", "glob", "webSearch": searches += 1
        case "webFetch": fetches += 1
        case "todo": todos += 1
        case "mcp", "unknown": other += 1
        default: break
        }
    }
    var segments: [String] = []
    if commands > 0 { segments.append("ran \(plural(commands, "command", "commands"))") }
    if !edited.isEmpty { segments.append("edited \(plural(edited.count, "file", "files"))") }
    if reads > 0 { segments.append("read \(plural(reads, "file", "files"))") }
    if searches > 0 { segments.append("searched \(plural(searches, "time", "times"))") }
    if fetches > 0 { segments.append("fetched \(plural(fetches, "page", "pages"))") }
    if todos > 0 { segments.append("updated todos") }
    if other > 0 { segments.append("called \(plural(other, "tool", "tools"))") }
    if segments.isEmpty { segments.append(plural(tools.count, "tool", "tools")) }
    if failed > 0 { segments.append("\(failed) failed") }
    guard let first = segments.first else { return "" }
    let capped = first.prefix(1).uppercased() + first.dropFirst()
    return ([String(capped)] + segments.dropFirst()).joined(separator: " · ")
}

private func plural(_ count: Int, _ one: String, _ many: String) -> String {
    "\(count) \(count == 1 ? one : many)"
}

/// Single-line collapse (`view.rs` single_line): runs of whitespace become
/// one space, trimmed.
func singleLine(_ text: String) -> String {
    text.split(whereSeparator: \.isWhitespace).joined(separator: " ")
}

/// First non-blank line, capped at 120 chars (`subagents.rs` one_line).
private func subagentOneLine(_ text: String) -> String? {
    guard let raw = text.split(separator: "\n").first(where: { !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }) else { return nil }
    let line = raw.trimmingCharacters(in: .whitespacesAndNewlines)
    return line.count > 120 ? String(line.prefix(120)) + "…" : line
}

// MARK: - Tool family tint (crates/ui/src/tool_palette.rs)

/// The display family of one tool call. Hues are the desktop's desaturated
/// (dark, light) pairs, deliberately off the session status sectors.
enum ToolFamily {
    case explore, change, delegate, neutral

    static func of(_ call: RenderToolCall, isThought: Bool = false) -> ToolFamily {
        if isThought { return .neutral }
        switch call.tag {
        case "readFile", "search", "glob", "webFetch", "webSearch": return .explore
        case "writeFile", "editFile", "applyPatch": return .change
        default:
            if call.isSubagentSpawn { return .delegate }
            if call.tag == "unknown" && call.string("name") == "Wait for agents" { return .delegate }
            return .neutral
        }
    }

    var color: Color? { color(dark: AppearanceSettings.shared.isDark) }

    func color(dark: Bool) -> Color? {
        func pick(_ onDark: UInt32, _ onLight: UInt32) -> Color {
            AppearanceSettings.color(String(format: "#%06x", dark ? onDark : onLight))
        }
        switch self {
        case .explore: return pick(0x7cc4bd, 0x2f7f78)
        case .change: return pick(0xd9b26f, 0x946514)
        case .delegate: return pick(0xc9a0dc, 0x8a4a9e)
        case .neutral: return nil
        }
    }
}

// MARK: - Subagent phase presentation

extension SubagentPhase {
    var active: Bool { self == .running || self == .started }

    var label: String {
        switch self {
        case .running: return "Running"
        case .started: return "Started"
        case .done: return "Done"
        case .failed: return "Failed"
        }
    }

    /// Status hues come only from SessionState; Started stays neutral.
    var tint: Color {
        switch self {
        case .running: return Theme.statusWorking
        case .started: return Theme.textFaint
        case .done: return Theme.statusCompleted
        case .failed: return Theme.danger
        }
    }
}
