import SwiftUI

// MARK: - Live store

/// Keeps one checkout diff live for a session: an initial `GetCheckoutDiff`,
/// then the matching frames of `WatchCheckoutDiffs`. The session header and the
/// Changes sheet read the same store, so there is a single watch per session.
@MainActor @Observable
final class CompanionChangesStore {
    var diff: HostCheckoutDiff?
    var comparison: HostGitComparison?
    var error: String?
    var loaded = false
    private var comparedAt = Date.distantPast

    func run(model: CompanionModel, chat: HostChat) async {
        guard model.online else { return }
        let connection = model.connection
        while !Task.isCancelled, model.online, model.connection === connection {
            do {
                let first = try await model.checkoutDiff(for: chat)
                guard !Task.isCancelled else { return }
                accept(first, model: model, chat: chat)
                for try await frame in try await model.watchCheckoutDiff(checkoutId: first.checkoutId) {
                    guard !Task.isCancelled else { return }
                    accept(frame, model: model, chat: chat)
                }
            } catch {
                if Task.isCancelled { return }
                if diff == nil { self.error = error.localizedDescription }
                loaded = true
            }
            do { try await Task.sleep(for: .seconds(diff == nil ? 15 : 3)) } catch { return }
        }
    }

    func refresh(model: CompanionModel, chat: HostChat) async {
        do {
            let next = try await model.checkoutDiff(for: chat)
            accept(next, model: model, chat: chat, force: true)
        } catch { if !Task.isCancelled { self.error = error.localizedDescription } }
    }

    private func accept(_ next: HostCheckoutDiff, model: CompanionModel, chat: HostChat, force: Bool = false) {
        if next != diff { diff = next }
        error = nil
        loaded = true
        guard force || Date().timeIntervalSince(comparedAt) > 15 else { return }
        comparedAt = Date()
        let cwd = next.cwd
        Task {
            if let value = try? await model.gitComparison(for: chat, cwd: cwd), value != comparison { comparison = value }
        }
    }
}

// MARK: - Shared presentation

extension HostDiffFile {
    var statusTint: Color {
        switch status {
        case "added": return Theme.statusCompleted
        case "deleted": return Theme.danger
        case "modified", "renamed", "copied": return ToolFamily.change.color ?? Theme.textMuted
        default: return Theme.warning
        }
    }
    var directory: String { let d = (path as NSString).deletingLastPathComponent; return d.isEmpty ? "" : d + "/" }
    var name: String { (path as NSString).lastPathComponent }
}

/// `+N −M` in the diff colors (used by the session header and the Changes sheet).
struct DiffStat: View {
    let additions: Int
    let deletions: Int
    var body: some View {
        HStack(spacing: 6) {
            if additions > 0 || deletions == 0 { Text("+\(additions)").foregroundStyle(Theme.statusCompleted) }
            if deletions > 0 || additions == 0 { Text("−\(deletions)").foregroundStyle(Theme.danger) }
        }
        .font(Theme.mono(12))
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(additions) additions, \(deletions) deletions")
    }
}

/// Token-colored text for one line; colors come only from the theme's syntax roles.
enum CompanionSyntax {
    static func language(forPath path: String) -> HighlightLanguage? {
        HighlightLanguage.forTag((path as NSString).pathExtension)
    }

    static func attributed(_ line: String, language: HighlightLanguage?) -> AttributedString {
        var result = AttributedString(line.isEmpty ? " " : line)
        guard let language, !line.isEmpty, line.count <= 600 else { return result }
        var carry = LineCarry()
        let chars = Array(line)
        let spans = Highlighter.tokenizeLine(chars, language: language, carry: &carry)
        for span in spans {
            guard span.range.lowerBound >= 0, span.range.upperBound <= chars.count,
                  let low = index(in: result, offset: span.range.lowerBound),
                  let high = index(in: result, offset: span.range.upperBound) else { continue }
            switch span.cls {
            case .keyword: result[low..<high].foregroundColor = Theme.tokenKeyword
            case .stringLit: result[low..<high].foregroundColor = Theme.tokenString
            case .number: result[low..<high].foregroundColor = Theme.tokenNumber
            case .comment: result[low..<high].foregroundColor = Theme.textFaint
            }
        }
        return result
    }

    private static func index(in text: AttributedString, offset: Int) -> AttributedString.Index? {
        text.characters.index(text.startIndex, offsetBy: offset, limitedBy: text.endIndex)
    }
}

// MARK: - Changes list

struct CompanionChangesView: View {
    let model: CompanionModel
    let chat: HostChat
    let store: CompanionChangesStore
    @State private var confirming = false
    @State private var discarding = false
    @State private var actionError: String?

