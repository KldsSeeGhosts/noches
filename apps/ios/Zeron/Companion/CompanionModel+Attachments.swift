import Foundation

/// One image uploaded to the chat's host. `path` is the durable absolute path
/// the engine returned from `UploadCommit`; the host serves the bytes back
/// through `ReadAttachmentChunk`.
struct HostAttachment: Hashable {
    var path: String
    var name: String
    var mimeType: String
}

extension HostAttachment {
    /// How attachments ride the prompt (`attachments.rs` `with_attachments`):
    /// plain local paths appended to the text, which is what persists in the
    /// doc. The refs trailer is what the transcript parser strips.
    static func composed(_ text: String, paths: [String]) -> String {
        guard !paths.isEmpty else { return text }
        let body = text.isEmpty ? "See the attached image(s)." : text
        let refs = paths.map { "- \($0)" }.joined(separator: "\n")
        return "\(body)\n\nAttached images (local files - open them to view):\n\(refs)"
    }
}

extension CompanionModel {
    /// Base64 characters per `UploadChunk` (`attachments.rs`
    /// `UPLOAD_CHUNK_B64_CHARS`), sized to survive the relay's frame ceiling.
    static let uploadChunkCharacters = 680_000
    /// `use-attachments.ts` `MAX_ATTACHMENT_BYTES`.
    static let maxAttachmentBytes = 24 * 1024 * 1024

    /// Chunked upload: base64 the whole file, positional `seq` slices (a retry
    /// overwrites its own slot), then commit for the durable path.
    func uploadImage(data: Data, name: String, mimeType: String) async throws -> HostAttachment {
        guard online else { throw RelayError.notConnected }
        guard data.count <= Self.maxAttachmentBytes else {
            throw RelayError.rpc("\(name) is too large (24 MB max).")
        }
        let uploadId = UUID().uuidString.lowercased()
        let encoded = data.base64EncodedString()
        var seq = 0
        var start = encoded.startIndex
        repeat {
            let end = encoded.index(start, offsetBy: Self.uploadChunkCharacters, limitedBy: encoded.endIndex) ?? encoded.endIndex
            _ = try await connection.call("UploadChunk", ["uploadId": uploadId, "seq": seq, "data": String(encoded[start..<end])])
            start = end
            seq += 1
        } while start < encoded.endIndex
        let reply = try await connection.call("UploadCommit", ["uploadId": uploadId, "fileName": name])
        guard let path = reply.objectValue?["path"]?.stringValue else {
            throw RelayError.rpc("The host did not return the uploaded file path.")
        }
        return HostAttachment(path: path, name: name, mimeType: mimeType)
    }
}
