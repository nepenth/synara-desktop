import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

struct RootShellView: View {
    let environment: AppEnvironment
    @ObservedObject private var router: AppRouter
    @ObservedObject private var session: AppSessionStore
    @ObservedObject private var connectionStatus: ConnectionStatusStore
    @State private var tabBadgeCounts = TabBadgeCounts()
    @State private var tabBadgeUpdatesTask: Task<Void, Never>?
    @StateObject private var cryptoVerification = CryptoVerificationPresentation()
    @State private var cryptoVerificationUpdatesTask: Task<Void, Never>?
    @State private var cryptoVerificationSessionKey: String?
    @State private var cryptoVerificationActionError: String?
    @State private var signOutError: String?
    @StateObject private var verifyDevicePrompt = VerifyDevicePromptPresentation()
    @StateObject private var bannerSignOutConfirmation = SignOutConfirmationModel()
    @State private var tabBarScrollTailHeight: CGFloat = 0
    @ObservedObject private var themePaint = SynaraThemePaint.shared
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass

    init(environment: AppEnvironment = .mock()) {
        self.environment = environment
        self.router = environment.router
        self.session = environment.session
        self.connectionStatus = environment.connectionStatus
    }

    var body: some View {
        content
            .sheet(item: $router.sheetDestination) { destination in
                // Sheets are hosted outside the presenting view's subtree, so the
                // live environment must be injected here explicitly; otherwise the
                // sheet resolves `AppEnvironmentKey.defaultValue` (a signed-out
                // mock) and its Settings / Log Out actions target the wrong router.
                SheetPlaceholderView(destination: destination)
                    .environment(\.appEnvironment, environment)
                    .environment(\.synaraThemeBaseHex, themePaint.baseHex)
            }
            .environment(\.appEnvironment, environment)
            .environment(\.synaraThemeBaseHex, themePaint.baseHex)
            .onOpenURL { url in
                environment.logger.info("Opening deep link \(url.absoluteString)", category: .routing)
                let sessionIsSignedIn: Bool
                if case .signedIn = session.currentState {
                    sessionIsSignedIn = true
                } else {
                    sessionIsSignedIn = false
                }
                _ = router.open(url: url, sessionIsSignedIn: sessionIsSignedIn)
            }
    }

    @ViewBuilder
    private var content: some View {
        switch session.currentState {
        case .signedOut:
            signedOutShell
        case .signedIn(let authenticatedSession):
            signedInShell(session: authenticatedSession)
        }
    }

    private var signedOutShell: some View {
        NavigationStack(path: $router.authPath) {
            HomeserverSelectionView()
                .navigationDestination(for: AppRoute.self) { route in
                    RoutePlaceholderView(route: route)
                }
        }
        .onAppear {
            // Session crypto status belongs to the account that signed out.
            environment.sessionCrypto.reset()
        }
    }