    var body: some View {
        Group {
            if let diff = store.diff {
                if diff.files.isEmpty {
                    VStack(spacing: 0) {
                        summary(diff)
                        ContentUnavailableView("No uncommitted changes", systemImage: "checkmark",
                                               description: Text("No changes in this working tree."))
                    }
                } else {
                    List {
                        summary(diff).listRowInsets(EdgeInsets()).listRowBackground(Theme.bg)
                            .listRowSeparator(.hidden)
                        ForEach(diff.files) { file in
                            NavigationLink(value: file) { row(file) }
                                .listRowBackground(Theme.bg)
                        }
                        if diff.truncated {
                            Text("Partial snapshot. Very large diffs are cut off on the computer.")
                                .font(Theme.sans(12)).foregroundStyle(Theme.warning)
                                .listRowBackground(Theme.bg)
                        }
                    }
                    .listStyle(.plain).scrollContentBackground(.hidden)
                }
            } else if let error = store.error {
                CompanionReadError(message: error) { Task { await store.refresh(model: model, chat: chat) } }.padding(24)
            } else {
                ProgressView("Loading changes…")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Theme.bg).foregroundStyle(Theme.text)
        .navigationTitle("Changes").navigationBarTitleDisplayMode(.inline)
        .navigationDestination(for: HostDiffFile.self) { file in
            CompanionFileDiffView(model: model, chat: chat, store: store, file: file)
        }
        .toolbar {
            ToolbarItem(placement: .topBarLeading) {
                Button("Refresh", systemImage: "arrow.clockwise") { Task { await store.refresh(model: model, chat: chat) } }
                    .frame(minWidth: 44, minHeight: 44)
            }
            ToolbarItem(placement: .topBarTrailing) {
                if let diff = store.diff, !diff.files.isEmpty {
                    Button("Discard all changes", systemImage: "trash", role: .destructive) { confirming = true }
                        .tint(Theme.danger).disabled(discarding || !model.online)
                        .frame(minWidth: 44, minHeight: 44)
                }
            }
        }
        .confirmationDialog("Discard all changes?", isPresented: $confirming, titleVisibility: .visible) {
            Button("Discard all changes", role: .destructive) { discardAll() }
            Button("Cancel", role: .cancel) {}
        } message: {
            let count = store.diff?.files.count ?? 0
            Text("This permanently restores the working tree to its last commit, dropping \(count) changed \(count == 1 ? "file" : "files"). It cannot be undone.")
        }
        .alert("Could not discard changes", isPresented: Binding(get: { actionError != nil }, set: { if !$0 { actionError = nil } })) {
            Button("OK", role: .cancel) { actionError = nil }
        } message: { Text(actionError ?? "") }
    }

    private func summary(_ diff: HostCheckoutDiff) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                if let branch = chat.branch, !branch.isEmpty {
                    Label(branch, systemImage: "arrow.triangle.branch")
                        .font(Theme.mono(12)).foregroundStyle(Theme.textMuted)
                        .lineLimit(1).truncationMode(.middle)
                }
                if let c = store.comparison, c.ahead > 0 || c.behind > 0 {
                    Text([c.ahead > 0 ? "↑\(c.ahead)" : nil, c.behind > 0 ? "↓\(c.behind)" : nil]
                        .compactMap { $0 }.joined(separator: " ") + " \(c.base)")
                        .font(Theme.mono(12)).foregroundStyle(Theme.textFaint)
                        .lineLimit(1)
                }
                Spacer(minLength: 0)
            }
            HStack(spacing: 14) {
                Text("\(diff.files.count) changed \(diff.files.count == 1 ? "file" : "files")")
                    .font(Theme.mono(12)).foregroundStyle(Theme.textMuted)
                Spacer()
                DiffStat(additions: diff.additions, deletions: diff.deletions)
            }
        }
        .padding(.horizontal, 20).padding(.vertical, 12)
    }

    private func row(_ file: HostDiffFile) -> some View {
        HStack(spacing: 12) {
            Text(file.letter)
                .font(Theme.mono(12, weight: .semibold)).foregroundStyle(file.statusTint)
                .frame(width: 16)
            VStack(alignment: .leading, spacing: 1) {
                Text(file.name).font(Theme.sans(14)).lineLimit(1).truncationMode(.middle)
                if !file.directory.isEmpty {
                    Text(file.directory).font(Theme.mono(11)).foregroundStyle(Theme.textFaint)
                        .lineLimit(1).truncationMode(.head)
                }
            }
            Spacer(minLength: 8)
            if file.binary { Text("binary").font(Theme.mono(11)).foregroundStyle(Theme.textFaint) }
            else { DiffStat(additions: file.additions, deletions: file.deletions) }
        }
        .frame(minHeight: 44)
        .accessibilityElement(children: .combine)
    }

    private func discardAll() {
        guard let diff = store.diff else { return }
        discarding = true
        Task {
            do {
                try await model.discardWorkingTree(for: chat, diff: diff)
                await store.refresh(model: model, chat: chat)
            } catch { actionError = error.localizedDescription }
            discarding = false
        }
    }
}

