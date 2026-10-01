import SwiftUI

// MARK: - Screen buffer

/// A minimal terminal screen for scrollback display: printable text, `\r`,
/// `\n`, backspace, tab, erase-in-line, horizontal cursor moves, clear screen,
/// and basic 16-color foregrounds. Every other escape sequence is consumed and
/// ignored. Input may arrive in arbitrary byte chunks (UTF-8 and escape
/// sequences are carried across calls).
struct TerminalBuffer {
    struct Cell: Equatable { var ch: Character; var color: UInt8? }

    private(set) var lines: [[Cell]] = [[]]
    /// Lines dropped from the top; keeps row identities stable.
    private(set) var dropped = 0
    private var column = 0
    private var color: UInt8?
    private var pendingBytes = Data()
    private var escape: [Character]?
    private let limit: Int

    init(limit: Int = 4000) { self.limit = limit }

    mutating func feed(_ bytes: Data) {
        pendingBytes.append(bytes)
        // Hold back an incomplete trailing UTF-8 sequence.
        var end = pendingBytes.count
        var back = 0
        while back < 4, end - 1 - back >= 0 {
            let byte = pendingBytes[pendingBytes.startIndex + end - 1 - back]
            if byte & 0xC0 == 0x80 { back += 1; continue }
            let need = byte >= 0xF0 ? 4 : byte >= 0xE0 ? 3 : byte >= 0xC0 ? 2 : 1
            if need > back + 1 { end -= back + 1 }
            break
        }
        let ready = pendingBytes.prefix(end)
        pendingBytes = Data(pendingBytes.dropFirst(end))
        feed(String(decoding: ready, as: UTF8.self))
    }

    mutating func feed(_ text: String) {
        for ch in text { consume(ch) }
        if lines.count > limit {
            let extra = lines.count - limit
            lines.removeFirst(extra); dropped += extra
        }
    }

    /// Plain text of every line (colors dropped), for tests and copy.
    var plainLines: [String] { lines.map { String($0.map(\.ch)) } }

    private mutating func consume(_ ch: Character) {
        if var seq = escape {
            seq.append(ch)
            escape = seq
            if finishEscape(seq) { escape = nil }
            return
        }
        switch ch {
        case "\u{1B}": escape = []
        case "\r", "\r\n": column = 0; if ch == "\r\n" { newline() }
        case "\n": newline()
        case "\u{08}", "\u{7F}": column = max(0, column - 1)
        case "\t": for _ in 0..<(8 - column % 8) { put(" ") }
        case "\u{07}": break
        default:
            if let scalar = ch.unicodeScalars.first, scalar.value < 0x20 { return }
            put(ch)
        }
    }

    /// `seq` holds the characters after ESC. Returns true when the sequence is complete.
    private mutating func finishEscape(_ seq: [Character]) -> Bool {
        guard let first = seq.first else { return false }
        switch first {
        case "[":
            guard seq.count > 1, let last = seq.last, let scalar = last.unicodeScalars.first,
                  (0x40...0x7E).contains(scalar.value) else { return seq.count > 64 }
            csi(params: String(seq.dropFirst().dropLast()), final: last)
            return true
        case "]", "P", "_", "^", "X":
            // OSC and friends end with BEL or ST (ESC \).
            if seq.last == "\u{07}" { return true }
            if seq.count >= 2, seq[seq.count - 2] == "\u{1B}", seq.last == "\\" { return true }
            return seq.count > 512
        case "(", ")", "*", "+", "#", "%", " ":
            return seq.count >= 2 // charset/alignment selectors take one more byte
        default:
            return true // two-byte escapes such as ESC = or ESC 7
        }
    }