    private func signedInShell(session authenticatedSession: AuthenticatedSession) -> some View {
        VStack(spacing: 0) {
            ConnectionStatusBanner(
                store: connectionStatus,
                onRetry: {
                    Task {
                        await environment.matrix.start(session: authenticatedSession)
                        await MainActor.run {
                            environment.connectionStatus.update(environment.matrix.syncStatus)
                        }
                    }
                },
                onSignOut: {
                    // Only opens Settings' Log Out confirmation (with the
                    // last-device warning); nothing is wiped on this tap.
                    bannerSignOutConfirmation.request(using: environment.sessionCrypto)
                }
            )
            TabView(selection: $router.selectedTab) {
                tab(.rooms)
                tab(.later)
                tab(.notifications)
                tab(.settings)
            }
            #if canImport(UIKit)
                .overlay(alignment: .topLeading) {
                    SynaraTabBarGeometryReader(scrollTailHeight: $tabBarScrollTailHeight)
                        .frame(width: 0, height: 0)
                        .allowsHitTesting(false)
                        .accessibilityHidden(true)
                }
            #endif
        }
        .onChange(of: connectionStatus.status) { status in
            // The only flush trigger outside a send: messages Core could not
            // take while the session was not ready go out now.
            guard OutgoingSendPolicy.isSendReady(status) else {
                return
            }
            Task {
                await environment.outgoingSends.flush()
            }
        }
        .task(id: "\(authenticatedSession.userID)-\(authenticatedSession.deviceID)-\(session.sessionEpoch)") {
            // Drafts and unsent messages on disk belong to this account only.
            environment.outgoingSends.bindAccount(authenticatedSession.userID)
            let signpostID = PerformanceTrace.begin("SignedInSessionStart")
            await SessionCoordinator.startSignedInSession(environment: environment, session: authenticatedSession)
            PerformanceTrace.end("SignedInSessionStart", id: signpostID)
            // Messages sent before the app was last terminated, not yet
            // accepted by Core, are handed over under their original ids.
            Task {
                await environment.outgoingSends.flush()
            }
            environment.router.replayPendingDeepLinkIfNeeded(sessionIsSignedIn: true)
            startTabBadgeUpdates()
            startCryptoVerificationUpdates(sessionKey: "\(authenticatedSession.userID)-\(authenticatedSession.deviceID)-\(session.sessionEpoch)")
            // "Verify This Device", once after signing in on a new device of
            // an account that already has an identity. Core decides.
            await verifyDevicePrompt.showIfNeeded(crypto: environment.crypto)
        }
        .sheet(isPresented: cryptoVerificationSheetBinding) {
            CryptoVerificationSheetHost(
                presentation: cryptoVerification,
                cryptoStore: environment.sessionCrypto,
                onAccept: { runCryptoVerificationAction { await environment.crypto.acceptVerificationRequest() } },
                onStartSas: { runCryptoVerificationAction { await environment.crypto.startSasVerification() } },
                onApprove: { runCryptoVerificationAction { await environment.crypto.approveVerification() } },
                onDecline: { runCryptoVerificationAction { await environment.crypto.declineVerification() } },
                onCancel: { runCryptoVerificationAction { await environment.crypto.cancelVerification() } },
                onDismissTerminal: {
                    dismissCryptoVerification()
                }
            )
        }
        .sheet(isPresented: $verifyDevicePrompt.isPresented, onDismiss: {
            // Runs once the prompt sheet is gone, so the verification sheet
            // it may start can present.
            Task {
                guard let result = await verifyDevicePrompt.didDismiss(crypto: environment.crypto) else {
                    return
                }
                if case .completed = result { return }
                cryptoVerificationActionError = result.message
            }
        }) {
            VerifyThisDevicePromptSheet(onVerifyWithAnotherDevice: {
                verifyDevicePrompt.verifyWithAnotherDevice()
            })
            .environment(\.appEnvironment, environment)
            .environment(\.synaraThemeBaseHex, themePaint.baseHex)
        }
        .alert("Verification action failed", isPresented: cryptoVerificationActionErrorBinding) {
            Button("OK", role: .cancel) {
                cryptoVerificationActionError = nil
            }
        } message: {
            Text(cryptoVerificationActionError ?? String(localized: "Try the verification step again."))
        }
        // On the shell, not the banner: the banner may hide while it is open.
        .signOutConfirmation(
            bannerSignOutConfirmation,
            confirmIdentifier: "ConnectionStatusConfirmSignOutButton"
        ) {
            signOut()
        }
        .alert("Could not log out", isPresented: signOutErrorBinding) {
            Button("Try Again") {
                signOutError = nil
                signOut()
            }
            Button("OK", role: .cancel) {
                signOutError = nil
            }
        } message: {
            Text(signOutError ?? LocalWipeError.sessionDeleteFailed.localizedDescription)
        }
        .onDisappear {
            tabBadgeUpdatesTask?.cancel()
            tabBadgeUpdatesTask = nil
            cryptoVerificationUpdatesTask?.cancel()
            cryptoVerificationUpdatesTask = nil
        }
    }

    private var cryptoVerificationSheetBinding: Binding<Bool> {
        Binding(
            get: { cryptoVerification.snapshot != nil },
            set: { isPresented in
                guard isPresented == false else { return }
                guard CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(cryptoVerification.snapshot?.state) else {
                    return
                }
                dismissCryptoVerification()
            }
        )
    }

    private var cryptoVerificationActionErrorBinding: Binding<Bool> {
        Binding(
            get: { cryptoVerificationActionError != nil },
            set: { isPresented in
                if isPresented == false {
                    cryptoVerificationActionError = nil
                }
            }
        )
    }

    private var signOutErrorBinding: Binding<Bool> {
        Binding(
            get: { signOutError != nil },
            set: { isPresented in
                if isPresented == false {
                    signOutError = nil
                }
            }
        )
    }

