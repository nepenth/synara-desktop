import SwiftUI

struct SettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: SettingsState = .idle
    @State private var isLogoutConfirmationPresented = false

    var body: some View {
        Form {
            Section {
                switch environment.session.currentState {
                case .signedIn(let session):
                    NavigationLink {
                        AccountSettingsView(session: session)
                    } label: {
                        SettingsSummaryRow(
                            title: session.userID,
                            subtitle: session.homeserverURL.host ?? session.homeserverURL.absoluteString,
                            systemImage: "person.crop.circle.fill"
                        )
                    }
                    .accessibilityIdentifier("AccountSettingsLink")
                case .signedOut:
                    Text("Not signed in")
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("SettingsAccountSignedOut")
                }
            }

            Section("Preferences") {
                NavigationLink {
                    NotificationSettingsView()
                } label: {
                    SettingsNavigationRow(title: "Notifications", systemImage: "bell")
                }
                .accessibilityIdentifier("NotificationSettingsLink")

                NavigationLink {
                    AppearanceSettingsView()
                } label: {
                    SettingsNavigationRow(title: "Appearance", systemImage: "textformat.size")
                }
                .accessibilityIdentifier("AppearanceSettingsLink")

                NavigationLink {
                    SecuritySettingsView()
                } label: {
                    SettingsNavigationRow(title: "Security & Recovery", systemImage: "lock.shield")
                }
                .accessibilityIdentifier("SecuritySettingsLink")
            }

            Section("About") {
                NavigationLink {
                    AboutSettingsView()
                } label: {
                    SettingsNavigationRow(title: "About Synara", systemImage: "info.circle")
                }
                .accessibilityIdentifier("AboutSettingsLink")

                NavigationLink {
                    LicensesSettingsView()
                } label: {
                    SettingsNavigationRow(title: "Licenses", systemImage: "doc.text")
                }
                .accessibilityIdentifier("LicensesSettingsLink")

                NavigationLink {
                    PrivacyPolicySettingsView()
                } label: {
                    SettingsNavigationRow(title: "Privacy Policy", systemImage: "hand.raised")
                }
                .accessibilityIdentifier("PrivacyPolicySettingsLink")

                NavigationLink {
                    SupportSettingsView()
                } label: {
                    SettingsNavigationRow(title: "Support", systemImage: "questionmark.circle")
                }
                .accessibilityIdentifier("SupportSettingsLink")
            }

            Section("Danger Zone") {
                Button(role: .destructive) {
                    isLogoutConfirmationPresented = true
                } label: {
                    if state.isLoading {
                        ProgressView()
                    } else {
                        Text("Log Out")
                    }
                }
                .disabled(state.isLoading)
                .accessibilityIdentifier("LogoutButton")
                .accessibilityHint("Requires confirmation and clears local session data from this device")
                .confirmationDialog(
                    "Log out of Synara?",
                    isPresented: $isLogoutConfirmationPresented,
                    titleVisibility: .visible
                ) {
                    Button("Log Out", role: .destructive) {
                        logout()
                    }
                    .accessibilityIdentifier("ConfirmLogoutButton")
                    Button("Cancel", role: .cancel) {}
                } message: {
                    Text("This clears local session data, sync state, cached rooms, and push registration on this device.")
                }
            }

            if case .failed(let message) = state {
                Section {
                    Text(message)
                        .foregroundStyle(.red)
                        .accessibilityIdentifier("LogoutErrorText")
                }
            }
        }
        .scrollContentBackground(.hidden)
        .background(SynaraChrome.settings)
        .settingsTabBarClearance()
        .navigationTitle("Settings")
        .accessibilityIdentifier("SettingsScreen")
    }

    private func logout() {
        state = .loading

        Task {
            do {
                try await environment.wipe.logoutAndWipe()
                await MainActor.run {
                    state = .idle
                    environment.logger.info("Local logout completed", category: .auth)
                }
            } catch {
                await MainActor.run {
                    state = .failed(LocalWipeError.sessionDeleteFailed.localizedDescription)
                    environment.logger.error("Local logout failed", category: .auth)
                }
            }
        }
    }
}

private struct AccountSettingsView: View {
    @Environment(\.appEnvironment) private var environment
    let session: AuthenticatedSession
    @State private var sessionDevices: [SharedCoreSessionDevice] = []
    @State private var ownPresence: SharedCorePresence?
    @State private var coreSessionIdentity: CoreSessionIdentity?

