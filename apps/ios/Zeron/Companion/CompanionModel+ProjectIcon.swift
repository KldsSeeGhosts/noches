import UIKit

extension CompanionModel {
    /// Raster candidates from `crates/ui/src/shell/project_icon.rs`, in
    /// priority order. SVG is skipped: the phone has no SVG renderer, so the
    /// monogram is the better fallback.
    static let projectIconPaths = [
        "public/apple-touch-icon.png",
        "apple-touch-icon.png",
        "public/favicon.png",
        "public/icon.png",
        "public/logo.png",
        "favicon.png",
        "app/icon.png",
        "src/app/icon.png",
        "public/favicon.ico",
        "favicon.ico",
        "app/favicon.ico",
        "static/favicon.ico",
        "src-tauri/icons/icon.png",
        "assets/icon.png",
        "src/assets/icon.png",
    ]
    /// `project_icon.rs` `ICON_TTL`.
    static let projectIconTTL: TimeInterval = 300
    private static let projectIconChunkLimit = 8 * 1024 * 1024
    private static let projectIconMaxChunks = 32

    /// The repository's badge artwork, or nil when the project has none (the
    /// caller draws its monogram). A miss is cached for the TTL, so a project
    /// without art is probed once per five minutes, not once per row.
    func projectIcon(for space: HostSpace) async -> UIImage? {
        guard online, space.deviceId == selected?.deviceId else { return nil }
        let key = "\(space.deviceId)/\(space.path)"
        if let entry = projectIcons[key], Date().timeIntervalSince(entry.at) < Self.projectIconTTL { return entry.image }
        if let task = projectIconTasks[key] { return await task.value }
        let task = Task { await self.loadProjectIcon(for: space) }
        projectIconTasks[key] = task
        let image = await task.value
        projectIconTasks[key] = nil
        projectIcons[key] = (image, Date())
        return image
    }

    func monogramSeed(for chat: HostChat) -> String {
        localSpaces.first { $0.id == chat.spaceId }?.path ?? "home"
    }

    func projectName(for chat: HostChat) -> String {
        localSpaces.first { $0.id == chat.spaceId }?.displayName ?? "Home"
    }

    private func loadProjectIcon(for space: HostSpace) async -> UIImage? {
        for path in Self.projectIconPaths {
            guard !Task.isCancelled else { return nil }
            guard let file = try? await workspaceCall("ReadWorkspaceFile", space, ["spaceId": space.id, "path": path]),
                  let checkoutId = file.objectValue?["checkoutId"]?.stringValue, !checkoutId.isEmpty else { continue }
            if let image = try? await readImage(space: space, path: path, checkoutId: checkoutId) { return image }
        }
        return nil
    }

    private struct WorkspaceImageChunk: Decodable {
        let checkoutId: String
        let contentHash: String
        let mimeType: String
        let data: String
        let nextOffset: Int
        let size: Int
        let done: Bool
    }

    /// `WorkspaceFilesClient::read_image`: bounded base64 chunks, identity and
    /// offset checked between frames.
    private func readImage(space: HostSpace, path: String, checkoutId: String) async throws -> UIImage? {
        var offset = 0
        var expectedHash: String?
        var bytes = Data()
        for _ in 0..<Self.projectIconMaxChunks {
            var params: [String: Any] = ["spaceId": space.id, "path": path,
                "expectedCheckoutId": checkoutId, "offset": offset]
            if let expectedHash { params["expectedContentHash"] = expectedHash }
            let chunk = try Self.decode(WorkspaceImageChunk.self, await workspaceCall("ReadWorkspaceImage", space, params))
            guard chunk.checkoutId == checkoutId,
                  let part = Data(base64Encoded: chunk.data), !part.isEmpty,
                  offset + part.count == chunk.nextOffset,
                  chunk.nextOffset <= chunk.size,
                  chunk.done == (chunk.nextOffset == chunk.size) else { return nil }
            bytes.append(part)
            guard bytes.count <= Self.projectIconChunkLimit else { return nil }
            if chunk.done { return UIImage(data: bytes) }
            offset = chunk.nextOffset
            expectedHash = chunk.contentHash
        }
        return nil
    }

    private func workspaceCall(_ method: String, _ space: HostSpace, _ params: [String: Any]) async throws -> JSONValue {
        guard online, space.deviceId == selected?.deviceId else { throw RelayError.notConnected }
        return try await connection.call(method, params)
    }
}
