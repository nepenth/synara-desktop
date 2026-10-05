import SwiftUI

struct AgentNotificationSettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.scenePhase) private var scenePhase
    @State private var draft: SynaraAgentNotificationPreferences?
    @State private var loaded: SynaraAgentNotificationPreferences?
    @State private var agentID = ""
    @State private var isLoading = false
    @State private var isSaving = false
    @State private var message: String?

    private var hasChanges: Bool { draft != loaded }

    var body: some View {
        Form {
            Section {
                if isLoading { ProgressView("Loading settings…") }
                if let draft {
                    ForEach(draft.agentUserIDs, id: \.self) { id in
                        HStack {
                            Text(id)
                                .textSelection(.enabled)
                            Spacer()
                            Button("Remove", role: .destructive) {
                                self.draft?.agentUserIDs.removeAll { $0 == id }
                                message = nil
                            }
                            .disabled(isSaving || isLoading)
                            .accessibilityLabel("Remove \(id)")
                        }
                    }
                    if draft.agentUserIDs.isEmpty {
                        Text("No agents selected. All message categories continue to notify.")
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                    TextField("@hermes:example.org", text: $agentID)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .disabled(isSaving || isLoading)
                        .accessibilityLabel("Agent Matrix user ID")
                        .accessibilityIdentifier("AgentNotificationUserIDField")
                    Button("Add Agent") {
                        let id = agentID.trimmingCharacters(in: .whitespacesAndNewlines)
                        if !id.isEmpty && !draft.agentUserIDs.contains(id) {
                            self.draft?.agentUserIDs.append(id)
                            message = nil
                        }
                        agentID = ""
                    }
                    .disabled(isSaving || isLoading || agentID.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            } header: {
                Text("Agent Accounts")
            } footer: {
                Text("Add each agent’s full Matrix user ID. These rules apply only to the listed agents and sync with your account across Synara clients.")
            }

            if draft != nil {
                Section {
                    Toggle("Notify for tool activity", isOn: preference(\.notifyToolActivity))
                        .accessibilityIdentifier("AgentNotifyToolActivityToggle")
                    Toggle("Notify for commentary", isOn: preference(\.notifyCommentary))
                        .accessibilityIdentifier("AgentNotifyCommentaryToggle")
                    Toggle("Notify for final responses", isOn: preference(\.notifyFinalResponses))
                        .accessibilityIdentifier("AgentNotifyFinalResponsesToggle")
                } header: {
                    Text("Message Categories")
                } footer: {
                    Text("Synara recognizes messages starting with 🛠 Tool activity or 💬 Commentary, including Markdown headings. Other messages from the selected agents use the final response setting. Approval requests remain eligible for urgent alerts. Messages remain in the conversation.")
                }
                .disabled(isSaving || isLoading)
            }

            Section {
                Button(isSaving ? "Saving…" : "Save to Account") {
                    Task { await save() }
                }
                .disabled(!hasChanges || isSaving || isLoading)
                .accessibilityIdentifier("SaveAgentNotificationPreferencesButton")
                Button("Refresh Settings") {
                    Task { await refresh() }
                }
                .disabled(hasChanges || isSaving || isLoading)
                .accessibilityIdentifier("RefreshAgentNotificationPreferencesButton")
                if hasChanges {
                    Text("Unsaved changes.")
                        .foregroundStyle(SynaraColor.secondaryText)
                }
                if let message {
                    Text(message)
                        .accessibilityIdentifier("AgentNotificationPreferencesStatus")
                }
            } footer: {
                Text("Encrypted background alerts may still appear as generic activity unless the installed build supports Apple’s Notification Filtering capability. Hiding message previews does not suppress those alerts.")
            }
        }
        .scrollContentBackground(.hidden)
        .background(SynaraChrome.settings.ignoresSafeArea())
        .navigationTitle("Agent Notifications")
        .accessibilityIdentifier("AgentNotificationSettingsScreen")
        .task {
            let updates = environment.matrix.agentNotificationPreferenceUpdates()
            await refresh()
            for await _ in updates {
                guard !Task.isCancelled else { break }
                await refresh()
            }
        }
        .onChange(of: scenePhase) { phase in
            if phase == .active { Task { await refresh() } }
        }
    }

    private func preference(_ keyPath: WritableKeyPath<SynaraAgentNotificationPreferences, Bool>) -> Binding<Bool> {
        Binding(
            get: { draft?[keyPath: keyPath] ?? true },
            set: { value in
                draft?[keyPath: keyPath] = value
                message = nil
            }
        )
    }

    @MainActor
    private func refresh() async {
        guard !hasChanges && !isLoading && !isSaving else { return }
        isLoading = true
        defer { isLoading = false }
        do {
            let preferences = try await environment.matrix.agentNotificationPreferences()
            loaded = preferences
            draft = preferences
            message = nil
        } catch {
            message = "Agent notification settings could not be loaded. Check your connection and try again."
        }
    }

    @MainActor
    private func save() async {
        guard let draft, !isSaving else { return }
        isSaving = true
        defer { isSaving = false }
        do {
            let saved = try await environment.matrix.setAgentNotificationPreferences(draft)
            loaded = saved
            self.draft = saved
            message = "Saved to your account."
        } catch {
            message = "Settings were not saved. Check your connection and agent IDs, then try again."
        }
    }
}
