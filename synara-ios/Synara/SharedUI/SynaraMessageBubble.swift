import SwiftUI

enum SynaraMessageBubbleAlignment {
    case own
    case other
}

enum SynaraMessageBubbleVariant {
    case standard
    case agent
    /// An agent request that needs a decision: one neutral plane with a
    /// softened critical edge, so the risk reads once.
    case critical
    case encrypted
}

struct SynaraMessageBubble<Content: View>: View {
    let alignment: SynaraMessageBubbleAlignment
    let variant: SynaraMessageBubbleVariant
    var depth: SynaraDepthLevel = SynaraSurfaceDepthRole.standardMessage
    let isGrouped: Bool
    let showsBackground: Bool
    let deliveryStatus: TimelineDeliveryStatus?
    var statusEventID: String? = nil
    var onRetryFailedSend: (() -> Void)? = nil
    /// Discard for a failed own message; shown beside Retry, or alone when
    /// the cause cannot succeed on retry (too large, not joined).
    var onDiscardFailedSend: (() -> Void)? = nil
    /// SDK authenticity shield for a message in an encrypted room.
    var encryptionShield: TimelineEncryptionShield? = nil
    @ViewBuilder let content: () -> Content
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    var body: some View {
        // Status never covers text: the clock, check and spinner sit in a
        // reserved trailing lane, and Retry / Discard get a row of their own.
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            content()
                .padding(.trailing, statusLane.reservesTrailingLane ? trailingLaneInset : 0)
            if statusLane.showsFailedRow {
                deliveryStatusIndicator
            }
        }
        .padding(showsBackground ? SynaraSpacing.small : 0)
        .frame(maxWidth: .infinity, alignment: frameAlignment)
        .background {
            if showsBackground {
                bubbleShape
                    .fill(resolvedFillColor)
            }
        }
        .modifier(SynaraMessageBubbleClipModifier(showsBackground: showsBackground, shape: bubbleShape))
        .synaraDepth(
            showsBackground ? depth : .content,
            shape: bubbleShape,
            boundaryColor: boundaryColor
        )
        .opacity(deliveryStatusOpacity)
        .overlay(alignment: .bottomTrailing) {
            if statusLane.showsCompactIndicator {
                deliveryStatusIndicator
                    .frame(
                        width: SynaraMessageBubbleMetrics.statusIndicatorSize,
                        height: SynaraMessageBubbleMetrics.statusIndicatorSize
                    )
                    .padding(SynaraSpacing.xSmall)
            }
        }
        .overlay(alignment: .topTrailing) {
            encryptionShieldBadge
                .frame(
                    width: SynaraMessageBubbleMetrics.statusIndicatorSize,
                    height: SynaraMessageBubbleMetrics.statusIndicatorSize
                )
                .padding(SynaraSpacing.xSmall)
        }
    }

    private var statusLane: SynaraMessageBubbleStatusLane {
        SynaraMessageBubbleMetrics.statusLane(
            deliveryStatus: deliveryStatus,
            alignment: alignment,
            hasEncryptionShield: encryptionShield != nil
        )
    }

    /// Text stops short of the indicators, which sit `xSmall` in from the
    /// bubble edge while the content sits `small` in.
    private var trailingLaneInset: CGFloat {
        SynaraMessageBubbleMetrics.trailingLaneInset(contentPadding: showsBackground ? SynaraSpacing.small : 0)
    }

    @ViewBuilder
    private var encryptionShieldBadge: some View {
        if let encryptionShield {
            Image(systemName: encryptionShield.systemImageName)
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(
                    encryptionShield.tone == .red ? SynaraColor.critical : SynaraColor.secondaryText
                )
                .accessibilityLabel(encryptionShield.label)
                .accessibilityIdentifier(statusIdentifier("TimelineItemShield"))
                .help(encryptionShield.label)
        }
    }

    private var deliveryStatusOpacity: Double {
        switch deliveryStatus {
        case .sending, .queued:
            return 0.72
        default:
            return 1
        }
    }

    @ViewBuilder
    private var deliveryStatusIndicator: some View {
        switch deliveryStatus {
        case .sending where alignment == .own:
            ProgressView()
                .controlSize(.mini)
                .accessibilityLabel("Sending")
                .accessibilityIdentifier(statusIdentifier("TimelineItemSending"))
                .allowsHitTesting(false)
        case .queued where alignment == .own:
            Image(systemName: "clock")
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityLabel("Queued")
                .accessibilityIdentifier(statusIdentifier("TimelineItemQueued"))
                .allowsHitTesting(false)
        case .sent where alignment == .own:
            Image(systemName: "checkmark")
                .font(.system(size: 11, weight: .bold))
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityLabel("Sent")
                .allowsHitTesting(false)
        case .failed where alignment == .own:
            HStack(spacing: SynaraSpacing.xSmall) {
                if onRetryFailedSend != nil || onDiscardFailedSend == nil {
                    retryChip
                }
                if let onDiscardFailedSend {
                    discardChip(onDiscardFailedSend)
                }
            }
        default:
            EmptyView()
        }
    }

    private func discardChip(_ action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Label("Discard", systemImage: "trash")
                .font(SynaraTypography.chipLabel)
                .foregroundStyle(SynaraColor.secondaryText)
                .padding(.horizontal, SynaraSpacing.small)
                .padding(.vertical, SynaraSpacing.xSmall)
                .background(SynaraColor.secondaryText.opacity(0.12))
                .clipShape(Capsule())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Discard unsent message")
        .accessibilityIdentifier(statusIdentifier("TimelineItemDiscard"))
    }

    @ViewBuilder
    private var retryChip: some View {
        let chip = Label("Retry", systemImage: "arrow.clockwise")
            .font(SynaraTypography.chipLabel)
            .foregroundStyle(SynaraColor.criticalText)
            .padding(.horizontal, SynaraSpacing.small)
            .padding(.vertical, SynaraSpacing.xSmall)
            .background(SynaraColor.critical.opacity(0.12))
            .clipShape(Capsule())

        if let onRetryFailedSend {
            Button(action: onRetryFailedSend) {
                chip
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Failed to send. Tap to retry.")
            .accessibilityIdentifier(statusIdentifier("TimelineItemRetry"))
        } else {
            chip
                .accessibilityLabel("Failed to send. Tap to retry.")
                .accessibilityIdentifier(statusIdentifier("TimelineItemRetry"))
                .allowsHitTesting(false)
        }
    }

    private func statusIdentifier(_ prefix: String) -> String {
        if let statusEventID, statusEventID.isEmpty == false {
            return "\(prefix)-\(statusEventID)"
        }
        return prefix
    }

    private var frameAlignment: Alignment {
        SynaraMessageBubbleMetrics.usesLeadingContentColumn(alignment: alignment)
            ? .leading
            : .trailing
    }

    private var bubbleShape: UnevenRoundedRectangle {
        let corners = SynaraMessageBubbleMetrics.cornerRadii(
            alignment: alignment,
            isGrouped: isGrouped
        )
        return UnevenRoundedRectangle(
            topLeadingRadius: corners.topLeading,
            bottomLeadingRadius: corners.bottomLeading,
            bottomTrailingRadius: corners.bottomTrailing,
            topTrailingRadius: corners.topTrailing,
            style: .continuous
        )
    }

    private var fillColor: Color {
        switch variant {
        case .standard:
            switch alignment {
            case .own:
                return SynaraColor.accent.opacity(0.12)
            case .other:
                return SynaraColor.secondarySurface
            }
        case .agent:
            return SynaraColor.agent.opacity(0.08)
        case .critical:
            return SynaraColor.secondarySurface
        case .encrypted:
            return SynaraColor.mutedControl.opacity(0.55)
        }
    }

    private var resolvedFillColor: Color {
        guard reduceTransparency else {
            return fillColor
        }

        switch variant {
        case .standard:
            return alignment == .own ? SynaraColor.elevatedSurface : SynaraColor.secondarySurface
        case .agent:
            return SynaraColor.agentReviewSurface
        case .critical:
            return SynaraColor.secondarySurface
        case .encrypted:
            return SynaraColor.mutedControl
        }
    }

    private var boundaryColor: Color {
        switch variant {
        case .standard:
            switch alignment {
            case .own:
                return SynaraColor.accent
            case .other:
                return SynaraColor.separator
            }
        case .agent:
            return SynaraColor.agent
        case .critical:
            return SynaraColor.critical.opacity(0.6)
        case .encrypted:
            return SynaraColor.separator
        }
    }

}