    var body: some View {
        Form {
            Section("Account") {
                let displayIdentity = SettingsAccountIdentitySelection.matchingCoreIdentity(
                    coreSessionIdentity,
                    for: session
                )
                SettingsInfoRow(title: "User", value: displayIdentity?.userID ?? session.userID)
                    .accessibilityIdentifier("SettingsAccountUser")
                SettingsInfoRow(title: "Device", value: displayIdentity?.deviceID ?? session.deviceID)
                    .accessibilityIdentifier("SettingsAccountDevice")
                SettingsInfoRow(
                    title: "Homeserver",
                    value: SettingsAccountIdentitySelection.homeserverDisplayValue(
                        for: displayIdentity,
                        fallback: session.homeserverURL
                    )
                )
                .accessibilityIdentifier("SettingsAccountHomeserver")
            }

            if let ownPresence,
               ownPresence.displayName != "Unknown" || ownPresence.statusMessage?.isEmpty == false
            {
                Section("Presence") {
                    SettingsInfoRow(title: "Status", value: ownPresence.displayName)
                        .accessibilityIdentifier("SettingsPresenceStatus")
                    if let status = ownPresence.statusMessage, status.isEmpty == false {
                        SettingsInfoRow(title: "Message", value: status)
                            .accessibilityIdentifier("SettingsPresenceMessage")
                    }
                }
            }

            if sessionDevices.isEmpty == false {
                Section("Sessions") {
                    ForEach(sessionDevices) { device in
                        SettingsInfoRow(
                            title: device.isCurrent ? "This device" : "Device",
                            value: device.displayName
                        )
                    }
                }
            }
        }
        .settingsTabBarClearance()
        .navigationTitle("Account")
        .accessibilityIdentifier("AccountSettingsScreen")
        .task {
            await refreshCoreSessionIdentity()
            let presence = await environment.matrix.presence(userID: session.userID)
            let devices = await environment.crypto.sessionDevices()
            await MainActor.run {
                ownPresence = presence
                sessionDevices = devices
            }
        }
    }

    private func refreshCoreSessionIdentity() async {
        let identity = await environment.matrix.coreSessionIdentity()
        await MainActor.run {
            coreSessionIdentity = identity
        }
    }
}

private struct NotificationSettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var notificationStatus: NotificationPermissionStatus = .unavailable
    @State private var isRequestingNotifications = false
    @State private var isRegisteringPush = false
    @State private var showLockScreenMessagePreviews = SynaraSharedConstants.defaultLockScreenMessagePreviews

    var body: some View {
        Form {
            Section("Permission") {
                VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                    Text(notificationStatus.displayName)
                        .font(SynaraTypography.body)
                    Text(notificationStatus.detail)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                }
                .accessibilityIdentifier("NotificationPermissionStatus")

                Button {
                    requestNotifications()
                } label: {
                    if isRequestingNotifications {
                        ProgressView()
                    } else {
                        Text(notificationStatus == .notDetermined ? "Enable Notifications" : "Refresh Notification Status")
                    }
                }
                .disabled(isRequestingNotifications)
                .accessibilityIdentifier("NotificationPermissionButton")
            }

            Section {
                SettingsInfoRow(title: "Status", value: environment.push.registrationStateDescription)
                Button {
                    isRegisteringPush = true
                    environment.push.beginRegistration()
                    isRegisteringPush = false
                } label: {
                    if isRegisteringPush {
                        ProgressView()
                    } else {
                        Text(environment.push.isRegistered ? "Re-register Push" : "Register Push")
                    }
                }
                .disabled(isRegisteringPush || environment.push.isRegistrationAvailable == false)
                .accessibilityIdentifier("PushRegistrationButton")
            } header: {
                Text("Delivery")
            } footer: {
                Text(environment.push.pushGatewayURL == nil ? "Push delivery is unavailable in this build." : "Push delivery is configured for this build.")
            }

            Section {
                Toggle("Show message previews on lock screen", isOn: $showLockScreenMessagePreviews)
                    .accessibilityIdentifier("LockScreenMessagePreviewsToggle")
                    .onChange(of: showLockScreenMessagePreviews) { value in
                        environment.settings.set(value, for: SynaraSharedConstants.lockScreenMessagePreviewsKey)
                    }
            } header: {
                Text("Privacy")
            } footer: {
                Text("Message content is looked up on this device and is not sent through the push gateway.")
                    .accessibilityIdentifier("LockScreenMessagePreviewsHelp")
            }
        }
        .settingsTabBarClearance()
        .navigationTitle("Notifications")
        .accessibilityIdentifier("NotificationSettingsScreen")
        .task {
            showLockScreenMessagePreviews = environment.settings.bool(
                for: SynaraSharedConstants.lockScreenMessagePreviewsKey
            )
            notificationStatus = await environment.notificationPermission.currentStatus()
        }
    }

    private func requestNotifications() {
        isRequestingNotifications = true
        Task {
            let status = await environment.notificationPermission.requestAuthorization()
            await MainActor.run {
                notificationStatus = status
                isRequestingNotifications = false
                environment.logger.info("Notification permission status refreshed", category: .push)
                if status.allowsPushRegistration {
                    environment.push.beginRegistration()
                }
            }
        }
    }
}

