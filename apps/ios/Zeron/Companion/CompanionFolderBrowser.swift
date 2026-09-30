import SwiftUI

/// A phone-native browser for the paired computer's folders (`ListFolders`,
/// `ListDrives`). Push it into a NavigationStack; descending happens in place
/// so one back tap leaves the browser. The pinned button hands the current
/// folder and its repo flag to `onChoose`.
struct CompanionFolderBrowser: View {
    let model: CompanionModel
    var title = "Choose folder"
    var actionTitle = "Use this folder"
    let onChoose: (_ path: String, _ isRepo: Bool) -> Void

    @State private var listing: HostFolderListing?
    @State private var drives: [HostDrive] = []
    @State private var currentIsRepo = false
    @State private var loading = false
    @State private var error: String?
    @State private var filter = ""
    @State private var typing = false
    @State private var typedPath = ""
    @State private var requestToken = 0
    @FocusState private var pathFocused: Bool

    private var folders: [HostFolderEntry] {
        let query = filter.trimmingCharacters(in: .whitespacesAndNewlines)
        return (listing?.entries ?? []).filter { $0.isDir && (query.isEmpty || $0.name.localizedCaseInsensitiveContains(query)) }
            .sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            Rectangle().fill(Theme.border).frame(height: 0.5)
            content
            Rectangle().fill(Theme.border).frame(height: 0.5)
            chooseBar
        }
        .background(Theme.bg).foregroundStyle(Theme.text)
        .navigationTitle(title).navigationBarTitleDisplayMode(.inline)
        .task { await start() }
    }

    // MARK: - Header

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    rootChip("Home", icon: "house") { go(nil) }
                    ForEach(drives) { drive in
                        rootChip(drive.name, icon: "externaldrive") { go(drive.path) }
                    }
                }.padding(.horizontal, 16)
            }
            if typing { pathField } else { breadcrumb }
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass").font(.system(size: 13)).foregroundStyle(Theme.textFaint)
                TextField("Filter folders", text: $filter)
                    .font(Theme.sans(15)).textInputAutocapitalization(.never).autocorrectionDisabled()
                if !filter.isEmpty {
                    Button { filter = "" } label: {
                        Image(systemName: "xmark.circle.fill").foregroundStyle(Theme.textFaint)
                            .frame(width: 44, height: 44).contentShape(Rectangle())
                    }.buttonStyle(.plain).accessibilityLabel("Clear filter")
                }
            }
            .padding(.leading, 12).frame(minHeight: 44)
            .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 12, style: .continuous).stroke(Theme.border, lineWidth: 1))
            .padding(.horizontal, 16)
        }
        .padding(.vertical, 10)
    }

    private func rootChip(_ name: String, icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: icon).font(.system(size: 12))
                Text(name).font(Theme.mono(12)).lineLimit(1)
            }
            .foregroundStyle(Theme.textMuted)
            .padding(.horizontal, 12).frame(minHeight: 44)
            .background(Theme.text.opacity(0.06), in: Capsule())
            .contentShape(Capsule())
        }.buttonStyle(.plain)
    }

    /// Tappable path segments, scrolled to the trailing (current) folder.
    private var breadcrumb: some View {
        HStack(spacing: 0) {
            ScrollViewReader { proxy in
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 2) {
                        ForEach(Array(segments.enumerated()), id: \.offset) { index, segment in
                            if index > 0 {
                                Image(systemName: "chevron.right").font(.system(size: 9, weight: .semibold))
                                    .foregroundStyle(Theme.textFaint)
                            }
                            Button { go(segment.path) } label: {
                                Text(segment.name).font(Theme.mono(12))
                                    .foregroundStyle(index == segments.count - 1 ? Theme.text : Theme.textMuted)
                                    .padding(.horizontal, 6).frame(minHeight: 44)
                                    .contentShape(Rectangle())
                            }.buttonStyle(.plain).id(index)
                        }
                    }.padding(.leading, 10)
                }
                .onChange(of: segments.count) { _, count in proxy.scrollTo(count - 1, anchor: .trailing) }
                .onAppear { proxy.scrollTo(segments.count - 1, anchor: .trailing) }
            }
            Button { typedPath = listing?.path ?? ""; typing = true; pathFocused = true } label: {
                Image(systemName: "keyboard").font(.system(size: 14)).foregroundStyle(Theme.textMuted)
                    .frame(width: 44, height: 44).contentShape(Rectangle())
            }.buttonStyle(.plain).accessibilityLabel("Type a path")
        }
    }

    private var pathField: some View {
        HStack(spacing: 4) {
            TextField("/path/to/folder", text: $typedPath)
                .font(Theme.mono(12)).textInputAutocapitalization(.never).autocorrectionDisabled()
                .keyboardType(.URL).submitLabel(.go).focused($pathFocused)
                .onSubmit { typing = false; go(typedPath) }
                .padding(.leading, 16).frame(minHeight: 44)
            Button { typing = false } label: {
                Text("Cancel").font(Theme.sans(14)).foregroundStyle(Theme.textMuted)
                    .padding(.horizontal, 12).frame(minHeight: 44).contentShape(Rectangle())
            }.buttonStyle(.plain)
        }
    }

    private var segments: [(name: String, path: String)] {
        guard let path = listing?.path, !path.isEmpty else { return [("~", "")] }
        var result: [(String, String)] = []
        var cursor: String? = path
        while let current = cursor {
            let name = (current as NSString).lastPathComponent
            result.insert((name.isEmpty || name == "/" ? current : name, current), at: 0)
            cursor = CompanionModel.parentHostPath(current)
        }
        return result
    }

    // MARK: - List

    @ViewBuilder private var content: some View {
        if let error, listing == nil {
            message(icon: "exclamationmark.triangle", text: error, retry: true)
        } else if loading, listing == nil {
            VStack { ProgressView().padding(.top, 48); Spacer() }.frame(maxWidth: .infinity)
        } else {
            ScrollView {
                LazyVStack(spacing: 0) {
                    if let parent = listing.flatMap({ CompanionModel.parentHostPath($0.path) }) {
                        row(name: "..", isRepo: false, icon: "arrow.turn.left.up") { go(parent) }
                    }
                    ForEach(folders) { entry in
                        row(name: entry.name, isRepo: entry.isRepo, icon: "folder") { descend(entry) }
                    }
                    if folders.isEmpty, !loading {
                        Text(filter.isEmpty ? "No subfolders here." : "No folders match \u{201C}\(filter)\u{201D}.")
                            .font(Theme.sans(14)).foregroundStyle(Theme.textFaint)
                            .frame(maxWidth: .infinity).padding(.top, 32)
                    }
                    if listing?.truncated == true {
                        Text("Only the first folders are shown. Filter or type a path to narrow it down.")
                            .font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
                            .multilineTextAlignment(.center).padding(20)
                    }
                    if let error {
                        Text(error).font(Theme.sans(13)).foregroundStyle(Theme.danger)
                            .multilineTextAlignment(.center).padding(20)
                    }
                }
            }
            .refreshable { await load(listing?.path, repo: currentIsRepo) }
            .scrollDismissesKeyboard(.interactively)
            .opacity(loading ? 0.5 : 1)
        }
    }

    private func row(name: String, isRepo: Bool, icon: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: icon).font(.system(size: 15)).foregroundStyle(Theme.textMuted).frame(width: 22)
                Text(name).font(Theme.sans(15)).foregroundStyle(Theme.text).lineLimit(1)
                Spacer(minLength: 8)
                if isRepo {
                    HStack(spacing: 4) {
                        LineIconView(.gitBranch, size: 12, color: Theme.textMuted)
                        Text("repo").font(Theme.mono(12)).foregroundStyle(Theme.textMuted)
                    }
                    .accessibilityLabel("Git repository")
                }
                Image(systemName: "chevron.right").font(.system(size: 11, weight: .semibold)).foregroundStyle(Theme.textFaint)
            }
            .padding(.horizontal, 16).frame(minHeight: 48)
            .contentShape(Rectangle())
        }
        .buttonStyle(CompanionPressStyle())
        .overlay(alignment: .bottom) { Rectangle().fill(Theme.border).frame(height: 0.5).padding(.leading, 50) }
    }

    private func message(icon: String, text: String, retry: Bool) -> some View {
        VStack(spacing: 12) {
            Image(systemName: icon).font(.system(size: 22)).foregroundStyle(Theme.textFaint)
            Text(text).font(Theme.sans(14)).foregroundStyle(Theme.textMuted).multilineTextAlignment(.center)
            if retry {
                Button("Try again") { go(nil) }.font(Theme.sans(15, weight: .medium)).frame(minHeight: 44)
            }
        }
        .padding(32).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top).padding(.top, 24)
    }

    private var chooseBar: some View {
        VStack(spacing: 8) {
            if let path = listing?.path {
                Text(path).font(Theme.mono(12)).foregroundStyle(Theme.textFaint)
                    .lineLimit(1).truncationMode(.head).frame(maxWidth: .infinity, alignment: .leading)
            }
            Button {
                if let listing { onChoose(listing.path, currentIsRepo) }
            } label: {
                Text(actionTitle).font(Theme.sans(16, weight: .medium))
                    .frame(maxWidth: .infinity, minHeight: 50)
                    .background(Theme.text, in: Capsule()).foregroundStyle(Theme.bg)
                    .opacity(listing == nil ? 0.4 : 1)
            }
            .disabled(listing == nil)
            .accessibilityIdentifier("use-folder")
        }
        .padding(.horizontal, 16).padding(.vertical, 12)
        .background(Theme.bg)
    }

    // MARK: - Loading

    private func start() async {
        async let found = try? model.listDrives()
        await load(nil, repo: false)
        drives = await found ?? []
    }

    private func go(_ path: String?) {
        filter = ""
        Task { await load(path?.isEmpty == true ? nil : path, repo: false) }
    }

    private func descend(_ entry: HostFolderEntry) {
        guard let base = listing?.path else { return }
        let separator = base.contains("\\") && !base.contains("/") ? "\\" : "/"
        let next = base.hasSuffix(separator) ? base + entry.name : base + separator + entry.name
        filter = ""
        Task { await load(next, repo: entry.isRepo) }
    }

    /// Latest request wins; a slow reply for an earlier folder is dropped.
    private func load(_ path: String?, repo: Bool) async {
        requestToken += 1
        let token = requestToken
        loading = true; error = nil
        do {
            let result = try await model.listFolders(path: path)
            guard token == requestToken else { return }
            listing = result
            currentIsRepo = repo
        } catch {
            guard token == requestToken else { return }
            self.error = error.localizedDescription
        }
        loading = false
    }
}
