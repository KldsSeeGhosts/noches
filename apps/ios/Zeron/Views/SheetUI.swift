// Sheet design language — the mobile app's grouped-card system (a port of the
// old app's sheet-ui.tsx: panel cards, hairline-separated rows, centered
// headers), restated in this app's monochrome theme. Every sheet composes
// these pieces so they all feel like one product.

import SwiftUI

enum SheetStyle {
    static let cardRadius: CGFloat = 20
    static let cardFill = Theme.wash(0.045)
    static let rowSeparator = Theme.wash(0.06)
    static let panel = grey(0x14)
}

/// Grouped card: rows separated by inset hairlines.
struct SheetCard<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        VStack(spacing: 0) {
            content
        }
        .background(SheetStyle.cardFill, in: RoundedRectangle(cornerRadius: SheetStyle.cardRadius))
        .overlay(RoundedRectangle(cornerRadius: SheetStyle.cardRadius)
            .strokeBorder(Theme.wash(0.06), lineWidth: 1))
    }
}

/// Inset hairline between card rows.
struct SheetSeparator: View {
    var body: some View {
        Rectangle()
            .fill(SheetStyle.rowSeparator)
            .frame(height: 1)
            .padding(.leading, 16)
    }
}

/// Selectable row: title + optional subtitle, accent check when selected.
struct SheetSelectRow: View {
    let title: String
    var subtitle: String?
    var selected: Bool
    var leading: AnyView?
    let action: () -> Void

    var body: some View {
        Button {
            UISelectionFeedbackGenerator().selectionChanged()
            action()
        } label: {
            HStack(spacing: 12) {
                if let leading {
                    leading
                }
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(Theme.sans(15))
                        .foregroundStyle(Theme.text)
                    if let subtitle, !subtitle.isEmpty {
                        Text(subtitle)
                            .font(Theme.sans(12.5))
                            .foregroundStyle(Theme.textMuted)
                            .lineLimit(2)
                    }
                }
                Spacer(minLength: 8)
                Image(systemName: "checkmark")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(Theme.text)                    .opacity(selected ? 1 : 0)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 11)
            .contentShape(Rectangle())
        }
        .buttonStyle(SheetRowButtonStyle())
    }
}

/// Navigation-style row: title + trailing detail + chevron.
struct SheetLinkRow: View {
    let title: String
    var detail: String?
    var systemImage: String?
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 12) {
                if let systemImage {
                    Image(systemName: systemImage)
                        .font(.system(size: 15))
                        .foregroundStyle(Theme.textMuted)
                        .frame(width: 22)
                }
                Text(title)
                    .font(Theme.sans(15))
                    .foregroundStyle(Theme.text)
                Spacer(minLength: 8)
                if let detail {
                    Text(detail)
                        .font(Theme.sans(14))
                        .foregroundStyle(Theme.textMuted)
                        .lineLimit(1)
                }
                Image(systemName: "chevron.right")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(Theme.textFaint)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .contentShape(Rectangle())
        }
        .buttonStyle(SheetRowButtonStyle())
    }
}

/// Uppercase tracked section label above a card.
struct SheetLabel: View {
    let text: String

    init(_ text: String) {
        self.text = text
    }

    var body: some View {
        Text(text.uppercased())
            .font(Theme.sans(11, weight: .medium))
            .kerning(1)
            .foregroundStyle(Theme.textMuted.opacity(0.6))
            .padding(.horizontal, 4)
    }
}

/// Row press feedback: brief white wash, like UIKit cell highlight.
struct SheetRowButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(configuration.isPressed ? Theme.wash(0.06) : .clear)
    }
}

/// Primary pill button pinned at a sheet's bottom.
struct SheetPrimaryButton: View {
    let title: String
    var enabled = true
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(title)
                .font(Theme.sans(15, weight: .semibold))
                .foregroundStyle(enabled ? Theme.bg : Theme.textFaint)
                .frame(maxWidth: .infinity)
                .frame(height: 50)
                .background(enabled ? AnyShapeStyle(Theme.text) : AnyShapeStyle(Theme.wash(0.08)),
                            in: Capsule())
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
    }
}

/// Pressed-state wash for tappable rows and chips — the desktop's
/// `element_hover` (white 6%) translated to touch. Fades out on release.
struct PressWashButtonStyle: ButtonStyle {
    var cornerRadius: CGFloat = 8

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(configuration.isPressed ? Theme.elementHover : Color.clear,
                        in: RoundedRectangle(cornerRadius: cornerRadius))
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}

/// Capsule variant for chips: deepens the existing fill while pressed.
struct ChipPressButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .overlay(Capsule().fill(configuration.isPressed ? Theme.wash(0.06) : .clear))
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}

// MARK: - Companion sheet language
//
// Flat and quiet, per docs/design/control-plane.md: sentence-case section
// labels, hairline-divided rows on the shell backdrop, no filled islands, one
// pinned primary action. Every companion sheet (new session, pairing, folder
// browser, terminal) composes these so they read as one product.

