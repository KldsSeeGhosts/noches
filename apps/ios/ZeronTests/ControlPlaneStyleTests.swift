import SwiftUI
import XCTest
@testable import Zeron

/// The phone's control-plane style values must stay byte-exact with the
/// desktop: status hues, the FNV monogram slot, elapsed formatting, and the
/// context fill buckets. Expected values are computed from the Rust sources
/// (status_palette.rs, project_icon.rs, shell.rs, zeron_proto::view, and
/// context_usage.rs), not from a second implementation.
final class ControlPlaneStyleTests: XCTestCase {
    func testMonogramSlotsMatchDesktopFNV() {
        // FNV-1a 32 over UTF-8, folded by hand from project_icon.rs.
        XCTAssertEqual(Monogram.slot(seed: "home"), 6)
        XCTAssertEqual(Monogram.slot(seed: "/Users/x/noches"), 2)
        XCTAssertEqual(Monogram.slot(seed: "studio"), 3)
        XCTAssertEqual(Monogram.slot(seed: "website"), 2)
        XCTAssertEqual(Monogram.slot(seed: "zeron"), 7)
        XCTAssertEqual(Monogram.slot(seed: ""), 5)

        // The palette keeps the desktop order; slot 6 ("home") is teal.
        XCTAssertEqual(Monogram.palette.count, 8)
        XCTAssertEqual(Monogram.palette[6].dark, 0x5eead4)
        XCTAssertEqual(Monogram.palette[6].light, 0x0f766e)
        XCTAssertEqual(Monogram.tone(seed: "home"), Color(hexPair: (0x5eead4, 0x0f766e)))

        XCTAssertEqual(Monogram.initial("  noches"), "N")
        XCTAssertEqual(Monogram.initial(""), "?")
    }

    func testElapsedWorkingMirrorsDesktop() {
        XCTAssertEqual(Elapsed.working(-3), "0s")
        XCTAssertEqual(Elapsed.working(0), "0s")
        XCTAssertEqual(Elapsed.working(45), "45s")
        XCTAssertEqual(Elapsed.working(59), "59s")
        XCTAssertEqual(Elapsed.working(60), "1m")
        XCTAssertEqual(Elapsed.working(119), "1m")
        XCTAssertEqual(Elapsed.working(3599), "59m")
        XCTAssertEqual(Elapsed.working(3600), "1h")
        XCTAssertEqual(Elapsed.working(3840), "1h 4m")
        XCTAssertEqual(Elapsed.working(10_800), "3h")
    }

    func testElapsedRelativeMirrorsDesktop() {
        let now = Date()
        func ago(_ seconds: TimeInterval) -> String {
            Elapsed.relative(now.addingTimeInterval(-seconds), now: now)
        }
        XCTAssertEqual(ago(0), "now")
        XCTAssertEqual(ago(59), "now")
        XCTAssertEqual(ago(60), "1m")
        XCTAssertEqual(ago(59 * 60), "59m")
        XCTAssertEqual(ago(3600), "1h")
        XCTAssertEqual(ago(23 * 3600 + 3599), "23h")
        XCTAssertEqual(ago(24 * 3600), "1d")
        XCTAssertEqual(ago(6 * 86400), "6d")
        XCTAssertEqual(ago(7 * 86400), "1w")
        XCTAssertEqual(ago(30 * 86400), "4w")
        XCTAssertEqual(ago(35 * 86400), "1mo")
        XCTAssertEqual(ago(400 * 86400), "1y")
        // Clock skew clamps to now.
        XCTAssertEqual(Elapsed.relative(now.addingTimeInterval(7200), now: now), "now")
    }

    func testContextFractionAndFillBuckets() throws {
        let usage = ContextUsage(tokens: 42_000, window: 200_000, compactAt: nil, session: nil)
        XCTAssertEqual(try XCTUnwrap(usage.fraction), 0.21, accuracy: 1e-12)
        XCTAssertNil(ContextUsage(tokens: nil, window: 200_000, compactAt: nil, session: nil).fraction)
        XCTAssertNil(ContextUsage(tokens: 1_000, window: 0, compactAt: nil, session: nil).fraction)
        XCTAssertNil(ContextUsage(tokens: 1_000, window: nil, compactAt: nil, session: nil).fraction)

        // context_usage.rs fill_color: muted below 75%, warning from 75%,
        // danger from 90%, waiting when unmeasured.
        XCTAssertEqual(ContextFill(fraction: nil), .waiting)
        XCTAssertEqual(ContextFill(fraction: 0), .muted)
        XCTAssertEqual(ContextFill(fraction: 0.749), .muted)
        XCTAssertEqual(ContextFill(fraction: 0.75), .warning)
        XCTAssertEqual(ContextFill(fraction: 0.899), .warning)
        XCTAssertEqual(ContextFill(fraction: 0.9), .danger)
        XCTAssertEqual(ContextFill(fraction: 1), .danger)
    }

    func testCompactTokenCounts() {
        XCTAssertEqual(Tokens.compact(0), "0")
        XCTAssertEqual(Tokens.compact(999), "999")
        XCTAssertEqual(Tokens.compact(1_000), "1k")
        XCTAssertEqual(Tokens.compact(5_417), "5.4k")
        XCTAssertEqual(Tokens.compact(9_999), "10k")
        XCTAssertEqual(Tokens.compact(128_000), "128k")
        XCTAssertEqual(Tokens.compact(200_000), "200k")
        XCTAssertEqual(Tokens.compact(999_999), "1M")
        XCTAssertEqual(Tokens.compact(1_048_576), "1M")
        XCTAssertEqual(Tokens.compact(1_200_000), "1.2M")
        XCTAssertEqual(Tokens.compact(999_999_999), "1B")
    }

    func testSessionAndSubagentColors() {
        XCTAssertEqual(SessionState.failed.color, Theme.danger)
        XCTAssertNotNil(SessionState.working.color)
        XCTAssertNotNil(SessionState.awaitingInput.color)
        XCTAssertNotNil(SessionState.completed.color)
        XCTAssertNil(SessionState.queued.color)
        XCTAssertNil(SessionState.idle.color)

        XCTAssertEqual(SubagentPhase.running.color, SessionState.working.color)
        XCTAssertEqual(SubagentPhase.done.color, SessionState.completed.color)
        XCTAssertEqual(SubagentPhase.failed.color, Theme.danger)
        XCTAssertNil(SubagentPhase.started.color)
    }
}
