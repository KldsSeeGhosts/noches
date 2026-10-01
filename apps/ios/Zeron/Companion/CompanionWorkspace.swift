import SwiftUI

struct HostDirectory: Decodable {
    let directory: String
    let entries: [HostFileEntry]
    let nextCursor: String?
    let truncated: Bool
}
struct HostFileEntry: Decodable, Identifiable, Hashable {
    let path: String
    let name: String
    let kind: String
    var id: String { path }
}
/// `WorkspaceFileText`. The optional fields only exist on newer hosts; editing is
/// offered only when all of them arrive.
struct HostFileText: Decodable {
    let path: String
    let text: String?
    let encoding: String
    let truncated: Bool
    var checkoutId: String?
    var contentHash: String?
    var lineEnding: String?
    var readOnlyReason: String?

    /// Safe to write back: plain UTF-8, LF or CRLF, complete, and hash-guarded.
    var editable: Bool {
        text != nil && !truncated && readOnlyReason == nil && checkoutId != nil && contentHash != nil
            && (encoding == "utf8" || encoding == "utf8Bom")
            && (lineEnding == nil || lineEnding == "lf" || lineEnding == "crlf" || lineEnding == "none")
    }
}

enum CompanionInspector: String, Identifiable {
    case files, changes, terminal
    var id: String { rawValue }
}

struct CompanionWorkspaceSheet: View {
    @Environment(\.dismiss) private var dismiss
    let model: CompanionModel
    let chat: HostChat
    let tab: CompanionInspector
    let changes: CompanionChangesStore
    var body: some View {
        NavigationStack {
            Group {
                switch tab {
                case .files: CompanionDirectoryView(model: model, chat: chat, directory: "", close: { dismiss() })
                case .changes: CompanionChangesView(model: model, chat: chat, store: changes, close: { dismiss() })
                case .terminal: CompanionTerminalView(model: model, chat: chat)
                }
            }
            .toolbar {
                if tab != .files { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
            }
        }.tint(Theme.text).presentationDragIndicator(.visible)
    }
}

struct CompanionDirectoryView: View {
    let model: CompanionModel
    let chat: HostChat
    let directory: String
    var close: () -> Void = {}
    @State private var entries: [HostFileEntry] = []
    @State private var cursor: String?
    @State private var partial = false
    @State private var loaded = false
    @State private var busy = false
    @State private var error: String?
    @State private var query = ""
    @State private var matches: [HostFileEntry] = []
    @State private var searching = false
    private var trimmedQuery: String { query.trimmingCharacters(in: .whitespacesAndNewlines) }

