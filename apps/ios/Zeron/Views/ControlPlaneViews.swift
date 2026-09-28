// Control-plane views for the phone: status glyphs and slots, project badges,
// section headers, the context ring and card, and agent pills. Every view is
// pure and data-agnostic - inputs are plain values from Models/ControlPlane
// and the caller, never a model object. Sizes follow docs/design/mobile.md;
// hues come from Theme/ControlPlaneStyle.swift.

import SwiftUI

// MARK: - Status

/// The desktop sidebar's three-bar equalizer (loaders.rs `mini_equalizer`):
/// a 1050ms wave, bars offset 0.17 apart, each breathing between 28% and
/// 100% height. Reduce motion holds every bar at full height.
private struct EqualizerBars: View {
    let tint: Color
    let size: CGFloat
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 30.0, paused: reduceMotion)) { timeline in
            let delta = timeline.date.timeIntervalSinceReferenceDate / 1.05
            HStack(alignment: .center, spacing: size / 5) {
                ForEach(0..<3, id: \.self) { bar in
                    let phase = delta + Double(bar) * 0.17
                    let wave = (cos(phase.truncatingRemainder(dividingBy: 1) * 2 * .pi) + 1) / 2
                    let height = reduceMotion ? size * 0.75 : size * 0.75 * (0.28 + 0.72 * wave)
                    Capsule()
                        .fill(tint)
                        .frame(width: size / 6, height: height)
                }
            }
            .frame(width: size * 0.9, height: size * 0.75)
        }
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }
}

/// The awaiting-input hollow ring: a gentle 2.4s breath, still under reduce
/// motion.
private struct AwaitingRing: View {
    let tint: Color
    let size: CGFloat
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 30.0, paused: reduceMotion)) { timeline in
            let pulse = (sin(timeline.date.timeIntervalSinceReferenceDate / Motion.zeronPulsePeriod * 2 * .pi) + 1) / 2
            let scale = reduceMotion ? 1 : 0.86 + 0.14 * pulse
            Circle()
                .strokeBorder(tint, lineWidth: max(1, size * 0.13))
                .frame(width: size * scale, height: size * scale)
                .opacity(reduceMotion ? 1 : 0.7 + 0.3 * pulse)
                .frame(width: size, height: size)
        }
        .accessibilityHidden(true)
    }
}

/// One session state's glyph, in `SessionState.color`: Working equalizer,
/// Awaiting input hollow ring, Completed check, Failed triangle, Queued
/// clock, Idle nothing. `size` is the square glyph box.
struct StatusGlyph: View {
    let state: SessionState
    var size: CGFloat = 12

    @ViewBuilder var body: some View {
        switch state {
        case .working:
            EqualizerBars(tint: state.color ?? Theme.textMuted, size: size)
        case .awaitingInput:
            AwaitingRing(tint: state.color ?? Theme.textMuted, size: size)
        case .completed:
            symbol("checkmark", weight: .semibold, tint: state.color ?? Theme.textMuted)
        case .failed:
            symbol("exclamationmark.triangle.fill", weight: .regular, tint: Theme.danger)
        case .queued:
            symbol("clock", weight: .regular, tint: Theme.textFaint)
        case .idle:
            EmptyView()
        }
    }

    private func symbol(_ name: String, weight: Font.Weight, tint: Color) -> some View {
        Image(systemName: name)
            .font(.system(size: size * 0.9, weight: weight))
            .foregroundStyle(tint)
            .frame(width: size, height: size)
    }
}

/// One subagent phase's glyph (subagents.rs `status_glyph`): Running
/// equalizer, Started neutral dot, Done check, Failed triangle.
struct SubagentGlyph: View {
    let phase: SubagentPhase
    var size: CGFloat = 12

    @ViewBuilder var body: some View {
        switch phase {
        case .running:
            EqualizerBars(tint: phase.color ?? Theme.textMuted, size: size)
        case .started:
            Circle()
                .fill(Theme.textFaint)
                .frame(width: max(3, size / 3), height: max(3, size / 3))
                .frame(width: size, height: size)
                .accessibilityHidden(true)
        case .done:
            Image(systemName: "checkmark")
                .font(.system(size: size * 0.9, weight: .semibold))
                .foregroundStyle(phase.color ?? Theme.textMuted)
                .frame(width: size, height: size)
        case .failed:
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: size * 0.9))
                .foregroundStyle(Theme.danger)
                .frame(width: size, height: size)
        }
    }
}