    private mutating func csi(params: String, final: Character) {
        let numbers = params.drop { !$0.isNumber && $0 != ";" }.split(separator: ";", omittingEmptySubsequences: false).map { Int($0) }
        let n = (numbers.first ?? nil) ?? 0
        switch final {
        case "m": sgr(numbers.map { $0 ?? 0 })
        case "K":
            let row = lines.count - 1
            switch n {
            case 0: if column < lines[row].count { lines[row].removeSubrange(column...) }
            case 1: for i in 0..<min(column, lines[row].count) { lines[row][i] = Cell(ch: " ", color: nil) }
            default: lines[row] = []
            }
        case "C": column += max(1, n)
        case "D": column = max(0, column - max(1, n))
        case "G": column = max(0, (n == 0 ? 1 : n) - 1)
        case "J":
            if n == 2 || n == 3 { lines = [[]]; column = 0 }
        case "P": // delete characters
            let row = lines.count - 1
            if column < lines[row].count { lines[row].removeSubrange(column..<min(lines[row].count, column + max(1, n))) }
        case "@": // insert blanks
            let row = lines.count - 1
            if column <= lines[row].count {
                lines[row].insert(contentsOf: Array(repeating: Cell(ch: " ", color: nil), count: max(1, n)), at: column)
            }
        default: break
        }
    }

    private mutating func sgr(_ codes: [Int]) {
        var i = 0
        let list = codes.isEmpty ? [0] : codes
        while i < list.count {
            let code = list[i]
            switch code {
            case 0, 39: color = nil
            case 30...37: color = UInt8(code - 30)
            case 90...97: color = UInt8(code - 90 + 8)
            case 38, 48: i += list.count > i + 1 && list[i + 1] == 5 ? 2 : list.count > i + 1 && list[i + 1] == 2 ? 4 : 0
            default: break
            }
            i += 1
        }
    }

    private mutating func put(_ ch: Character) {
        let row = lines.count - 1
        while lines[row].count < column { lines[row].append(Cell(ch: " ", color: nil)) }
        let cell = Cell(ch: ch, color: color)
        if column < lines[row].count { lines[row][column] = cell } else { lines[row].append(cell) }
        column += 1
    }

    private mutating func newline() {
        lines.append([])
        column = 0
    }
}

extension TerminalBuffer {
    /// Bytes for the accessory keys.
    enum Key {
        case escape, tab, ctrlC, ctrlD, up, down, left, right
        var bytes: Data {
            switch self {
            case .escape: return Data([0x1B])
            case .tab: return Data([0x09])
            case .ctrlC: return Data([0x03])
            case .ctrlD: return Data([0x04])
            case .up: return Data("\u{1B}[A".utf8)
            case .down: return Data("\u{1B}[B".utf8)
            case .right: return Data("\u{1B}[C".utf8)
            case .left: return Data("\u{1B}[D".utf8)
            }
        }
    }
}

// MARK: - Session registry

/// Remembers each chat's live terminal so closing the sheet detaches rather than
/// kills the shell (the host keeps it running) and reopening reattaches.
@MainActor
final class CompanionTerminalRegistry {
    static let shared = CompanionTerminalRegistry()
    var terminals: [String: String] = [:]
}

// MARK: - Session controller

@MainActor @Observable
final class CompanionTerminalSession {
    var buffer = TerminalBuffer()
    var revision = 0
    var status = "Starting terminal…"
    var exited = false
    var error: String?
    private(set) var terminalId: String?
    @ObservationIgnored private var lastSeq: UInt64?
    @ObservationIgnored private var cols = 80
    @ObservationIgnored private var rows = 24

    func run(model: CompanionModel, chat: HostChat) async {
        guard model.online else { status = "Reconnecting to your computer…"; return }
        let key = model.terminalKey(for: chat)
        let connection = model.connection
        if let known = CompanionTerminalRegistry.shared.terminals[key], known != terminalId {
            reset(); terminalId = known
        }
        while !Task.isCancelled, model.online, model.connection === connection, !exited {
            do {
                if terminalId == nil {
                    let session = try await model.openTerminal(for: chat, cols: cols, rows: rows)
                    terminalId = session.id
                    CompanionTerminalRegistry.shared.terminals[key] = session.id
                }
                guard let id = terminalId else { return }
                status = "Connected"
                for try await event in try await model.subscribeTerminal(id, afterSeq: lastSeq) {
                    guard !Task.isCancelled else { return }
                    switch event {
                    case .data(let seq, let bytes):
                        lastSeq = seq
                        buffer.feed(bytes)
                        revision += 1
                    case .exit(let seq, let code, let signal):
                        lastSeq = seq
                        exited = true
                        status = signal.map { "Exited (\($0))" } ?? "Exited with code \(code)"
                        CompanionTerminalRegistry.shared.terminals[key] = nil
                    }
                }
                if exited { return }
            } catch {
                if Task.isCancelled { return }
                // An unknown id means the shell is gone; start a fresh one next pass.
                if terminalId != nil, lastSeq == nil { terminalId = nil; CompanionTerminalRegistry.shared.terminals[key] = nil }
                self.error = error.localizedDescription
                status = "Reconnecting…"
            }
            do { try await Task.sleep(for: .seconds(2)) } catch { return }
        }
    }

