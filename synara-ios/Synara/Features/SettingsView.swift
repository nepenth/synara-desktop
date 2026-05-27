import SwiftUI

struct SettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: SettingsState = .idle
    @State private var notificationStatus: NotificationPermissionStatus = .unavailable
    @State private var isRequestingNotifications = false
    @State private var isRegisteringPush = false

    var body: some View {
        Form {
            Section("Notifications") {
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

                VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                    Text(environment.push.registrationStateDescription)
                        .font(SynaraTypography.supporting)
                    if let gatewayURL = environment.push.pushGatewayURL {
                        Text("Gateway: \(gatewayURL)")
                            .font(.caption)
                            .foregroundStyle(SynaraColor.secondaryText)
                    } else {
                        Text("Gateway: not configured")
                            .font(.caption)
                            .foregroundStyle(.orange)
                    }
                    if let tokenSnippet = environment.push.tokenSnippet {
                        Text("APNs token: \(tokenSnippet)")
                            .font(.caption)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }

                    Button {
                        registerForPush()
                    } label: {
                        if isRegisteringPush {
                            ProgressView()
                        } else {
                            Text(environment.push.isRegistered ? "Re-register Push" : "Register Push")
                        }
                    }
                    .disabled(isRegisteringPush || environment.push.isRegistrationAvailable == false)
                    .accessibilityIdentifier("PushRegistrationButton")
                }
                .font(SynaraTypography.body)
            }

            Section("Account") {
                Button(role: .destructive) {
                    logout()
                } label: {
                    if state.isLoading {
                        ProgressView()
                    } else {
                        Text("Log Out")
                    }
                }
                .disabled(state.isLoading)
                .accessibilityIdentifier("LogoutButton")
            }

            if case .failed(let message) = state {
                Section {
                    Text(message)
                        .foregroundStyle(.red)
                        .accessibilityIdentifier("LogoutErrorText")
                }
            }
        }
        .navigationTitle("Settings")
        .accessibilityIdentifier("SettingsScreen")
        .task {
            await refreshNotificationStatus()
        }
    }

    private func refreshNotificationStatus() async {
        let status = await environment.notificationPermission.currentStatus()
        await MainActor.run {
            notificationStatus = status
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
                if status == .authorized || status == .provisional || status == .ephemeral {
                    environment.push.beginRegistration()
                }
            }
        }
    }

    private func registerForPush() {
        isRegisteringPush = true
        Task {
            await MainActor.run {
                environment.push.beginRegistration()
                isRegisteringPush = false
            }
        }
    }

    private func logout() {
        state = .loading

        Task {
            do {
                try await environment.wipe.logoutAndWipe()
                await MainActor.run {
                    state = .idle
                    environment.router.resetForAccountChange()
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

struct SettingsView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            SettingsView()
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
