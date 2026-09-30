import Foundation

/// One row of a host folder listing (`FolderEntry` in `crates/proto/src/entities.rs`).
struct HostFolderEntry: Decodable, Identifiable, Hashable {
    let name: String
    let isDir: Bool
    let isRepo: Bool
    var id: String { name }
}

/// `ListFolders` reply (`FolderListing`). `path` is the resolved absolute
/// directory, so a nil request path comes back as the host's home folder.
struct HostFolderListing: Decodable, Hashable {
    let path: String
    let entries: [HostFolderEntry]
    let truncated: Bool

    init(path: String, entries: [HostFolderEntry], truncated: Bool = false) {
        self.path = path; self.entries = entries; self.truncated = truncated
    }
    private enum CodingKeys: String, CodingKey { case path, entries, truncated }
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        path = try values.decode(String.self, forKey: .path)
        entries = try values.decodeIfPresent([HostFolderEntry].self, forKey: .entries) ?? []
        truncated = try values.decodeIfPresent(Bool.self, forKey: .truncated) ?? false
    }
}

/// A mounted volume offered as a browse root (`DriveEntry`).
struct HostDrive: Decodable, Identifiable, Hashable {
    let name: String
    let path: String
    var id: String { path }
}

/// `CreateWorktree` reply (`CreateWorktreeOutcome`, flattened `Worktree`).
struct HostWorktree: Decodable, Hashable {
    let repoPath: String
    let path: String
    let branch: String
    var name: String?
    var checkoutId: String?
    var setupError: String?
}

/// Host folder browsing and project (space) management over the direct
/// connection. `WatchSpaces` owns `spaces`; writes only nudge it.
extension CompanionModel {
    func listFolders(path: String?) async throws -> HostFolderListing {
        guard online else { throw RelayError.notConnected }
        var params: [String: Any] = [:]
        if let path, !path.isEmpty { params["path"] = path }
        return try Self.decode(HostFolderListing.self, await connection.call("ListFolders", params))
    }

    /// Mounted volumes beyond home. Older hosts without `ListDrives` yield none.
    func listDrives() async throws -> [HostDrive] {
        guard online else { throw RelayError.notConnected }
        struct Listing: Decodable { var drives: [HostDrive] }
        return try Self.decode(Listing.self, await connection.call("ListDrives")).drives
    }

    /// Add a host folder as a project. A live duplicate `(device, path)` returns
    /// the existing project. Waits briefly for the row to arrive through
    /// `WatchSpaces`, then falls back to an optimistic row so the caller can
    /// select it immediately.
    @discardableResult
    func addProject(path: String, name: String?, isRepo: Bool) async throws -> HostSpace {
        guard online, let host = selected else { throw RelayError.notConnected }
        let path = Self.normalizedHostPath(path)
        guard !path.isEmpty else { throw RelayError.rpc("Choose a folder first.") }
        if let existing = localSpaces.first(where: { $0.path == path }) { return existing }
        let id = UUID().uuidString.lowercased()
        let label = name?.trimmingCharacters(in: .whitespacesAndNewlines)
        var params: [String: Any] = ["op": "createSpace", "spaceId": id, "deviceId": host.deviceId,
                                     "path": path, "gitDetected": isRepo]
        if let label, !label.isEmpty { params["name"] = label }
        _ = try await connection.call("Mutate", params)
        for _ in 0..<12 {
            if let added = spaces.first(where: { $0.id == id }) { return added }
            try? await Task.sleep(for: .milliseconds(250))
        }
        let optimistic = HostSpace(id: id, deviceId: host.deviceId, path: path,
                                   name: label?.isEmpty == false ? label : nil)
        if !spaces.contains(where: { $0.id == id }) { spaces.append(optimistic) }
        return optimistic
    }

    /// Set the display name; an empty name clears back to the folder name.
    func renameProject(_ space: HostSpace, name: String) async throws {
        guard online, space.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        let name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        var params: [String: Any] = ["op": "renameSpace", "spaceId": space.id]
        if !name.isEmpty { params["name"] = name }
        _ = try await connection.call("Mutate", params)
    }

    /// Hard delete: the host removes every session in the project.
    func removeProject(_ space: HostSpace) async throws {
        guard online, space.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        _ = try await connection.call("Mutate", ["op": "deleteSpace", "spaceId": space.id])
        spaces.removeAll { $0.id == space.id }
    }

    /// Branch names, default branch first.
    func branches(for space: HostSpace) async throws -> [String] {
        guard online, space.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        return try Self.decode([String].self, await connection.call("ListBranches", ["repoPath": space.path]))
    }

    /// An isolated checkout on a fresh `zeron/<name>` branch off `base`.
    func createWorktree(for space: HostSpace, base: String) async throws -> HostWorktree {
        guard online, space.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        let reply = try await connection.call("CreateWorktree",
                                              ["repoPath": space.path, "branch": base, "spaceId": space.id])
        return try Self.decode(HostWorktree.self, reply)
    }

    /// `create(spaceID:…)` plus the picked ref and an optional cwd override
    /// (an isolated worktree path), both accepted by `createChat`.
    func create(space: HostSpace, harness: String, model: String? = nil, reasoning: String? = nil,
                branch: String?, cwd: String?) async throws -> HostChat {
        guard online, let host = selected else { throw RelayError.notConnected }
        guard harnesses.contains(where: { $0.id == harness }),
              localSpaces.contains(where: { $0.id == space.id }) else {
            throw RelayError.rpc("Refresh the computer's projects and agents before creating a session.")
        }
        let id = UUID().uuidString.lowercased()
        var config: [String: Any] = ["harness": harness, "sandbox": "workspace-write", "modelOptions": [:]]
        config["model"] = model
        config["reasoning"] = reasoning
        var params: [String: Any] = ["op": "createChat", "chatId": id, "deviceId": host.deviceId,
                                     "spaceId": space.id, "config": config]
        if let branch, !branch.isEmpty { params["branch"] = branch }
        if let cwd, !cwd.isEmpty { params["cwd"] = cwd }
        _ = try await connection.call("Mutate", params)
        return HostChat(id: id, deviceId: host.deviceId, archived: false, cwd: cwd ?? space.path,
                        branch: branch, spaceId: space.id,
                        config: ChatConfig(harness: harness, model: model, reasoning: reasoning, sandbox: "workspace-write"),
                        createdAt: ISO8601DateFormatter().string(from: Date()))
    }

    /// Trim whitespace and a trailing separator (keeping a lone root).
    nonisolated static func normalizedHostPath(_ path: String) -> String {
        var value = path.trimmingCharacters(in: .whitespacesAndNewlines)
        while value.count > 1, value.hasSuffix("/") || value.hasSuffix("\\"),
              !(value.count == 3 && value.dropFirst().hasPrefix(":")) { value.removeLast() }
        return value
    }

    /// Parent directory of a host path, or nil at a root. Handles POSIX and drive paths.
    nonisolated static func parentHostPath(_ path: String) -> String? {
        let value = normalizedHostPath(path)
        let separator: Character = value.contains("\\") && !value.contains("/") ? "\\" : "/"
        if value == "/" || value.isEmpty { return nil }
        if value.count <= 3, value.dropFirst().hasPrefix(":") { return nil }
        guard let cut = value.lastIndex(of: separator) else { return nil }
        let head = String(value[..<cut])
        if head.isEmpty { return "/" }
        if head.count == 2, head.hasSuffix(":") { return head + String(separator) }
        return head
    }
}
