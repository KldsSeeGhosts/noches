import Foundation

/// The `@file` / `/command` token at the end of the draft. The phone's
/// TextField exposes no caret, so suggestions key off the trailing token.
struct ComposerTrigger: Equatable {
    enum Kind: Equatable { case mention, command }
    let kind: Kind
    /// Text after the `@` or `/`.
    let query: String
    /// Range of the whole token, sigil included.
    let range: Range<String.Index>

    /// The trailing token when it starts with `@` (any position), or with `/`
    /// as the very first character of the draft. Nil once whitespace follows.
    static func active(in text: String) -> ComposerTrigger? {
        guard let last = text.last, !last.isWhitespace else { return nil }
        let start = text.lastIndex(where: { $0.isWhitespace }).map { text.index(after: $0) } ?? text.startIndex
        let token = text[start...]
        guard let sigil = token.first else { return nil }
        let query = String(token.dropFirst())
        switch sigil {
        case "@": return ComposerTrigger(kind: .mention, query: query, range: start..<text.endIndex)
        case "/" where start == text.startIndex && !query.contains("/"):
            return ComposerTrigger(kind: .command, query: query, range: start..<text.endIndex)
        default: return nil
        }
    }

    /// Replace the token with the picked value plus a trailing space.
    func inserting(_ value: String, into text: String) -> String {
        var out = text
        let sigil = kind == .mention ? "@" : "/"
        out.replaceSubrange(range, with: sigil + value + " ")
        return out
    }
}

struct HostFileMatch: Decodable, Identifiable, Hashable {
    let path: String
    var isDir: Bool = false
    var id: String { path }
    var name: String { (path as NSString).lastPathComponent }
    var parent: String { (path as NSString).deletingLastPathComponent }
}

struct HostSlashCommand: Decodable, Identifiable, Hashable {
    let name: String
    var description: String = ""
    var inputHint: String?
    var id: String { name }

    private enum CodingKeys: String, CodingKey { case name, description, inputHint }
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        name = try values.decode(String.self, forKey: .name)
        description = try values.decodeIfPresent(String.self, forKey: .description) ?? ""
        inputHint = try values.decodeIfPresent(String.self, forKey: .inputHint)
    }
}

extension HostFileMatch {
    private enum CodingKeys: String, CodingKey { case path, isDir }
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy: CodingKeys.self)
        path = try values.decode(String.self, forKey: .path)
        isDir = try values.decodeIfPresent(Bool.self, forKey: .isDir) ?? false
    }
}

/// Ranking for the `/` list: prefix matches first, then substring.
func filterSlashCommands(_ commands: [HostSlashCommand], query: String) -> [HostSlashCommand] {
    let needle = query.lowercased()
    guard !needle.isEmpty else { return commands }
    let prefix = commands.filter { $0.name.lowercased().hasPrefix(needle) }
    let rest = commands.filter { !$0.name.lowercased().hasPrefix(needle) && $0.name.lowercased().contains(needle) }
    return prefix + rest
}