    private func reset() {
        buffer = TerminalBuffer(); lastSeq = nil; revision += 1; exited = false
    }

    func send(_ bytes: Data, model: CompanionModel, chat: HostChat) {
        guard let id = terminalId, !exited else { return }
        Task {
            do { try await model.writeTerminal(id, chat: chat, bytes: bytes); error = nil }
            catch { self.error = error.localizedDescription }
        }
    }

    func resize(cols: Int, rows: Int, model: CompanionModel, chat: HostChat) {
        guard cols > 10, rows > 2, cols != self.cols || rows != self.rows else { return }
        self.cols = cols; self.rows = rows
        guard let id = terminalId else { return }
        Task { try? await model.resizeTerminal(id, chat: chat, cols: cols, rows: rows) }
    }

    func close(model: CompanionModel, chat: HostChat) {
        guard let id = terminalId else { return }
        CompanionTerminalRegistry.shared.terminals[model.terminalKey(for: chat)] = nil
        Task { try? await model.closeTerminal(id, chat: chat) }
        terminalId = nil; exited = true; status = "Closed"
    }
}

// MARK: - View

struct CompanionTerminalView: View {
    let model: CompanionModel
    let chat: HostChat
    @State private var session = CompanionTerminalSession()
    @State private var input = ""
    @FocusState private var focused: Bool
    private static let charWidth = ("M" as NSString).size(withAttributes: [.font: Theme.monoUI(12)]).width
    private static let lineHeight: CGFloat = 16

    var body: some View {
        VStack(spacing: 0) {
            statusStrip
            scrollback
            SheetHairline()
            keyRow.padding(.top, 8)
            inputRow
        }
        .background(Theme.bg).foregroundStyle(Theme.text)
        .navigationTitle("Terminal").navigationBarTitleDisplayMode(.inline)
        .task(id: "\(model.generation)-\(model.online)") { await session.run(model: model, chat: chat) }
    }