    private func signOut() {
        signOutError = nil
        Task {
            do {
                try await environment.wipe.logoutAndWipe()
            } catch {
                await MainActor.run {
                    signOutError = LocalWipeError.displayMessage(for: error)
                }
            }
        }
    }

    private func tab(_ tab: AppTab) -> some View {
        Group {
            if tab.usesConversationCanvas {
                SynaraConversationCanvas(tab: tab, router: router) {
                    tab.content
                }
            } else {
                NavigationStack(path: router.binding(for: tab)) {
                    tab.content
                        .navigationDestination(for: AppRoute.self) { route in
                            RoutePlaceholderView(route: route)
                        }
                }
            }
        }
        .synaraTabRootContentReachability(scrollTailHeight: scrollTailHeight(for: tab))
        .tabItem {
            tab.label(badgeCounts: tabBadgeCounts)
        }
        .tag(tab)
    }

    private func scrollTailHeight(for tab: AppTab) -> CGFloat {
        if tab.usesConversationCanvas,
           SynaraCanvasLayoutPolicy.layout(horizontalSizeClass: horizontalSizeClass) == .stacked,
           SynaraConversationPath.splitRoot(in: router.binding(for: tab).wrappedValue) != nil {
            // The compact conversation hides the tab bar. Its root scroll tail
            // would otherwise reserve a blank strip below the composer.
            return 0
        }
        return tabBarScrollTailHeight
    }

    private func startTabBadgeUpdates() {
        tabBadgeUpdatesTask?.cancel()
        tabBadgeUpdatesTask = Task {
            for await update in environment.roomList.roomUpdates() {
                guard Task.isCancelled == false else {
                    return
                }

                let rooms = rooms(from: update)

                await MainActor.run {
                    tabBadgeCounts = TabBadgeCounts.make(from: rooms)
                }

                switch update {
                case .loaded, .empty:
                    await AppIconBadge.applyCurrentSummary(
                        rooms: rooms,
                        later: environment.later,
                        push: environment.push
                    )
                case .idle, .loading, .failed:
                    break
                }
            }
        }
    }

    private func startCryptoVerificationUpdates(sessionKey: String) {
        cryptoVerificationUpdatesTask?.cancel()
        if cryptoVerificationSessionKey != sessionKey {
            cryptoVerification.reset()
            cryptoVerificationSessionKey = sessionKey
        }
        cryptoVerificationUpdatesTask = Task {
            for await update in environment.crypto.verificationUpdates() {
                guard !Task.isCancelled else { return }
                cryptoVerification.receive(update)
            }
        }
    }

    private func dismissCryptoVerification() {
        Task { await cryptoVerification.dismiss(using: environment.crypto) }
    }

    private func runCryptoVerificationAction(_ action: @escaping () async -> CryptoActionResult) {
        Task {
            let result = await action()
            await MainActor.run {
                switch result {
                case .completed:
                    cryptoVerificationActionError = nil
                case .failed(let message), .unavailable(let message):
                    cryptoVerificationActionError = message
                }
            }
        }
    }

    private func rooms(from state: RoomListState) -> [RoomSummary] {
        guard case .loaded(let rooms) = state else {
            return []
        }
        return rooms
    }
}

#if canImport(UIKit)
private struct SynaraTabBarGeometryReader: UIViewRepresentable {
    @Binding var scrollTailHeight: CGFloat

    func makeUIView(context: Context) -> TabBarGeometryProbeView {
        let view = TabBarGeometryProbeView()
        view.onGeometryChange = { height in
            guard abs(scrollTailHeight - height) > 0.5 else {
                return
            }
            DispatchQueue.main.async {
                scrollTailHeight = height
            }
        }
        return view
    }

    func updateUIView(_ uiView: TabBarGeometryProbeView, context: Context) {
        uiView.onGeometryChange = { height in
            guard abs(scrollTailHeight - height) > 0.5 else {
                return
            }
            DispatchQueue.main.async {
                scrollTailHeight = height
            }
        }
        uiView.resolveGeometry()
    }
}

private final class TabBarGeometryProbeView: UIView {
    var onGeometryChange: ((CGFloat) -> Void)?

    override func didMoveToWindow() {
        super.didMoveToWindow()
        resolveGeometry()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        resolveGeometry()
    }

