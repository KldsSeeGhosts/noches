import Foundation

/// One changed file in a checkout diff (`DiffFileSummary`).
struct HostDiffFile: Decodable, Identifiable, Hashable {
    let path: String
    let oldPath: String?
    let status: String
    let additions: Int
    let deletions: Int
    let binary: Bool
    var id: String { path }

    /// Single-letter git status (A/M/D/R/C/U).
    var letter: String {
        switch status {
        case "added": return "A"
        case "deleted": return "D"
        case "renamed": return "R"
        case "copied": return "C"
        case "unmerged": return "U"
        default: return "M"
        }
    }

    private enum CodingKeys: String, CodingKey { case path, oldPath, status, additions, deletions, binary }
    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        path = try c.decode(String.self, forKey: .path)
        oldPath = try c.decodeIfPresent(String.self, forKey: .oldPath)
        status = try c.decodeIfPresent(String.self, forKey: .status) ?? "modified"
        additions = try c.decodeIfPresent(Int.self, forKey: .additions) ?? 0
        deletions = try c.decodeIfPresent(Int.self, forKey: .deletions) ?? 0
        binary = try c.decodeIfPresent(Bool.self, forKey: .binary) ?? false
    }
    init(path: String, oldPath: String? = nil, status: String = "modified",
         additions: Int = 0, deletions: Int = 0, binary: Bool = false) {
        self.path = path; self.oldPath = oldPath; self.status = status
        self.additions = additions; self.deletions = deletions; self.binary = binary
    }
}

/// `CheckoutDiff` as sent by `GetCheckoutDiff` and `WatchCheckoutDiffs`.
struct HostCheckoutDiff: Decodable, Equatable {
    let checkoutId: String
    let cwd: String
    let patch: String
    let files: [HostDiffFile]
    let additions: Int
    let deletions: Int
    let truncated: Bool
    let checksum: String

    static func == (a: HostCheckoutDiff, b: HostCheckoutDiff) -> Bool {
        a.checkoutId == b.checkoutId && a.checksum == b.checksum
    }
}

/// `CheckoutFileDiffText`: the old and new file contents for one diff entry.
struct HostFileDiffText: Decodable {
    let diffChecksum: String
    let oldText: String?
    let newText: String?
    let binary: Bool
    let truncated: Bool
    let stale: Bool?
}

/// `GitHistoryComparison`: commits ahead of and behind the integration branch.
struct HostGitComparison: Decodable, Equatable {
    let base: String
    let ahead: Int
    let behind: Int
}

private struct HostGitHistoryHead: Decodable { let comparison: HostGitComparison? }

extension CompanionModel {
    /// `GetCheckoutDiff {cwd, chatId, mode: "working"}`.
    func checkoutDiff(for chat: HostChat) async throws -> HostCheckoutDiff {
        try Self.decode(HostCheckoutDiff.self, await read("GetCheckoutDiff", chat: chat,
            params: ["cwd": chat.cwd ?? "~", "chatId": chat.id, "mode": "working"]))
    }

    /// `WatchCheckoutDiffs` streams every tracked checkout's latest diff as one
    /// array per change. The stream here keeps only the frames for `checkoutId`.
    func watchCheckoutDiff(checkoutId: String) async throws -> AsyncThrowingStream<HostCheckoutDiff, Error> {
        guard online else { throw RelayError.notConnected }
        let source = try await connection.watch("WatchCheckoutDiffs")
        return AsyncThrowingStream { continuation in
            let task = Task {
                do {
                    for try await value in source {
                        guard let items = value.arrayValue else { continue }
                        for item in items where item.objectValue?["checkoutId"]?.stringValue == checkoutId {
                            continuation.yield(try Self.decode(HostCheckoutDiff.self, item))
                        }
                    }
                    continuation.finish()
                } catch { continuation.finish(throwing: error) }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    /// `GetCheckoutFileDiffText {checkoutId, cwd, path, mode: "working", diffChecksum}`.
    func fileDiffText(for chat: HostChat, diff: HostCheckoutDiff, path: String) async throws -> HostFileDiffText {
        try Self.decode(HostFileDiffText.self, await read("GetCheckoutFileDiffText", chat: chat,
            params: ["checkoutId": diff.checkoutId, "cwd": diff.cwd, "path": path,
                     "mode": "working", "diffChecksum": diff.checksum]))
    }

    /// `DiscardWorkingTree {chatId, checkoutId, expectedChecksum}`. The host
    /// refuses while an agent is active in the tree or when the checksum is stale.
    func discardWorkingTree(for chat: HostChat, diff: HostCheckoutDiff) async throws {
        _ = try await read("DiscardWorkingTree", chat: chat, params: [
            "chatId": chat.id, "checkoutId": diff.checkoutId, "expectedChecksum": diff.checksum])
    }

    /// `ListGitHistory {cwd, limit: 1}`; only the `comparison` (ahead/behind) is used.
    func gitComparison(for chat: HostChat, cwd: String) async throws -> HostGitComparison? {
        try Self.decode(HostGitHistoryHead.self, await read("ListGitHistory", chat: chat,
            params: ["cwd": cwd, "limit": 1])).comparison
    }

    /// `SearchWorkspaceFiles {chatId, query, limit}`.
    func searchWorkspaceFiles(_ query: String, chat: HostChat, limit: Int = 50) async throws -> [HostFileEntry] {
        struct Match: Decodable { let path: String; let name: String; let kind: String }
        let matches = try Self.decode([Match].self, await read("SearchWorkspaceFiles", chat: chat,
            params: ["chatId": chat.id, "query": query, "limit": limit]))
        return matches.map { HostFileEntry(path: $0.path, name: $0.name, kind: $0.kind) }
    }
}