private struct AppearanceSettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.colorScheme) private var colorScheme
    @State private var baseColor = Color(synaraHex: SynaraThemeRamp.defaultBaseHex)
    @State private var hasCustomBaseColor = false
    @State private var didLoadBaseColor = false
    @State private var isBaseColorDirty = false
    @State private var persistTask: Task<Void, Never>?

    var body: some View {
        Form {
            Section("Theme") {
                SettingsInfoRow(title: "Appearance", value: "System")
                    .accessibilityIdentifier("AppearanceThemeRow")
                themeRampPreview
                ColorPicker("Base Color", selection: $baseColor, supportsOpacity: false)
                    .accessibilityIdentifier("AppearanceBaseColorPicker")
                    .onChange(of: baseColor) { newValue in
                        guard didLoadBaseColor else { return }
                        isBaseColorDirty = true
                        schedulePersist(newValue)
                    }
                Button("Reset Base Color") {
                    persistTask?.cancel()
                    resetBaseColor()
                }
                .disabled(hasCustomBaseColor == false)
                .accessibilityIdentifier("AppearanceBaseColorReset")
            } footer: {
                Text("Hue tint for chrome. Lightness is mapped to stacked greys (rail / room list / chat); this is not the fill color.")
            }
            Section {
                SettingsInfoRow(title: "Text Size", value: "Uses iOS Dynamic Type")
                    .accessibilityIdentifier("AppearanceTextSizeRow")
            } header: {
                Text("Text")
            } footer: {
                Text("Adjust text size in iOS Settings. Synara follows your system accessibility preferences.")
            }
        }
        .scrollContentBackground(.hidden)
        .background(SynaraChrome.settings)
        .settingsTabBarClearance()
        .navigationTitle("Appearance")
        .accessibilityIdentifier("AppearanceSettingsScreen")
        .onAppear {
            let stored = environment.settings.string(for: SynaraThemeRamp.storageKey)
            hasCustomBaseColor = SynaraThemeRamp.normalize(stored) != nil
            baseColor = Color(synaraHex: SynaraThemeRamp.resolve(stored))
            DispatchQueue.main.async {
                didLoadBaseColor = true
            }
        }
        .onDisappear {
            persistTask?.cancel()
            if isBaseColorDirty {
                commitBaseColor(baseColor)
            }
        }
    }

    private var themeRampPreview: some View {
        let tokens = SynaraThemeRamp.tokens(
            baseHex: SynaraThemeRamp.normalize(baseColor.synaraHexString()) ?? SynaraThemeRamp.defaultBaseHex,
            dark: colorScheme == .dark
        )
        return HStack(spacing: SynaraSpacing.medium) {
            rampSwatch(title: "Rail", hex: tokens.groupedSurface)
            rampSwatch(title: "List", hex: tokens.secondarySurface)
            rampSwatch(title: "Chat", hex: tokens.surface)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Theme ramp preview")
    }

    private func rampSwatch(title: String, hex: String) -> some View {
        VStack(spacing: SynaraSpacing.xSmall) {
            RoundedRectangle(cornerRadius: 4, style: .continuous)
                .fill(Color(synaraHex: hex))
                .frame(width: 28, height: 36)
                .overlay(
                    RoundedRectangle(cornerRadius: 4, style: .continuous)
                        .stroke(SynaraColor.separator.opacity(0.4), lineWidth: 0.5)
                )
            Text(title)
                .font(SynaraTypography.fineMeta)
                .foregroundStyle(SynaraColor.secondaryText)
        }
    }

    private func schedulePersist(_ color: Color) {
        persistTask?.cancel()
        persistTask = Task { @MainActor in
            try? await Task.sleep(nanoseconds: 280_000_000)
            guard Task.isCancelled == false else { return }
            commitBaseColor(color)
        }
    }

    private func commitBaseColor(_ color: Color) {
        guard let hex = color.synaraHexString() else { return }
        persistBaseColor(hex)
    }

    private func persistBaseColor(_ hex: String) {
        guard let normalized = SynaraThemeRamp.normalize(hex) else { return }
        environment.settings.setString(normalized, for: SynaraThemeRamp.storageKey)
        SynaraThemePaint.shared.reload()
        hasCustomBaseColor = true
        isBaseColorDirty = false
    }

    private func resetBaseColor() {
        persistTask?.cancel()
        environment.settings.setString(nil, for: SynaraThemeRamp.storageKey)
        SynaraThemePaint.shared.reload()
        hasCustomBaseColor = false
        isBaseColorDirty = false
        baseColor = Color(synaraHex: SynaraThemeRamp.defaultBaseHex)
    }
}

