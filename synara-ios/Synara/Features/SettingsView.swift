import SwiftUI

struct SettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: SettingsState = .idle

    var body: some View {
        Form {
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
