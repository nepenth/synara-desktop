import SwiftUI

enum SynaraMessageBubbleAlignment {
    case own
    case other
}

enum SynaraMessageBubbleVariant {
    case standard
    case agent
    case encrypted
}

struct SynaraMessageBubble<Content: View>: View {
    let alignment: SynaraMessageBubbleAlignment
    let variant: SynaraMessageBubbleVariant
    let isGrouped: Bool
    let showsBackground: Bool
    let deliveryStatus: TimelineDeliveryStatus?
    var statusEventID: String? = nil
    var onRetryFailedSend: (() -> Void)? = nil
    @ViewBuilder let content: () -> Content

    var body: some View {
        content()
            .padding(showsBackground ? SynaraSpacing.small : 0)
            .frame(maxWidth: .infinity, alignment: frameAlignment)
            .background {
                if showsBackground {
                    bubbleShape
                        .fill(fillColor)
                }
            }
            .overlay {
                if showsBackground {
                    bubbleShape
                        .stroke(strokeColor, lineWidth: 0.5)
                        .allowsHitTesting(false)
                }
            }
            .modifier(SynaraMessageBubbleClipModifier(showsBackground: showsBackground, shape: bubbleShape))
            .opacity(deliveryStatusOpacity)
            .overlay(alignment: overlayAlignment) {
                deliveryStatusIndicator
                    .padding(.horizontal, SynaraSpacing.xSmall)
                    .padding(.vertical, SynaraSpacing.xSmall)
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
            retryChip
        default:
            EmptyView()
        }
    }

    @ViewBuilder
    private var retryChip: some View {
        let chip = Label("Retry", systemImage: "arrow.clockwise")
            .font(SynaraTypography.chipLabel)
            .foregroundStyle(SynaraColor.critical)
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
        guard showsBackground else {
            return .leading
        }

        switch alignment {
        case .own:
            return .trailing
        case .other:
            return .leading
        }
    }

    private var overlayAlignment: Alignment {
        switch alignment {
        case .own:
            return .bottomTrailing
        case .other:
            return .bottomLeading
        }
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
        case .encrypted:
            return SynaraColor.mutedControl.opacity(0.55)
        }
    }

    private var strokeColor: Color {
        switch variant {
        case .standard:
            switch alignment {
            case .own:
                return SynaraColor.accent.opacity(0.22)
            case .other:
                return SynaraColor.separator.opacity(0.35)
            }
        case .agent:
            return SynaraColor.agent.opacity(0.28)
        case .encrypted:
            return SynaraColor.separator.opacity(0.28)
        }
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

        switch alignment {
        case .other:
            return CornerRadii(
                topLeading: groupedRadius,
                bottomLeading: groupedRadius,
                bottomTrailing: largeRadius,
                topTrailing: largeRadius
            )
        case .own:
            return CornerRadii(
                topLeading: largeRadius,
                bottomLeading: largeRadius,
                bottomTrailing: groupedRadius,
                topTrailing: groupedRadius
            )
        }
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

private func synaraMessageBubbleTextColor(for variant: SynaraMessageBubbleVariant) -> Color {
    switch variant {
    case .standard, .agent:
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
        showsBackground: Bool = false,
        deliveryStatus: TimelineDeliveryStatus? = nil,
        statusEventID: String? = nil,
        onRetryFailedSend: (() -> Void)? = nil
    ) {
        self.alignment = alignment
        self.variant = variant
        self.isGrouped = isGrouped
        self.showsBackground = showsBackground
        self.deliveryStatus = deliveryStatus
        self.statusEventID = statusEventID
        self.onRetryFailedSend = onRetryFailedSend
        self.content = {
            AnyView(
                Text(text)
                    .font(SynaraTypography.messageBody)
                    .foregroundStyle(synaraMessageBubbleTextColor(for: variant))
                    .lineSpacing(2.5)
                    .lineLimit(nil)
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: showsBackground && alignment == .own ? .trailing : .leading)
                    .fixedSize(horizontal: false, vertical: true)
            )
        }
    }
}