private struct SecuritySettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var sessionCryptoStatus: SessionCryptoStatus = .unknown
    @State private var recoveryKey = ""
    @State private var cryptoActionMessage: String?
    @State private var isRunningCryptoAction = false

    var body: some View {
        Form {
            Section("Protection") {
                SettingsInfoRow(title: "Session Storage", value: "Keychain")
                    .accessibilityIdentifier("SecuritySessionStorageRow")
                SettingsInfoRow(title: "Message Security", value: "Matrix Rust SDK")
                    .accessibilityIdentifier("SecurityMatrixSDKRow")
                SettingsInfoRow(title: "Device Verification", value: sessionCryptoStatus.verification.settingsDisplayName)
                    .accessibilityIdentifier("SecurityDeviceVerificationRow")
                SettingsInfoRow(title: "Key Recovery", value: sessionCryptoStatus.recovery.settingsDisplayName)
                    .accessibilityIdentifier("SecurityKeyRecoveryRow")
                SettingsInfoRow(title: "Key Backup", value: sessionCryptoStatus.backup.settingsDisplayName)
                    .accessibilityIdentifier("SecurityKeyBackupRow")
                SettingsInfoRow(title: "Decryption Issues", value: sessionCryptoStatus.unableToDecryptCount == 0 ? "None" : "\(sessionCryptoStatus.unableToDecryptCount)")
                    .accessibilityIdentifier("SecurityDecryptionIssuesRow")
            }

            if sessionCryptoStatus.verification != .verified,
               sessionCryptoStatus.hasDevicesToVerifyAgainst == true
            {
                Section {
                    Button {
                        runCryptoAction {
                            await environment.crypto.requestDeviceVerification()
                        }
                    } label: {
                        cryptoActionLabel("Verify This Device")
                    }
                    .disabled(isRunningCryptoAction)
                    .accessibilityIdentifier("RequestDeviceVerificationButton")
                } footer: {
                    Text("Compare emoji or number codes with another already verified session. Synara does not mark this device verified until both sides confirm.")
                }
            }

            Section {
                SecureField("Recovery key", text: $recoveryKey)
                    .textContentType(.oneTimeCode)
                    .accessibilityIdentifier("RecoveryKeyField")
                Button {
                    let key = recoveryKey
                    runCryptoAction {
                        await environment.crypto.recover(recoveryKey: key)
                    } onComplete: {
                        recoveryKey = ""
                    }
                } label: {
                    cryptoActionLabel("Recover Keys")
                }
                .disabled(isRunningCryptoAction || recoveryKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .accessibilityIdentifier("RecoverKeysButton")
            } header: {
                Text("Recovery")
            } footer: {
                Text("Recovery keys are used only for this request and are not stored by Synara.")
            }

            if let cryptoActionMessage {
                Section {
                    Text(cryptoActionMessage)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("CryptoActionMessage")
                }
            }
        }
        .settingsTabBarClearance()
        .navigationTitle("Security")
        .accessibilityIdentifier("SecuritySettingsScreen")
        .task {
            sessionCryptoStatus = await environment.crypto.sessionStatus()
        }
    }

    @ViewBuilder
    private func cryptoActionLabel(_ title: String) -> some View {
        if isRunningCryptoAction {
            ProgressView()
        } else {
            Text(title)
        }
    }

    private func runCryptoAction(
        _ action: @escaping () async -> CryptoActionResult,
        onComplete: @escaping @MainActor () -> Void = {}
    ) {
        isRunningCryptoAction = true
        cryptoActionMessage = nil
        Task {
            let result = await action()
            let status = await environment.crypto.sessionStatus()
            await MainActor.run {
                sessionCryptoStatus = status
                cryptoActionMessage = result.message
                isRunningCryptoAction = false
                onComplete()
            }
        }
    }
}