    func resolveGeometry() {
        guard let window else {
            onGeometryChange?(0)
            return
        }
        guard let tabBar = findTabBarController()?.tabBar else {
            onGeometryChange?(0)
            return
        }
        let frame = tabBar.convert(tabBar.bounds, to: window)
        let isVisible = tabBar.isHidden == false && tabBar.alpha > 0.01
        onGeometryChange?(
            SynaraTabRootContentReachability.scrollTailHeight(
                windowBounds: window.bounds,
                tabBarFrame: frame,
                isVisible: isVisible
            )
        )
    }

    private func findTabBarController() -> UITabBarController? {
        var responder: UIResponder? = self
        while let current = responder {
            if let tabBarController = current as? UITabBarController {
                return tabBarController
            }
            responder = current.next
        }
        return findTabBarController(in: window?.rootViewController)
    }

    private func findTabBarController(in controller: UIViewController?) -> UITabBarController? {
        guard let controller else {
            return nil
        }
        if let tabBarController = controller as? UITabBarController {
            return tabBarController
        }
        for child in controller.children {
            if let tabBarController = findTabBarController(in: child) {
                return tabBarController
            }
        }
        if let presented = controller.presentedViewController {
            return findTabBarController(in: presented)
        }
        return nil
    }
}
#endif

struct RootShellView_Previews: PreviewProvider {
    static var previews: some View {
        RootShellView(environment: .mock())
    }
}

/// Keeps the presented sheet subscribed to verification state changes.
///
/// SwiftUI evaluates an `isPresented` sheet's content closure when the sheet is
/// presented. Passing the unwrapped state value there freezes that snapshot, so
/// an in-flight verification can remain visually stuck on "Waiting" even while
/// the SDK has advanced to `sas_ready`. Observing the presentation owner makes
/// each protocol update invalidate the hierarchy without dismissing the sheet.
private struct CryptoVerificationSheetHost: View {
    @ObservedObject var presentation: CryptoVerificationPresentation
    let cryptoStore: SessionCryptoStore
    let onAccept: () -> Void
    let onStartSas: () -> Void
    let onApprove: () -> Void
    let onDecline: () -> Void
    let onCancel: () -> Void
    let onDismissTerminal: () -> Void

    var body: some View {
        if let snapshot = presentation.snapshot {
            CryptoVerificationSheet(
                state: snapshot.state,
                cryptoStore: cryptoStore,
                isDismissing: presentation.isDismissing,
                dismissalError: presentation.error,
                onAccept: onAccept,
                onStartSas: onStartSas,
                onApprove: onApprove,
                onDecline: onDecline,
                onCancel: onCancel,
                onDismissTerminal: onDismissTerminal
            )
            .interactiveDismissDisabled(
                CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(snapshot.state) == false || presentation.isDismissing
            )
        }
    }
}

