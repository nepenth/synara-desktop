import SwiftUI

/// Signed-in connection/sync chrome. Hidden in steady Connected; Lost waits
/// for the store hold so SDK blips do not paint a banner.
struct ConnectionStatusBanner: View {
    @ObservedObject var store: ConnectionStatusStore
    var onRetry: (() -> Void)?
    var onSignOut: (() -> Void)?

    var body: some View {
        if store.isBannerVisible {
            banner
        }
    }

    private var banner: some View {
        let status = store.status
        let variant = ConnectionStatusCopy.variant(status)
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: ConnectionStatusCopy.systemImage(status))
                .font(.system(size: 13, weight: .semibold))
                .accessibilityHidden(true)
            Text(ConnectionStatusCopy.banner(status))
                .font(SynaraTypography.fineMetaBold)
                .lineLimit(2)
                .minimumScaleFactor(0.85)
            Spacer(minLength: SynaraSpacing.small)
            if ConnectionStatusCopy.showsRetryAction(status), let onRetry {
                Button("Retry", action: onRetry)
                    .font(SynaraTypography.fineMetaBold)
                    .buttonStyle(.bordered)
                    .controlSize(.mini)
                    .accessibilityIdentifier("ConnectionStatusRetryButton")
            }
            if ConnectionStatusCopy.showsSignOutAction(status), let onSignOut {
                Button("Sign Out", action: onSignOut)
                    .font(SynaraTypography.fineMetaBold)
                    .buttonStyle(.bordered)
                    .controlSize(.mini)
                    .accessibilityIdentifier("ConnectionStatusSignOutButton")
            }
        }
        .foregroundStyle(foreground(variant))
        .padding(.horizontal, SynaraSpacing.large)
        .padding(.vertical, SynaraSpacing.small)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(background(variant))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("ConnectionStatusBanner")
        .accessibilityLabel(ConnectionStatusCopy.banner(status))
    }

    private func foreground(_ variant: ConnectionStatusCopy.Variant) -> Color {
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