enum SheetMetrics {
    /// Horizontal page margin shared by every sheet.
    static let margin: CGFloat = 16
    static let rowHeight: CGFloat = 44
}

/// 13pt medium section label with an optional trailing accessory.
struct SheetSectionLabel<Trailing: View>: View {
    let title: String
    @ViewBuilder var trailing: Trailing

    var body: some View {
        HStack(spacing: 8) {
            Text(title).font(Theme.sans(13, weight: .medium)).foregroundStyle(Theme.textMuted)
            Spacer(minLength: 0)
            trailing
        }
        .accessibilityAddTraits(.isHeader)
    }
}

extension SheetSectionLabel where Trailing == EmptyView {
    init(_ title: String) {
        self.init(title: title) { EmptyView() }
    }
}

/// A section: label, hairline-separated rows between top and bottom hairlines,
/// and a footnote attached underneath. Rows run edge to edge; the hairlines
/// inset by `separatorInset` so they line up with the row text.
struct SheetGroup<Content: View>: View {
    var title: String?
    var footer: String?
    var separatorInset: CGFloat = SheetMetrics.margin
    @ViewBuilder var content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let title {
                SheetSectionLabel(title)
                    .padding(.horizontal, SheetMetrics.margin).padding(.bottom, 8)
            }
            SheetHairline()
            Group(subviews: content) { rows in
                ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                    if index > 0 { SheetHairline().padding(.leading, separatorInset) }
                    row
                }
            }
            SheetHairline()
            if let footer {
                Text(footer)
                    .font(Theme.sans(12)).foregroundStyle(Theme.textFaint)
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(.horizontal, SheetMetrics.margin).padding(.top, 8)
            }
        }
    }
}

struct SheetHairline: View {
    var body: some View {
        Rectangle().fill(Theme.border).frame(height: 0.5)
    }
}

/// Pressed-row wash for edge-to-edge rows.
struct SheetPressStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(configuration.isPressed ? Theme.elementHover : Color.clear)
    }
}

/// Full-width segmented control: one flat track, a raised thumb on the
/// selection. Replaces fixed-width tile rows and mono pill strips.
struct SheetSegmented<Value: Hashable>: View {
    struct Option: Identifiable {
        let value: Value
        let title: String
        var icon: AnyView?
        var identifier: String?
        var id: String { identifier ?? title }
    }

    let options: [Option]
    @Binding var selection: Value
    var height: CGFloat = 40
    @Namespace private var thumb

    var body: some View {
        HStack(spacing: 2) {
            ForEach(options) { option in
                let selected = option.value == selection
                Button {
                    guard !selected else { return }
                    UISelectionFeedbackGenerator().selectionChanged()
                    withAnimation(.snappy(duration: 0.2)) { selection = option.value }
                } label: {
                    HStack(spacing: 6) {
                        if let icon = option.icon { icon }
                        Text(option.title)
                            .font(Theme.sans(13, weight: selected ? .medium : .regular))
                            .lineLimit(1).minimumScaleFactor(0.8)
                    }
                    .foregroundStyle(selected ? Theme.text : Theme.textMuted)
                    .frame(maxWidth: .infinity, minHeight: height - 6)
                    .background {
                        if selected {
                            RoundedRectangle(cornerRadius: 9, style: .continuous)
                                .fill(AppearanceSettings.shared.isDark ? Theme.wash(0.14) : Theme.bg)
                                .overlay(RoundedRectangle(cornerRadius: 9, style: .continuous)
                                    .stroke(Theme.border, lineWidth: 0.5))
                                .matchedGeometryEffect(id: "thumb", in: thumb)
                        }
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier(option.identifier ?? option.title)
                .accessibilityAddTraits(selected ? .isSelected : [])
            }
        }
        .padding(3)
        .frame(minHeight: height)
        .background(Theme.wash(0.06), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
    }
}

/// The bottom bar that pins a sheet's primary action above the home
/// indicator, separated by a hairline.
struct SheetPinnedBar<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        VStack(spacing: 8) {
            content
        }
        .padding(.horizontal, SheetMetrics.margin).padding(.top, 12).padding(.bottom, 8)
        .frame(maxWidth: .infinity)
        .background(Theme.bg)
        .overlay(alignment: .top) { SheetHairline() }
    }
}

/// Primary action as a button style, so call sites keep their own action,
/// label text, and identifier. Disabled reads as a quiet wash, not a faded
/// black pill.
struct SheetPrimaryButtonStyle: ButtonStyle {
    var enabled = true

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(Theme.sans(16, weight: .medium))
            .foregroundStyle(enabled ? Theme.bg : Theme.textFaint)
            .frame(maxWidth: .infinity, minHeight: 50)
            .background(enabled ? AnyShapeStyle(Theme.text) : AnyShapeStyle(Theme.wash(0.08)), in: Capsule())
            .opacity(configuration.isPressed ? 0.85 : 1)
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}