enum SynaraMessageBubbleStatusLane: Equatable {
    case none
    /// Encryption shield at the top-trailing corner; no delivery status.
    case shieldOnly
    /// Clock, spinner or check at the bottom-trailing corner.
    case compactTrailing
    /// Retry / Discard chips on their own row under the content.
    case failedRow
    case failedRowWithShield

    var reservesTrailingLane: Bool {
        self == .shieldOnly || self == .compactTrailing || self == .failedRowWithShield
    }

    var showsCompactIndicator: Bool {
        self == .compactTrailing
    }

    var showsFailedRow: Bool {
        self == .failedRow || self == .failedRowWithShield
    }
}

enum SynaraMessageBubbleMetrics {
    static let largeRadius: CGFloat = 16
    static let groupedRadius: CGFloat = 6

    struct CornerRadii: Equatable {
        let topLeading: CGFloat
        let bottomLeading: CGFloat
        let bottomTrailing: CGFloat
        let topTrailing: CGFloat
    }

    static func usesLeadingContentColumn(alignment _: SynaraMessageBubbleAlignment) -> Bool {
        true
    }

    /// The box the clock, check, spinner and shield are drawn in.
    static let statusIndicatorSize: CGFloat = 16

    /// Where a bubble shows its delivery status. Own messages only; the
    /// shield shares the trailing lane at the top corner.
    static func statusLane(
        deliveryStatus: TimelineDeliveryStatus?,
        alignment: SynaraMessageBubbleAlignment,
        hasEncryptionShield: Bool
    ) -> SynaraMessageBubbleStatusLane {
        if alignment == .own {
            switch deliveryStatus {
            case .sending, .queued, .sent:
                return .compactTrailing
            case .failed:
                return hasEncryptionShield ? .failedRowWithShield : .failedRow
            default:
                break
            }
        }
        return hasEncryptionShield ? .shieldOnly : .none
    }

