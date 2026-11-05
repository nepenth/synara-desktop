import SwiftUI

/// Signed-in connection/sync chrome. Hidden in steady Connected; Lost waits
/// for the store hold so SDK blips do not paint a banner.
///
/// Only actionable states paint the band: Connection Lost (with Retry), a
/// session that could not be restored (Log Out), or a server without sliding
/// sync. Routine states (cold-launch Connecting…, the Connected recovery
/// flash) stay out of the layout; the Rooms empty state still names them.
///
/// `onSignOut` only asks for sign-out: the owner shows the same confirmation
/// as Settings → Log Out before anything is wiped.
struct ConnectionStatusBanner: View {
    @ObservedObject var store: ConnectionStatusStore
    var onRetry: (() -> Void)?
    var onSignOut: (() -> Void)?

    var body: some View {
        if store.presentsBanner, Self.isActionable(store.status, slidingSyncUnsupported: store.slidingSyncUnsupported) {
            banner
        }
    }

    /// Whether this status is worth a band inside the current tab.
    static func isActionable(_ status: MatrixSyncStatus, slidingSyncUnsupported: Bool) -> Bool {
        if slidingSyncUnsupported {
            return true
        }
        switch status {
        case .reconnecting, .disconnected, .restoreFailed, .failed:
            return true
        case .connected, .syncing, .starting, .stopped:
            return false
        }
    }

    @ViewBuilder
    private var banner: some View {
        let variant = store.bannerVariant
        let hasAction = (store.showsRetryAction && onRetry != nil)
            || (store.showsSignOutAction && onSignOut != nil)
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: store.bannerSystemImage)
                .font(.footnote.weight(.semibold))
                .foregroundStyle(iconTint(variant))
                .accessibilityHidden(true)
            Text(store.bannerText)
                .font(.footnote.weight(.semibold))
                .foregroundStyle(textColor(variant))
                .lineLimit(store.slidingSyncUnsupported ? 4 : 2)
                .accessibilityIdentifier("ConnectionStatusText")
            Spacer(minLength: SynaraSpacing.small)
            // Retry and Sign Out never show together (ConnectionStatusCopy).
            if store.showsRetryAction, let onRetry {
                Button("Retry", action: onRetry)
                    .buttonStyle(SynaraCompactBorderedButtonStyle())
                    .accessibilityIdentifier("ConnectionStatusRetryButton")
            }
            if store.showsSignOutAction, let onSignOut {
                Button("Log Out", action: onSignOut)
                    .buttonStyle(SynaraCompactBorderedButtonStyle())
                    .accessibilityHint("Asks for confirmation before logging out of this device")
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
///
/// While connected, `connectedMessage` and `action` replace the connection
/// copy, so an account with no rooms gets a way forward instead of the word
/// "Connected".
struct HeldConnectionEmptyState: View {
    struct Action {
        let title: String
        let identifier: String
        let handler: () -> Void

        init(title: String, identifier: String, handler: @escaping () -> Void) {
            self.title = title
            self.identifier = identifier
            self.handler = handler
        }
    }

    let title: String
    let systemImage: String
    @ObservedObject var store: ConnectionStatusStore
    var connectedMessage: String? = nil
    var action: Action? = nil

    private var isConnected: Bool {
        store.slidingSyncUnsupported == false && (store.status == .connected || store.status == .syncing)
    }

    var body: some View {
        VStack(spacing: SynaraSpacing.large) {
            SynaraEmptyState(
                title: title,
                systemImage: systemImage,
                message: isConnected ? (connectedMessage ?? store.emptyStateMessage) : store.emptyStateMessage,
                paintsBackground: false
            )
            .fixedSize(horizontal: false, vertical: true)

            if isConnected, let action {
                Button(action.title, action: action.handler)
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier(action.identifier)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

/// A thin line under the navigation bar that says the connection is coming
/// back: a red pulse while trying, a short green fill once connected. It is
/// zero height in layout, so lighting it never moves content.
struct ConnectionBeam: View {
    @ObservedObject var store: ConnectionStatusStore
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    static let thickness: CGFloat = 2
    /// One sweep of the pulse across the width.
    static let sweepDuration: TimeInterval = 1.4

    var body: some View {
        Color.clear
            .frame(height: 0)
            .overlay(alignment: .top) {
                beam
                    .frame(height: Self.thickness)
                    .frame(maxWidth: .infinity)
                    .allowsHitTesting(false)
                    .accessibilityHidden(true)
            }
            .animation(.easeOut(duration: 0.35), value: store.beamPhase)
            .accessibilityIdentifier("ConnectionBeam")
            .accessibilityValue(Self.accessibilityValue(store.beamPhase))
    }

    static func accessibilityValue(_ phase: ConnectionBeamPhase) -> String {
        switch phase {
        case .hidden: return "hidden"
        case .trying: return "trying"
        case .connected: return "connected"
        }
    }

    @ViewBuilder
    private var beam: some View {
        switch store.beamPhase {
        case .hidden:
            Color.clear
        case .trying:
            ZStack(alignment: .leading) {
                SynaraColor.critical.opacity(reduceMotion ? 1 : 0.35)
                if !reduceMotion {
                    pulse
                }
            }
            .clipped()
            .transition(.opacity)
        case .connected:
            SynaraColor.success
                .transition(.opacity)
        }
    }

    private var pulse: some View {
        TimelineView(.animation) { context in
            GeometryReader { geometry in
                let width = geometry.size.width
                let segment = width * 0.4
                let elapsed = context.date.timeIntervalSinceReferenceDate
                let progress = elapsed.truncatingRemainder(dividingBy: Self.sweepDuration)
                    / Self.sweepDuration
                LinearGradient(
                    colors: [.clear, SynaraColor.critical, .clear],
                    startPoint: .leading,
                    endPoint: .trailing
                )
                .frame(width: segment)
                .offset(x: -segment + (width + segment) * progress)
            }
        }
    }
}

/// What the connection banner's buttons do. The signed-in shell provides
/// them; screens that draw their own header read them from the environment.
struct ConnectionStatusActions {
    var onRetry: (() -> Void)?
    var onSignOut: (() -> Void)?
}

private struct ConnectionStatusActionsKey: EnvironmentKey {
    static let defaultValue = ConnectionStatusActions()
}

extension EnvironmentValues {
    var synaraConnectionActions: ConnectionStatusActions {
        get { self[ConnectionStatusActionsKey.self] }
        set { self[ConnectionStatusActionsKey.self] = newValue }
    }
}

/// The connection banner (actionable states only) with the beam along its
/// bottom edge. Placed directly under a screen's header, so the beam runs
/// under the server or room name.
struct ConnectionStatusChrome: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.synaraConnectionActions) private var actions

    var body: some View {
        VStack(spacing: 0) {
            ConnectionStatusBanner(
                store: environment.connectionStatus,
                onRetry: actions.onRetry,
                onSignOut: actions.onSignOut
            )
            ConnectionBeam(store: environment.connectionStatus)
        }
        // The beam hangs below this view over the content that follows;
        // draw it above that sibling.
        .zIndex(1)
    }
}

extension View {
    /// Connection chrome under this screen's system navigation bar, for
    /// screens that do not draw a header of their own.
    func synaraConnectionChrome() -> some View {
        safeAreaInset(edge: .top, spacing: 0) {
            ConnectionStatusChrome()
        }
    }
}
