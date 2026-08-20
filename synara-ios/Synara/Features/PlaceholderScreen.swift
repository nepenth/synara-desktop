import SwiftUI

struct PlaceholderScreen: View {
    let title: String
    let systemImage: String
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
                SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: "Accounts") {
                    environment.router.present(.accountSwitcher)
                }
            }
        }
        .accessibilityIdentifier("\(title)Screen")
    }
}

struct RoutePlaceholderView: View {
    let route: AppRoute

    var body: some View {
        switch route {
        case .login(let homeserverURL):
            LoginView(homeserverURLString: homeserverURL)
        case .room(let id, let eventID, let title):
            RoomTimelineView(
                roomID: id,
                roomTitle: title,
                focusedEventID: eventID
            )
            .id("\(id)-\(eventID ?? "")")
            .synaraInteractiveSwipeBack()
        case .thread(let roomID, let rootEventID, let roomTitle, let rootTitle):
            ThreadTimelineView(
                roomID: roomID,
                rootEventID: rootEventID,
                roomTitle: roomTitle,
                rootTitle: rootTitle
            )
            .synaraInteractiveSwipeBack()
        case .settings:
            EmptyView()
        case .notifications:
            PlaceholderScreen(title: "Notifications", systemImage: "bell")
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
    @State private var isLogoutConfirmationPresented = false
    @State private var isLoggingOut = false
    @State private var logoutError: String?

    var body: some View {
        NavigationStack {
            Form {
                Section("Account") {
                    switch environment.session.currentState {
                    case .signedIn(let session):
                        AccountMenuInfoRow(title: "User", value: session.userID)
                        AccountMenuInfoRow(title: "Homeserver", value: session.homeserverURL.host ?? session.homeserverURL.absoluteString)
                        AccountMenuInfoRow(title: "Device", value: session.deviceID)
                    case .signedOut:
                        Text("Not signed in")
                            .foregroundStyle(SynaraColor.secondaryText)
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
                        isLogoutConfirmationPresented = true
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
                            .foregroundStyle(.red)
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
            .confirmationDialog(
                "Log out of Synara?",
                isPresented: $isLogoutConfirmationPresented,
                titleVisibility: .visible
            ) {
                Button("Log Out", role: .destructive) {
                    logout()
                }
                .accessibilityIdentifier("AccountMenuConfirmLogoutButton")
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("This clears local session data, sync state, cached rooms, and push registration on this device.")
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
                    logoutError = LocalWipeError.sessionDeleteFailed.localizedDescription
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
            PlaceholderScreen(title: "Rooms", systemImage: "bubble.left.and.bubble.right")
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
