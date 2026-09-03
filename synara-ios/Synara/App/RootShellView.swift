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
    @State private var cryptoVerificationState: CryptoVerificationState?
    @State private var cryptoVerificationUpdatesTask: Task<Void, Never>?
    @State private var cryptoVerificationActionError: String?
    @State private var signOutError: String?
    @State private var tabBarScrollTailHeight: CGFloat = 0
    @ObservedObject private var themePaint = SynaraThemePaint.shared

    init(environment: AppEnvironment = .mock()) {
        self.environment = environment
        self.router = environment.router
        self.session = environment.session
        self.connectionStatus = environment.connectionStatus
    }

    var body: some View {
        content
            .environment(\.appEnvironment, environment)
            .environment(\.synaraThemeBaseHex, themePaint.baseHex)
            .sheet(item: $router.sheetDestination) { destination in
                SheetPlaceholderView(destination: destination)
            }
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
                    signOut()
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
            guard OutgoingSendPolicy.isSendReady(status) else {
                return
            }
            Task {
                await environment.outgoingSends.flushWhenSendReady()
            }
        }
        .task(id: "\(authenticatedSession.userID)-\(authenticatedSession.deviceID)-\(session.sessionEpoch)") {
            let signpostID = PerformanceTrace.begin("SignedInSessionStart")
            await SessionCoordinator.startSignedInSession(environment: environment, session: authenticatedSession)
            PerformanceTrace.end("SignedInSessionStart", id: signpostID)
            environment.router.replayPendingDeepLinkIfNeeded(sessionIsSignedIn: true)
            startTabBadgeUpdates()
            startCryptoVerificationUpdates()
        }
        .sheet(isPresented: cryptoVerificationSheetBinding) {
            CryptoVerificationSheetHost(
                state: $cryptoVerificationState,
                onAccept: { runCryptoVerificationAction { await environment.crypto.acceptVerificationRequest() } },
                onStartSas: { runCryptoVerificationAction { await environment.crypto.startSasVerification() } },
                onApprove: { runCryptoVerificationAction { await environment.crypto.approveVerification() } },
                onDecline: { runCryptoVerificationAction { await environment.crypto.declineVerification() } },
                onCancel: { runCryptoVerificationAction { await environment.crypto.cancelVerification() } },
                onDismissTerminal: {
                    runCryptoVerificationAction { await environment.crypto.dismissVerification() }
                    self.cryptoVerificationState = nil
                }
            )
        }
        .alert("Verification action failed", isPresented: cryptoVerificationActionErrorBinding) {
            Button("OK", role: .cancel) {
                cryptoVerificationActionError = nil
            }
        } message: {
            Text(cryptoVerificationActionError ?? "Try the verification step again.")
        }
        .alert("Could not sign out", isPresented: signOutErrorBinding) {
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
            get: { cryptoVerificationState != nil },
            set: { isPresented in
                guard isPresented == false else { return }
                guard CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(cryptoVerificationState) else {
                    return
                }
                runCryptoVerificationAction { await environment.crypto.dismissVerification() }
                cryptoVerificationState = nil
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
        NavigationStack(path: router.binding(for: tab)) {
            tab.content
                .navigationDestination(for: AppRoute.self) { route in
                    RoutePlaceholderView(route: route)
                }
        }
        .synaraTabRootContentReachability(scrollTailHeight: tabBarScrollTailHeight)
        .tabItem {
            tab.label(badgeCounts: tabBadgeCounts)
        }
        .tag(tab)
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
            }
        }
    }

    private func startCryptoVerificationUpdates() {
        cryptoVerificationUpdatesTask?.cancel()
        cryptoVerificationUpdatesTask = Task {
            await withTaskGroup(of: Void.self) { group in
                group.addTask {
                    await self.consumeCryptoVerificationUpdates()
                }
                group.addTask {
                    await self.pollClearedCryptoVerification()
                }
            }
        }
    }

    private func consumeCryptoVerificationUpdates() async {
        for await update in environment.crypto.verificationUpdates() {
            guard Task.isCancelled == false else {
                return
            }
            await MainActor.run {
                cryptoVerificationState = update
            }
            if update.isTerminal {
                if case .finished = update {
                    // Verification succeeded — kick a crypto status refresh.
                    // Any open timeline that is showing the "Encrypted history" / "Retry Decryption"
                    // banner will re-compute on its next status poll and should clear or become actionable.
                    Task {
                        _ = await environment.crypto.sessionStatus()
                    }
                }
                try? await Task.sleep(nanoseconds: 1_500_000_000)
                _ = await environment.crypto.dismissVerification()
                await MainActor.run {
                    if cryptoVerificationState == update {
                        cryptoVerificationState = nil
                    }
                }
            }
        }
    }

    private func pollClearedCryptoVerification() async {
        while Task.isCancelled == false {
            try? await Task.sleep(nanoseconds: 500_000_000)
            guard Task.isCancelled == false else {
                return
            }
            let latest = await environment.crypto.currentVerificationState()
            await MainActor.run {
                if let restored = CryptoVerificationPresentationPolicy.restoredStateIfCleared(
                    presented: cryptoVerificationState,
                    latest: latest
                ), restored != cryptoVerificationState {
                    cryptoVerificationState = restored
                }
            }
        }
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
/// the SDK has advanced to `sas_ready`. The binding makes each protocol update
/// invalidate the presented hierarchy without dismissing the sheet.
private struct CryptoVerificationSheetHost: View {
    @Binding var state: CryptoVerificationState?
    let onAccept: () -> Void
    let onStartSas: () -> Void
    let onApprove: () -> Void
    let onDecline: () -> Void
    let onCancel: () -> Void
    let onDismissTerminal: () -> Void

    var body: some View {
        if let state {
            CryptoVerificationSheet(
                state: state,
                onAccept: onAccept,
                onStartSas: onStartSas,
                onApprove: onApprove,
                onDecline: onDecline,
                onCancel: onCancel,
                onDismissTerminal: onDismissTerminal
            )
            .interactiveDismissDisabled(
                CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(state) == false
            )
        }
    }
}

private struct CryptoVerificationSheet: View {
    let state: CryptoVerificationState
    let onAccept: () -> Void
    let onStartSas: () -> Void
    let onApprove: () -> Void
    let onDecline: () -> Void
    let onCancel: () -> Void
    let onDismissTerminal: () -> Void

    var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: SynaraSpacing.large) {
                ScrollView {
                    VStack(alignment: .leading, spacing: SynaraSpacing.large) {
                        header
                        content
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                actions
            }
            .padding(SynaraSpacing.xLarge)
        }
        // Comparison needs the full height so all seven values and both
        // decisions remain simultaneously readable at larger text sizes.
        .presentationDetents(comparisonPresentationDetents)
        .accessibilityIdentifier("DeviceVerificationSheet")
    }

    private var comparisonPresentationDetents: Set<PresentationDetent> {
        switch state {
        case .emojis, .decimals:
            return [.large]
        default:
            return [.medium, .large]
        }
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
                CryptoVerificationInfoRow(title: "User", value: request.displayName ?? request.userID)
                CryptoVerificationInfoRow(title: "Device", value: request.deviceDisplayName ?? request.deviceID)
            }
        case .emojis(let emojis):
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 86), spacing: SynaraSpacing.small)], spacing: SynaraSpacing.small) {
                ForEach(Array(emojis.enumerated()), id: \.offset) { index, emoji in
                    VStack(spacing: SynaraSpacing.xSmall) {
                        Text(emoji.symbol)
                            .font(.system(size: 34))
                        Text(emoji.description)
                            .font(SynaraTypography.fineMetaBold)
                            .multilineTextAlignment(.center)
                    }
                    .frame(maxWidth: .infinity, minHeight: 78)
                    .padding(SynaraSpacing.small)
                    .synaraCard()
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("VerificationEmoji-\(index)")
                }
            }
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
        case .finished, .cancelled, .failed, .mismatched:
            Image(systemName: terminalSystemImage)
                .font(.system(size: 44, weight: .semibold))
                .foregroundStyle(terminalTint)
                .frame(maxWidth: .infinity)
                .padding(.vertical, SynaraSpacing.large)
        }
    }

    @ViewBuilder
    private var actions: some View {
        switch state {
        case .requestReceived:
            HStack(spacing: SynaraSpacing.small) {
                Button("Decline", role: .cancel, action: onDecline)
                    .buttonStyle(.bordered)
                Button("Accept", action: onAccept)
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("AcceptDeviceVerificationButton")
            }
        case .requestSent, .sasStarted, .keysExchanging:
            Button("Cancel Verification", role: .cancel, action: onCancel)
                .buttonStyle(.bordered)
        case .accepted:
            HStack(spacing: SynaraSpacing.small) {
                Button("Cancel", role: .cancel, action: onCancel)
                    .buttonStyle(.bordered)
                Button("Start Comparison", action: onStartSas)
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("StartDeviceVerificationSasButton")
            }
        case .emojis, .decimals:
            VStack(spacing: SynaraSpacing.small) {
                Button("They Match", action: onApprove)
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("ConfirmDeviceVerificationButton")
                Button("They Do Not Match", role: .destructive, action: onDecline)
                    .buttonStyle(.bordered)
            }
        case .confirmed:
            Button("Cancel", role: .cancel, action: onCancel)
                .buttonStyle(.bordered)
        case .finished, .cancelled, .failed, .mismatched:
            Button("Done", action: onDismissTerminal)
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("DismissDeviceVerificationButton")
        }
    }

    private var title: String {
        switch state {
        case .requestReceived:
            return "Verification request"
        case .requestSent:
            return "Request sent"
        case .accepted:
            return "Ready to compare"
        case .sasStarted:
            return "Waiting"
        case .keysExchanging:
            return "Exchanging keys"
        case .emojis, .decimals:
            return "Compare on both devices"
        case .confirmed:
            return "Waiting for the other device"
        case .finished:
            return "Device verified"
        case .cancelled:
            return "Verification cancelled"
        case .failed:
            return "Verification failed"
        case .mismatched:
            return "Codes did not match"
        }
    }

    private var detail: String {
        switch state {
        case .requestReceived:
            return "Another session wants to verify this device."
        case .requestSent:
            return "Approve the request from one of your already trusted sessions."
        case .accepted:
            return "Start a secure emoji or number comparison. Only this device should start."
        case .sasStarted:
            return "Waiting for the other device to start comparison, or for codes to appear."
        case .keysExchanging:
            return "Both devices accepted the comparison. Secure codes will appear when the key exchange completes."
        case .emojis, .decimals:
            return "Only approve if the values match exactly on both devices."
        case .confirmed:
            return "This device accepted the codes. Wait for the other session to finish."
        case .finished:
            return "This device is now verified for encrypted Matrix sessions."
        case .cancelled:
            return "The verification flow was cancelled."
        case .failed:
            return "The verification flow could not be completed."
        case .mismatched:
            return "The security codes did not match. Verification was cancelled safely."
        }
    }

    private var terminalSystemImage: String {
        switch state {
        case .finished:
            return "checkmark.seal.fill"
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
            return .green
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