    /// Connection state and the one destructive action, in a quiet strip under
    /// the nav bar rather than a bare glyph in it.
    private var statusStrip: some View {
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Circle().fill(statusTint).frame(width: 6, height: 6)
                Text(session.error.map { "\(session.status): \($0)" } ?? session.status)
                    .font(Theme.mono(11)).foregroundStyle(session.exited || session.error != nil ? Theme.warning : Theme.textFaint)
                    .lineLimit(1)
                Spacer(minLength: 8)
                Button { session.close(model: model, chat: chat) } label: {
                    Text("Close").font(Theme.sans(13, weight: .medium)).foregroundStyle(Theme.textMuted)
                        .padding(.horizontal, 8).frame(minHeight: 44).contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Close terminal")
                .disabled(session.terminalId == nil)
            }
            .padding(.leading, SheetMetrics.margin).padding(.trailing, 8)
            SheetHairline()
        }
    }

    private var statusTint: Color {
        if session.exited || session.error != nil { return Theme.warning }
        return session.status == "Connected" ? Theme.statusCompleted : Theme.textFaint
    }

    private var scrollback: some View {
        GeometryReader { proxy in
            CompanionTerminalScrollback(session: session)
                .onChange(of: proxy.size) { _, size in resizeTerminal(to: size) }
                .onAppear { resizeTerminal(to: proxy.size) }
        }
    }

    private func resizeTerminal(to size: CGSize) {
        let cols = Int((size.width - 32) / Self.charWidth)
        let rows = Int((size.height - 16) / Self.lineHeight)
        session.resize(cols: cols, rows: rows, model: model, chat: chat)
    }

    private var keyRow: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                key("esc", .escape); key("tab", .tab); key("ctrl-c", .ctrlC); key("ctrl-d", .ctrlD)
                key("↑", .up); key("↓", .down); key("←", .left); key("→", .right)
                ForEach(["/", "~", "|", "-"], id: \.self) { symbol in
                    keyButton(symbol) { session.send(Data(symbol.utf8), model: model, chat: chat) }
                }
            }
            .padding(.horizontal, SheetMetrics.margin)
        }
    }

    private func key(_ label: String, _ key: TerminalBuffer.Key) -> some View {
        keyButton(label) { session.send(key.bytes, model: model, chat: chat) }
    }

    private func keyButton(_ label: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(label).font(Theme.mono(13)).foregroundStyle(Theme.text)
                .frame(minWidth: 44, minHeight: 36).padding(.horizontal, 4)
                .background(Theme.wash(0.06), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                .frame(minHeight: 44)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain).disabled(session.exited)
    }

    private var inputRow: some View {
        HStack(spacing: 8) {
            TextField("Command", text: $input)
                .font(Theme.mono(14)).focused($focused)
                .textInputAutocapitalization(.never).autocorrectionDisabled()
                .submitLabel(.send).onSubmit(send)
                .padding(.horizontal, 14).frame(minHeight: 44)
                .overlay(Capsule().stroke(Theme.border, lineWidth: 1))
            Button(action: send) {
                Image(systemName: "arrow.up")
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(canSend ? Theme.bg : Theme.textFaint)
                    .frame(width: 36, height: 36)
                    .background(canSend ? AnyShapeStyle(Theme.text) : AnyShapeStyle(Theme.wash(0.08)), in: Circle())
                    .frame(width: 44, height: 44).contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Send")
            .accessibilityIdentifier("terminal-send")
            .disabled(session.exited)
        }
        .padding(.horizontal, SheetMetrics.margin).padding(.bottom, 8)
    }

    private var canSend: Bool { !session.exited && !input.isEmpty }

    private func send() {
        session.send(Data((input + "\r").utf8), model: model, chat: chat)
        input = ""
        focused = true
    }

}

/// Separate compiler boundaries for the reader, viewport, stack and line keep
/// the terminal expression small on every supported Xcode version.
private struct CompanionTerminalScrollback: View {
    let session: CompanionTerminalSession

    var body: some View {
        ScrollViewReader { reader in
            viewport(reader: reader)
        }
    }

    private func viewport(reader: ScrollViewProxy) -> some View {
        ScrollView {
            lines
        }
        .defaultScrollAnchor(.bottom)
        .onChange(of: session.revision) { _, _ in reader.scrollTo("end", anchor: .bottom) }
    }

    private var lines: some View {
        LazyVStack(alignment: .leading, spacing: 0) {
            ForEach(Array(session.buffer.lines.enumerated()), id: \.offset) { row in
                CompanionTerminalLine(line: row.element).id(row.offset)
            }
            Color.clear.frame(height: 1).id("end")
        }
        .padding(.horizontal, SheetMetrics.margin).padding(.vertical, 8)
        .textSelection(.enabled)
    }
}

private struct CompanionTerminalLine: View {
    let line: [TerminalBuffer.Cell]

    var body: some View {
        Text(Self.attributed(line))
            .font(Theme.mono(12)).lineLimit(nil)
            .frame(maxWidth: .infinity, minHeight: 16, alignment: .leading)
    }

    private static func attributed(_ line: [TerminalBuffer.Cell]) -> AttributedString {
        if line.isEmpty { return AttributedString(" ") }
        var result = AttributedString()
        var index = 0
        while index < line.count {
            let color = line[index].color
            var run = ""
            while index < line.count, line[index].color == color { run.append(line[index].ch); index += 1 }
            var piece = AttributedString(run)
            piece.foregroundColor = tint(color)
            result.append(piece)
        }
        return result
    }

    /// ANSI slots map to theme roles, not fixed hex values.
    private static func tint(_ code: UInt8?) -> Color {
        switch code {
        case 1, 9: return Theme.danger
        case 2, 10: return Theme.statusCompleted
        case 3, 11: return Theme.warning
        case 4, 12: return Theme.statusWorking
        case 5, 13: return ToolFamily.delegate.color ?? Theme.accent
        case 6, 14: return ToolFamily.explore.color ?? Theme.accent
        case 0, 8: return Theme.textFaint
        default: return Theme.text
        }
    }
}
