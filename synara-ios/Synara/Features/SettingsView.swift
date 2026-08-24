import PhotosUI
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
        .settingsTabBarClearance()
        .background {
            // Include the clearance inset in the same opaque content plane and
            // continue it behind the floating navigation and tab-bar layers.
            SynaraChrome.settings
                .ignoresSafeArea()
        }
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
    @State private var presenceDraft = "online"
    @State private var isSettingPresence = false
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
                    SettingsInfoRow(title: "Message", value: status)
                        .accessibilityIdentifier("SettingsPresenceMessage")
                }
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
                Text("Sign out a session to revoke it on the homeserver. This device uses Log Out instead.")
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
                    .disabled(isRequestingVerification)
                    .accessibilityIdentifier("AccountVerifyThisDeviceButton")
                } footer: {
                    Text("Compare emoji or number codes with another signed-in Synara or Element session.")
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
        .settingsTabBarClearance()
        .navigationTitle("Account")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("AccountSettingsScreen")
        .alert(
            "Sign out session?",
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
            Button("Sign Out", role: .destructive) {
                confirmSignOut()
            }
            .disabled(isSigningOut || signOutPassword.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            .accessibilityIdentifier("ConfirmSignOutSessionButton")
            Button("Cancel", role: .cancel) {
                signOutDevice = nil
                signOutPassword = ""
            }
        } message: {
            Text("Enter your account password to revoke \(signOutDevice?.displayName ?? "this session").")
        }
        .task {
            await sessionCrypto.start(crypto: environment.crypto)
        }
        .onChange(of: selectedAvatarPhoto) { item in
            if let item {
                uploadAvatar(item)
            }
        }
        .task {
            await refreshCoreSessionIdentity()
            let presence = await environment.matrix.presence(userID: session.userID)
            let devices = await environment.crypto.sessionDevices()
            let loadedEmails = await environment.matrix.threepidEmails()
            let loadedIgnored = await environment.matrix.ignoredUserIDs()
            let profile = await environment.matrix.ownProfile()
            await MainActor.run {
                ownPresence = presence
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
                let ok = await environment.matrix.uploadOwnAvatar(payload: data, mimeType: "image/jpeg")
                await MainActor.run {
                    selectedAvatarPhoto = nil
                    isUploadingAvatar = false
                    displayNameMessage = ok ? "Avatar updated." : "Could not update avatar."
                }
            } catch {
                await MainActor.run {
                    selectedAvatarPhoto = nil
                    isUploadingAvatar = false
                    displayNameMessage = "Could not update avatar."
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
                await MainActor.run { emailMessage = "Could not send verification email." }
                return
            }
            let status = await environment.matrix.addThreepidEmail()
            await MainActor.run {
                if status == "authenticationRequired" {
                    needsEmailPassword = true
                    emailMessage = "Enter your account password to confirm this email."
                } else if status == "ok" {
                    emailDraft = ""
                    needsEmailPassword = false
                    emailMessage = "Email attached."
                } else {
                    emailMessage = "Could not attach this email."
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
                if status == "ok" {
                    emailDraft = ""
                    needsEmailPassword = false
                    emailMessage = "Email attached."
                } else {
                    emailMessage = "Could not confirm this email."
                }
            }
            emails = await environment.matrix.threepidEmails()
        }
    }

    private func blockUser() {
        let userID = ignoreDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard userID.hasPrefix("@") else {
            ignoreMessage = "Enter a full Matrix user ID."
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
                    ignoreMessage = "Could not block this user."
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
            let ok = await environment.matrix.setOwnDisplayName(name)
            await MainActor.run {
                isSavingDisplayName = false
                displayNameMessage = ok
                    ? "Display name updated."
                    : "Could not update display name."
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
                Text(device.isCurrent ? "This device" : device.displayName)
                    .font(SynaraTypography.body)
                    .foregroundStyle(SynaraColor.primaryText)
                Spacer()
                if device.isCurrent == false {
                    Menu {
                        Button("Verify", systemImage: "checkmark.shield", action: onVerify)
                            .disabled(isRequestingVerification)
                            .accessibilityIdentifier("VerifySessionButton-\(device.id)")
                        Button("Sign Out", systemImage: "rectangle.portrait.and.arrow.right", role: .destructive, action: onSignOut)
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
            if let lastActivity = SharedCoreDevicesLive.lastActivityDisplay(lastSeenTs: device.lastSeenTs) {
                Text(lastActivity)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
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

private struct NotificationSettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var notificationStatus: NotificationPermissionStatus = .unavailable
    @State private var isRequestingNotifications = false
    @State private var isRegisteringPush = false
    @State private var showLockScreenMessagePreviews = SynaraSharedConstants.defaultLockScreenMessagePreviews
    @State private var pushRules: SynaraPushRulesSnapshot?
    @State private var keywordDraft = ""
    @State private var pushRulesMessage: String?

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

            if let pushRules {
                Section("1-to-1 Chats") {
                    pushModePicker("Unencrypted", current: pushRules.dm) { mode in
                        await updateDefault(encrypted: false, oneToOne: true, mode: mode)
                    }
                    pushModePicker("Encrypted", current: pushRules.dmEncrypted) { mode in
                        await updateDefault(encrypted: true, oneToOne: true, mode: mode)
                    }
                }
                Section("Rooms") {
                    pushModePicker("Unencrypted", current: pushRules.group) { mode in
                        await updateDefault(encrypted: false, oneToOne: false, mode: mode)
                    }
                    pushModePicker("Encrypted", current: pushRules.groupEncrypted) { mode in
                        await updateDefault(encrypted: true, oneToOne: false, mode: mode)
                    }
                }
                Section("Mentions") {
                    mentionToggle("User ID", enabled: pushRules.mentions.userMention, ruleID: "userMention")
                    mentionToggle("Display name", enabled: pushRules.mentions.displayName, ruleID: "displayName")
                    mentionToggle("Username", enabled: pushRules.mentions.userName, ruleID: "userName")
                    mentionToggle("@room mention", enabled: pushRules.mentions.roomMention, ruleID: "roomMention")
                    mentionToggle("Contains @room", enabled: pushRules.mentions.atRoom, ruleID: "atRoom")
                }
                Section("Keywords") {
                    ForEach(pushRules.keywords, id: \.self) { keyword in
                        Button(role: .destructive) {
                            Task {
                                _ = await environment.matrix.removePushKeyword(keyword)
                                self.pushRules = await environment.matrix.pushRulesSnapshot()
                            }
                        } label: {
                            Text(keyword)
                        }
                    }
                    TextField("Keyword", text: $keywordDraft)
                        .accessibilityIdentifier("NotificationKeywordField")
                    Button("Add Keyword") {
                        let keyword = keywordDraft.trimmingCharacters(in: .whitespacesAndNewlines)
                        guard keyword.isEmpty == false else { return }
                        Task {
                            _ = await environment.matrix.addPushKeyword(keyword)
                            keywordDraft = ""
                            self.pushRules = await environment.matrix.pushRulesSnapshot()
                        }
                    }
                    .disabled(keywordDraft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .accessibilityIdentifier("NotificationAddKeywordButton")
                }
                if let pushRulesMessage {
                    Section {
                        Text(pushRulesMessage)
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                }
            }
        }
        .settingsTabBarClearance()
        .navigationTitle("Notifications")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("NotificationSettingsScreen")
        .task {
            showLockScreenMessagePreviews = environment.settings.bool(
                for: SynaraSharedConstants.lockScreenMessagePreviewsKey
            )
            notificationStatus = await environment.notificationPermission.currentStatus()
            pushRules = await environment.matrix.pushRulesSnapshot()
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

    private func mentionToggle(_ title: String, enabled: Bool, ruleID: String) -> some View {
        Toggle(title, isOn: Binding(
            get: { enabled },
            set: { value in
                Task {
                    _ = await environment.matrix.setPushRuleMention(ruleID: ruleID, enabled: value)
                    pushRules = await environment.matrix.pushRulesSnapshot()
                }
            }
        ))
    }

    private func updateDefault(encrypted: Bool, oneToOne: Bool, mode: String) async {
        let ok = await environment.matrix.setPushRuleDefault(
            encrypted: encrypted,
            oneToOne: oneToOne,
            mode: mode
        )
        pushRules = await environment.matrix.pushRulesSnapshot()
        if ok == false {
            pushRulesMessage = "Could not update push rules."
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
    @State private var hour24Clock = false
    @State private var hideActivity = false

    var body: some View {
        Form {
            Section {
                SettingsInfoRow(title: "Appearance", value: "System")
                    .accessibilityIdentifier("AppearanceThemeRow")
                themeRampPreview
                HStack(spacing: SynaraSpacing.small) {
                    ForEach(SynaraThemeRamp.presets, id: \.hex) { preset in
                        Button {
                            persistBaseColor(preset.hex)
                            baseColor = Color(synaraHex: preset.hex)
                        } label: {
                            Circle()
                                .fill(Color(synaraHex: preset.hex))
                                .frame(width: 22, height: 22)
                                .overlay(
                                    Circle().stroke(
                                        SynaraThemeRamp.normalize(baseColor.synaraHexString()) == preset.hex
                                            ? SynaraColor.primaryText
                                            : SynaraColor.separator.opacity(0.5),
                                        lineWidth: 1.5
                                    )
                                )
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel(preset.label)
                    }
                }
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
                Toggle("Hide Typing & Read Receipts", isOn: $hideActivity)
                    .accessibilityIdentifier("AppearanceHideActivityToggle")
                    .onChange(of: hideActivity) { value in
                        environment.settings.set(value, for: SynaraSharedConstants.hideActivityKey)
                    }
            } header: {
                Text("Privacy")
            } footer: {
                Text("When on, this device does not send read receipts. Matches desktop Editor → Hide Typing & Read Receipts.")
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
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("AppearanceSettingsScreen")
        .onAppear {
            let stored = environment.settings.string(for: SynaraThemeRamp.storageKey)
            hasCustomBaseColor = SynaraThemeRamp.normalize(stored) != nil
            baseColor = Color(synaraHex: SynaraThemeRamp.resolve(stored))
            hour24Clock = environment.settings.bool(for: SynaraSharedConstants.hour24ClockKey)
            hideActivity = environment.settings.bool(for: SynaraSharedConstants.hideActivityKey)
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

@MainActor
final class SessionCryptoStatusObserver: ObservableObject {
    @Published private(set) var status: SessionCryptoStatus = .unknown

    var showsVerifyThisDevice: Bool {
        SecuritySettingsVerificationPolicy.showsVerifyThisDevice(status)
    }

    func start(crypto: CryptoStatusServicing) async {
        await refresh(crypto: crypto)
        for await _ in crypto.verificationUpdates() {
            await refresh(crypto: crypto)
        }
    }

    func refresh(crypto: CryptoStatusServicing) async {
        status = await crypto.sessionStatus()
    }

    func apply(_ status: SessionCryptoStatus) {
        self.status = status
    }
}

private struct SecuritySettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @StateObject private var sessionCrypto = SessionCryptoStatusObserver()
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
                SettingsInfoRow(title: "Device Verification", value: sessionCrypto.status.verification.settingsDisplayName)
                    .accessibilityIdentifier("SecurityDeviceVerificationRow")
                SettingsInfoRow(title: "Key Recovery", value: sessionCrypto.status.recovery.settingsDisplayName)
                    .accessibilityIdentifier("SecurityKeyRecoveryRow")
                SettingsInfoRow(title: "Key Backup", value: sessionCrypto.status.backup.settingsDisplayName)
                    .accessibilityIdentifier("SecurityKeyBackupRow")
                SettingsInfoRow(title: "Decryption Issues", value: sessionCrypto.status.unableToDecryptCount == 0 ? "None" : "\(sessionCrypto.status.unableToDecryptCount)")
                    .accessibilityIdentifier("SecurityDecryptionIssuesRow")
            }

            if sessionCrypto.showsVerifyThisDevice {
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
                    Text("Compare emoji or number codes with another signed-in Synara or Element session. Synara does not mark this device verified until both sides confirm.")
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
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("SecuritySettingsScreen")
        .task {
            await sessionCrypto.start(crypto: environment.crypto)
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
                sessionCrypto.apply(status)
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
        .navigationBarTitleDisplayMode(.inline)
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
                DataInventoryRow(title: "Account", detail: "Matrix user ID, device ID, homeserver, and access tokens are stored locally for session restore.")
                DataInventoryRow(title: "Messages and Rooms", detail: "Room metadata, timeline content, media, and account data come from the selected Matrix homeserver.")
                DataInventoryRow(title: "Notifications", detail: "APNs device tokens and Matrix pusher registration are used only when notifications are enabled.")
                DataInventoryRow(title: "Diagnostics", detail: "No analytics or crash SDK is enabled. Logs are local and redacted.")
            }
        }
        .settingsTabBarClearance()
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
                SettingsInfoRow(title: "Version", value: AppBuildInfo.version)
                SettingsInfoRow(title: "Build", value: AppBuildInfo.build)
                SettingsInfoRow(title: "Bundle", value: AppBuildInfo.bundleIdentifier)
            }
        }
        .settingsTabBarClearance()
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

private extension View {
    func settingsTabBarClearance() -> some View {
        safeAreaInset(edge: .bottom, spacing: 0) {
            Color.clear
                // The iOS floating tab bar occupies more than its visible
                // capsule: content also needs room for the bar's outer margin
                // and a readable separation above it. Keep the final Form row
                // scrollable completely above that obstruction.
                .frame(height: 104)
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