    var body: some View {
        List {
            if !trimmedQuery.isEmpty {
                if let error { CompanionReadError(message: error) { Task { await search() } } }
                if searching && matches.isEmpty { ProgressView("Searching…") }
                if !searching && matches.isEmpty && error == nil {
                    Text("No files match “\(trimmedQuery)”.").foregroundStyle(Theme.textMuted)
                }
                ForEach(matches) { file in
                    NavigationLink(value: file) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(file.name).font(Theme.sans(15)).lineLimit(1)
                            Text(file.path).font(Theme.mono(11)).foregroundStyle(Theme.textFaint)
                                .lineLimit(1).truncationMode(.head)
                        }.frame(minHeight: 44, alignment: .leading)
                    }.listRowBackground(Theme.bg).listRowSeparatorTint(Theme.border)
                }
            } else {
                if let error { CompanionReadError(message: error) { Task { await load(more: false) } } }
                if !loaded && error == nil { ProgressView("Loading files…") }
                if loaded && entries.isEmpty { Text("This folder is empty.").foregroundStyle(Theme.textMuted) }
                ForEach(entries) { file in
                    NavigationLink(value: file) {
                        HStack(spacing: 12) {
                            Image(systemName: file.kind == "directory" ? "folder" : file.kind == "symlink" ? "link" : "doc.text")
                                .font(.system(size: 15)).foregroundStyle(Theme.textMuted).frame(width: 24)
                            Text(file.name).font(Theme.sans(15)).lineLimit(1).truncationMode(.middle)
                        }
                        .frame(minHeight: 44, alignment: .leading)
                    }.listRowBackground(Theme.bg).listRowSeparatorTint(Theme.border)
                }
                if cursor != nil {
                    Button(busy ? "Loading…" : "Load more files") { Task { await load(more: true) } }
                        .frame(minHeight: 44).disabled(busy)
                } else if partial { Text("The host returned a partial directory listing.").font(Theme.sans(12)).foregroundStyle(Theme.textMuted) }
            }
        }
        .listStyle(.plain).scrollContentBackground(.hidden).background(Theme.bg).foregroundStyle(Theme.text)
        .navigationTitle(directory.isEmpty ? "Files" : (directory as NSString).lastPathComponent)
        .navigationBarTitleDisplayMode(.inline)
        .navigationDestination(for: HostFileEntry.self) { file in
            if file.kind == "directory" { CompanionDirectoryView(model: model, chat: chat, directory: file.path, close: close) }
            else { CompanionFileView(model: model, chat: chat, path: file.path, close: close) }
        }
        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done", action: close) } }
        .modifier(SearchIfRoot(enabled: directory.isEmpty, text: $query))
        .task(id: model.generation) { await load(more: false) }
        .task(id: trimmedQuery) { await search() }
        .refreshable { await load(more: false) }
    }

    private func search() async {
        let text = trimmedQuery
        guard !text.isEmpty else { matches = []; searching = false; error = nil; return }
        searching = true; error = nil
        defer { if !Task.isCancelled { searching = false } }
        do {
            try await Task.sleep(for: .milliseconds(250))
            let found = try await model.searchWorkspaceFiles(text, chat: chat)
            guard !Task.isCancelled else { return }
            matches = found
        } catch { if !Task.isCancelled { self.error = error.localizedDescription } }
    }

    private func load(more: Bool) async {
        guard !busy else { return }
        busy = true; error = nil
        defer { busy = false }
        do {
            var params: [String: Any] = ["chatId": chat.id, "directory": directory]
            if more { params["cursor"] = cursor }
            let page = try CompanionModel.decode(HostDirectory.self, await model.read("ListWorkspaceDirectory", chat: chat, params: params))
            guard !Task.isCancelled else { return }
            let merged = (more ? entries : []) + page.entries
            var seen = Set<String>()
            entries = merged.filter { seen.insert($0.id).inserted }
            cursor = page.nextCursor; partial = page.truncated; loaded = true
        } catch { if !Task.isCancelled { self.error = error.localizedDescription } }
    }
}

private struct SearchIfRoot: ViewModifier {
    let enabled: Bool
    @Binding var text: String
    func body(content: Content) -> some View {
        if enabled {
            content.searchable(text: $text, placement: .navigationBarDrawer(displayMode: .automatic), prompt: "Search files")
                .textInputAutocapitalization(.never).autocorrectionDisabled()
        } else { content }
    }
}

struct CompanionFileView: View {
    let model: CompanionModel
    let chat: HostChat
    let path: String
    var close: () -> Void = {}
    @State private var file: HostFileText?
    @State private var error: String?
    @State private var editing = false
    @State private var draft = ""
    @State private var saving = false
    @State private var saveError: String?
    @State private var conflict = false
    @FocusState private var editorFocused: Bool

    var body: some View {
        Group {
            if let file {
                if editing {
                    TextEditor(text: $draft)
                        .focused($editorFocused)
                        .onAppear { editorFocused = true }
                        .font(Theme.mono(12)).scrollContentBackground(.hidden)
                        .textInputAutocapitalization(.never).autocorrectionDisabled()
                        .padding(.horizontal, 12)
                } else if let text = file.text {
                    CompanionCodeView(text: text, path: path, partial: file.truncated)
                } else {
                    ContentUnavailableView("Preview unavailable", systemImage: "doc", description: Text("This file uses \(file.encoding) encoding. Open it on your computer."))
                }
            } else if let error { CompanionReadError(message: error) { Task { await load() } }.padding(24) }
            else { ProgressView("Reading file…") }
        }.frame(maxWidth: .infinity, maxHeight: .infinity).background(Theme.bg).foregroundStyle(Theme.text)
            .navigationTitle((path as NSString).lastPathComponent).navigationBarTitleDisplayMode(.inline)
            .navigationSubtitle((path as NSString).deletingLastPathComponent)
            .navigationBarBackButtonHidden(editing)
            .toolbar {
                if editing {
                    ToolbarItem(placement: .topBarLeading) {
                        Button("Cancel") { editing = false; saveError = nil }.disabled(saving).frame(minHeight: 44)
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button(saving ? "Saving…" : "Save") { Task { await save() } }
                            .disabled(saving || draft == file?.text).frame(minHeight: 44)
                    }
                } else {
                    if file?.editable == true {
                        ToolbarItem(placement: .topBarTrailing) {
                            Button("Edit") { draft = file?.text ?? ""; editing = true }
                                .disabled(!model.online).frame(minHeight: 44)
                        }
                    }
                    ToolbarItem(placement: .confirmationAction) { Button("Done", action: close) }
                }
            }
            .safeAreaInset(edge: .bottom) {
                if let saveError {
                    Text(saveError).font(Theme.sans(13)).foregroundStyle(Theme.danger)
                        .frame(maxWidth: .infinity, alignment: .leading).padding(12).background(Theme.surfaceRaised)
                }
            }
            .alert("File changed on your computer", isPresented: $conflict) {
                Button("Reload and discard my edits", role: .destructive) { editing = false; saveError = nil; Task { await load() } }
                Button("Keep editing", role: .cancel) {}
            } message: {
                Text("Someone or something else changed this file after you opened it, so the save was not applied.")
            }
            .task(id: model.generation) { await load() }
    }

