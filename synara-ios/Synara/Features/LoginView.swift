import SwiftUI

/// Keyboard flow of the sign-in form: Return moves from the username to the
/// password, and Return in the password signs in.
enum LoginFormField: Hashable {
    case username
    case password

    enum SubmitAction: Equatable {
        case focus(LoginFormField)
        case signIn
    }

    var submitAction: SubmitAction {
        switch self {
        case .username:
            return .focus(.password)
        case .password:
            return .signIn
        }
    }

    var submitLabel: SubmitLabel {
        switch self {
        case .username:
            return .next
        case .password:
            return .go
        }
    }

    /// Row kept above the keyboard while this field is focused: the Log In
    /// button, so it is never hidden under the keyboard.
    static let submitRowID = "LoginSubmitRow"
}

/// Neutral sign-in copy: the same screen serves first-time users.
enum LoginCopy {
    static let title = String(localized: "Sign In")
    static let subtitle = String(localized: "Use your Matrix account.")
}

struct LoginView: View {
    let homeserverURLString: String
    /// What the user typed on the server screen; shown instead of a delegated
    /// client-API URL, and names the server for a bare username.
    var requestedAddress: String? = nil
    @Environment(\.appEnvironment) private var environment
    @State private var username: String = ""
    @State private var password: String = ""
    @State private var state: LoginViewState = .idle
    /// Set when sign-in found this account's local data unreadable; the
    /// failure section then offers a confirmed reset.
    @State private var storeResetRequest: LoginRequest?
    @State private var confirmingStoreReset = false
    @FocusState private var focusedField: LoginFormField?

    var body: some View {
        ScrollViewReader { proxy in
            form
                .onChange(of: focusedField) { field in
                    guard field != nil else { return }
                    // Wait for the keyboard inset, then bring Log In above it.
                    Task { @MainActor in
                        try? await Task.sleep(nanoseconds: 350_000_000)
                        withAnimation {
                            proxy.scrollTo(LoginFormField.submitRowID, anchor: .bottom)
                        }
                    }
                }
        }
    }

    private var form: some View {
        Form {
            Section {
                SynaraProductHeader(
                    title: LoginCopy.title,
                    subtitle: LoginCopy.subtitle
                )
                .accessibilityIdentifier("LoginProductHeader")
            }
            .listRowBackground(Color.clear)

            Section {
                Text(requestedAddress ?? homeserverURLString)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .accessibilityIdentifier("LoginHomeserverText")

                TextField("Username", text: $username)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .textContentType(.username)
                    .focused($focusedField, equals: .username)
                    .submitLabel(LoginFormField.username.submitLabel)
                    .onSubmit { submit(from: .username) }
                    .accessibilityIdentifier("LoginUsernameField")

                SecureField("Password", text: $password)
                    .textContentType(.password)
                    .focused($focusedField, equals: .password)
                    .submitLabel(LoginFormField.password.submitLabel)
                    .onSubmit { submit(from: .password) }
                    .accessibilityIdentifier("LoginPasswordField")

                Button {
                    login()
                } label: {
                    if state.isLoading {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                    } else {
                        Text("Log In")
                            .frame(maxWidth: .infinity)
                    }
                }
                .disabled(state.isLoading)
                .buttonStyle(.borderedProminent)
                .id(LoginFormField.submitRowID)
                .accessibilityIdentifier("LoginSubmitButton")
            } header: {
                Text("Matrix Account")
            }

            if case .failed(let message) = state {
                Section {
                    Text(message)
                        .foregroundStyle(SynaraColor.criticalText)
                        .accessibilityIdentifier("LoginErrorText")
                    if storeResetRequest != nil {
                        Button("Reset Local Data", role: .destructive) {
                            confirmingStoreReset = true
                        }
                        .accessibilityIdentifier("LoginResetLocalDataButton")
                    }
                }
            }
        }
        .scrollDismissesKeyboard(.interactively)
        .navigationTitle("Login")
        .accessibilityIdentifier("LoginScreen")
        .confirmationDialog(
            "Reset local data for this account?",
            isPresented: $confirmingStoreReset,
            titleVisibility: .visible
        ) {
            Button("Reset and Sign In", role: .destructive) {
                resetLocalDataAndLogin()
            }
        } message: {
            Text("Synara deletes this account's saved data on this iPhone, then signs in again. Rooms come back from your homeserver; use your recovery key afterwards to read older encrypted messages.")
        }
    }

    private func resetLocalDataAndLogin() {
        guard let request = storeResetRequest else { return }
        state = .loading
        Task {
            do {
                try await environment.auth.resetLocalData(for: request)
            } catch {
                await MainActor.run {
                    state = .failed(String(localized: "Synara could not reset local data. Try again."))
                    environment.logger.error("Local data reset failed", category: .auth)
                }
                return
            }
            await MainActor.run {
                storeResetRequest = nil
                environment.logger.info("Local data reset before sign-in", category: .auth)
                login()
            }
        }
    }

    private func submit(from field: LoginFormField) {
        switch field.submitAction {
        case .focus(let next):
            focusedField = next
        case .signIn:
            guard state.isLoading == false else { return }
            focusedField = nil
            login()
        }
    }

    private func login() {
        state = .loading
        storeResetRequest = nil

        guard let homeserverURL = URL(string: homeserverURLString) else {
            state = .failed(HomeserverDiscoveryError.invalidURL.localizedDescription)
            return
        }

        let request = LoginRequest(
            homeserverURL: homeserverURL,
            username: username,
            password: password,
            accountServerName: requestedAddress.flatMap { URL(string: $0)?.host }
        )

        Task {
            do {
                let session = try await environment.auth.login(request)
                await MainActor.run {
                    do {
                        try environment.session.completeLogin(session)
                        state = .authenticated
                        environment.router.resetNavigationPathsForAccountChange()
                        environment.router.replayPendingDeepLinkIfNeeded(sessionIsSignedIn: true)
                        environment.logger.info("Password login succeeded", category: .auth)
                    } catch let error as SecureSessionStoreError {
                        state = .failed(LoginError.sessionPersistenceFailed.localizedDescription)
                        environment.logger.error("Password login failed: \(error.logDescription)", category: .auth)
                    } catch {
                        state = .failed(LoginError.sessionPersistenceFailed.localizedDescription)
                        environment.logger.error("Password login failed: session persistence failed", category: .auth)
                    }
                }
            } catch let error as LoginError {
                await MainActor.run {
                    storeResetRequest = error == .localDataUnreadable ? request : nil
                    state = .failed(error.localizedDescription)
                    environment.logger.error("Password login failed: \(error.localizedDescription)", category: .auth)
                }
            } catch {
                await MainActor.run {
                    state = .failed(LoginError.networkFailure.localizedDescription)
                    environment.logger.error("Password login failed", category: .auth)
                }
            }
        }
    }
}

private enum LoginViewState {
    case idle
    case loading
    case authenticated
    case failed(String)

    var isLoading: Bool {
        if case .loading = self {
            return true
        }
        return false
    }
}

struct LoginView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            LoginView(homeserverURLString: "https://matrix.org")
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
