import SwiftUI

struct PlaceholderScreen: View {
    let title: String
    let systemImage: String
    /// Stable, non-localized prefix for the screen's accessibility identifier.
    let identifier: String
    @Environment(\.appEnvironment) private var environment

    var body: some View {
        HeldConnectionEmptyState(
            title: title,
            systemImage: systemImage,
            store: environment.connectionStatus
        )
        .navigationTitle(title)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: String(localized: "Accounts")) {
                    environment.router.present(.accountSwitcher)
                }
            }
        }
        .accessibilityIdentifier("\(identifier)Screen")
    }
}

struct RoutePlaceholderView: View {
    let route: AppRoute
    @Environment(\.synaraCanvasLayout) private var canvasLayout

    var body: some View {
        switch route {
        case .login(let homeserverURL, let requestedAddress):
            LoginView(homeserverURLString: homeserverURL, requestedAddress: requestedAddress)
        case .room(let id, let eventID, let title):
            RoomTimelineView(
                roomID: id,
                roomTitle: title,
                focusedEventID: eventID
            )
            .id("\(id)-\(eventID ?? "")")
            .synaraInteractiveSwipeBack()
            .environment(\.synaraAllowsConversationSwipeBack, canvasLayout == .stacked)
        case .thread(let roomID, let rootEventID, let roomTitle, let rootTitle, let focusedEventID):
            ThreadTimelineView(
                roomID: roomID,
                rootEventID: rootEventID,
                roomTitle: roomTitle,
                rootTitle: rootTitle,
                focusedEventID: focusedEventID
            )
            .synaraInteractiveSwipeBack()
            .environment(\.synaraAllowsConversationSwipeBack, true)
        case .settings:
            EmptyView()
        case .notifications:
            PlaceholderScreen(title: String(localized: "Notifications"), systemImage: "bell", identifier: "Notifications")
        case .later:
            LaterListView()
        }
    }
}

struct SheetPlaceholderView: View {
    let destination: SheetDestination

    var body: some View {
        switch destination {
        case .accountSwitcher:
            AccountMenuSheet()
        }
    }
}

private struct AccountMenuSheet: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @StateObject private var logoutConfirmation = SignOutConfirmationModel()
    @State private var isLoggingOut = false
    @State private var logoutError: String?

    var body: some View {
        NavigationStack {
            Form {
                Section("Account") {
                    switch environment.session.currentState {
                    case .signedIn(let session):
                        AccountMenuInfoRow(title: String(localized: "User"), value: session.userID)
                            .accessibilityIdentifier("AccountMenuUserRow")
                        AccountMenuInfoRow(
                            title: String(localized: "Homeserver"),
                            value: session.homeserverURL.host ?? session.homeserverURL.absoluteString
                        )
                            .accessibilityIdentifier("AccountMenuHomeserverRow")
                        AccountMenuInfoRow(title: String(localized: "Device"), value: session.deviceID)
                            .accessibilityIdentifier("AccountMenuDeviceRow")
                    case .signedOut:
                        Text("Not signed in")
                            .foregroundStyle(SynaraColor.secondaryText)
                            .accessibilityIdentifier("AccountMenuSignedOutText")
                    }
                }

                Section {
                    Button {
                        dismiss()
                        environment.router.route(to: .settings)
                    } label: {
                        Label("Settings", systemImage: "gearshape")
                    }
                    .accessibilityIdentifier("AccountMenuSettingsButton")

                    Button(role: .destructive) {
                        logoutConfirmation.request(using: environment.sessionCrypto)
                    } label: {
                        if isLoggingOut {
                            ProgressView()
                        } else {
                            Label("Log Out", systemImage: "rectangle.portrait.and.arrow.right")
                        }
                    }
                    .disabled(isLoggingOut)
                    .accessibilityIdentifier("AccountMenuLogoutButton")
                }

                if let logoutError {
                    Section {
                        Text(logoutError)
                            .foregroundStyle(SynaraColor.criticalText)
                            .accessibilityIdentifier("AccountMenuLogoutErrorText")
                    }
                }
            }
            .navigationTitle("Account")
            .accessibilityIdentifier("AccountMenuSheet")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            // Same confirmation as Settings, last-device warning included.
            .signOutConfirmation(logoutConfirmation, confirmIdentifier: "AccountMenuConfirmLogoutButton") {
                logout()
            }
        }
    }

    private func logout() {
        isLoggingOut = true
        logoutError = nil

        Task {
            do {
                try await environment.wipe.logoutAndWipe()
                await MainActor.run {
                    isLoggingOut = false
                    environment.logger.info("Local logout completed from account menu", category: .auth)
                    dismiss()
                }
            } catch {
                await MainActor.run {
                    isLoggingOut = false
                    logoutError = LocalWipeError.displayMessage(for: error)
                    environment.logger.error("Local logout failed from account menu", category: .auth)
                }
            }
        }
    }
}

private struct AccountMenuInfoRow: View {
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

struct PlaceholderScreen_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            PlaceholderScreen(title: "Rooms", systemImage: "bubble.left.and.bubble.right", identifier: "Rooms")
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