/// A card's top-right status slot: the 12pt glyph, the 13pt medium label in
/// the state hue, and - while Working - mono 12pt elapsed time from `since`,
/// ticking once a second. Idle renders nothing.
struct StatusSlot: View {
    let state: SessionState
    let since: Date?

    @ViewBuilder var body: some View {
        if state == .idle {
            EmptyView()
        } else if state == .working, since != nil {
            TimelineView(.periodic(from: .now, by: 1)) { context in
                row(
                    color: state.color ?? Theme.textFaint,
                    elapsed: since.map { Elapsed.working(Int(context.date.timeIntervalSince($0))) })
            }
        } else {
            row(color: state.color ?? Theme.textFaint, elapsed: nil)
        }
    }

    private func row(color: Color, elapsed: String?) -> some View {
        HStack(spacing: 4) {
            StatusGlyph(state: state)
            if let label = state.label {
                Text(label)
                    .font(Theme.sans(13, weight: .medium))
                    .foregroundStyle(color)
                    .lineLimit(1)
            }
            if let elapsed {
                Text(elapsed)
                    .font(Theme.mono(12))
                    .foregroundStyle(color)
            }
        }
        .fixedSize()
        .accessibilityElement(children: .combine)
    }
}

// MARK: - Project badge

/// An 18pt project badge: the repo favicon when one is available, else the
/// desktop monogram - the first letter in Geist Mono semibold on a 0.16 tone
/// wash, corners at `size * 0.28` (5pt at 18).
struct ProjectBadge: View {
    let name: String
    let seed: String
    var image: UIImage? = nil
    var size: CGFloat = 18

    private var corner: CGFloat { size * 0.28 }

    @ViewBuilder var body: some View {
        if let image {
            Image(uiImage: image)
                .resizable()
                .interpolation(.high)
                .scaledToFill()
                .frame(width: size, height: size)
                .clipShape(RoundedRectangle(cornerRadius: corner, style: .continuous))
        } else {
            let tone = Monogram.tone(seed: seed)
            Text(Monogram.initial(name))
                .font(Theme.mono(size * 0.5, weight: .semibold))
                .foregroundStyle(tone)
                .frame(width: size, height: size)
                .background(tone.opacity(0.16), in: RoundedRectangle(cornerRadius: corner, style: .continuous))
        }
    }
}

// MARK: - Section header

/// A state section header: optional 6pt dot, a mono capital eyebrow, a
/// hairline running to the right edge, and the mono count.
struct SectionHeader: View {
    let title: String
    let dotColor: Color?
    let count: Int?

    var body: some View {
        HStack(spacing: 8) {
            if let dotColor {
                Circle()
                    .fill(dotColor)
                    .frame(width: 6, height: 6)
            }
            Eyebrow(text: title, color: Theme.textMuted)
            Rectangle().fill(Theme.border).frame(height: 0.5).frame(maxWidth: .infinity)
            if let count {
                Text("\(count)")
                    .font(Theme.mono(12))
                    .foregroundStyle(Theme.textFaint)
            }
        }
    }
}

// MARK: - Context

/// The composer's context ring: an 18pt, 2pt-stroke track with a top-anchored
/// fill arc. Hidden when the harness reports no window; a dashed track marks
/// the post-compaction "waiting" state.
struct ContextRing: View {
    let usage: ContextUsage?
    var size: CGFloat = 18

    private let stroke: CGFloat = 2

    @ViewBuilder var body: some View {
        if let window = usage?.window, window > 0 {
            ring
                .frame(width: size, height: size)
                .accessibilityLabel(accessibilityLabel)
        }
    }

    private var ring: some View {
        let fill = ContextFill(fraction: usage?.fraction)
        return ZStack {
            Circle()
                .inset(by: stroke / 2)
                .stroke(
                    Theme.textFaint.opacity(0.35),
                    style: StrokeStyle(lineWidth: stroke, dash: fill == .waiting ? [2.5, 2.5] : []))
            if let fraction = usage?.fraction {
                Circle()
                    .inset(by: stroke / 2)
                    .trim(from: 0, to: fraction)
                    .stroke(fill.color, style: StrokeStyle(lineWidth: stroke, lineCap: .round))
                    .rotationEffect(.degrees(-90))
            }
        }
    }