private struct SettingsInfoRow: View {
    let title: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            Text(title)
                .font(.caption)
                .foregroundStyle(SynaraColor.secondaryText)
            Text(value)
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.primaryText)
                .textSelection(.enabled)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(title), \(value)")
    }
}

private struct SettingsSummaryRow: View {
    let title: String
    let subtitle: String
    let systemImage: String

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Image(systemName: systemImage)
                .font(.title2)
                .foregroundStyle(SynaraColor.accent)
                .frame(width: 32, height: 32)
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(title)
                    .font(SynaraTypography.body)
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
                Text(subtitle)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(1)
            }
        }
        .padding(.vertical, SynaraSpacing.xSmall)
    }
}

private struct SettingsNavigationRow: View {
    let title: String
    let systemImage: String

    var body: some View {
        Label(title, systemImage: systemImage)
            .font(SynaraTypography.body)
    }
}

private extension SynaraCryptoVerificationStatus {
    var settingsDisplayName: String {
        switch self {
        case .verified:
            return "Verified"
        case .unverified:
            return "Unverified"
        case .unknown:
            return "Unknown"
        }
    }
}

private extension SynaraCryptoRecoveryStatus {
    var settingsDisplayName: String {
        switch self {
        case .enabled:
            return "Enabled"
        case .disabled:
            return "Disabled"
        case .incomplete:
            return "Needs Recovery"
        case .unknown:
            return "Unknown"
        }
    }
}

private extension SynaraCryptoBackupStatus {
    var settingsDisplayName: String {
        switch self {
        case .enabled:
            return "Enabled"
        case .unavailable:
            return "Unavailable"
        case .syncing:
            return "Syncing"
        case .unknown:
            return "Unknown"
        }
    }
}

private struct AboutSettingsView: View {
    var body: some View {
        List {
            Section {
                VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                    Text("Synara")
                        .font(SynaraTypography.screenTitle)
                    Text("Agentic native Matrix client for iOS, macOS, and Linux.")
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                }
                .padding(.vertical, SynaraSpacing.small)
                .accessibilityIdentifier("AboutSummary")
            }

            Section("Build") {
                SettingsInfoRow(title: "Version", value: AppBuildInfo.version)
                    .accessibilityIdentifier("AboutVersionRow")
                SettingsInfoRow(title: "Build", value: AppBuildInfo.build)
                    .accessibilityIdentifier("AboutBuildRow")
                SettingsInfoRow(title: "Bundle", value: AppBuildInfo.bundleIdentifier)
                    .accessibilityIdentifier("AboutBundleRow")
            }

            Section("Links") {
                Link(destination: SettingsLink.privacy.url) {
                    SettingsNavigationRow(title: "Privacy Policy", systemImage: "hand.raised")
                }
                .accessibilityIdentifier("AboutPrivacyLink")

                Link(destination: SettingsLink.support.url) {
                    SettingsNavigationRow(title: "Support", systemImage: "questionmark.circle")
                }
                .accessibilityIdentifier("AboutSupportLink")
            }
        }
        .settingsTabBarClearance()
        .navigationTitle("About")
        .accessibilityIdentifier("AboutSettingsScreen")
    }
}

private struct LicensesSettingsView: View {
    var body: some View {
        List {
            Section("App") {
                LicenseRow(name: "Synara", license: "AGPL-3.0-only", note: "Final App Store distribution requires legal review of repository licensing.")
            }

            Section("Dependencies") {
                LicenseRow(name: "Matrix Rust SDK Swift", license: "Apache-2.0", note: "Pinned through Swift Package Manager.")
                LicenseRow(name: "Apple SwiftUI and iOS SDK", license: "Apple platform SDK", note: "Provided by Xcode and iOS.")
            }
        }
        .settingsTabBarClearance()
        .navigationTitle("Licenses")
        .accessibilityIdentifier("LicensesSettingsScreen")
    }
}

