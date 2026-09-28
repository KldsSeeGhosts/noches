// Control-plane visual language: the phone's status hues, project monogram
// tones, elapsed/token formatting, and context fill rule. Every value here
// mirrors the desktop exactly - status_palette.rs, project_icon.rs,
// context_usage.rs, shell.rs, and zeron_proto::view - so the two surfaces
// read the same state the same way.

import SwiftUI

// MARK: - Appearance-paired colors

extension Color {
    /// A (dark, light) 0xRRGGBB pair resolved for the current appearance -
    /// the shared shape of the desktop's explicit status and monogram
    /// palettes. Theme roles stay theme-resolved; these hues are fixed.
    init(hexPair: (dark: UInt32, light: UInt32)) {
        let value = AppearanceSettings.shared.isDark ? hexPair.dark : hexPair.light
        self.init(
            .sRGB,
            red: Double((value >> 16) & 0xFF) / 255,
            green: Double((value >> 8) & 0xFF) / 255,
            blue: Double(value & 0xFF) / 255,
            opacity: 1)
    }
}

// MARK: - Session status

extension SessionState {
    /// status_palette.rs `SessionState::color`: Working is sky, Awaiting input
    /// is indigo, Completed-but-unseen is emerald, Failed takes the theme's
    /// danger role. Queued and Idle render neutral (`nil`).
    var color: Color? {
        switch self {
        case .failed: Theme.danger
        case .awaitingInput: Color(hexPair: (0xa5b4fc, 0x4f46e5))
        case .working: Color(hexPair: (0x7dd3fc, 0x0284c7))
        case .completed: Color(hexPair: (0x6ee7b7, 0x059669))
        case .queued, .idle: nil
        }
    }
}

extension SubagentPhase {
    /// subagents.rs `status_glyph`: Running rides the Working hue, Done the
    /// Completed hue, Failed the danger role; Started stays a neutral dot.
    var color: Color? {
        switch self {
        case .running: SessionState.working.color
        case .done: SessionState.completed.color
        case .failed: Theme.danger
        case .started: nil
        }
    }
}

// MARK: - Project monograms

enum Monogram {
    /// project_icon.rs `MONOGRAM_PALETTE`, same order so a project keeps its
    /// assigned tone across surfaces and launches.
    static let palette: [(dark: UInt32, light: UInt32)] = [
        (0x94a3b8, 0x475569),  // slate
        (0x93c5fd, 0x2563eb),  // blue
        (0xc4b5fd, 0x7c3aed),  // violet
        (0xfda4af, 0xbe123c),  // rose
        (0xfcd34d, 0xa16207),  // amber
        (0x6ee7b7, 0x047857),  // emerald
        (0x5eead4, 0x0f766e),  // teal
        (0xfdba74, 0xc2410c),  // orange
    ]

    /// FNV-1a over the seed's UTF-8 bytes, byte-exact with
    /// `project_icon.rs::monogram_tone` (UInt32 wrapping, offset basis
    /// 2166136261, prime 16777619).
    static func slot(seed: String) -> Int {
        let hash = seed.utf8.reduce(UInt32(2166136261)) { hash, byte in
            (hash ^ UInt32(byte)) &* 16777619
        }
        return Int(hash % UInt32(palette.count))
    }

    /// The stable tone for a project seed (`"home"` when there is no project).
    static func tone(seed: String) -> Color {
        Color(hexPair: palette[slot(seed: seed)])
    }

    /// The badge letter: first non-space character, uppercased, `?` when the
    /// name is empty (`project_icon.rs::monogram`).
    static func initial(_ name: String) -> String {
        name.trimmingCharacters(in: .whitespacesAndNewlines)
            .first
            .map { String($0).uppercased() } ?? "?"
    }
}

// MARK: - Elapsed time

enum Elapsed {
    /// shell.rs `format_working_elapsed`: `45s`, `2m`, `1h 4m`, `3h`.
    /// Rounds down to whole units; negative input reads `0s`.
    static func working(_ seconds: Int) -> String {
        let seconds = max(0, seconds)
        if seconds < 60 { return "\(seconds)s" }
        if seconds < 3600 { return "\(seconds / 60)m" }
        let hours = seconds / 3600
        let minutes = (seconds % 3600) / 60
        return minutes == 0 ? "\(hours)h" : "\(hours)h \(minutes)m"
    }

    /// zeron_proto::view `format_time_ago`: `now`, `5m`, `3h`, `2d`, `3w`,
    /// `2mo`, `1y`. Clock skew (a future timestamp) clamps to `now`.
    static func relative(_ date: Date, now: Date) -> String {
        let seconds = max(0, Int(now.timeIntervalSince(date)))
        if seconds < 60 { return "now" }
        let minutes = seconds / 60
        if minutes < 60 { return "\(minutes)m" }
        let hours = minutes / 60
        if hours < 24 { return "\(hours)h" }
        let days = hours / 24
        if days < 7 { return "\(days)d" }
        let weeks = days / 7
        if weeks < 5 { return "\(weeks)w" }
        let months = days / 30
        if months < 12 { return "\(months)mo" }
        return "\(days / 365)y"
    }
}

// MARK: - Token counts

enum Tokens {
    /// Compact counts for the phone's narrow context card: `999`, `5.4k`,
    /// `128k`, `1.2M`, `1B`. The desktop's mono lines group thousands; the
    /// same magnitudes collapse here so the card never wraps.
    static func compact(_ count: UInt64) -> String {
        let value = Double(count)
        let units: [(divisor: Double, suffix: String)] = [
            (1_000_000_000_000, "T"), (1_000_000_000, "B"),
            (1_000_000, "M"), (1_000, "k"),
        ]
        for (index, unit) in units.enumerated() where value >= unit.divisor {
            // 999,999 rounds to 1000k; promote to the next unit instead.
            if value / unit.divisor >= 999.95, index > 0 {
                let next = units[index - 1]
                return format(value / next.divisor, next.suffix)
            }
            return format(value / unit.divisor, unit.suffix)
        }
        return "\(count)"
    }

    /// One decimal only while it adds detail (`5.4k`, `1.2M`), never a
    /// dangling `.0` (`128k`, `200k`).
    private static func format(_ scaled: Double, _ suffix: String) -> String {
        let tenths = Int((scaled * 10).rounded())
        if tenths < 1000, tenths % 10 != 0 {
            return "\(tenths / 10).\(tenths % 10)\(suffix)"
        }
        return "\(Int(scaled.rounded()))\(suffix)"
    }
}

// MARK: - Context occupancy

/// The context ring and card-bar fill rule (context_usage.rs `fill_color`):
/// danger at >=90%, warning at >=75%, muted below, and the honest
/// post-compaction `waiting` state when nothing has been measured.
enum ContextFill: Equatable {
    case waiting
    case muted
    case warning
    case danger

    init(fraction: Double?) {
        switch fraction {
        case .some(let fraction) where fraction >= 0.9: self = .danger
        case .some(let fraction) where fraction >= 0.75: self = .warning
        case .some: self = .muted
        case .none: self = .waiting
        }
    }

    var color: Color {
        switch self {
        case .waiting: Theme.textFaint
        case .muted: Theme.textMuted
        case .warning: Theme.warning
        case .danger: Theme.danger
        }
    }
}
