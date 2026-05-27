import SwiftUI

struct HomeserverSelectionView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var address: String = ""
    @State private var state: HomeserverSelectionState = .idle

    var body: some View {
        Form {
            Section {
                SynaraProductHeader(
                    title: "Synara",
                    subtitle: "Native Matrix rooms, agent approvals, and private workflows across your devices."
                )
                .accessibilityIdentifier("HomeserverProductHeader")
            }
            .listRowBackground(Color.clear)

            Section {
                TextField("matrix.org", text: $address)
                    .keyboardType(.URL)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .accessibilityIdentifier("HomeserverAddressField")

                Button {
                    validateAndContinue()
                } label: {
                    if state.isLoading {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                    } else {
                        Text("Continue")
                            .frame(maxWidth: .infinity)
                    }
                }
                .disabled(state.isLoading)
                .buttonStyle(.borderedProminent)
                .accessibilityIdentifier("HomeserverContinueButton")
            } header: {
                Text("Homeserver")
            } footer: {
                Text("Use the server that hosts your Matrix account.")
            }

            if environment.homeserverDiscovery.suggestions.isEmpty == false {
                Section("Suggested") {
                    ForEach(environment.homeserverDiscovery.suggestions) { suggestion in
                        Button {
                            address = suggestion.address
                            validateAndContinue(suggestion.address)
                        } label: {
                            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                                Text(suggestion.name)
                                    .font(SynaraTypography.body)
                                Text(suggestion.address)
                                    .font(SynaraTypography.supporting)
                                    .foregroundStyle(SynaraColor.secondaryText)
                            }
                        }
                        .accessibilityIdentifier("HomeserverSuggestion-\(suggestion.address)")
                    }
                }
            }

            if case .failed(let message) = state {
                Section {
                    Text(message)
                        .foregroundStyle(.red)
                        .accessibilityIdentifier("HomeserverErrorText")
                }
            }
        }
        .navigationTitle("Choose Server")
        .accessibilityIdentifier("HomeserverSelectionScreen")
    }

    private func validateAndContinue(_ submittedAddress: String? = nil) {
        state = .loading
        let submittedAddress = submittedAddress ?? address

        Task {
            do {
                let result = try await environment.homeserverDiscovery.discover(rawAddress: submittedAddress)
                await MainActor.run {
                    state = .ready(result)
                    environment.logger.info("Homeserver discovery succeeded", category: .auth)
                    environment.router.route(to: .login(homeserverURL: result.homeserverBaseURL.absoluteString))
                }
            } catch let error as HomeserverDiscoveryError {
                await MainActor.run {
                    state = .failed(error.localizedDescription)
                    environment.logger.error("Homeserver discovery failed: \(error.localizedDescription)", category: .auth)
                }
            } catch {
                await MainActor.run {
                    state = .failed(HomeserverDiscoveryError.discoveryFailed.localizedDescription)
                    environment.logger.error("Homeserver discovery failed", category: .auth)
                }
            }
        }
    }
}

private enum HomeserverSelectionState {
    case idle
    case loading
    case ready(HomeserverDiscoveryResult)
    case failed(String)

    var isLoading: Bool {
        if case .loading = self {
            return true
        }
        return false
    }
}

struct HomeserverSelectionView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            HomeserverSelectionView()
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