// MARK: - Per-file diff

struct CompanionFileDiffView: View {
    let model: CompanionModel
    let chat: HostChat
    let store: CompanionChangesStore
    let file: HostDiffFile
    @State private var lines: [DiffLine]?
    @State private var note: String?
    @State private var error: String?

    var body: some View {
        Group {
            if let note {
                ContentUnavailableView(note, systemImage: "doc")
            } else if let lines {
                if lines.isEmpty {
                    ContentUnavailableView("No textual changes", systemImage: "checkmark",
                                           description: Text("This file has no line changes to show."))
                } else {
                    DiffLinesView(lines: lines, language: CompanionSyntax.language(forPath: file.path))
                }
            } else if let error {
                CompanionReadError(message: error) { Task { await load() } }.padding(24)
            } else {
                ProgressView("Loading diff…")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Theme.bg).foregroundStyle(Theme.text)
        .navigationTitle(file.name).navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .topBarTrailing) { DiffStat(additions: file.additions, deletions: file.deletions) } }
        .task(id: "\(model.generation)-\(store.diff?.checksum ?? "")") { await load() }
    }

    private func load() async {
        guard let diff = store.diff else { return }
        error = nil
        if file.binary { note = "Binary file"; return }
        let patch = diff.patch, path = file.path
        let parsed = await Task.detached { UnifiedDiff.parse(patch).first { $0.path == path } }.value
        guard !Task.isCancelled else { return }
        if let parsed {
            if parsed.binary { note = "Binary file"; return }
            note = nil; lines = parsed.lines; return
        }
        // The patch was cut off before this file: read both sides from the host.
        do {
            let text = try await model.fileDiffText(for: chat, diff: diff, path: path)
            guard !Task.isCancelled else { return }
            if text.stale == true { await store.refresh(model: model, chat: chat); return }
            if text.binary { note = "Binary file"; return }
            note = nil
            let old = text.oldText, new = text.newText
            lines = await Task.detached { UnifiedDiff.lines(old: old, new: new) }.value
        } catch { if !Task.isCancelled { self.error = error.localizedDescription } }
    }
}

/// Unified diff rows: old and new line numbers, hunk headers, add and remove
/// backgrounds, and horizontal scrolling for long lines.
struct DiffLinesView: View {
    let lines: [DiffLine]
    let language: HighlightLanguage?
    private static let charWidth = ("M" as NSString).size(withAttributes: [.font: Theme.monoUI(12)]).width
    private let gutter: CGFloat = 40

    var body: some View {
        GeometryReader { proxy in
            let longest = min(lines.lazy.filter { $0.kind != .hunk }.map { $0.text.count }.max() ?? 0, 1200)
            let width = max(proxy.size.width, (gutter + 4) * 2 + 22 + CGFloat(longest) * Self.charWidth + 24)
            ScrollView([.horizontal, .vertical]) {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(lines) { line in row(line, width: width) }
                }
                .frame(width: width, alignment: .leading)
            }
        }
    }

    @ViewBuilder private func row(_ line: DiffLine, width: CGFloat) -> some View {
        switch line.kind {
        case .hunk:
            Text(line.text).font(Theme.mono(11)).foregroundStyle(Theme.accent)
                .lineLimit(1).padding(.horizontal, 12)
                .frame(width: width, height: 26, alignment: .leading)
                .background(Theme.accent.opacity(0.08))
        case .meta:
            Text(line.text).font(Theme.mono(11)).foregroundStyle(Theme.textFaint)
                .padding(.horizontal, 12).frame(width: width, height: 22, alignment: .leading)
        default:
            HStack(spacing: 0) {
                number(line.oldNumber)
                number(line.newNumber)
                Text(line.kind == .add ? "+" : line.kind == .remove ? "−" : " ")
                    .foregroundStyle(line.kind == .add ? Theme.statusCompleted : line.kind == .remove ? Theme.danger : Theme.textFaint)
                    .frame(width: 22)
                Text(CompanionSyntax.attributed(line.text, language: language))
                    .foregroundStyle(Theme.text).lineLimit(1).fixedSize()
                Spacer(minLength: 0)
            }
            .font(Theme.mono(12))
            .frame(width: width, height: 21, alignment: .leading)
            .background(line.kind == .add ? Theme.statusCompleted.opacity(0.09)
                        : line.kind == .remove ? Theme.danger.opacity(0.09) : Color.clear)
        }
    }

    private func number(_ value: Int?) -> some View {
        Text(value.map(String.init) ?? "")
            .font(Theme.mono(11)).foregroundStyle(Theme.textFaint)
            .frame(width: gutter, alignment: .trailing).padding(.trailing, 4)
    }
}
