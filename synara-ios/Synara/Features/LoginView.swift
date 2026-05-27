import SwiftUI

struct LoginView: View {
    let homeserverURLString: String
    @Environment(\.appEnvironment) private var environment
    @State private var username: String = ""
    @State private var password: String = ""
    @State private var state: LoginViewState = .idle

    var body: some View {
        Form {
            Section {
                Text(homeserverURLString)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .accessibilityIdentifier("LoginHomeserverText")

                TextField("Username", text: $username)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .textContentType(.username)
                    .accessibilityIdentifier("LoginUsernameField")

                SecureField("Password", text: $password)
                    .textContentType(.password)
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
                .accessibilityIdentifier("LoginSubmitButton")
            } header: {
                Text("Matrix Account")
            }

            if case .failed(let message) = state {
                Section {
                    Text(message)
                        .foregroundStyle(.red)
                        .accessibilityIdentifier("LoginErrorText")
                }
            }
        }
        .navigationTitle("Login")
        .accessibilityIdentifier("LoginScreen")
    }

    private func login() {
        state = .loading

        guard let homeserverURL = URL(string: homeserverURLString) else {
            state = .failed(HomeserverDiscoveryError.invalidURL.localizedDescription)
            return
        }

        let request = LoginRequest(
            homeserverURL: homeserverURL,
            username: username,
            password: password
        )

        Task {
            do {
                let session = try await environment.auth.login(request)
                await MainActor.run {
                    do {
                        try environment.session.completeLogin(session)
                        state = .authenticated
                        environment.router.resetForAccountChange()
                        environment.logger.info("Password login succeeded", category: .auth)
                        Task {
                            await environment.matrix.start(session: session)
                        }
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
