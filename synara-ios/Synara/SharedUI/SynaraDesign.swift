import SwiftUI

enum SynaraSpacing {
    static let xSmall: CGFloat = 4
    static let small: CGFloat = 8
    static let medium: CGFloat = 12
    static let large: CGFloat = 16
    static let xLarge: CGFloat = 24
}

enum SynaraColor {
    static let surface = Color(.systemBackground)
    static let secondarySurface = Color(.secondarySystemBackground)
    static let elevatedSurface = Color(.tertiarySystemBackground)
    static let groupedSurface = Color(.systemGroupedBackground)
    static let primaryText = Color(.label)
    static let secondaryText = Color(.secondaryLabel)
    static let tertiaryText = Color(.tertiaryLabel)
    static let accent = Color.accentColor
    static let agent = Color(.systemTeal)
    static let success = Color(.systemGreen)
    static let warning = Color(.systemOrange)
    static let critical = Color(.systemRed)
    static let separator = Color(.separator)
    static let secure = Color(.systemOrange)
    static let design = Color(.systemPurple)
    static let ops = Color(.systemTeal)
    static let mutedControl = Color(.systemGray5)
    static let agentReviewBackground = Color(red: 0.06, green: 0.10, blue: 0.13)
    static let agentReviewSurface = Color(red: 0.10, green: 0.16, blue: 0.20)
}

enum SynaraRadius {
    static let small: CGFloat = 6
    static let card: CGFloat = 8
    static let control: CGFloat = 8
    static let composer: CGFloat = 22
}

enum SynaraTypography {
    static let screenTitle = Font.title2.weight(.semibold)
    static let sectionTitle = Font.headline
    static let body = Font.body
    static let supporting = Font.callout
    static let emphasis = Font.body.weight(.semibold)
    static let messageBody = Font.callout
    static let messageMeta = Font.caption
    static let roomPreview = Font.callout
    static let chipLabel = Font.caption.weight(.semibold)
    static let fineMeta = Font.caption2
    static let fineMetaBold = Font.caption2.weight(.bold)
    static let monoBody = Font.system(.callout, design: .monospaced)
    static let composerPlaceholder = Font.callout
    static let composerMetric = Font.caption2
}

struct SynaraAvatar: View {
    let title: String
    let systemImage: String?
    let tint: Color
    var size: CGFloat

    init(title: String, systemImage: String? = nil, tint: Color = SynaraColor.accent, size: CGFloat = 38) {
        self.title = title
        self.systemImage = systemImage
        self.tint = tint
        self.size = size
    }

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: SynaraRadius.card)
                .fill(tint.opacity(0.18))

            if let systemImage {
                Image(systemName: systemImage)
                    .font(.system(size: size * 0.44, weight: .semibold))
                    .foregroundStyle(tint)
            } else {
                Text(initials)
                    .font(.system(size: size * 0.34, weight: .semibold))
                    .foregroundStyle(tint)
                    .minimumScaleFactor(0.7)
            }
        }
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }

    private var initials: String {
        let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return "#"
        }

        let words = trimmed.replacingOccurrences(of: "#", with: "").split(separator: " ")
        if words.count >= 2 {
            return words.prefix(2).compactMap(\.first).map(String.init).joined().uppercased()
        }

        return String(trimmed.prefix(2)).uppercased()
    }
}

struct SynaraBrandMark: View {
    var assetName = "SynaraLogo"
    var size: CGFloat = 48
    var hasBackground = true

    var body: some View {
        Image(assetName)
            .resizable()
            .interpolation(.high)
            .scaledToFit()
            .padding(size * 0.12)
            .frame(width: size, height: size)
            .background {
                if hasBackground {
                    RoundedRectangle(cornerRadius: SynaraRadius.card)
                        .fill(SynaraColor.secondarySurface)
                }
            }
            .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.card))
            .accessibilityHidden(true)
    }
}

struct SynaraStatusChip: View {
    let title: String
    let tint: Color
    var systemImage: String?