    /// Extra trailing inset for content so it ends before an indicator that
    /// sits `xSmall` in from the bubble edge, with an `xSmall` gap.
    static func trailingLaneInset(contentPadding: CGFloat) -> CGFloat {
        max(0, SynaraSpacing.xSmall + statusIndicatorSize + SynaraSpacing.xSmall - contentPadding)
    }

    static func cornerRadii(
        alignment: SynaraMessageBubbleAlignment,
        isGrouped: Bool
    ) -> CornerRadii {
        guard isGrouped else {
            return CornerRadii(
                topLeading: largeRadius,
                bottomLeading: largeRadius,
                bottomTrailing: largeRadius,
                topTrailing: largeRadius
            )
        }

        return CornerRadii(
            topLeading: groupedRadius,
            bottomLeading: groupedRadius,
            bottomTrailing: largeRadius,
            topTrailing: largeRadius
        )
    }
}

private struct SynaraMessageBubbleClipModifier: ViewModifier {
    let showsBackground: Bool
    let shape: UnevenRoundedRectangle

    func body(content: Content) -> some View {
        if showsBackground {
            content.clipShape(shape)
        } else {
            content
        }
    }
}

/// Bare web links and email addresses in a plain-text body, made tappable.
/// HTML bodies keep the links their sender wrote.
enum SynaraPlainTextLinks {
    static let allowedSchemes: Set<String> = ["http", "https", "mailto"]

    private static let detector = try? NSDataDetector(types: NSTextCheckingResult.CheckingType.link.rawValue)
    private static let cache: NSCache<NSString, CachedText> = {
        let cache = NSCache<NSString, CachedText>()
        cache.countLimit = 512
        return cache
    }()

    private final class CachedText {
        let value: AttributedString

        init(_ value: AttributedString) {
            self.value = value
        }
    }

    /// The links in `text`, in order, as (UTF-16 range, URL).
    static func links(in text: String) -> [(range: NSRange, url: URL)] {
        guard let detector, text.isEmpty == false else {
            return []
        }
        let matches = detector.matches(in: text, range: NSRange(location: 0, length: (text as NSString).length))
        return matches.compactMap { match in
            guard let url = match.url,
                  let scheme = url.scheme?.lowercased(),
                  allowedSchemes.contains(scheme)
            else {
                return nil
            }
            return (match.range, url)
        }
    }

    /// `text` with a `.link` on each detected link; cached by body.
    static func attributedText(_ text: String) -> AttributedString {
        let key = text as NSString
        if let cached = cache.object(forKey: key) {
            return cached.value
        }
        var attributed = AttributedString(text)
        for link in links(in: text) {
            guard let stringRange = Range(link.range, in: text),
                  let lower = AttributedString.Index(stringRange.lowerBound, within: attributed),
                  let upper = AttributedString.Index(stringRange.upperBound, within: attributed)
            else {
                continue
            }
            attributed[lower ..< upper].link = link.url
        }
        cache.setObject(CachedText(attributed), forKey: key)
        return attributed
    }
}

private func synaraMessageBubbleTextColor(for variant: SynaraMessageBubbleVariant) -> Color {
    switch variant {
    case .standard, .agent, .critical:
        return SynaraColor.primaryText
    case .encrypted:
        return SynaraColor.secondaryText
    }
}

extension SynaraMessageBubble where Content == AnyView {
    init(
        text: String,
        alignment: SynaraMessageBubbleAlignment,
        variant: SynaraMessageBubbleVariant = .standard,
        isGrouped: Bool = false,
        showsBackground: Bool = SynaraSurfaceDepthRole.standardMessageShowsBackground,
        deliveryStatus: TimelineDeliveryStatus? = nil,
        statusEventID: String? = nil,
        onRetryFailedSend: (() -> Void)? = nil,
        onDiscardFailedSend: (() -> Void)? = nil,
        encryptionShield: TimelineEncryptionShield? = nil
    ) {
        self.alignment = alignment
        self.variant = variant
        self.isGrouped = isGrouped
        self.showsBackground = showsBackground
        self.deliveryStatus = deliveryStatus
        self.statusEventID = statusEventID
        self.onRetryFailedSend = onRetryFailedSend
        self.onDiscardFailedSend = onDiscardFailedSend
        self.encryptionShield = encryptionShield
        self.content = {
            AnyView(
                Text(SynaraPlainTextLinks.attributedText(text))
                    .font(SynaraTypography.messageBody)
                    .foregroundStyle(synaraMessageBubbleTextColor(for: variant))
                    .lineSpacing(2.5)
                    .lineLimit(nil)
                    .textSelection(.enabled)
                    .frame(
                        maxWidth: .infinity,
                        alignment: SynaraMessageBubbleMetrics.usesLeadingContentColumn(alignment: alignment)
                            ? .leading
                            : .trailing
                    )
                    .fixedSize(horizontal: false, vertical: true)
            )
        }
    }
}
