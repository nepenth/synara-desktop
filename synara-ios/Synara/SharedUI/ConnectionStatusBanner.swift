import SwiftUI

/// Signed-in connection/sync chrome. Hidden in steady Connected; Lost waits
/// for the store hold so SDK blips do not paint a banner.
///
/// `onSignOut` only asks for sign-out: the owner shows the same confirmation
/// as Settings → Log Out before anything is wiped.
struct ConnectionStatusBanner: View {
    @ObservedObject var store: ConnectionStatusStore
    var onRetry: (() -> Void)?
    var onSignOut: (() -> Void)?

    var body: some View {
        if store.presentsBanner {
            banner
        }
    }

    @ViewBuilder
    private var banner: some View {
        let variant = store.bannerVariant
        let hasAction = (store.showsRetryAction && onRetry != nil)
            || (store.showsSignOutAction && onSignOut != nil)
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: store.bannerSystemImage)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(iconTint(variant))
                .accessibilityHidden(true)
            Text(store.bannerText)
                .font(SynaraTypography.fineMetaBold)
                .foregroundStyle(textColor(variant))
                .lineLimit(store.slidingSyncUnsupported ? 4 : 2)
                .minimumScaleFactor(0.85)
                .accessibilityIdentifier("ConnectionStatusText")
            Spacer(minLength: SynaraSpacing.small)
            // Retry and Sign Out never show together (ConnectionStatusCopy).
            if store.showsRetryAction, let onRetry {
                Button("Retry", action: onRetry)
                    .buttonStyle(SynaraCompactBorderedButtonStyle())
                    .accessibilityIdentifier("ConnectionStatusRetryButton")
            }
            if store.showsSignOutAction, let onSignOut {
                Button("Sign Out", action: onSignOut)
                    .buttonStyle(SynaraCompactBorderedButtonStyle())
                    .accessibilityHint("Asks for confirmation before signing out of this device")
                    .accessibilityIdentifier("ConnectionStatusSignOutButton")
            }
        }
        .padding(.horizontal, SynaraSpacing.large)
        // The buttons carry their own 44-point touch area.
        .padding(.vertical, hasAction ? 0 : SynaraSpacing.small)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(background(variant))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("ConnectionStatusBanner")
        .accessibilityLabel(store.bannerText)
    }

    /// Text color per variant; light mode would fail AA with the raw
    /// system status colors (orange 1.96:1, green 2.01:1, red 3.01:1).
    static func textPalette(for variant: ConnectionStatusCopy.Variant) -> SynaraStatusTextPalette.Pair? {
        switch variant {
        case .success:
            return SynaraStatusTextPalette.success
        case .warning:
            return SynaraStatusTextPalette.warning
        case .critical:
            return SynaraStatusTextPalette.critical
        case .neutral:
            return nil
        }
    }

    private func textColor(_ variant: ConnectionStatusCopy.Variant) -> Color {
        switch variant {
        case .success:
            return SynaraColor.successText
        case .warning:
            return SynaraColor.warningText
        case .critical:
            return SynaraColor.criticalText
        case .neutral:
            return SynaraColor.secondaryText
        }
    }

    private func iconTint(_ variant: ConnectionStatusCopy.Variant) -> Color {
        switch variant {
        case .success:
            return SynaraColor.success
        case .warning:
            return SynaraColor.warning
        case .critical:
            return SynaraColor.critical
        case .neutral:
            return SynaraColor.secondaryText
        }
    }

    private func background(_ variant: ConnectionStatusCopy.Variant) -> Color {
        switch variant {
        case .success:
            return SynaraColor.success.opacity(0.12)
        case .warning:
            return SynaraColor.warning.opacity(0.14)
        case .critical:
            return SynaraColor.critical.opacity(0.12)
        case .neutral:
            return SynaraColor.secondarySurface
        }
    }
}

/// Empty-state copy from held connection chrome so SDK blips stay silent.
struct HeldConnectionEmptyState: View {
    let title: String
    let systemImage: String
    @ObservedObject var store: ConnectionStatusStore

    var body: some View {
        SynaraEmptyState(
            title: title,
            systemImage: systemImage,
            message: store.emptyStateMessage
        )
    }
}