    private var accessibilityLabel: String {
        if let fraction = usage?.fraction {
            return "\(Int((fraction * 100).rounded()))% of context window used"
        }
        return "Waiting for context usage"
    }
}

/// The context card: "Context window" with the percent in mono, a 4pt usage
/// bar, then mono 12pt detail lines. `tokens == nil` after compaction reads
/// as "Waiting for context usage", never 0%.
struct ContextCard: View {
    let usage: ContextUsage?

    private var fraction: Double? { usage?.fraction }
    private var fill: ContextFill { ContextFill(fraction: fraction) }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 16) {
                Text("Context window")
                    .font(Theme.sans(13, weight: .medium))
                    .foregroundStyle(Theme.text)
                Spacer(minLength: 0)
                if let fraction {
                    Text("\(Int((fraction * 100).rounded()))%")
                        .font(Theme.mono(12))
                        .foregroundStyle(fill.color)
                }
            }
            bar
            VStack(alignment: .leading, spacing: 2) {
                ForEach(details, id: \.self) { detail in
                    Text(detail.text)
                        .font(Theme.mono(12))
                        .foregroundStyle(detail.faint ? Theme.textFaint : Theme.textMuted)
                }
            }
        }
        .padding(12)
    }

    private var bar: some View {
        GeometryReader { geometry in
            ZStack(alignment: .leading) {
                Capsule().fill(Theme.textFaint.opacity(0.35))
                Capsule()
                    .fill(fill.color)
                    .frame(width: geometry.size.width * (fraction ?? 0))
            }
        }
        .frame(height: 4)
    }

    private struct Detail: Hashable {
        let text: String
        let faint: Bool
    }

    /// context_usage.rs `detail_lines`, with compact token counts for the
    /// phone's narrower card.
    private var details: [Detail] {
        guard let usage else {
            return [Detail(text: "Context usage not reported by this harness yet", faint: true)]
        }
        var lines: [Detail] = []
        let window = usage.window.flatMap { $0 > 0 ? $0 : nil }
        switch (usage.tokens, window) {
        case let (tokens?, window?):
            lines.append(Detail(text: "\(Tokens.compact(tokens)) / \(Tokens.compact(window)) tokens", faint: false))
            lines.append(Detail(text: "\(Tokens.compact(window - min(tokens, window))) left", faint: false))
            if let compactAt = usage.compactAt {
                let percent = Int((Double(compactAt) / Double(window) * 100).rounded())
                lines.append(Detail(text: "Auto-compacts at ~\(percent)%", faint: true))
            }
        case let (tokens?, nil):
            lines.append(Detail(text: "\(Tokens.compact(tokens)) tokens used", faint: false))
            lines.append(Detail(text: "Context limit not reported", faint: true))
        case let (nil, window?):
            lines.append(Detail(text: "\(Tokens.compact(window)) token capacity", faint: false))
            lines.append(Detail(text: "Waiting for context usage", faint: true))
        case (nil, nil):
            lines.append(Detail(text: "Context usage not reported by this harness yet", faint: true))
        }
        if let session = usage.session {
            lines.append(Detail(
                text: "\(Tokens.compact(session.input)) in · \(Tokens.compact(session.output)) out · \(Tokens.compact(session.cacheRead)) cache",
                faint: true))
        }
        return lines
    }
}

// MARK: - Agent pill

/// One composer agent pill: 32pt capsule on a 0.06 text wash, the phase
/// glyph, a truncating 13pt `textMuted` title, and mono 12pt `textFaint`
/// elapsed time.
struct AgentPill: View {
    let phase: SubagentPhase
    let title: String
    let elapsed: String?

    var body: some View {
        HStack(spacing: 6) {
            SubagentGlyph(phase: phase)
            Text(title)
                .font(Theme.sans(13))
                .foregroundStyle(Theme.textMuted)
                .lineLimit(1)
                .truncationMode(.tail)
                .frame(maxWidth: 160, alignment: .leading)
            if let elapsed {
                Text(elapsed)
                    .font(Theme.mono(12))
                    .foregroundStyle(Theme.textFaint)
            }
        }
        .padding(.horizontal, 12)
        .frame(height: 32)
        .background(Theme.text.opacity(0.06), in: Capsule())
    }
}

// MARK: - Previews