    var body: some View {
        Label {
            Text(title)
                .font(SynaraTypography.chipLabel)
                .lineLimit(1)
                .minimumScaleFactor(0.8)
        } icon: {
            if let systemImage {
                Image(systemName: systemImage)
                    .font(SynaraTypography.chipLabel)
            }
        }
        .labelStyle(.titleAndIcon)
        .padding(.horizontal, SynaraSpacing.small)
        .padding(.vertical, SynaraSpacing.xSmall)
        .background(tint.opacity(0.14))
        .foregroundStyle(tint)
        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.small))
    }
}

struct SynaraUnreadBadge: View {
    let count: Int
    let highlighted: Bool

    var body: some View {
        if count > 0 {
            Text(count > 99 ? "99+" : "\(count)")
                .font(SynaraTypography.fineMetaBold)
                .monospacedDigit()
                .padding(.horizontal, SynaraSpacing.small)
                .frame(minWidth: 24, minHeight: 20)
                .background(highlighted ? SynaraColor.accent : SynaraColor.secondarySurface)
                .foregroundStyle(highlighted ? Color.white : SynaraColor.primaryText)
                .clipShape(Capsule())
                .accessibilityLabel("\(count) unread")
        }
    }
}

struct SynaraListRowButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(
                RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous)
                    .fill(configuration.isPressed ? SynaraColor.secondaryText.opacity(0.10) : Color.clear)
            )
            .animation(.easeOut(duration: 0.14), value: configuration.isPressed)
    }
}

struct SynaraFilterChip: View {
    let title: String
    var badgeCount: Int? = nil
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: SynaraSpacing.xSmall) {
            Text(title)
                .font(SynaraTypography.chipLabel.weight(.medium))
                    .lineLimit(1)

                if let badgeCount, badgeCount > 0 {
                    Text(badgeCount > 99 ? "99+" : "\(badgeCount)")
                        .font(SynaraTypography.fineMetaBold)
                        .monospacedDigit()
                        .padding(.horizontal, SynaraSpacing.xSmall)
                        .frame(minWidth: 18, minHeight: 18)
                        .background(isSelected ? Color.white.opacity(0.22) : SynaraColor.accent.opacity(0.14))
                        .foregroundStyle(isSelected ? Color.white : SynaraColor.accent)
                        .clipShape(Capsule())
                }
            }
            .padding(.horizontal, SynaraSpacing.medium)
            .frame(height: 32)
            .background(isSelected ? SynaraColor.accent : SynaraColor.secondarySurface)
            .foregroundStyle(isSelected ? Color.white : SynaraColor.secondaryText)
            .clipShape(Capsule())
            .overlay(
                Capsule()
                    .stroke(isSelected ? Color.clear : SynaraColor.separator.opacity(0.55), lineWidth: 0.5)
                    .allowsHitTesting(false)
            )
        }
        .buttonStyle(.plain)
        .accessibilityLabel(accessibilityLabel)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    private var accessibilityLabel: String {
        if let badgeCount, badgeCount > 0 {
            return "\(title), \(badgeCount) unread"
        }
        return title
    }
}

struct SynaraActionIconButton: View {
    let systemImage: String
    let accessibilityLabel: String
    let tint: Color
    let action: () -> Void

    init(
        systemImage: String,
        accessibilityLabel: String,
        tint: Color = SynaraColor.accent,
        action: @escaping () -> Void
    ) {
        self.systemImage = systemImage
        self.accessibilityLabel = accessibilityLabel
        self.tint = tint
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 17, weight: .semibold))
                .frame(width: 44, height: 44)
                .background(tint.opacity(0.12))
                .foregroundStyle(tint)
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control))
        }
        .buttonStyle(.plain)
        .contentShape(Rectangle())
        .accessibilityLabel(Text(accessibilityLabel))
    }
}

struct SynaraIconTile: View {
    let title: String
    let systemImage: String
    let tint: Color
    var size: CGFloat = 46

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: SynaraRadius.card)
                .fill(tint)
            Image(systemName: systemImage)
                .font(.system(size: size * 0.44, weight: .semibold))
                .foregroundStyle(.white)
        }
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }
}

struct SynaraProductHeader: View {
    let title: String
    let subtitle: String
    var systemImage: String = "sparkles"
    var assetName: String = "SynaraLogo"

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.medium) {
            SynaraBrandMark(assetName: assetName, size: 56)
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(title)
                    .font(.title2.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                Text(subtitle)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(nil)
            }
        }
        .padding(.vertical, SynaraSpacing.small)
        .accessibilityElement(children: .combine)
    }
}

