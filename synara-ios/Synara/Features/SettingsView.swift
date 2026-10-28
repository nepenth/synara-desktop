import PhotosUI
import SwiftUI
import SynaraCore
import UIKit
import UniformTypeIdentifiers
import UserNotifications

/// Log Out confirmation copy, including the last-device warning.
enum SignOutCopy {
    static let standardMessage =
        String(localized: "This clears local sign-in data and cached rooms. Synara also attempts to revoke this session and remove its push registration; remote cleanup requires a connection.")
    static let lastDeviceWarning =
        String(localized: "This is your last signed-in device. Save your recovery key before logging out, or you will lose access to your encrypted messages.")

    /// `isLastDevice` is `nil` while the device list is unknown; only a
    /// confirmed last device shows the warning.
    static func message(isLastDevice: Bool?) -> String {
        isLastDevice == true ? "\(lastDeviceWarning)\n\n\(standardMessage)" : standardMessage
    }

    static func isOtherSignedInDevice(isCurrent: Bool, trust: DeviceTrustDto) -> Bool {
        isCurrent == false && trust != .dehydrated
    }
}

/// The one Log Out confirmation behind every sign-out entry point (Settings,
/// the account menu, the connection banner). A request only opens the dialog
/// and reads the device list for the last-device warning; nothing signs out
/// until `confirm`.
@MainActor
final class SignOutConfirmationModel: ObservableObject {
    @Published var isPresented = false
    @Published private(set) var isLastDevice: Bool?
    private var requestGeneration = 0

    var message: String {
        SignOutCopy.message(isLastDevice: isLastDevice)
    }

    /// `known` is the crypto status already on hand; `refresh` reads the
    /// current device list.
    func request(
        known: SessionCryptoStatus,
        refresh: @escaping () async -> SessionCryptoStatus
    ) {
        requestGeneration += 1
        let generation = requestGeneration
        isLastDevice = known.isLastDevice
        isPresented = true
        Task { @MainActor in
            let status = await refresh()
            guard generation == requestGeneration else { return }
            isLastDevice = status.isLastDevice
        }
    }

    func request(using store: SessionCryptoStore) {
        request(known: store.status, refresh: { await store.refresh() })
    }

    func confirm(_ signOut: () -> Void) {
        isPresented = false
        signOut()
    }
}

private struct SignOutConfirmationDialog: ViewModifier {
    @ObservedObject var model: SignOutConfirmationModel
    let confirmIdentifier: String
    let onConfirm: () -> Void

    func body(content: Content) -> some View {
        content.confirmationDialog(
            "Log out of Synara?",
            isPresented: $model.isPresented,
            titleVisibility: .visible
        ) {
            Button("Log Out", role: .destructive) {
                model.confirm(onConfirm)
            }
            .accessibilityIdentifier(confirmIdentifier)
            Button("Cancel", role: .cancel) {}
        } message: {
            Text(model.message)
        }
    }
}

extension View {
    /// Settings' Log Out confirmation, including the last-device warning.
    func signOutConfirmation(
        _ model: SignOutConfirmationModel,
        confirmIdentifier: String,
        onConfirm: @escaping () -> Void
    ) -> some View {
        modifier(SignOutConfirmationDialog(model: model, confirmIdentifier: confirmIdentifier, onConfirm: onConfirm))
    }
}

