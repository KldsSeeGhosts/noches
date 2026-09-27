import Foundation

/// RFC3339 timestamps as chrono prints them: fractional seconds of up to
/// nanosecond precision, `Z` or a numeric offset. `ISO8601DateFormatter`
/// without the right options drops the offset form, so both are tried.
enum HostDate {
    private static let fractional: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()
    private static let plain = ISO8601DateFormatter()

    static func parse(_ text: String?) -> Date? {
        guard let text = text?.trimmingCharacters(in: .whitespacesAndNewlines), !text.isEmpty else { return nil }
        if let date = fractional.date(from: trimFraction(text)) ?? plain.date(from: text) { return date }
        return nil
    }

    /// Epoch milliseconds, the queue/doc wire shape.
    static func millis(_ value: Int64?) -> Date? {
        value.map { Date(timeIntervalSince1970: Double($0) / 1000) }
    }

    /// chrono never prints more than nine fractional digits; anything longer
    /// makes ICU misread the remainder, so it is dropped before parsing.
    private static func trimFraction(_ text: String) -> String {
        guard let dot = text.firstIndex(of: ".") else { return text }
        var end = text.index(after: dot)
        var digits = 0
        while end < text.endIndex, text[end].isNumber, digits < 9 {
            digits += 1
            end = text.index(after: end)
        }
        var rest = text[end...]
        while rest.first?.isNumber == true { rest = rest.dropFirst() }
        return String(text[..<dot]) + text[dot..<end] + rest
    }
}

extension HostChat {
    /// Last transcript activity, parsed once so views never compare strings.
    var lastActivity: Date? { HostDate.parse(lastMessageAt) }
    var lastSeen: Date? { HostDate.parse(lastSeenAt) }
    var created: Date { HostDate.parse(createdAt) ?? .distantPast }
    var activityDate: Date { lastActivity ?? created }

    /// Mirrors `Chat::unseen` in crates/proto/src/entities.rs.
    var unseen: Bool {
        guard let lastActivity else { return false }
        guard let lastSeen else { return true }
        return lastActivity > lastSeen
    }
}

extension HostSession {
    var started: Date? { HostDate.parse(startedAt) }
    var updated: Date? { HostDate.parse(updatedAt) }
}