private struct CryptoVerificationSheet: View {
    let state: CryptoVerificationState
    let cryptoStore: SessionCryptoStore
    let isDismissing: Bool
    let dismissalError: String?
    @StateObject private var sessionCrypto = SessionCryptoStatusObserver()
    @State private var contentHeight: CGFloat = 360
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @ScaledMetric(relativeTo: .title) private var emojiSize: CGFloat = 30
    let onAccept: () -> Void
    let onStartSas: () -> Void
    let onApprove: () -> Void
    let onDecline: () -> Void
    let onCancel: () -> Void
    let onDismissTerminal: () -> Void

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: SynaraSpacing.large) {
                header
                content
                if let dismissalError {
                    Text(dismissalError)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.criticalText)
                        .accessibilityIdentifier("VerificationDismissalError")
                }
                actions
                    .controlSize(.large)
                    .disabled(isDismissing)
            }
            .padding(SynaraSpacing.xLarge)
            .padding(.top, SynaraSpacing.small)
            .frame(maxWidth: .infinity, alignment: .leading)
            .fixedSize(horizontal: false, vertical: true)
            .background {
                GeometryReader { geometry in
                    Color.clear.preference(key: VerificationSheetHeight.self, value: geometry.size.height)
                }
            }
        }
        .background(Color(uiColor: .systemBackground))
        .onPreferenceChange(VerificationSheetHeight.self) { height in
            guard height > 0, abs(contentHeight - height) > 1 else { return }
            contentHeight = ceil(height)
        }
        // Measure the actual content; the system caps the detent on small
        // screens, where scrolling still exposes every value and action.
        .presentationDetents([.height(contentHeight)])
        .presentationDragIndicator(.visible)
        .task(id: state == .finished) {
            if state == .finished { await sessionCrypto.start(store: cryptoStore) }
        }
        .accessibilityIdentifier("DeviceVerificationSheet")
    }

    @ViewBuilder
    private var header: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            Text(title)
                .font(SynaraTypography.screenTitle)
            Text(detail)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
        }
    }

    @ViewBuilder
    private var content: some View {
        switch state {
        case .requestReceived(let request):
            VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                CryptoVerificationInfoRow(
                    title: String(localized: "User"),
                    value: request.displayName ?? request.userID
                )
                CryptoVerificationInfoRow(
                    title: String(localized: "Device"),
                    value: request.deviceDisplayName ?? request.deviceID
                )
                Text("Only accept if you recognize this request. You’ll compare codes on both devices next.")
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
            }
        case .emojis(let emojis):
            let columns = dynamicTypeSize.isAccessibilitySize ? 2 : 4
            VStack(spacing: SynaraSpacing.small) {
                ForEach(Array(stride(from: 0, to: emojis.count, by: columns)), id: \.self) { start in
                    HStack(alignment: .top, spacing: SynaraSpacing.small) {
                        ForEach(start..<min(start + columns, emojis.count), id: \.self) { index in
                            VStack(spacing: SynaraSpacing.xSmall) {
                                Text(emojis[index].symbol)
                                    .font(.system(size: emojiSize))
                                Text(emojis[index].description)
                                    .font(SynaraTypography.fineMetaBold)
                                    .multilineTextAlignment(.center)
                                    .fixedSize(horizontal: false, vertical: true)
                            }
                            .padding(.vertical, SynaraSpacing.small)
                            .frame(maxWidth: .infinity)
                            .accessibilityElement(children: .combine)
                            .accessibilityIdentifier("VerificationEmoji-\(index)")
                        }
                    }
                }
            }
            .padding(SynaraSpacing.small)
            .background(Color(uiColor: .secondarySystemBackground), in: RoundedRectangle(cornerRadius: 16))
        case .decimals(let values):
            HStack(spacing: SynaraSpacing.medium) {
                ForEach(Array(values.enumerated()), id: \.offset) { index, value in
                    Text(String(value))
                        .font(.system(.title2, design: .monospaced).weight(.semibold))
                        .frame(maxWidth: .infinity)
                        .padding(SynaraSpacing.medium)
                        .synaraCard()
                        .accessibilityIdentifier("VerificationDecimal-\(index)")
                }
            }
        case .requestSent, .accepted, .sasStarted, .keysExchanging, .confirmed:
            ProgressView()
                .controlSize(.large)
                .frame(maxWidth: .infinity, alignment: .center)
                .padding(.vertical, SynaraSpacing.large)
        case .finished:
            VStack(spacing: SynaraSpacing.medium) {
                Image(systemName: terminalSystemImage)
                    .font(.system(size: 40, weight: .semibold))
                    .foregroundStyle(terminalTint)
                CryptoVerificationInfoRow(
                    title: String(localized: "This device"),
                    value: sessionCrypto.status.verification.settingsDisplayName
                )
                .accessibilityIdentifier("VerificationOwnDeviceStatus")
            }
        case .cancelled, .failed, .mismatched:
            Image(systemName: terminalSystemImage)
                .font(.system(size: 44, weight: .semibold))
                .foregroundStyle(terminalTint)
                .frame(maxWidth: .infinity)
                .padding(.vertical, SynaraSpacing.large)
        }
    }

    @ViewBuilder
    private var actions: some View {
        VStack(spacing: SynaraSpacing.small) {
            switch state {
            case .requestReceived:
                primaryButton(String(localized: "Accept"), identifier: "AcceptDeviceVerificationButton", action: onAccept)
                secondaryButton(String(localized: "Decline"), role: .cancel, action: onDecline)
            case .requestSent, .sasStarted, .keysExchanging, .confirmed:
                secondaryButton(String(localized: "Cancel Verification"), role: .cancel, action: onCancel)
            case .accepted:
                primaryButton(
                    String(localized: "Start Comparison"),
                    identifier: "StartDeviceVerificationSasButton",
                    action: onStartSas
                )
                secondaryButton(String(localized: "Cancel"), role: .cancel, action: onCancel)
            case .emojis, .decimals:
                primaryButton(String(localized: "They Match"), identifier: "ConfirmDeviceVerificationButton", action: onApprove)
                secondaryButton(String(localized: "They Do Not Match"), role: .destructive, action: onDecline)
            case .finished, .cancelled, .failed, .mismatched:
                primaryButton(
                    isDismissing ? String(localized: "Closing…") : String(localized: "Done"),
                    identifier: "DismissDeviceVerificationButton",
                    action: onDismissTerminal
                )
            }
        }
    }

    private func primaryButton(_ title: String, identifier: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .frame(maxWidth: .infinity, minHeight: 32)
        }
        .buttonStyle(.borderedProminent)
        .accessibilityIdentifier(identifier)
    }

    private func secondaryButton(_ title: String, role: ButtonRole, action: @escaping () -> Void) -> some View {
        Button(role: role, action: action) {
            Text(title)
                .frame(maxWidth: .infinity, minHeight: 32)
        }
        .buttonStyle(.bordered)
    }

    private var title: String {
        switch state {
        case .requestReceived:
            return String(localized: "Verification request")
        case .requestSent:
            return String(localized: "Request sent")
        case .accepted:
            return String(localized: "Ready to compare")
        case .sasStarted:
            return String(localized: "Waiting")
        case .keysExchanging:
            return String(localized: "Exchanging keys")
        case .emojis, .decimals:
            return String(localized: "Compare on both devices")
        case .confirmed:
            return String(localized: "Waiting for the other device")
        case .finished:
            return sessionCrypto.status.verification == .verified
                ? String(localized: "Device verified")
                : String(localized: "Verification complete")
        case .cancelled:
            return String(localized: "Verification cancelled")
        case .failed:
            return String(localized: "Verification failed")
        case .mismatched:
            return String(localized: "Codes did not match")
        }
    }

    private var detail: String {
        switch state {
        case .requestReceived:
            return String(localized: "Another session wants to verify this device.")
        case .requestSent:
            return String(localized: "Approve the request from one of your already trusted sessions.")
        case .accepted:
            return String(localized: "Start a secure emoji or number comparison. Only this device should start.")
        case .sasStarted:
            return String(localized: "Waiting for the other device to start comparison, or for codes to appear.")
        case .keysExchanging:
            return String(localized: "Both devices accepted the comparison. Secure codes will appear when the key exchange completes.")
        case .emojis, .decimals:
            return String(localized: "Only approve if the values match exactly on both devices.")
        case .confirmed:
            return String(localized: "This device accepted the codes. Wait for the other session to finish.")
        case .finished:
            return sessionCrypto.status.verification == .verified
                ? String(localized: "The codes matched on both devices. This device is verified for encrypted conversations.")
                : String(localized: "The codes matched on both devices. This device’s verified status is not confirmed yet. Check Security settings if it does not update.")
        case .cancelled:
            return String(localized: "The verification flow was cancelled.")
        case .failed:
            return String(localized: "The verification flow could not be completed.")
        case .mismatched:
            return String(localized: "The security codes did not match. Verification was cancelled safely.")
        }
    }

    private var terminalSystemImage: String {
        switch state {
        case .finished:
            return sessionCrypto.status.verification == .verified ? "checkmark.seal.fill" : "checkmark.circle"
        case .cancelled:
            return "xmark.circle.fill"
        case .failed, .mismatched:
            return "exclamationmark.triangle.fill"
        case .requestReceived, .requestSent, .accepted, .sasStarted, .keysExchanging, .emojis, .decimals, .confirmed:
            return "lock.shield"
        }
    }

    private var terminalTint: Color {
        switch state {
        case .finished:
            return sessionCrypto.status.verification == .verified ? .green : SynaraColor.accent
        case .cancelled:
            return SynaraColor.secondaryText
        case .failed, .mismatched:
            return SynaraColor.critical
        case .requestReceived, .requestSent, .accepted, .sasStarted, .keysExchanging, .emojis, .decimals, .confirmed:
            return SynaraColor.accent
        }
    }
}

private struct CryptoVerificationInfoRow: View {
    let title: String
    let value: String

    var body: some View {
        HStack {
            Text(title)
                .foregroundStyle(SynaraColor.secondaryText)
            Spacer(minLength: SynaraSpacing.medium)
            Text(value)
                .multilineTextAlignment(.trailing)
        }
        .font(SynaraTypography.body)
    }
}

private struct VerificationSheetHeight: PreferenceKey {
    static var defaultValue: CGFloat = 0
    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}