struct SettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: SettingsState = .idle
    @State private var logoutAttempt = 0
    @State private var inFlightLogout: Task<Void, Error>?
    @StateObject private var logoutConfirmation = SignOutConfirmationModel()

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
                    SettingsNavigationRow(title: String(localized: "Notifications"), systemImage: "bell")
                }
                .accessibilityIdentifier("NotificationSettingsLink")

                NavigationLink {
                    AppearanceSettingsView()
                } label: {
                    SettingsNavigationRow(title: String(localized: "Appearance"), systemImage: "textformat.size")
                }
                .accessibilityIdentifier("AppearanceSettingsLink")

                NavigationLink {
                    SecuritySettingsView()
                } label: {
                    SettingsNavigationRow(title: String(localized: "Security & Recovery"), systemImage: "lock.shield")
                }
                .accessibilityIdentifier("SecuritySettingsLink")

                NavigationLink {
                    RichLinkSettingsView()
                } label: {
                    SettingsNavigationRow(title: String(localized: "GIFs & Link Previews"), systemImage: "play.rectangle")
                }
                .accessibilityIdentifier("RichLinkSettingsLink")
            }

            Section("About") {
                NavigationLink {
                    AboutSettingsView()
                } label: {
                    SettingsNavigationRow(title: String(localized: "About Synara"), systemImage: "info.circle")
                }
                .accessibilityIdentifier("AboutSettingsLink")

                NavigationLink {
                    LicensesSettingsView()
                } label: {
                    SettingsNavigationRow(title: String(localized: "Licenses"), systemImage: "doc.text")
                }
                .accessibilityIdentifier("LicensesSettingsLink")

                NavigationLink {
                    PrivacyPolicySettingsView()
                } label: {
                    SettingsNavigationRow(title: String(localized: "Privacy Policy"), systemImage: "hand.raised")
                }
                .accessibilityIdentifier("PrivacyPolicySettingsLink")

                NavigationLink {
                    SupportSettingsView()
                } label: {
                    SettingsNavigationRow(title: String(localized: "Support"), systemImage: "questionmark.circle")
                }
                .accessibilityIdentifier("SupportSettingsLink")
            }

            Section("Danger Zone") {
                Button(role: .destructive) {
                    logoutConfirmation.request(using: environment.sessionCrypto)
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
                .signOutConfirmation(logoutConfirmation, confirmIdentifier: "ConfirmLogoutButton") {
                    logout()
                }
            }

            if case .failed(let message) = state {
                Section {
                    Text(message)
                        .foregroundStyle(SynaraColor.criticalText)
                        .accessibilityIdentifier("LogoutErrorText")
                }
            }
        }
        .scrollContentBackground(.hidden)
        .background {
            // Continue the settings plane behind translucent system chrome;
            // RootShellView owns the clear scroll tail above the tab bar.
            SynaraChrome.settings
                .ignoresSafeArea()
        }
        .navigationTitle("Settings")
        .accessibilityIdentifier("SettingsScreen")
    }

    private func logout() {
        state = .loading
        logoutAttempt += 1
        let attempt = logoutAttempt
        // Reuse a logout that outlived an earlier timeout instead of starting a
        // second one; Sign Out never adds a store wipe of its own here.
        let work: Task<Void, Error>
        if let inFlightLogout {
            work = inFlightLogout
        } else {
            let wipe = environment.wipe
            work = Task { try await wipe.logoutAndWipe() }
            inFlightLogout = work
            // Forget the task once it settles, even after a timeout, so a later
            // tap starts a fresh attempt instead of rereading a stale failure.
            Task {
                _ = try? await work.value
                await MainActor.run {
                    if inFlightLogout == work {
                        inFlightLogout = nil
                    }
                }
            }
        }

        Task {
            let outcome: Result<Void, Error>
            do {
                try await BoundedSignOut.wait(for: work)
                outcome = .success(())
            } catch {
                outcome = .failure(error)
            }
            await MainActor.run {
                // A later attempt owns the screen state. A timeout leaves the
                // spinner with the retry copy while the logout keeps running.
                guard attempt == logoutAttempt else {
                    return
                }
                switch outcome {
                case .success:
                    state = .idle
                    environment.logger.info("Local logout completed", category: .auth)
                case .failure(let error):
                    state = .failed(LocalWipeError.displayMessage(for: error))
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
    @State private var presenceDraft = "online"
    @State private var isSettingPresence = false
    @State private var statusEmojiDraft = ""
    @State private var statusTextDraft = ""
    @State private var isSettingStatus = false
    @State private var statusMessage: String?
    @State private var rtcTransports: SharedCoreRtcTransportsSnapshot?
    @State private var isRefreshingRtc = false
    @State private var coreSessionIdentity: CoreSessionIdentity?
    @State private var signOutDevice: SharedCoreSessionDevice?
    @State private var signOutPassword = ""
    @State private var signOutMessage: String?
    @State private var isSigningOut = false
    @State private var isLoadingSessions = true
    @StateObject private var sessionCrypto = SessionCryptoStatusObserver()
    @State private var isRequestingVerification = false
    @State private var verificationMessage: String?
    @State private var displayNameDraft = ""
    @State private var isSavingDisplayName = false
    @State private var displayNameMessage: String?
    @State private var selectedAvatarPhoto: PhotosPickerItem?
    @State private var isUploadingAvatar = false
    @State private var emails: [String] = []
    @State private var emailDraft = ""
    @State private var emailPassword = ""
    @State private var needsEmailPassword = false
    @State private var emailMessage: String?
    @State private var ignoredUserIDs: [String] = []
    @State private var ignoreDraft = ""
    @State private var ignoreMessage: String?

    var body: some View {
        Form {
            Section("Account") {
                let displayIdentity = SettingsAccountIdentitySelection.matchingCoreIdentity(
                    coreSessionIdentity,
                    for: session
                )
                SettingsInfoRow(title: String(localized: "User"), value: displayIdentity?.userID ?? session.userID)
                    .accessibilityIdentifier("SettingsAccountUser")
                SettingsInfoRow(
                    title: String(localized: "Device"),
                    value: displayIdentity?.deviceID ?? session.deviceID
                )
                    .accessibilityIdentifier("SettingsAccountDevice")
                SettingsInfoRow(
                    title: String(localized: "Homeserver"),
                    value: SettingsAccountIdentitySelection.homeserverDisplayValue(
                        for: displayIdentity,
                        fallback: session.homeserverURL
                    )
                )
                .accessibilityIdentifier("SettingsAccountHomeserver")
            }

            Section {
                TextField("Display name", text: $displayNameDraft)
                    .textInputAutocapitalization(.words)
                    .accessibilityIdentifier("SettingsDisplayNameField")
                Button {
                    saveDisplayName()
                } label: {
                    if isSavingDisplayName {
                        ProgressView()
                    } else {
                        Text("Save Display Name")
                    }
                }
                .disabled(isSavingDisplayName || displayNameDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .accessibilityIdentifier("SettingsSaveDisplayNameButton")
            } header: {
                Text("Profile")
            } footer: {
                Text("This is the name other people see in rooms. It is stored on your homeserver.")
            }

            Section {
                PhotosPicker(selection: $selectedAvatarPhoto, matching: .images) {
                    if isUploadingAvatar {
                        ProgressView()
                    } else {
                        Label("Upload Avatar", systemImage: "photo")
                    }
                }
                .disabled(isUploadingAvatar)
                .accessibilityIdentifier("SettingsAvatarUploadButton")
            } header: {
                Text("Avatar")
            } footer: {
                Text("Photos are uploaded to your homeserver, then set as your profile avatar.")
            }

            Section {
                ForEach(emails, id: \.self) { address in
                    Button(role: .destructive) {
                        Task {
                            _ = await environment.matrix.deleteThreepidEmail(address)
                            emails = await environment.matrix.threepidEmails()
                        }
                    } label: {
                        Text(address)
                    }
                }
                TextField("you@example.org", text: $emailDraft)
                    .textInputAutocapitalization(.never)
                    .keyboardType(.emailAddress)
                    .accessibilityIdentifier("SettingsEmailField")
                Button("Add Email") {
                    addEmail()
                }
                .disabled(emailDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .accessibilityIdentifier("SettingsAddEmailButton")
                if needsEmailPassword {
                    SecureField("Account password", text: $emailPassword)
                    Button("Confirm Email") {
                        confirmEmail()
                    }
                    .disabled(emailPassword.isEmpty)
                }
                if let emailMessage {
                    Text(emailMessage)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                }
            } header: {
                Text("Contact")
            }

            Section {
                ForEach(ignoredUserIDs, id: \.self) { userID in
                    Button(role: .destructive) {
                        Task {
                            _ = await environment.matrix.unignoreUser(userID)
                            ignoredUserIDs = await environment.matrix.ignoredUserIDs()
                        }
                    } label: {
                        Text(userID)
                    }
                }
                TextField("@user:server", text: $ignoreDraft)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .accessibilityIdentifier("SettingsIgnoreUserField")
                Button("Block User") {
                    blockUser()
                }
                .disabled(ignoreDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .accessibilityIdentifier("SettingsBlockUserButton")
                if let ignoreMessage {
                    Text(ignoreMessage)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                }
            } header: {
                Text("Blocked Users")
            } footer: {
                Text("Blocked users cannot message or invite you.")
            }

            if let displayNameMessage {
                Section {
                    Text(displayNameMessage)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("SettingsDisplayNameMessage")
                }
            }

            Section("Presence") {
                Picker("Status", selection: $presenceDraft) {
                    Text("Online").tag("online")
                    Text("Away").tag("unavailable")
                    Text("Offline").tag("offline")
                }
                .disabled(isSettingPresence)
                .accessibilityIdentifier("SettingsPresenceStatus")
                .onChange(of: presenceDraft) { newValue in
                    applyPresence(newValue)
                }
                if let status = ownPresence?.statusMessage, status.isEmpty == false {
                    SettingsInfoRow(title: String(localized: "Message"), value: status)
                        .accessibilityIdentifier("SettingsPresenceMessage")
                }
            }

            Section("Status") {
                TextField("Emoji", text: $statusEmojiDraft)
                    .disabled(isSettingStatus)
                    .accessibilityIdentifier("SettingsUserStatusEmoji")
                TextField("What are you doing?", text: $statusTextDraft)
                    .disabled(isSettingStatus)
                    .accessibilityIdentifier("SettingsUserStatusText")
                Button("Save status") {
                    applyUserStatus()
                }
                .disabled(isSettingStatus)
                .accessibilityIdentifier("SettingsUserStatusSave")
                Button("Clear status") {
                    clearUserStatus()
                }
                .disabled(isSettingStatus)
                .accessibilityIdentifier("SettingsUserStatusClear")
                if let statusMessage {
                    Text(statusMessage)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("SettingsUserStatusMessage")
                }
            }

            Section {
                Text(
                    rtcTransports.map(SharedCoreRtcTransports.diagnosticCopy)
                        ?? String(localized: "Native MatrixRTC transports are unavailable.")
                )
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityIdentifier("SettingsRtcTransportsDiagnostic")
                Button {
                    refreshRtcTransports()
                } label: {
                    if isRefreshingRtc {
                        ProgressView()
                    } else {
                        Text("Refresh transport")
                    }
                }
                .disabled(isRefreshingRtc)
                .accessibilityIdentifier("SettingsRtcTransportsRefresh")
            } header: {
                Text("MatrixRTC")
            } footer: {
                Text("Homeserver call transport discovery. Synara does not join calls from this screen.")
            }

            Section {
                if isLoadingSessions && sessionDevices.isEmpty {
                    ProgressView()
                        .accessibilityIdentifier("SettingsSessionsLoading")
                } else if sessionDevices.isEmpty {
                    Text("No other sessions were returned for this account.")
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("SettingsSessionsEmpty")
                } else {
                    ForEach(sessionDevices) { device in
                        SessionDeviceRow(
                            device: device,
                            isSigningOut: isSigningOut,
                            isRequestingVerification: isRequestingVerification
                        ) {
                            requestVerification(with: device)
                        } onSignOut: {
                            signOutPassword = ""
                            signOutMessage = nil
                            signOutDevice = device
                        }
                    }
                }
            } header: {
                Text("Sessions")
            } footer: {
                Text("Log out a session to revoke it on the homeserver. This device uses Log Out instead.")
            }

            if sessionCrypto.showsVerifyThisDevice {
                Section {
                    Button {
                        isRequestingVerification = true
                        verificationMessage = nil
                        Task {
                            let result = await environment.crypto.requestDeviceVerification()
                            await MainActor.run {
                                verificationMessage = result.message
                                isRequestingVerification = false
                            }
                        }
                    } label: {
                        if isRequestingVerification {
                            ProgressView()
                        } else {
                            Text("Verify This Device")
                        }
                    }
                    .disabled(isRequestingVerification || sessionCrypto.enablesVerifyThisDevice == false)
                    .accessibilityIdentifier("AccountVerifyThisDeviceButton")

                    if sessionCrypto.status.hasDevicesToVerifyAgainst == nil {
                        Button("Retry Availability Check") {
                            Task {
                                await sessionCrypto.refresh(store: environment.sessionCrypto)
                            }
                        }
                        .disabled(isRequestingVerification)
                        .accessibilityIdentifier("AccountRetryVerificationAvailabilityButton")
                    }
                } footer: {
                    Text(sessionCrypto.verificationAvailabilityMessage)
                }
            }

            if let signOutMessage {
                Section {
                    Text(signOutMessage)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("SettingsSessionSignOutMessage")
                }
            }

            if let verificationMessage {
                Section {
                    Text(verificationMessage)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("AccountVerificationMessage")
                }
            }
        }
        .refreshable {
            await refreshSessionDevices()
        }
        .navigationTitle("Account")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    Task {
                        await refreshSessionDevices()
                    }
                } label: {
                    Label("Refresh Sessions", systemImage: "arrow.clockwise")
                }
                .disabled(isLoadingSessions)
                .accessibilityIdentifier("RefreshSessionsButton")
            }
        }
        .accessibilityIdentifier("AccountSettingsScreen")
        .alert(
            "Log out session?",
            isPresented: Binding(
                get: { signOutDevice != nil },
                set: { presented in
                    if presented == false {
                        signOutDevice = nil
                        signOutPassword = ""
                    }
                }
            )
        ) {
            SecureField("Account password", text: $signOutPassword)
                .textContentType(.password)
            Button("Log Out", role: .destructive) {
                confirmSignOut()
            }
            .disabled(isSigningOut || signOutPassword.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            .accessibilityIdentifier("ConfirmSignOutSessionButton")
            Button("Cancel", role: .cancel) {
                signOutDevice = nil
                signOutPassword = ""
            }
        } message: {
            Text("Enter your account password to revoke \(signOutDevice?.displayName ?? String(localized: "this session")).")
        }
        .task {
            await sessionCrypto.start(store: environment.sessionCrypto)
        }
        // The session list follows the store's coalesced device wakeups
        // instead of a second subscription of its own.
        .onReceive(environment.sessionCrypto.$wakeupRevision.dropFirst()) { _ in
            Task { await refreshSessionDevices() }
        }
        .onChange(of: selectedAvatarPhoto) { item in
            if let item {
                uploadAvatar(item)
            }
        }
        .task {
            await refreshCoreSessionIdentity()
            let presence = await environment.matrix.presence(userID: session.userID)
            let status = await environment.matrix.userStatus(userID: session.userID)
            let rtc = await environment.matrix.rtcTransportsSnapshot()
            let devices = await environment.crypto.sessionDevices()
            let loadedEmails = await environment.matrix.threepidEmails()
            let loadedIgnored = await environment.matrix.ignoredUserIDs()
            let profile = await environment.matrix.ownProfile()
            await MainActor.run {
                ownPresence = presence
                statusEmojiDraft = status?.userStatus?.emoji ?? ""
                statusTextDraft = status?.userStatus?.text ?? ""
                rtcTransports = rtc
                if let state = presence?.state, ["online", "unavailable", "offline"].contains(state) {
                    presenceDraft = state
                }
                sessionDevices = devices
                emails = loadedEmails
                ignoredUserIDs = loadedIgnored
                isLoadingSessions = false
                if let homeserverDisplayName = profile?.displayName?
                    .trimmingCharacters(in: .whitespacesAndNewlines),
                   homeserverDisplayName.isEmpty == false {
                    displayNameDraft = homeserverDisplayName
                } else if displayNameDraft.isEmpty {
                    displayNameDraft = session.userID.split(separator: ":").first.map(String.init)?
                        .replacingOccurrences(of: "@", with: "")
                        ?? session.userID
                }
            }
        }
    }

    private func refreshRtcTransports() {
        isRefreshingRtc = true
        Task {
            let next = await environment.matrix.rtcTransportsRefresh()
            await MainActor.run {
                rtcTransports = next
                isRefreshingRtc = false
            }
        }
    }

    private func applyUserStatus() {
        isSettingStatus = true
        statusMessage = nil
        Task {
            let ok = await environment.matrix.setOwnUserStatus(
                emoji: statusEmojiDraft,
                text: statusTextDraft
            )
            await MainActor.run {
                isSettingStatus = false
                statusMessage = ok
                    ? String(localized: "Status updated.")
                    : String(localized: "This homeserver does not support user status.")
            }
        }
    }

    private func clearUserStatus() {
        isSettingStatus = true
        statusMessage = nil
        Task {
            let ok = await environment.matrix.clearOwnUserStatus()
            await MainActor.run {
                isSettingStatus = false
                if ok {
                    statusEmojiDraft = ""
                    statusTextDraft = ""
                    statusMessage = String(localized: "Status cleared.")
                } else {
                    statusMessage = String(localized: "This homeserver does not support user status.")
                }
            }
        }
    }

    private func applyPresence(_ state: String) {
        guard ["online", "unavailable", "offline"].contains(state) else { return }
        if state == ownPresence?.state { return }
        isSettingPresence = true
        Task {
            let ok = await environment.matrix.setOwnPresence(state)
            await MainActor.run {
                isSettingPresence = false
                if ok {
                    ownPresence = SharedCorePresenceLive.presence(
                        userId: session.userID,
                        state: state,
                        currentlyActive: state == "online",
                        statusMsg: ownPresence?.statusMessage
                    )
                } else if let previous = ownPresence?.state,
                          ["online", "unavailable", "offline"].contains(previous) {
                    presenceDraft = previous
                }
            }
        }
    }

    private func uploadAvatar(_ item: PhotosPickerItem) {
        isUploadingAvatar = true
        displayNameMessage = nil
        Task {
            do {
                guard let data = try await item.loadTransferable(type: Data.self), data.isEmpty == false else {
                    throw NSError(domain: "synara.avatar", code: 1)
                }
                let status = await environment.matrix.uploadOwnAvatarStatus(payload: data, mimeType: "image/jpeg")
                await MainActor.run {
                    selectedAvatarPhoto = nil
                    isUploadingAvatar = false
                    displayNameMessage = status.message(for: .avatar)
                }
            } catch {
                await MainActor.run {
                    selectedAvatarPhoto = nil
                    isUploadingAvatar = false
                    displayNameMessage = String(localized: "Could not update avatar.")
                }
            }
        }
    }

    private func addEmail() {
        let email = emailDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard email.isEmpty == false else { return }
        emailMessage = nil
        Task {
            let requested = await environment.matrix.requestThreepidEmailToken(email)
            guard requested else {
                await MainActor.run { emailMessage = String(localized: "Could not send verification email.") }
                return
            }
            let status = await environment.matrix.addThreepidEmail()
            await MainActor.run {
                if status == .authenticationRequired {
                    needsEmailPassword = true
                    emailMessage = String(localized: "Enter your account password to confirm this email.")
                } else if status == .ok {
                    emailDraft = ""
                    needsEmailPassword = false
                    emailMessage = String(localized: "Email attached.")
                } else {
                    emailMessage = String(localized: "Could not attach this email.")
                }
            }
            emails = await environment.matrix.threepidEmails()
        }
    }

    private func confirmEmail() {
        let password = emailPassword
        emailPassword = ""
        Task {
            let status = await environment.matrix.addThreepidEmailPassword(password)
            await MainActor.run {
                if status == .ok {
                    emailDraft = ""
                    needsEmailPassword = false
                    emailMessage = String(localized: "Email attached.")
                } else {
                    emailMessage = String(localized: "Could not confirm this email.")
                }
            }
            emails = await environment.matrix.threepidEmails()
        }
    }

    private func blockUser() {
        let userID = ignoreDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard userID.hasPrefix("@") else {
            ignoreMessage = String(localized: "Enter a full Matrix user ID.")
            return
        }
        ignoreMessage = nil
        Task {
            let ok = await environment.matrix.ignoreUser(userID)
            let loaded = await environment.matrix.ignoredUserIDs()
            await MainActor.run {
                ignoredUserIDs = loaded
                if ok {
                    ignoreDraft = ""
                } else {
                    ignoreMessage = String(localized: "Could not block this user.")
                }
            }
        }
    }

    private func saveDisplayName() {
        let name = displayNameDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard name.isEmpty == false else { return }
        isSavingDisplayName = true
        displayNameMessage = nil
        Task {
            let status = await environment.matrix.setOwnDisplayNameStatus(name)
            await MainActor.run {
                isSavingDisplayName = false
                displayNameMessage = status.message(for: .displayName)
            }
        }
    }

    private func refreshCoreSessionIdentity() async {
        let identity = await environment.matrix.coreSessionIdentity()
        await MainActor.run {
            coreSessionIdentity = identity
        }
    }

    private func refreshSessionDevices() async {
        let devices = await environment.crypto.sessionDevices()
        await MainActor.run {
            sessionDevices = devices
            isLoadingSessions = false
        }
    }

    private func confirmSignOut() {
        guard let device = signOutDevice, device.isCurrent == false else {
            signOutDevice = nil
            signOutPassword = ""
            return
        }
        let password = signOutPassword
        signOutDevice = nil
        signOutPassword = ""
        isSigningOut = true
        signOutMessage = nil
        Task {
            let result = await environment.crypto.signOutSession(
                deviceId: device.id,
                password: password
            )
            let devices = await environment.crypto.sessionDevices()
            await MainActor.run {
                sessionDevices = devices
                signOutMessage = result.message
                isSigningOut = false
            }
        }
    }

    private func requestVerification(with device: SharedCoreSessionDevice) {
        guard device.isCurrent == false else { return }
        isRequestingVerification = true
        verificationMessage = nil
        Task {
            let result = await environment.crypto.requestDeviceVerification(deviceId: device.id)
            await MainActor.run {
                verificationMessage = result.message
                isRequestingVerification = false
            }
        }
    }
}

private struct SessionDeviceRow: View {
    let device: SharedCoreSessionDevice
    let isSigningOut: Bool
    let isRequestingVerification: Bool
    let onVerify: () -> Void
    let onSignOut: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            HStack(alignment: .firstTextBaseline) {
                Text(device.isCurrent ? String(localized: "This device") : device.displayName)
                    .font(SynaraTypography.body)
                    .foregroundStyle(SynaraColor.primaryText)
                Spacer()
                if device.isCurrent == false {
                    Menu {
                        Button("Verify", systemImage: "checkmark.shield", action: onVerify)
                            .disabled(isRequestingVerification)
                            .accessibilityIdentifier("VerifySessionButton-\(device.id)")
                        Button("Log Out", systemImage: "rectangle.portrait.and.arrow.right", role: .destructive, action: onSignOut)
                            .disabled(isSigningOut)
                            .accessibilityIdentifier("SignOutSessionButton-\(device.id)")
                    } label: {
                        Image(systemName: "ellipsis.circle")
                            .accessibilityLabel("Actions for \(device.displayName)")
                    }
                    .accessibilityIdentifier("SessionActionsButton-\(device.id)")
                }
            }
            if device.isCurrent == false {
                Text(device.displayName)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .textSelection(.enabled)
            }
            Text(SharedCoreDevicesLive.trustDisplayName(device.trust))
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityIdentifier("SettingsSessionTrust-\(device.id)")
            if device.isCrossSignedByOwner {
                Text("Signed by your identity")
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
            }
            if let firstSeen = SharedCoreDevicesLive.firstSeenDisplay(firstSeenTs: device.firstSeenTs) {
                Text(firstSeen)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
            }
            if let lastActivity = SharedCoreDevicesLive.lastActivityDisplay(lastSeenTs: device.lastSeenTs) {
                Text(lastActivity)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
            }
            if let fingerprint = device.ed25519Fingerprint {
                Text("Identity key \(fingerprint)")
                    .font(SynaraTypography.fineMeta)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .textSelection(.enabled)
            }
            Text(device.id)
                .font(SynaraTypography.fineMeta)
                .foregroundStyle(SynaraColor.secondaryText)
                .textSelection(.enabled)
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(device.isCurrent ? "SettingsCurrentSessionRow" : "SettingsSessionRow-\(device.id)")
    }
}

/// Push rules on the Notifications screen. Toggles and pickers move at once
/// and roll back with an inline message when the server refuses; they used
/// to snap back until the round trip finished, and failures were silent.
@MainActor
final class PushRuleSettingsModel: ObservableObject {
    @Published private(set) var snapshot: SynaraPushRulesSnapshot?
    @Published private(set) var message: String?
    /// Bumps per edit; an older write neither rolls back nor overwrites a
    /// newer edit.
    private var revision = 0

    static let failureMessage = String(localized: "Could not update push rules. Check your connection and try again.")

    func load(using matrix: PushRuleSettingsServicing) async {
        let started = revision
        let loaded = await matrix.pushRulesSnapshot()
        guard started == revision else { return }
        snapshot = loaded
    }

    func setMention(
        _ keyPath: WritableKeyPath<SynaraPushRuleMentions, Bool>,
        ruleID: String,
        enabled: Bool,
        using matrix: PushRuleSettingsServicing
    ) async {
        await apply({ $0.mentions[keyPath: keyPath] = enabled }, using: matrix) {
            await matrix.setPushRuleMention(ruleID: ruleID, enabled: enabled)
        }
    }

    func setDefault(
        _ keyPath: WritableKeyPath<SynaraPushRulesSnapshot, String>,
        encrypted: Bool,
        oneToOne: Bool,
        mode: String,
        using matrix: PushRuleSettingsServicing
    ) async {
        await apply({ $0[keyPath: keyPath] = mode }, using: matrix) {
            await matrix.setPushRuleDefault(encrypted: encrypted, oneToOne: oneToOne, mode: mode)
        }
    }

    @discardableResult
    func addKeyword(_ keyword: String, using matrix: PushRuleSettingsServicing) async -> Bool {
        await apply({ snapshot in
            if snapshot.keywords.contains(keyword) == false {
                snapshot.keywords.append(keyword)
            }
        }, using: matrix) {
            await matrix.addPushKeyword(keyword)
        }
    }

    func removeKeyword(_ keyword: String, using matrix: PushRuleSettingsServicing) async {
        await apply({ $0.keywords.removeAll { $0 == keyword } }, using: matrix) {
            await matrix.removePushKeyword(keyword)
        }
    }

    @discardableResult
    private func apply(
        _ change: (inout SynaraPushRulesSnapshot) -> Void,
        using matrix: PushRuleSettingsServicing,
        write: () async -> Bool
    ) async -> Bool {
        guard let previous = snapshot else { return false }
        var next = previous
        change(&next)
        revision += 1
        let edit = revision
        snapshot = next
        message = nil
        guard await write() else {
            if edit == revision {
                snapshot = previous
            }
            message = Self.failureMessage
            return false
        }
        // The server's rules, unless a newer edit is already showing.
        let refreshed = await matrix.pushRulesSnapshot()
        if edit == revision, let refreshed {
            snapshot = refreshed
        }
        return true
    }
}

private struct NotificationSettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var notificationStatus: NotificationPermissionStatus = .unavailable
    @State private var isRequestingNotifications = false
    @State private var isRegisteringPush = false
    @State private var pushRegistrationStatus = ""
    @State private var showLockScreenMessagePreviews = SynaraSharedConstants.defaultLockScreenMessagePreviews
    @State private var timeSensitiveAgentApprovals = SynaraSharedConstants.defaultTimeSensitiveAgentApprovals
    @State private var approvalNotificationDetails = SynaraSharedConstants.defaultApprovalNotificationDetails
    @State private var approveOnceWithoutUnlock = SynaraSharedConstants.defaultApproveOnceWithoutUnlock
    @StateObject private var pushRuleSettings = PushRuleSettingsModel()
    @State private var keywordDraft = ""
    @State private var notificationDiagnostics: [SynaraNotificationDiagnosticEntry] = []
    @State private var notificationDiagnosticReport = ""

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
                        Text(notificationStatus == .notDetermined ? "Enable Notifications" : "Request Notification Permission")
                    }
                }
                .disabled(isRequestingNotifications)
                .accessibilityIdentifier("NotificationPermissionButton")
            }

            Section {
                SettingsInfoRow(
                    title: String(localized: "Status"),
                    value: pushRegistrationStatus.isEmpty
                        ? environment.push.registrationStateDescription
                        : pushRegistrationStatus
                )
                Button {
                    registerPush()
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
                Toggle("Urgent agent approvals", isOn: $timeSensitiveAgentApprovals)
                    .accessibilityIdentifier("TimeSensitiveAgentApprovalsToggle")
                    .onChange(of: timeSensitiveAgentApprovals) { value in
                        environment.settings.set(
                            value,
                            for: SynaraSharedConstants.timeSensitiveAgentApprovalsKey
                        )
                    }
                Toggle("Show approval details", isOn: $approvalNotificationDetails)
                    .accessibilityIdentifier("ApprovalNotificationDetailsToggle")
                    .onChange(of: approvalNotificationDetails) { value in
                        environment.settings.set(
                            value,
                            for: SynaraSharedConstants.approvalNotificationDetailsKey
                        )
                    }
                Toggle(
                    SynaraApprovalNotificationSettingsCopy.approveOnceWithoutUnlockTitle,
                    isOn: $approveOnceWithoutUnlock
                )
                    .accessibilityIdentifier("ApproveOnceWithoutUnlockToggle")
                    .onChange(of: approveOnceWithoutUnlock) { value in
                        environment.settings.set(
                            value,
                            for: SynaraSharedConstants.approveOnceWithoutUnlockKey
                        )
                        SynaraNotificationActionContract.registerCategories(
                            approveOnceWithoutUnlock: value
                        )
                    }
            } header: {
                Text("Agent Approvals")
            } footer: {
                Text(
                    SynaraApprovalNotificationSettingsCopy.footer(
                        approveOnceWithoutUnlock: approveOnceWithoutUnlock
                    )
                )
                .accessibilityIdentifier("AgentApprovalsFooter")
            }

            Section {
                NavigationLink("Agent Notifications") {
                    AgentNotificationSettingsView()
                }
                .accessibilityIdentifier("AgentNotificationSettingsLink")
            }

            Section {
                Toggle("Show message content in notifications", isOn: $showLockScreenMessagePreviews)
                    .accessibilityIdentifier("LockScreenMessagePreviewsToggle")
                    .onChange(of: showLockScreenMessagePreviews) { value in
                        environment.settings.set(value, for: SynaraSharedConstants.lockScreenMessagePreviewsKey)
                    }
            } header: {
                Text("Privacy")
            } footer: {
                Text("Off by default. Synara resolves content on this device; the push gateway receives only event IDs. Your iOS Show Previews setting still controls Lock Screen visibility.")
                    .accessibilityIdentifier("LockScreenMessagePreviewsHelp")
            }

            Section {
                if notificationDiagnostics.isEmpty {
                    Text("No notification-service activity recorded yet.")
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("NotificationDiagnosticsEmpty")
                } else {
                    ForEach(notificationDiagnostics.suffix(24).reversed()) { entry in
                        LabeledContent(entry.stage) {
                            VStack(alignment: .trailing, spacing: 2) {
                                Text(entry.timestamp.formatted(date: .omitted, time: .standard))
                                if let runID = entry.runID {
                                    Text(String(runID.uuidString.prefix(8)).lowercased())
                                        .font(.caption2.monospaced())
                                }
                            }
                                .foregroundStyle(SynaraColor.secondaryText)
                        }
                        .accessibilityIdentifier("NotificationDiagnostic-\(entry.id.uuidString)")
                    }
                }
                Button("Refresh Diagnostics") {
                    reloadNotificationDiagnostics()
                }
                .accessibilityIdentifier("RefreshNotificationDiagnosticsButton")
                Button("Copy Diagnostic Report") {
                    UIPasteboard.general.string = notificationDiagnosticReport
                }
                .disabled(notificationDiagnosticReport.isEmpty)
                .accessibilityIdentifier("CopyNotificationDiagnosticReportButton")
                ShareLink(item: notificationDiagnosticReport) {
                    Text("Share Diagnostic Report")
                }
                .disabled(notificationDiagnosticReport.isEmpty)
                .accessibilityIdentifier("ShareNotificationDiagnosticReportButton")
                if notificationDiagnostics.isEmpty == false {
                    Button("Clear Diagnostics", role: .destructive) {
                        SynaraNotificationDiagnostics.clear()
                        reloadNotificationDiagnostics()
                    }
                    .accessibilityIdentifier("ClearNotificationDiagnosticsButton")
                }
            } header: {
                Text("Local Delivery Diagnostics")
            } footer: {
                Text("Copy or share a report after reproducing a missing preview. It includes app version, notification settings, push registration and shared-store readiness, up to 256 fixed stage codes and timestamps, and how many previews failed because the server rejected your session. Reports stay on this device until you share them and exclude message content, Matrix IDs, senders, push payloads, tokens, URLs and account secrets.")
            }

            if let pushRules = pushRuleSettings.snapshot {
                if let message = pushRuleSettings.message {
                    Section {
                        Text(message)
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.criticalText)
                            .accessibilityIdentifier("PushRulesErrorText")
                    }
                }
                Section("1-to-1 Chats") {
                    pushModePicker(String(localized: "Unencrypted"), current: pushRules.dm) { mode in
                        await updateDefault(\.dm, encrypted: false, oneToOne: true, mode: mode)
                    }
                    pushModePicker(String(localized: "Encrypted"), current: pushRules.dmEncrypted) { mode in
                        await updateDefault(\.dmEncrypted, encrypted: true, oneToOne: true, mode: mode)
                    }
                }
                Section("Rooms") {
                    pushModePicker(String(localized: "Unencrypted"), current: pushRules.group) { mode in
                        await updateDefault(\.group, encrypted: false, oneToOne: false, mode: mode)
                    }
                    pushModePicker(String(localized: "Encrypted"), current: pushRules.groupEncrypted) { mode in
                        await updateDefault(\.groupEncrypted, encrypted: true, oneToOne: false, mode: mode)
                    }
                }
                Section("Mentions") {
                    mentionToggle(
                        String(localized: "User ID"),
                        enabled: pushRules.mentions.userMention,
                        keyPath: \.userMention,
                        ruleID: "userMention"
                    )
                    mentionToggle(
                        String(localized: "Display name"),
                        enabled: pushRules.mentions.displayName,
                        keyPath: \.displayName,
                        ruleID: "displayName"
                    )
                    mentionToggle(
                        String(localized: "Username"),
                        enabled: pushRules.mentions.userName,
                        keyPath: \.userName,
                        ruleID: "userName"
                    )
                    mentionToggle(
                        String(localized: "@room mention"),
                        enabled: pushRules.mentions.roomMention,
                        keyPath: \.roomMention,
                        ruleID: "roomMention"
                    )
                    mentionToggle(
                        String(localized: "Contains @room"),
                        enabled: pushRules.mentions.atRoom,
                        keyPath: \.atRoom,
                        ruleID: "atRoom"
                    )
                }
                Section("Keywords") {
                    ForEach(pushRules.keywords, id: \.self) { keyword in
                        Button(role: .destructive) {
                            let matrix = environment.matrix
                            Task { await pushRuleSettings.removeKeyword(keyword, using: matrix) }
                        } label: {
                            Text(keyword)
                        }
                        .accessibilityHint("Removes this keyword")
                    }
                    TextField("Keyword", text: $keywordDraft)
                        .accessibilityIdentifier("NotificationKeywordField")
                    Button("Add Keyword") {
                        let keyword = keywordDraft.trimmingCharacters(in: .whitespacesAndNewlines)
                        guard keyword.isEmpty == false else { return }
                        let matrix = environment.matrix
                        Task {
                            // Keep the draft when the server refuses it.
                            if await pushRuleSettings.addKeyword(keyword, using: matrix) {
                                keywordDraft = ""
                            }
                        }
                    }
                    .disabled(keywordDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .accessibilityIdentifier("NotificationAddKeywordButton")
                }
            }
        }
        .navigationTitle("Notifications")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("NotificationSettingsScreen")
        .task {
            showLockScreenMessagePreviews = environment.settings.bool(
                for: SynaraSharedConstants.lockScreenMessagePreviewsKey
            )
            timeSensitiveAgentApprovals = environment.settings.bool(
                for: SynaraSharedConstants.timeSensitiveAgentApprovalsKey
            )
            approvalNotificationDetails = environment.settings.bool(
                for: SynaraSharedConstants.approvalNotificationDetailsKey
            )
            approveOnceWithoutUnlock = environment.settings.bool(
                for: SynaraSharedConstants.approveOnceWithoutUnlockKey
            )
            notificationStatus = await environment.notificationPermission.currentStatus()
            await pushRuleSettings.load(using: environment.matrix)
            reloadNotificationDiagnostics()
        }
    }

    private func reloadNotificationDiagnostics() {
        notificationDiagnostics = SynaraNotificationDiagnostics.entries()
        notificationDiagnosticReport = ""
        Task {
            let settings = await UNUserNotificationCenter.current().notificationSettings()
            let context = SynaraNotificationDiagnosticReport.Context.capture(
                settings: settings,
                pushRegistrationAvailable: environment.push.isRegistrationAvailable,
                pushRegistered: environment.push.isRegistered,
                pushGatewayConfigured: environment.push.pushGatewayURL != nil
            )
            notificationDiagnosticReport = SynaraNotificationDiagnosticReport.text(
                context: context,
                entries: notificationDiagnostics,
                sessionRejectedCount: SynaraNotificationDiagnostics.sessionRejectedCount()
            )
        }
    }

    @ViewBuilder
    private func pushModePicker(
        _ title: String,
        current: String,
        onChange: @escaping (String) async -> Void
    ) -> some View {
        Picker(title, selection: Binding(
            get: { current },
            set: { mode in
                Task { await onChange(mode) }
            }
        )) {
            Text("All").tag("all")
            Text("Mentions").tag("mentions")
            Text("Off").tag("mute")
        }
    }

    private func mentionToggle(
        _ title: String,
        enabled: Bool,
        keyPath: WritableKeyPath<SynaraPushRuleMentions, Bool>,
        ruleID: String
    ) -> some View {
        Toggle(title, isOn: Binding(
            get: { enabled },
            set: { value in
                let matrix = environment.matrix
                Task {
                    await pushRuleSettings.setMention(keyPath, ruleID: ruleID, enabled: value, using: matrix)
                }
            }
        ))
        .accessibilityIdentifier("PushRuleMention-\(ruleID)")
    }

    private func updateDefault(
        _ keyPath: WritableKeyPath<SynaraPushRulesSnapshot, String>,
        encrypted: Bool,
        oneToOne: Bool,
        mode: String
    ) async {
        await pushRuleSettings.setDefault(
            keyPath,
            encrypted: encrypted,
            oneToOne: oneToOne,
            mode: mode,
            using: environment.matrix
        )
    }

    /// APNs and the pusher answer after `beginRegistration` returns, so the
    /// spinner stays until the service's status moves (bounded).
    private func registerPush() {
        guard isRegisteringPush == false else { return }
        isRegisteringPush = true
        let push = environment.push
        let before = push.registrationStateDescription
        push.beginRegistration()
        Task { @MainActor in
            for _ in 0..<20 {
                try? await Task.sleep(nanoseconds: 500_000_000)
                if push.registrationStateDescription != before { break }
            }
            pushRegistrationStatus = push.registrationStateDescription
            isRegisteringPush = false
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

/// Desktop's Settings → General rich-link toggles. Each one decides whether
/// this device contacts a third party, which can then see its IP address.
private struct RichLinkSettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var inlineGifLinks = true
    @State private var gifSearch = true
    @State private var xEmbeds = true
    @State private var didLoad = false
    /// Builds without a Klipy key (local, dev and CI test builds) cannot search.
    private let gifSearchConfigured = KlipyGifProvider.configured() != nil

    var body: some View {
        Form {
            Section {
                Toggle("Play GIF Links Inline", isOn: $inlineGifLinks)
                    .accessibilityIdentifier("RichLinkInlineGifToggle")
                    .onChange(of: inlineGifLinks) { value in
                        guard didLoad else { return }
                        environment.settings.set(value, for: RichLinkSettings.inlineGifLinksKey)
                    }
            } footer: {
                Text("Show Klipy and GIPHY links as GIFs. Your device loads them from the provider, which can see your IP address.")
            }

            Section {
                Toggle("GIF Search", isOn: Binding(
                    get: { gifSearchConfigured && gifSearch },
                    set: { gifSearch = $0 }
                ))
                .disabled(gifSearchConfigured == false)
                .accessibilityIdentifier("RichLinkGifSearchToggle")
                .onChange(of: gifSearch) { value in
                    guard didLoad else { return }
                    environment.settings.set(value, for: RichLinkSettings.gifSearchKey)
                }
            } footer: {
                if gifSearchConfigured {
                    Text("Show GIF search in the composer and for /gif. Your searches go to KLIPY, which can see your IP address. A GIF is sent as its KLIPY link.")
                } else {
                    Text("GIF search is not set up for this build.")
                }
            }

            Section {
                Toggle("Show X Posts", isOn: $xEmbeds)
                    .accessibilityIdentifier("RichLinkXPostsToggle")
                    .onChange(of: xEmbeds) { value in
                        guard didLoad else { return }
                        environment.settings.set(value, for: RichLinkSettings.xEmbedsKey)
                    }
            } footer: {
                Text("Show the post and play its video for x.com links. Your device contacts X, which can see your IP address.")
            }
        }
        .scrollContentBackground(.hidden)
        .background(SynaraChrome.settings)
        .navigationTitle("GIFs & Link Previews")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("RichLinkSettingsScreen")
        .onAppear {
            inlineGifLinks = environment.settings.bool(for: RichLinkSettings.inlineGifLinksKey)
            gifSearch = environment.settings.bool(for: RichLinkSettings.gifSearchKey)
            xEmbeds = environment.settings.bool(for: RichLinkSettings.xEmbedsKey)
            DispatchQueue.main.async {
                didLoad = true
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
    @State private var hour24Clock = false
    @State private var hideActivity = false
    @State private var threadDisplay = ThreadDisplayMode.defaultMode

    var body: some View {
        Form {
            Section {
                SettingsInfoRow(title: String(localized: "Appearance"), value: String(localized: "System"))
                    .accessibilityIdentifier("AppearanceThemeRow")
                themeRampPreview
                HStack(spacing: 0) {
                    ForEach(SynaraThemeRamp.presets, id: \.hex) { preset in
                        let isSelected = SynaraThemeRamp.normalize(baseColor.synaraHexString()) == preset.hex
                        Button {
                            persistBaseColor(preset.hex)
                            baseColor = Color(synaraHex: preset.hex)
                        } label: {
                            Circle()
                                .fill(Color(synaraHex: preset.hex))
                                .frame(width: 22, height: 22)
                                .overlay(
                                    Circle().stroke(
                                        isSelected
                                            ? SynaraColor.primaryText
                                            : SynaraColor.separator.opacity(0.5),
                                        lineWidth: 1.5
                                    )
                                )
                                .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                                .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel(preset.label)
                        .accessibilityAddTraits(isSelected ? .isSelected : [])
                        .accessibilityIdentifier("AppearancePreset-\(preset.key)")
                    }
                }
                .padding(.leading, -11)
                ColorPicker("Custom", selection: $baseColor, supportsOpacity: false)
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
            } header: {
                Text("Theme")
            } footer: {
                Text("Hue tint for chrome. Lightness is mapped to stacked greys (rail / room list / chat); this is not the fill color.")
            }
            Section {
                Toggle("24-Hour Time", isOn: $hour24Clock)
                    .accessibilityIdentifier("AppearanceHour24Toggle")
                    .onChange(of: hour24Clock) { value in
                        environment.settings.set(value, for: SynaraSharedConstants.hour24ClockKey)
                    }
            } header: {
                Text("Date & Time")
            } footer: {
                Text("Matches desktop Settings → General. Room list and timeline clocks follow this toggle.")
            }

            Section {
                Picker("Thread Display", selection: $threadDisplay) {
                    ForEach(ThreadDisplayMode.allCases) { mode in
                        Text(mode.title)
                            .tag(mode)
                            .accessibilityIdentifier("AppearanceThreadDisplay-\(mode.rawValue)")
                    }
                }
                .pickerStyle(.inline)
                .labelsHidden()
                .accessibilityIdentifier("AppearanceThreadDisplayPicker")
                .onChange(of: threadDisplay) { value in
                    environment.settings.setString(value.rawValue, for: ThreadDisplayMode.settingsKey)
                }
            } header: {
                Text("Thread Display")
            } footer: {
                Text(String(localized: "\(threadDisplay.detail) Matches desktop Settings → Appearance → Thread Display."))
            }

            Section {
                Toggle("Hide Typing & Read Receipts", isOn: $hideActivity)
                    .accessibilityIdentifier("AppearanceHideActivityToggle")
                    .onChange(of: hideActivity) { value in
                        environment.settings.set(value, for: SynaraSharedConstants.hideActivityKey)
                    }
            } header: {
                Text("Privacy")
            } footer: {
                Text("When on, this device hides automatic typing status and read receipts. Choosing Mark as Read still sends a private receipt.")
            }

            Section {
                SettingsInfoRow(
                    title: String(localized: "Text Size"),
                    value: String(localized: "Uses iOS Dynamic Type")
                )
                    .accessibilityIdentifier("AppearanceTextSizeRow")
            } header: {
                Text("Text")
            } footer: {
                Text("Adjust text size in iOS Settings. Synara follows your system accessibility preferences.")
            }
        }
        .scrollContentBackground(.hidden)
        .background(SynaraChrome.settings)
        .navigationTitle("Appearance")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("AppearanceSettingsScreen")
        .onAppear {
            let stored = environment.settings.string(for: SynaraThemeRamp.storageKey)
            hasCustomBaseColor = SynaraThemeRamp.normalize(stored) != nil
            baseColor = Color(synaraHex: SynaraThemeRamp.resolve(stored))
            hour24Clock = environment.settings.bool(for: SynaraSharedConstants.hour24ClockKey)
            hideActivity = environment.settings.bool(for: SynaraSharedConstants.hideActivityKey)
            threadDisplay = ThreadDisplayMode.resolve(
                environment.settings.string(for: ThreadDisplayMode.settingsKey)
            )
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
            rampSwatch(title: String(localized: "Rail"), hex: tokens.groupedSurface)
            rampSwatch(title: String(localized: "List"), hex: tokens.secondarySurface)
            rampSwatch(title: String(localized: "Chat"), hex: tokens.surface)
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

@MainActor
final class SessionCryptoStatusObserver: ObservableObject {
    @Published private(set) var status: SessionCryptoStatus = .unknown

    var showsVerifyThisDevice: Bool {
        SecuritySettingsVerificationPolicy.showsVerifyThisDevice(status)
    }

    var enablesVerifyThisDevice: Bool {
        SecuritySettingsVerificationPolicy.enablesVerifyThisDevice(status)
    }

    var verificationAvailabilityMessage: String {
        SecuritySettingsVerificationPolicy.availabilityMessage(status)
    }

    /// Mirror the shared store while the calling view is on screen. Every
    /// screen shares the store's one wakeup subscription and its reads.
    func start(store: SessionCryptoStore) async {
        // Device authority can change without any SAS flow in the inbox.
        // Subscribe before the initial read so an in-flight key query's update
        // is retained while that read is being projected.
        let updates = store.updates()
        status = store.status
        status = await store.refresh()
        for await update in updates {
            guard Task.isCancelled == false else { break }
            status = update
        }
    }

    func refresh(store: SessionCryptoStore) async {
        status = await store.refresh()
    }

    func apply(_ status: SessionCryptoStatus) {
        self.status = status
    }
}

/// The generated recovery key stays on this device's pasteboard only, and
/// only briefly: no Universal Clipboard hand-off, gone after two minutes.
enum RecoveryKeyPasteboard {
    static let lifetime: TimeInterval = 120

    static func copy(_ key: String, to pasteboard: UIPasteboard = .general) {
        pasteboard.setItems(
            [[UTType.plainText.identifier: key]],
            options: [
                .localOnly: true,
                .expirationDate: Date().addingTimeInterval(lifetime),
            ]
        )
    }
}

/// Security → Set Up Recovery, in desktop's order: publish the account's
/// cross-signing identity (asking for the account password when the
/// homeserver wants it), then enrol secret storage and key backup with a
/// recovery passphrase and show the generated recovery key once.
///
/// Core refuses partial or existing prior state, so a repeated tap never
/// enrols twice; an already-enabled account reports `.finished` with no key.
@MainActor
final class RecoverySetupFlow: ObservableObject {
    enum Phase: Equatable {
        case idle
        case working
        case passwordRequired
        case showingKey(String)
        case finished
    }

    @Published private(set) var phase: Phase = .idle
    @Published private(set) var errorMessage: String?
    /// Held only between the passphrase step and the password step.
    private var pendingPassphrase: String?

    var isWorking: Bool { phase == .working }

    func start(passphrase: String, confirmation: String, crypto: CryptoStatusServicing) async {
        guard phase == .idle else { return }
        guard passphrase.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false else {
            errorMessage = String(localized: "Choose a recovery passphrase.")
            return
        }
        guard passphrase == confirmation else {
            errorMessage = String(localized: "Recovery passphrases do not match.")
            return
        }
        errorMessage = nil
        phase = .working
        switch await crypto.startCrossSigningSetup() {
        case .ready:
            await enrol(passphrase: passphrase, crypto: crypto)
        case .passwordRequired, .passwordRejected:
            pendingPassphrase = passphrase
            phase = .passwordRequired
        case .failed(let message):
            fail(message)
        }
    }

    func authenticate(password: String, crypto: CryptoStatusServicing) async {
        guard phase == .passwordRequired, let passphrase = pendingPassphrase else { return }
        guard password.isEmpty == false else {
            errorMessage = String(localized: "Enter your account password.")
            return
        }
        errorMessage = nil
        phase = .working
        switch await crypto.authenticateCrossSigningSetup(password: password) {
        case .ready:
            await enrol(passphrase: passphrase, crypto: crypto)
        case .passwordRequired, .passwordRejected:
            phase = .passwordRequired
            errorMessage = RecoverySetupCopy.passwordRejected
        case .failed(let message):
            fail(message)
        }
    }

    /// Leave the password step; nothing is enrolled.
    func cancel() {
        guard phase == .passwordRequired else { return }
        pendingPassphrase = nil
        errorMessage = nil
        phase = .idle
    }

    /// The user copied or noted the key. It is dropped from memory.
    func acknowledgeKey() {
        guard case .showingKey = phase else { return }
        phase = .finished
    }

    private func enrol(passphrase: String, crypto: CryptoStatusServicing) async {
        pendingPassphrase = nil
        switch await crypto.enableRecovery(passphrase: passphrase) {
        case .enabled(let recoveryKey):
            phase = .showingKey(recoveryKey)
        case .alreadyEnabled:
            phase = .finished
        case .failed(let message):
            fail(message)
        }
    }

    private func fail(_ message: String) {
        pendingPassphrase = nil
        errorMessage = message
        phase = .idle
    }
}

private struct SecuritySettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @StateObject private var sessionCrypto = SessionCryptoStatusObserver()
    @StateObject private var recoverySetup = RecoverySetupFlow()
    @State private var recoveryKey = ""
    @State private var cryptoActionMessage: String?
    @State private var isRunningCryptoAction = false
    @State private var showsIdentityReset = false

    /// Shown while the account has no recovery, and kept while a setup is in
    /// progress or its key is on screen, even after status refreshes.
    private var showsRecoverySetup: Bool {
        sessionCrypto.status.recoverySetupAvailable == true
            || (recoverySetup.phase != .idle && recoverySetup.phase != .finished)
    }

    var body: some View {
        Form {
            Section("Protection") {
                SettingsInfoRow(title: String(localized: "Session Storage"), value: String(localized: "Keychain"))
                    .accessibilityIdentifier("SecuritySessionStorageRow")
                SettingsInfoRow(title: String(localized: "Message Security"), value: "Matrix Rust SDK")
                    .accessibilityIdentifier("SecurityMatrixSDKRow")
                SettingsInfoRow(
                    title: String(localized: "Device Verification"),
                    value: sessionCrypto.status.verification.settingsDisplayName
                )
                .accessibilityIdentifier("SecurityDeviceVerificationRow")
                SettingsInfoRow(
                    title: String(localized: "Cross-Signing"),
                    value: sessionCrypto.status.crossSigning.settingsDisplayName
                )
                .accessibilityIdentifier("SecurityCrossSigningRow")
                SettingsInfoRow(
                    title: String(localized: "Key Recovery"),
                    value: sessionCrypto.status.recovery.settingsDisplayName
                )
                .accessibilityIdentifier("SecurityKeyRecoveryRow")
                SettingsInfoRow(
                    title: String(localized: "Key Backup"),
                    value: sessionCrypto.status.backup.settingsDisplayName
                )
                .accessibilityIdentifier("SecurityKeyBackupRow")
                SettingsInfoRow(
                    title: String(localized: "Decryption Issues"),
                    value: sessionCrypto.status.decryptionIssuesLabel
                )
                .accessibilityIdentifier("SecurityDecryptionIssuesRow")
            }

            if sessionCrypto.showsVerifyThisDevice {
                Section {
                    Button {
                        runCryptoAction {
                            await environment.crypto.requestDeviceVerification()
                        }
                    } label: {
                        cryptoActionLabel(String(localized: "Verify This Device"))
                    }
                    .disabled(isRunningCryptoAction || sessionCrypto.enablesVerifyThisDevice == false)
                    .accessibilityIdentifier("RequestDeviceVerificationButton")

                    if sessionCrypto.status.hasDevicesToVerifyAgainst == nil {
                        Button("Retry Availability Check") {
                            Task {
                                await sessionCrypto.refresh(store: environment.sessionCrypto)
                            }
                        }
                        .disabled(isRunningCryptoAction)
                        .accessibilityIdentifier("RetryVerificationAvailabilityButton")
                    }
                } footer: {
                    Text(sessionCrypto.verificationAvailabilityMessage)
                }
            }

            if showsRecoverySetup {
                RecoverySetupSection(flow: recoverySetup) {
                    sessionCrypto.apply(await environment.sessionCrypto.refresh())
                }
            } else {
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
                        cryptoActionLabel(String(localized: "Recover Keys"))
                    }
                    .disabled(isRunningCryptoAction || recoveryKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .accessibilityIdentifier("RecoverKeysButton")
                } header: {
                    Text("Recovery")
                } footer: {
                    Text("Recovery keys are used only for this request and are not stored by Synara.")
                }
            }

            if let cryptoActionMessage {
                Section {
                    Text(cryptoActionMessage)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("CryptoActionMessage")
                }
            }

            if sessionCrypto.status.crossSigning.offersIdentityReset {
                Section {
                    Button(IdentityResetCopy.title, role: .destructive) {
                        showsIdentityReset = true
                    }
                    .accessibilityIdentifier("ResetIdentityButton")
                } footer: {
                    Text(IdentityResetCopy.settingsDescription)
                }
            }
        }
        .sheet(isPresented: $showsIdentityReset, onDismiss: {
            Task { sessionCrypto.apply(await environment.sessionCrypto.refresh()) }
        }) {
            IdentityResetSheet()
                .environment(\.appEnvironment, environment)
        }
        .navigationTitle("Security")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("SecuritySettingsScreen")
        .task {
            await sessionCrypto.start(store: environment.sessionCrypto)
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
            let status = await environment.sessionCrypto.refresh()
            await MainActor.run {
                sessionCrypto.apply(status)
                cryptoActionMessage = result.message
                isRunningCryptoAction = false
                onComplete()
            }
        }
    }
}

/// Desktop's bootstrap steps and wording (Setup Device Verification, then
/// Set Up Recovery), on the existing Form controls. Security shows it while
/// the account has no recovery; Reset Identity shows it right after a reset.
struct RecoverySetupSection: View {
    @Environment(\.appEnvironment) private var environment
    @ObservedObject var flow: RecoverySetupFlow
    /// Re-read status after anything was set up.
    var onSetUp: () async -> Void = {}
    @State private var recoveryPassphrase = ""
    @State private var recoveryPassphraseConfirmation = ""
    @State private var accountPassword = ""
    @State private var copiedRecoveryKey = false

    var body: some View {
        Section {
            switch flow.phase {
            case .idle, .working, .finished:
                SecureField("Recovery Passphrase", text: $recoveryPassphrase)
                    .textContentType(.newPassword)
                    .accessibilityIdentifier("RecoveryPassphraseField")
                SecureField("Confirm Recovery Passphrase", text: $recoveryPassphraseConfirmation)
                    .textContentType(.newPassword)
                    .accessibilityIdentifier("RecoveryPassphraseConfirmField")
                Button {
                    let passphrase = recoveryPassphrase
                    let confirmation = recoveryPassphraseConfirmation
                    Task {
                        await flow.start(
                            passphrase: passphrase,
                            confirmation: confirmation,
                            crypto: environment.crypto
                        )
                        await finishRecoverySetupStep()
                    }
                } label: {
                    recoverySetupLabel(String(localized: "Set Up Recovery"))
                }
                .disabled(flow.isWorking || recoveryPassphrase.isEmpty)
                .accessibilityIdentifier("SetUpRecoveryButton")
            case .passwordRequired:
                Text("Synara will create and securely store your cross-signing identity on this device. Enter your account password to continue.")
                    .font(SynaraTypography.supporting)
                SecureField("Account Password", text: $accountPassword)
                    .textContentType(.password)
                    .accessibilityIdentifier("RecoveryAccountPasswordField")
                Button {
                    let password = accountPassword
                    Task {
                        await flow.authenticate(password: password, crypto: environment.crypto)
                        accountPassword = ""
                        await finishRecoverySetupStep()
                    }
                } label: {
                    recoverySetupLabel(String(localized: "Authenticate"))
                }
                .disabled(accountPassword.isEmpty)
                .accessibilityIdentifier("RecoveryAuthenticateButton")
                Button("Cancel", role: .cancel) {
                    accountPassword = ""
                    flow.cancel()
                }
                .accessibilityIdentifier("RecoveryCancelButton")
            case .showingKey(let key):
                Text("Recovery is ready. Copy this key now. Synara does not save it.")
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.primaryText)
                    .accessibilityIdentifier("RecoveryKeyReadyText")
                Text(key)
                    .font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
                    .accessibilityIdentifier("GeneratedRecoveryKeyText")
                Button {
                    RecoveryKeyPasteboard.copy(key)
                    copiedRecoveryKey = true
                } label: {
                    Label(copiedRecoveryKey ? "Copied" : "Copy Recovery Key", systemImage: "doc.on.doc")
                }
                .accessibilityIdentifier("CopyRecoveryKeyButton")
                Button("Done") {
                    copiedRecoveryKey = false
                    flow.acknowledgeKey()
                }
                .accessibilityIdentifier("RecoveryKeyDoneButton")
            }
            if let message = flow.errorMessage {
                Text(message)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.criticalText)
                    .accessibilityIdentifier("RecoverySetupErrorText")
            }
        } header: {
            Text("Set Up Recovery")
        } footer: {
            Text("Choose a recovery passphrase. Synara sets up device verification for this account, then protects it and your message keys with secret storage and key backup. The recovery key is shown once.")
        }
    }

    @ViewBuilder
    private func recoverySetupLabel(_ title: String) -> some View {
        if flow.isWorking {
            ProgressView()
        } else {
            Text(title)
        }
    }

    /// Clear typed secrets once a step leaves the form, and re-read status
    /// after anything was set up.
    @MainActor
    private func finishRecoverySetupStep() async {
        switch flow.phase {
        case .showingKey, .finished:
            recoveryPassphrase = ""
            recoveryPassphraseConfirmation = ""
            await onSetUp()
        case .idle, .working, .passwordRequired:
            break
        }
    }
}
extension SynaraCryptoCrossSigningStatus {
    /// Reset Identity is offered once the account has an identity.
    var offersIdentityReset: Bool {
        switch self {
        case .ready, .needsVerification, .needsRecovery:
            return true
        case .unknown, .notSetUp:
            return false
        }
    }
}

/// Reset Identity, in desktop's order: the warning, then the password or the
/// browser approval the homeserver asks for, then recovery setup.
///
/// Leaving the password or approval step cancels the reset in Core; the SDK
/// has already deleted key backup and disabled recovery by then, so the
/// cancelled state and terminal failures point back to recovery setup.
@MainActor
final class IdentityResetFlow: ObservableObject {
    enum Phase: Equatable {
        case warning
        case passwordRequired
        case approvalRequired(URL)
        case complete
        /// A pending reset was cancelled after Start.
        case cancelled
    }

    @Published private(set) var phase: Phase = .warning
    @Published private(set) var isWorking = false
    @Published private(set) var errorMessage: String?
    /// A step failed after Start in a way that may have left the account
    /// without key backup and recovery.
    @Published private(set) var recoveryMayBeOff = false

    /// A reset is waiting on the password or the approval.
    var isPending: Bool {
        switch phase {
        case .passwordRequired, .approvalRequired:
            return true
        case .warning, .complete, .cancelled:
            return false
        }
    }

    /// The sheet is done: closing it needs no cancel in Core.
    var isFinished: Bool {
        phase == .complete || phase == .cancelled
    }

    func start(crypto: CryptoStatusServicing) async {
        guard phase == .warning, isWorking == false else { return }
        await run { await crypto.startIdentityReset() }
    }

    func authenticate(password: String, crypto: CryptoStatusServicing) async {
        guard phase == .passwordRequired, isWorking == false else { return }
        guard password.isEmpty == false else {
            errorMessage = String(localized: "Enter your account password.")
            return
        }
        await run { await crypto.authenticateIdentityReset(password: password) }
    }

    func continueAfterApproval(crypto: CryptoStatusServicing) async {
        guard case .approvalRequired = phase, isWorking == false else { return }
        await run { await crypto.continueIdentityReset() }
    }

    /// Cancel a pending reset in Core and say what that left behind. With
    /// nothing pending this only clears the step.
    func cancel(crypto: CryptoStatusServicing) async {
        errorMessage = nil
        guard isPending else {
            if phase != .cancelled { phase = .warning }
            return
        }
        await crypto.cancelIdentityReset()
        phase = .cancelled
    }

    private func run(_ step: () async -> IdentityResetResult) async {
        isWorking = true
        errorMessage = nil
        let result = await step()
        isWorking = false
        switch result {
        case .complete:
            phase = .complete
        case .passwordRequired:
            phase = .passwordRequired
        case .approvalRequired(let url):
            phase = .approvalRequired(url)
        case .passwordRejected:
            phase = .passwordRequired
            errorMessage = IdentityResetCopy.passwordRejected
        case .approvalPending:
            errorMessage = IdentityResetCopy.approvalPending
        case .failed(let message):
            errorMessage = message
            recoveryMayBeOff = true
        }
    }
}

/// The Reset Identity steps as Form sections, shared by the Security sheet
/// and the "Verify This Device" prompt.
struct IdentityResetSections: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.openURL) private var openURL
    @ObservedObject var flow: IdentityResetFlow
    @ObservedObject var recoverySetup: RecoverySetupFlow
    let onDone: () -> Void
    @State private var accountPassword = ""
    @State private var openedApprovalURL: URL?

    var body: some View {
        switch flow.phase {
        case .warning:
            Section {
                Text(IdentityResetCopy.warningIntro)
                ForEach(IdentityResetCopy.warnings, id: \.self) { warning in
                    Label(warning, systemImage: "exclamationmark.triangle")
                        .foregroundStyle(SynaraColor.primaryText)
                }
            }
            .accessibilityIdentifier("IdentityResetWarning")
            Section {
                Button(role: .destructive) {
                    Task { await flow.start(crypto: environment.crypto) }
                } label: {
                    workingLabel(IdentityResetCopy.title)
                }
                .disabled(flow.isWorking)
                .accessibilityIdentifier("ConfirmResetIdentityButton")
            }
        case .passwordRequired:
            Section {
                Text(IdentityResetCopy.password)
                    .font(SynaraTypography.supporting)
                SecureField("Account Password", text: $accountPassword)
                    .textContentType(.password)
                    .accessibilityIdentifier("IdentityResetPasswordField")
                Button(role: .destructive) {
                    let password = accountPassword
                    Task {
                        await flow.authenticate(password: password, crypto: environment.crypto)
                        accountPassword = ""
                    }
                } label: {
                    workingLabel(String(localized: "Authenticate"))
                }
                .disabled(flow.isWorking || accountPassword.isEmpty)
                .accessibilityIdentifier("IdentityResetAuthenticateButton")
            }
        case .approvalRequired(let url):
            Section {
                Text(IdentityResetCopy.approval)
                    .font(SynaraTypography.supporting)
                Button(IdentityResetCopy.openAccountPage) {
                    openURL(url)
                }
                .disabled(flow.isWorking)
                .accessibilityIdentifier("IdentityResetOpenAccountPageButton")
                Button {
                    Task { await flow.continueAfterApproval(crypto: environment.crypto) }
                } label: {
                    workingLabel(String(localized: "Continue"))
                }
                .disabled(flow.isWorking)
                .accessibilityIdentifier("IdentityResetContinueButton")
            }
            .onAppear {
                // Open the account page once; the button reopens it.
                guard openedApprovalURL != url else { return }
                openedApprovalURL = url
                openURL(url)
            }
        case .cancelled:
            Section {
                Text(IdentityResetCopy.cancelled)
                Button("Close", action: onDone)
                    .accessibilityIdentifier("IdentityResetCloseButton")
            }
        case .complete:
            if recoverySetup.phase == .finished {
                Section {
                    Text(IdentityResetCopy.configured)
                    Button("Done", action: onDone)
                        .accessibilityIdentifier("IdentityResetDoneButton")
                }
            } else {
                Section {
                    Text(IdentityResetCopy.recovery)
                        .font(SynaraTypography.supporting)
                }
                RecoverySetupSection(flow: recoverySetup) {
                    _ = await environment.sessionCrypto.refresh()
                }
            }
        }
        if let message = flow.errorMessage {
            Section {
                Text(message)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.criticalText)
                    .accessibilityIdentifier("IdentityResetErrorText")
                if flow.recoveryMayBeOff {
                    Text(IdentityResetCopy.recoveryMayBeOff)
                        .font(SynaraTypography.supporting)
                        .accessibilityIdentifier("IdentityResetRecoveryOffText")
                }
            }
        }
    }

    @ViewBuilder
    private func workingLabel(_ title: String) -> some View {
        if flow.isWorking {
            ProgressView()
        } else {
            Text(title)
        }
    }
}

/// Security → Reset Identity.
struct IdentityResetSheet: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @StateObject private var flow = IdentityResetFlow()
    @StateObject private var recoverySetup = RecoverySetupFlow()

    var body: some View {
        NavigationStack {
            Form {
                IdentityResetSections(flow: flow, recoverySetup: recoverySetup) {
                    dismiss()
                }
            }
            .navigationTitle(IdentityResetCopy.title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(flow.isFinished ? "Close" : "Cancel") {
                        Task {
                            // A pending reset stays open on its cancelled state.
                            let wasPending = flow.isPending
                            await flow.cancel(crypto: environment.crypto)
                            if wasPending == false { dismiss() }
                        }
                    }
                    .disabled(flow.isWorking)
                    .accessibilityIdentifier("IdentityResetCancelButton")
                }
            }
        }
        .interactiveDismissDisabled(flow.isPending || flow.isWorking)
        .accessibilityIdentifier("IdentityResetSheet")
    }
}

