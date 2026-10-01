// The Noches identity: a crescent over the night palace. Las Noches is a white
// fortress under a moon that never sets, so the brand is exactly that - one
// crescent mark and a small "Noches" wordmark, used with restraint: the home
// header and the signed-out hero (Cinzel display capitals). Everything working
// (cards, metadata, controls) stays in Geist and Geist Mono.
//
// Brand surfaces are monochrome - `Theme.text` on the shell - so they never
// compete with the state hues the control plane reserves color for.

import SwiftUI

extension Theme {
    /// Cinzel is a variable font; the named instances resolve by PostScript
    /// name. Roman capitals only - use for the wordmark and display lines.
    static func display(_ size: CGFloat, weight: Font.Weight = .regular) -> Font {
        let name = weight == .bold || weight == .semibold || weight == .heavy || weight == .black
            ? "CinzelRoman-Bold" : "Cinzel-Regular"
        return .custom(name, size: size)
    }
}

/// The crescent: a full disc with a smaller, offset disc carved out, leaning
/// toward the upper right like a waxing moon over the palace.
struct CrescentShape: Shape {
    /// Radius of the carved disc relative to the moon (0...1).
    var bite: CGFloat = 0.82
    /// Offset of the carved disc, as a fraction of the moon's radius.
    var offset = CGSize(width: 0.36, height: -0.26)

    func path(in rect: CGRect) -> Path {
        let r = min(rect.width, rect.height) / 2
        let center = CGPoint(x: rect.midX, y: rect.midY)
        let moon = Path(ellipseIn: CGRect(x: center.x - r, y: center.y - r, width: r * 2, height: r * 2))
        let br = r * bite
        let bc = CGPoint(x: center.x + offset.width * r, y: center.y + offset.height * r)
        let carve = Path(ellipseIn: CGRect(x: bc.x - br, y: bc.y - br, width: br * 2, height: br * 2))
        return moon.subtracting(carve)
    }
}

/// The mark at any size, in the current text color.
struct NochesMark: View {
    var size: CGFloat = 18
    var body: some View {
        CrescentShape()
            .fill(Theme.text)
            .frame(width: size, height: size)
            .accessibilityHidden(true)
    }
}

/// The header lockup: a small crescent and "Noches" in the UI sans, sized
/// like a navigation title so it never competes with the list below it.
struct NochesWordmark: View {
    var size: CGFloat = 17
    var body: some View {
        HStack(spacing: 8) {
            NochesMark(size: size)
            Text("Noches")
                .font(Theme.sans(size, weight: .semibold))
                .tracking(-0.3)
                .foregroundStyle(Theme.text)
        }
        .fixedSize()
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Noches")
        .accessibilityAddTraits(.isHeader)
    }
}

/// A control-plane eyebrow: mono 11pt capitals, widely tracked, `textFaint`.
/// Used for section headers and small structural labels - the quiet grid the
/// cards hang from.
struct Eyebrow: View {
    let text: String
    var color: Color = Theme.textFaint
    var body: some View {
        Text(text)
            .textCase(.uppercase)
            .font(Theme.mono(11, weight: .medium))
            .tracking(1.3)
            .foregroundStyle(color)
            .lineLimit(1)
            .accessibilityLabel(text)
    }
}

/// The signed-out hero: the crescent risen over the palace. A hairline
/// horizon carrying a fine colonnade under a large moon - the only
/// illustrative moment in the app. The moon rises once on arrival; reduce
/// motion shows it in place.
struct NochesNightHero: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var risen = false

    var body: some View {
        GeometryReader { geo in
            let w = geo.size.width
            let h = geo.size.height
            let moon = min(w, h) * 0.40
            let horizon = h * 0.86
            ZStack {
                // Moon halo: the only soft light in the product. Night only -
                // on a light page a halo reads as a smudge.
                if AppearanceSettings.shared.isDark {
                    Circle()
                        .fill(RadialGradient(colors: [Theme.text.opacity(0.08), Theme.text.opacity(0)],
                                             center: .center, startRadius: moon * 0.3, endRadius: moon * 1.25))
                        .frame(width: moon * 2.6, height: moon * 2.6)
                        .position(x: w * 0.5, y: h * 0.38)
                }
                CrescentShape()
                    .fill(Theme.text)
                    .frame(width: moon, height: moon)
                    .position(x: w * 0.5, y: h * 0.38)
                    .offset(y: risen || reduceMotion ? 0 : 10)
                    .opacity(risen || reduceMotion ? 1 : 0)
                Colonnade()
                    .stroke(Theme.text.opacity(0.42), lineWidth: 1)
                    .frame(width: w * 0.56, height: 30)
                    .position(x: w * 0.5, y: horizon - 15)
                Rectangle()
                    .fill(LinearGradient(colors: [Theme.text.opacity(0), Theme.text.opacity(0.55), Theme.text.opacity(0)],
                                         startPoint: .leading, endPoint: .trailing))
                    .frame(width: w, height: 0.75)
                    .position(x: w * 0.5, y: horizon)
            }
        }
        .accessibilityHidden(true)
        .onAppear {
            guard !reduceMotion else { risen = true; return }
            withAnimation(.timingCurve(0.16, 1, 0.3, 1, duration: 1.4)) { risen = true }
        }
    }
}

/// A drawn-in-line colonnade: an architrave, evenly spaced columns with
/// small capitals, and a two-step base. Stroked at 1pt, it reads as an
/// architect's elevation rather than a clip-art building.
struct Colonnade: Shape {
    var columns = 13
    func path(in rect: CGRect) -> Path {
        var p = Path()
        let top = rect.minY + 1, bottom = rect.maxY
        let inset = rect.width * 0.04
        func line(_ a: CGPoint, _ b: CGPoint) { p.move(to: a); p.addLine(to: b) }
        // Architrave: two parallel lines.
        line(CGPoint(x: rect.minX, y: top), CGPoint(x: rect.maxX, y: top))
        line(CGPoint(x: rect.minX + inset * 0.5, y: top + 4), CGPoint(x: rect.maxX - inset * 0.5, y: top + 4))
        // Base steps.
        line(CGPoint(x: rect.minX + inset * 0.5, y: bottom - 4), CGPoint(x: rect.maxX - inset * 0.5, y: bottom - 4))
        // Columns with a small capital tick.
        let span = rect.width - inset * 2
        for i in 0..<columns {
            let x = rect.minX + inset + span * CGFloat(i) / CGFloat(columns - 1)
            line(CGPoint(x: x, y: top + 6), CGPoint(x: x, y: bottom - 5))
            line(CGPoint(x: x - 2.5, y: top + 6), CGPoint(x: x + 2.5, y: top + 6))
        }
        return p
    }
}