private struct LicenseRow: View {
    let name: String
    let license: String
    let note: String

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            Text(name)
                .font(SynaraTypography.body)
            Text(license)
                .font(.caption.weight(.semibold))
                .foregroundStyle(SynaraColor.secondaryText)
            Text(note)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
        }
        .accessibilityElement(children: .combine)
    }
}

private struct PrivacyPolicySettingsView: View {
    var body: some View {
        List {
            Section("Privacy Policy") {
                Link(SettingsLink.privacy.displayTitle, destination: SettingsLink.privacy.url)
                    .accessibilityIdentifier("PrivacyPolicyExternalLink")
                Text("External TestFlight and App Store submission remain blocked until the final privacy policy is approved.")
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
            }

            Section("Current Data Inventory") {
                DataInventoryRow(title: "Account", detail: "Matrix user ID, device ID, homeserver, and access tokens are stored locally for session restore.")
                DataInventoryRow(title: "Messages and Rooms", detail: "Room metadata, timeline content, media, and account data come from the selected Matrix homeserver.")
                DataInventoryRow(title: "Notifications", detail: "APNs device tokens and Matrix pusher registration are used only when notifications are enabled.")
                DataInventoryRow(title: "Diagnostics", detail: "No analytics or crash SDK is enabled. Logs are local and redacted.")
            }
        }
        .settingsTabBarClearance()
        .navigationTitle("Privacy")
        .accessibilityIdentifier("PrivacyPolicySettingsScreen")
    }
}

private struct SupportSettingsView: View {
    var body: some View {
        List {
            Section("Support") {
                Link(SettingsLink.support.displayTitle, destination: SettingsLink.support.url)
                    .accessibilityIdentifier("SupportExternalLink")
                Text("Include app version, build number, iOS version, homeserver domain, and a short description. Do not send passwords, access tokens, recovery keys, or private room content.")
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
            }

            Section("Diagnostics") {
                SettingsInfoRow(title: "Version", value: AppBuildInfo.version)
                SettingsInfoRow(title: "Build", value: AppBuildInfo.build)
                SettingsInfoRow(title: "Bundle", value: AppBuildInfo.bundleIdentifier)
            }
        }
        .settingsTabBarClearance()
        .navigationTitle("Support")
        .accessibilityIdentifier("SupportSettingsScreen")
    }
}

private struct DataInventoryRow: View {
    let title: String
    let detail: String

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            Text(title)
                .font(SynaraTypography.body)
            Text(detail)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
        }
        .accessibilityElement(children: .combine)
    }
}

private extension View {
    func settingsTabBarClearance() -> some View {
        safeAreaInset(edge: .bottom, spacing: 0) {
            Color.clear
                .frame(height: 72)
                .accessibilityHidden(true)
        }
    }
}

private enum SettingsLink {
    case privacy
    case support

    var displayTitle: String {
        switch self {
        case .privacy:
            return "https://synara.app/privacy"
        case .support:
            return "support@synara.app"
        }
    }

    var url: URL {
        switch self {
        case .privacy:
            return URL(string: "https://synara.app/privacy")!
        case .support:
            return URL(string: "mailto:support@synara.app")!
        }
    }
}

private enum AppBuildInfo {
    static var version: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "Unknown"
    }

    static var build: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "Unknown"
    }

    static var bundleIdentifier: String {
        Bundle.main.bundleIdentifier ?? "Unknown"
    }
}

private enum SettingsState {
    case idle
    case loading
    case failed(String)

    var isLoading: Bool {
        if case .loading = self {
            return true
        }
        return false
    }
}

/// Selects Core's optional values only as a matching account-display factor.
/// Swift session state remains authoritative for all lifecycle and security use.
enum SettingsAccountIdentitySelection {
    static func matchingCoreIdentity(
        _ candidate: CoreSessionIdentity?,
        for session: AuthenticatedSession
    ) -> CoreSessionIdentity? {
        guard let candidate,
              candidate.userID == session.userID,
              candidate.deviceID == session.deviceID,
              candidate.homeserverURL == session.homeserverURL.absoluteString
        else {
            return nil
        }
        return candidate
    }

    static func homeserverDisplayValue(for identity: CoreSessionIdentity?, fallback: URL) -> String {
        guard let identity, let homeserverURL = URL(string: identity.homeserverURL) else {
            return fallback.host ?? fallback.absoluteString
        }
        return homeserverURL.host ?? homeserverURL.absoluteString
    }
}

struct SettingsView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            SettingsView()
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