/// When the "Verify This Device" prompt shows, and what follows it. "Verify
/// from Another Device" only records the choice and closes the sheet; the
/// request starts from the sheet's `onDismiss`, once it is gone, so the
/// verification sheet can present.
@MainActor
final class VerifyDevicePromptPresentation: ObservableObject {
    @Published var isPresented = false
    private var verifyAfterDismiss = false

    func showIfNeeded(crypto: CryptoStatusServicing) async {
        if await crypto.deviceVerificationPromptVisible() {
            isPresented = true
        }
    }

    func verifyWithAnotherDevice() {
        verifyAfterDismiss = true
        isPresented = false
    }

    /// The sheet is gone: hide it in Core for the session, then start the
    /// recorded verification request. Nil when none was asked for.
    func didDismiss(crypto: CryptoStatusServicing) async -> CryptoActionResult? {
        await crypto.dismissDeviceVerificationPrompt()
        guard verifyAfterDismiss else { return nil }
        verifyAfterDismiss = false
        return await crypto.requestDeviceVerification()
    }
}

/// "Verify This Device", once after signing in on a new device of an account
/// that already has an identity. Core decides when; closing it in any way
/// dismisses it for the session, and Security keeps every entry.
struct VerifyThisDevicePromptSheet: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    /// Record the choice and close; verification starts once the sheet is gone.
    let onVerifyWithAnotherDevice: () -> Void
    @StateObject private var reset = IdentityResetFlow()
    @StateObject private var recoverySetup = RecoverySetupFlow()
    @State private var showsReset = false
    @State private var showsRecoveryKey = false
    @State private var recoveryKey = ""
    @State private var recoveryMessage: String?
    @State private var isRecovering = false

    var body: some View {
        NavigationStack {
            Form {
                if showsReset {
                    IdentityResetSections(flow: reset, recoverySetup: recoverySetup) {
                        dismiss()
                    }
                } else {
                    choices
                }
            }
            .navigationTitle(showsReset ? IdentityResetCopy.title : VerifyDevicePromptCopy.title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(closeTitle) {
                        Task {
                            let wasPending = reset.isPending
                            await reset.cancel(crypto: environment.crypto)
                            if wasPending == false { dismiss() }
                        }
                    }
                    .disabled(reset.isWorking || isRecovering)
                    .accessibilityIdentifier("VerifyDevicePromptLaterButton")
                }
            }
        }
        .interactiveDismissDisabled(reset.isPending || reset.isWorking)
        .accessibilityIdentifier("VerifyDevicePromptSheet")
    }

    private var closeTitle: String {
        if showsReset {
            return reset.isFinished ? String(localized: "Close") : String(localized: "Cancel")
        }
        return VerifyDevicePromptCopy.later
    }

    @ViewBuilder
    private var choices: some View {
        Section {
            Text(VerifyDevicePromptCopy.description)
        }
        Section {
            Button(VerifyDevicePromptCopy.verifyWithDevice) {
                onVerifyWithAnotherDevice()
            }
            .accessibilityIdentifier("VerifyDevicePromptVerifyButton")
            Button(VerifyDevicePromptCopy.recoveryKey) {
                showsRecoveryKey = true
            }
            .disabled(showsRecoveryKey)
            .accessibilityIdentifier("VerifyDevicePromptRecoveryKeyButton")
            if showsRecoveryKey {
                SecureField("Recovery key", text: $recoveryKey)
                    .textContentType(.oneTimeCode)
                    .accessibilityIdentifier("VerifyDevicePromptRecoveryKeyField")
                Button {
                    recover()
                } label: {
                    if isRecovering {
                        ProgressView()
                    } else {
                        Text("Recover Keys")
                    }
                }
                .disabled(isRecovering || recoveryKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .accessibilityIdentifier("VerifyDevicePromptRecoverButton")
            }
        } footer: {
            if let recoveryMessage {
                Text(recoveryMessage)
            } else if showsRecoveryKey {
                Text("Recovery keys are used only for this request and are not stored by Synara.")
            }
        }
        Section {
            Button(IdentityResetCopy.title, role: .destructive) {
                showsReset = true
            }
            .accessibilityIdentifier("VerifyDevicePromptResetButton")
        } header: {
            Text(VerifyDevicePromptCopy.cantVerify)
                .textCase(nil)
        }
    }

    private func recover() {
        let key = recoveryKey
        isRecovering = true
        recoveryMessage = nil
        Task {
            let result = await environment.crypto.recover(recoveryKey: key)
            _ = await environment.sessionCrypto.refresh()
            isRecovering = false
            recoveryKey = ""
            recoveryMessage = result.message
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
        // Keep the human title and machine-readable state separate. Besides
        // producing a clearer VoiceOver announcement, exact value predicates
        // cannot confuse states such as "Unverified" with "Verified".
        .accessibilityLabel(title)
        .accessibilityValue(value)
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

extension SynaraCryptoVerificationStatus {
    var settingsDisplayName: String {
        switch self {
        case .verified:
            return String(localized: "Verified")
        case .unverified:
            return String(localized: "Unverified")
        case .unknown:
            return String(localized: "Unknown")
        }
    }
}

private extension SynaraCryptoRecoveryStatus {
    var settingsDisplayName: String {
        switch self {
        case .enabled:
            return String(localized: "Enabled")
        case .disabled:
            return String(localized: "Disabled")
        case .incomplete:
            return String(localized: "Needs Recovery")
        case .unknown:
            return String(localized: "Unknown")
        }
    }
}

extension SynaraCryptoCrossSigningStatus {
    var settingsDisplayName: String {
        switch self {
        case .ready:
            return String(localized: "Ready")
        case .notSetUp:
            return String(localized: "Not Set Up")
        case .needsVerification:
            return String(localized: "Needs Verification")
        case .needsRecovery:
            return String(localized: "Needs Recovery")
        case .unknown:
            return String(localized: "Unknown")
        }
    }
}

private extension SynaraCryptoBackupStatus {
    var settingsDisplayName: String {
        switch self {
        case .enabled:
            return String(localized: "Enabled")
        case .unavailable:
            return String(localized: "Unavailable")
        case .syncing:
            return String(localized: "Syncing")
        case .unknown:
            return String(localized: "Unknown")
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
                SettingsInfoRow(title: String(localized: "Version"), value: AppBuildInfo.version)
                    .accessibilityIdentifier("AboutVersionRow")
                SettingsInfoRow(title: String(localized: "Build"), value: AppBuildInfo.build)
                    .accessibilityIdentifier("AboutBuildRow")
                SettingsInfoRow(title: String(localized: "Bundle"), value: AppBuildInfo.bundleIdentifier)
                    .accessibilityIdentifier("AboutBundleRow")
            }

            Section("Links") {
                Link(destination: SettingsLink.privacy.url) {
                    SettingsNavigationRow(title: String(localized: "Privacy Policy"), systemImage: "hand.raised")
                }
                .accessibilityIdentifier("AboutPrivacyLink")

                Link(destination: SettingsLink.support.url) {
                    SettingsNavigationRow(title: String(localized: "Support"), systemImage: "questionmark.circle")
                }
                .accessibilityIdentifier("AboutSupportLink")
            }
        }
        .navigationTitle("About")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("AboutSettingsScreen")
    }
}

private struct LicensesSettingsView: View {
    var body: some View {
        List {
            Section("App") {
                LicenseRow(
                    name: "Synara",
                    license: "AGPL-3.0-only",
                    note: String(localized: "Final App Store distribution requires legal review of repository licensing.")
                )
            }

            Section("Dependencies") {
                LicenseRow(
                    name: "Matrix Rust SDK Swift",
                    license: "Apache-2.0",
                    note: String(localized: "Pinned through Swift Package Manager.")
                )
                LicenseRow(
                    name: "Apple SwiftUI and iOS SDK",
                    license: String(localized: "Apple platform SDK"),
                    note: String(localized: "Provided by Xcode and iOS.")
                )
            }
        }
        .navigationTitle("Licenses")
        .navigationBarTitleDisplayMode(.inline)
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
                DataInventoryRow(
                    title: String(localized: "Account"),
                    detail: String(localized: "Matrix user ID, device ID, homeserver, and access tokens are stored locally for session restore.")
                )
                DataInventoryRow(
                    title: String(localized: "Messages and Rooms"),
                    detail: String(localized: "Room metadata, timeline content, media, and account data come from the selected Matrix homeserver.")
                )
                DataInventoryRow(
                    title: String(localized: "Notifications"),
                    detail: String(localized: "APNs device tokens and Matrix pusher registration are used only when notifications are enabled.")
                )
                DataInventoryRow(
                    title: String(localized: "Diagnostics"),
                    detail: String(localized: "No analytics or crash SDK is enabled. Logs are local and redacted.")
                )
            }
        }
        .navigationTitle("Privacy")
        .navigationBarTitleDisplayMode(.inline)
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
                SettingsInfoRow(title: String(localized: "Version"), value: AppBuildInfo.version)
                SettingsInfoRow(title: String(localized: "Build"), value: AppBuildInfo.build)
                SettingsInfoRow(title: String(localized: "Bundle"), value: AppBuildInfo.bundleIdentifier)
            }
        }
        .navigationTitle("Support")
        .navigationBarTitleDisplayMode(.inline)
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

private enum SettingsLink {
    case privacy
    case support

    var displayTitle: String {
        switch self {
        case .privacy:
            return "https://synara.app/privacy"
        case .support:
            return "synara-support@whyland.com"
        }
    }

    var url: URL {
        switch self {
        case .privacy:
            return URL(string: "https://synara.app/privacy")!
        case .support:
            return URL(string: "mailto:synara-support@whyland.com")!
        }
    }
}

private enum AppBuildInfo {
    static var version: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? String(localized: "Unknown")
    }

    static var build: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? String(localized: "Unknown")
    }

    static var bundleIdentifier: String {
        Bundle.main.bundleIdentifier ?? String(localized: "Unknown")
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