private struct ControlPlaneGallery: View {
    let dark: Bool

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 22) {
                group("Status glyphs") {
                    HStack(spacing: 16) {
                        StatusGlyph(state: .working)
                        StatusGlyph(state: .awaitingInput)
                        StatusGlyph(state: .completed)
                        StatusGlyph(state: .failed)
                        StatusGlyph(state: .queued)
                        StatusGlyph(state: .idle)
                    }
                }
                group("Status slots") {
                    VStack(alignment: .leading, spacing: 12) {
                        StatusSlot(state: .working, since: Date().addingTimeInterval(-64))
                        StatusSlot(state: .working, since: Date().addingTimeInterval(-3840))
                        StatusSlot(state: .awaitingInput, since: nil)
                        StatusSlot(state: .failed, since: nil)
                        StatusSlot(state: .completed, since: nil)
                        StatusSlot(state: .queued, since: nil)
                    }
                }
                group("Subagents") {
                    HStack(spacing: 14) {
                        SubagentGlyph(phase: .running)
                        SubagentGlyph(phase: .started)
                        SubagentGlyph(phase: .done)
                        SubagentGlyph(phase: .failed)
                    }
                }
                group("Agent pills") {
                    HStack(spacing: 8) {
                        AgentPill(phase: .running, title: "Explore the sidebar layout", elapsed: "2m")
                        AgentPill(phase: .done, title: "Write tests", elapsed: "45s")
                    }
                }
                group("Project badges") {
                    HStack(spacing: 10) {
                        ProjectBadge(name: "home", seed: "home")
                        ProjectBadge(name: "zeron", seed: "zeron")
                        ProjectBadge(name: "studio", seed: "studio")
                        ProjectBadge(name: "website", seed: "website")
                        ProjectBadge(name: "noches", seed: "/Users/x/noches", size: 24)
                        ProjectBadge(name: "icon", seed: "icon", image: Self.sampleBadge, size: 24)
                    }
                }
                group("Section headers") {
                    VStack(alignment: .leading, spacing: 12) {
                        SectionHeader(title: "Needs you", dotColor: SessionState.awaitingInput.color, count: 2)
                        SectionHeader(title: "Running", dotColor: SessionState.working.color, count: 1)
                        SectionHeader(title: "Recent", dotColor: nil, count: 6)
                    }
                }
                group("Context ring") {
                    HStack(spacing: 18) {
                        ContextRing(usage: Self.usage(0.42))
                        ContextRing(usage: Self.usage(0.8))
                        ContextRing(usage: Self.usage(0.94))
                        ContextRing(usage: Self.waiting)
                        ContextRing(usage: nil)
                    }
                }
                group("Context card") {
                    VStack(alignment: .leading, spacing: 12) {
                        ContextCard(usage: Self.full)
                            .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                        ContextCard(usage: Self.waiting)
                            .background(Theme.surfaceRaised, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                    }
                }
            }
            .padding(20)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .background(Theme.bg)
        .preferredColorScheme(dark ? .dark : .light)
        .onAppear { AppearanceSettings.shared.mode = dark ? "dark" : "light" }
    }

    @ViewBuilder
    private func group(_ title: String, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title.uppercased())
                .font(Theme.mono(11))
                .foregroundStyle(Theme.textFaint)
            content()
        }
    }

    private static func usage(_ fraction: Double) -> ContextUsage {
        ContextUsage(tokens: UInt64(200_000 * fraction), window: 200_000, compactAt: nil, session: nil)
    }

    private static let waiting = ContextUsage(tokens: nil, window: 200_000, compactAt: 168_000, session: nil)

    private static let full = ContextUsage(
        tokens: 84_000,
        window: 200_000,
        compactAt: 168_000,
        session: ContextUsage.Totals(input: 1_240_000, output: 96_000, cacheRead: 3_400_000))

    private static let sampleBadge: UIImage = {
        let size = CGSize(width: 64, height: 64)
        return UIGraphicsImageRenderer(size: size).image { context in
            let cg = context.cgContext
            cg.setFillColor(UIColor(red: 0.11, green: 0.45, blue: 0.86, alpha: 1).cgColor)
            cg.fill(CGRect(origin: .zero, size: size))
            cg.setFillColor(UIColor.white.cgColor)
            cg.fillEllipse(in: CGRect(x: 16, y: 16, width: 32, height: 32))
        }
    }()
}

#Preview("Control plane - dark") {
    ControlPlaneGallery(dark: true)
}

#Preview("Control plane - light") {
    ControlPlaneGallery(dark: false)
}