struct SynaraCardModifier: ViewModifier {
    var fill: Color = SynaraColor.secondarySurface
    var stroke: Color = SynaraColor.separator.opacity(0.35)

    func body(content: Content) -> some View {
        content
            .background(fill)
            .overlay(
                RoundedRectangle(cornerRadius: SynaraRadius.card)
                    .stroke(stroke, lineWidth: 0.5)
                    .allowsHitTesting(false)
            )
            .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.card))
    }
}

extension View {
    func synaraCard(
        fill: Color = SynaraColor.secondarySurface,
        stroke: Color = SynaraColor.separator.opacity(0.35)
    ) -> some View {
        modifier(SynaraCardModifier(fill: fill, stroke: stroke))
    }
}

struct SynaraEmptyState: View {
    let title: String
    let systemImage: String
    let message: String?

    init(title: String, systemImage: String, message: String? = nil) {
        self.title = title
        self.systemImage = systemImage
        self.message = message
    }

    var body: some View {
        VStack(spacing: SynaraSpacing.large) {
            Image(systemName: systemImage)
                .font(.system(size: 42, weight: .semibold))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)

            VStack(spacing: SynaraSpacing.small) {
                Text(title)
                    .font(SynaraTypography.screenTitle)
                    .foregroundStyle(SynaraColor.primaryText)
                    .multilineTextAlignment(.center)
                    .lineLimit(nil)

                if let message {
                    Text(message)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .multilineTextAlignment(.center)
                        .lineLimit(nil)
                }
            }
        }
        .padding(SynaraSpacing.xLarge)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(SynaraColor.surface)
    }
}

struct SynaraLoadingState: View {
    let title: String

    var body: some View {
        VStack(spacing: SynaraSpacing.medium) {
            ProgressView()
            Text(title)
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.secondaryText)
                .multilineTextAlignment(.center)
        }
        .padding(SynaraSpacing.xLarge)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct SynaraShimmerModifier: ViewModifier {
    @State private var phase: CGFloat = -1

    func body(content: Content) -> some View {
        content
            .overlay {
                GeometryReader { geometry in
                    LinearGradient(
                        colors: [
                            .clear,
                            SynaraColor.elevatedSurface.opacity(0.75),
                            .clear
                        ],
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                    .frame(width: geometry.size.width * 1.8)
                    .offset(x: geometry.size.width * phase)
                }
                .mask(content)
            }
            .onAppear {
                withAnimation(.linear(duration: 1.15).repeatForever(autoreverses: false)) {
                    phase = 1.8
                }
            }
    }
}

struct SynaraSkeletonBlock: View {
    var width: CGFloat? = nil
    var height: CGFloat
    var cornerRadius: CGFloat = SynaraRadius.small

    var body: some View {
        RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
            .fill(SynaraColor.mutedControl.opacity(0.65))
            .frame(width: width, height: height)
            .modifier(SynaraShimmerModifier())
            .accessibilityHidden(true)
    }
}

struct SynaraSkeletonRow: View {
    var showsAvatar = true
    var titleWidth: CGFloat = 148
    var subtitleWidth: CGFloat = 212

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            if showsAvatar {
                SynaraSkeletonBlock(width: 42, height: 42, cornerRadius: 11)
            }

            VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                SynaraSkeletonBlock(width: titleWidth, height: 14)
                SynaraSkeletonBlock(width: subtitleWidth, height: 12)
            }

            Spacer(minLength: SynaraSpacing.small)

            SynaraSkeletonBlock(width: 28, height: 20, cornerRadius: 10)
        }
        .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
        .padding(.vertical, SynaraSpacing.xSmall)
        .accessibilityLabel("Loading")
        .accessibilityIdentifier("SynaraSkeletonRow")
    }
}

struct SynaraSkeletonList: View {
    let rowCount: Int
    var showsAvatar = true