    private func load() async {
        error = nil; file = nil
        do {
            let result = try CompanionModel.decode(HostFileText.self, await model.read("ReadWorkspaceFile", chat: chat, params: ["chatId": chat.id, "path": path]))
            guard !Task.isCancelled else { return }; file = result
        } catch { if !Task.isCancelled { self.error = error.localizedDescription } }
    }

    /// `WriteWorkspaceFile` with the snapshot's checkout id and content hash, so the
    /// host rejects the write (`status: "conflict"`) if the file changed underneath.
    private func save() async {
        guard let current = file, let checkout = current.checkoutId, let hash = current.contentHash else { return }
        saving = true; saveError = nil
        defer { saving = false }
        do {
            let reply = try await model.read("WriteWorkspaceFile", chat: chat, params: [
                "chatId": chat.id, "path": path, "text": draft,
                "expectedCheckoutId": checkout, "expectedContentHash": hash,
                "encoding": current.encoding == "utf8Bom" ? "utf8Bom" : "utf8",
                "lineEnding": current.lineEnding == "crlf" ? "crlf" : "lf"])
            let object = reply.objectValue
            switch object?["status"]?.stringValue {
            case "written":
                let newHash = object?["file"]?.objectValue?["contentHash"]?.stringValue
                file = HostFileText(path: current.path, text: draft, encoding: current.encoding,
                                    truncated: false, checkoutId: checkout, contentHash: newHash ?? hash,
                                    lineEnding: current.lineEnding, readOnlyReason: nil)
                editing = false
            case "conflict": conflict = true
            default: saveError = "The computer returned an unexpected reply. Reload the file to check it."
            }
        } catch { saveError = error.localizedDescription }
    }
}

/// Read-only file preview with line numbers and syntax colors for known languages.
struct CompanionCodeView: View {
    let text: String
    let path: String
    let partial: Bool
    private let limit = 200_000
    var body: some View {
        let language = CompanionSyntax.language(forPath: path)
        let lines = String(text.prefix(limit)).components(separatedBy: "\n")
        ScrollView([.horizontal, .vertical]) {
            LazyVStack(alignment: .leading, spacing: 0) {
                if partial || text.count > limit {
                    Text("Partial preview. Open the full file on your computer.")
                        .font(Theme.sans(12)).foregroundStyle(Theme.warning).padding(.vertical, 12)
                }
                ForEach(Array(lines.enumerated()), id: \.offset) { index, line in
                    HStack(alignment: .top, spacing: 14) {
                        Text(String(index + 1)).foregroundStyle(Theme.textFaint).frame(width: 34, alignment: .trailing)
                        Text(CompanionSyntax.attributed(line, language: language)).foregroundStyle(Theme.text)
                            .lineLimit(1).fixedSize().textSelection(.enabled)
                    }.font(Theme.mono(12)).frame(minHeight: 21, alignment: .leading)
                }
            }.padding(16)
        }.background(Theme.bg)
    }
}

struct CompanionReadError: View {
    let message: String
    let retry: () -> Void
    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(message).font(Theme.sans(14)).foregroundStyle(Theme.textMuted)
            Button("Try again", action: retry).font(Theme.sans(14, weight: .medium)).frame(minHeight: 44)
        }
    }
}