    var body: some View {
        VStack(spacing: SynaraSpacing.small) {
            ForEach(0..<rowCount, id: \.self) { index in
                SynaraSkeletonRow(
                    showsAvatar: showsAvatar,
                    titleWidth: index.isMultiple(of: 2) ? 148 : 124,
                    subtitleWidth: index.isMultiple(of: 3) ? 196 : 228
                )
            }
        }
        .accessibilityIdentifier("SynaraSkeletonList")
    }
}

struct SynaraTimelineSkeletonRow: View {
    var isOutgoing = false
    var lineCount = 2

    var body: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.small) {
            if isOutgoing {
                Spacer(minLength: 48)
            } else {
                SynaraSkeletonBlock(width: 34, height: 34, cornerRadius: SynaraRadius.card)
            }

            VStack(alignment: isOutgoing ? .trailing : .leading, spacing: SynaraSpacing.small) {
                if isOutgoing == false {
                    SynaraSkeletonBlock(width: 92, height: 12)
                }

                VStack(alignment: isOutgoing ? .trailing : .leading, spacing: SynaraSpacing.xSmall) {
                    ForEach(0..<lineCount, id: \.self) { line in
                        SynaraSkeletonBlock(
                            width: line == lineCount - 1 ? 132 : 196,
                            height: 12
                        )
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: isOutgoing ? .trailing : .leading)

            if isOutgoing {
                SynaraSkeletonBlock(width: 34, height: 34, cornerRadius: SynaraRadius.card)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, SynaraSpacing.xSmall)
        .accessibilityLabel("Loading message")
        .accessibilityIdentifier("SynaraTimelineSkeletonRow")
    }
}

struct SynaraTimelineSkeletonList: View {
    let rowCount: Int

    var body: some View {
        VStack(spacing: SynaraSpacing.medium) {
            ForEach(0..<rowCount, id: \.self) { index in
                SynaraTimelineSkeletonRow(
                    isOutgoing: index.isMultiple(of: 3),
                    lineCount: index.isMultiple(of: 2) ? 1 : 2
                )
            }
        }
        .accessibilityIdentifier("SynaraTimelineSkeletonList")
    }
}

struct SynaraErrorState: View {
    let title: String
    let message: String
    let retry: (() -> Void)?

    var body: some View {
        VStack(spacing: SynaraSpacing.large) {
            SynaraEmptyState(title: title, systemImage: "exclamationmark.triangle", message: message)

            if let retry {
                Button("Retry", action: retry)
                    .buttonStyle(.borderedProminent)
            }
        }
    }
}

struct SynaraToolbarIconButton: View {
    let systemImage: String
    let accessibilityLabel: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .frame(width: 28, height: 28)
        }
        .accessibilityLabel(Text(accessibilityLabel))
    }
}

struct SynaraDesignTokenGallery: View {
    var body: some View {
        NavigationStack {
            List {
                Section("States") {
                    SynaraEmptyState(title: "No Rooms", systemImage: "bubble.left.and.bubble.right", message: "Joined rooms will appear here.")
                        .frame(minHeight: 220)
                    SynaraLoadingState(title: "Syncing rooms")
                        .frame(minHeight: 120)
                    SynaraSkeletonList(rowCount: 3)
                        .padding(.vertical, SynaraSpacing.small)
                }

                Section("Controls") {
                    HStack(spacing: SynaraSpacing.medium) {
                        SynaraToolbarIconButton(systemImage: "magnifyingglass", accessibilityLabel: "Search") {}
                        SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: "Account") {}
                        SynaraActionIconButton(systemImage: "paperplane.fill", accessibilityLabel: "Send") {}
                    }
                    SynaraStatusChip(title: "Agent", tint: SynaraColor.agent, systemImage: "sparkles")
                    SynaraAvatar(title: "Project Room")
                    SynaraUnreadBadge(count: 12, highlighted: true)
                }
            }
            .navigationTitle("Design Tokens")
        }
    }
}

struct SynaraDesignTokenGallery_Previews: PreviewProvider {
    static var previews: some View {
        Group {
            SynaraDesignTokenGallery()
                .preferredColorScheme(.light)
                .previewDisplayName("Light")

            SynaraDesignTokenGallery()
                .preferredColorScheme(.dark)
                .previewDisplayName("Dark")
        }
    }
}
