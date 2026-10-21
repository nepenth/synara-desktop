import SwiftUI

/// Agent notification preferences, saved to the account as they change, like
/// every other Settings toggle. The control moves at once; a refused save
/// rolls back to the last saved preferences and says so inline.
@MainActor
final class AgentNotificationPreferencesEditor: ObservableObject {
    typealias Preferences = SynaraAgentNotificationPreferences

    @Published private(set) var preferences: Preferences?
    @Published private(set) var isLoading = false
    @Published private(set) var isSaving = false
    @Published private(set) var message: String?

    static let savedMessage = String(localized: "Saved to your account.")
    static let saveFailedMessage = String(localized: "Settings were not saved. Check your connection and agent IDs, then try again.")
    static let loadFailedMessage = String(localized: "Agent notification settings could not be loaded. Check your connection and try again.")

    /// The account's last confirmed preferences.
    private var saved: Preferences?

    var hasUnsavedChanges: Bool {
        preferences != saved
    }

    /// Skipped while a save is pending, so a refresh never undoes an edit.
    func refresh(load: () async throws -> Preferences) async {
        guard isLoading == false, isSaving == false, hasUnsavedChanges == false else { return }
        isLoading = true
        defer { isLoading = false }
        do {
            let loaded = try await load()
            guard isSaving == false, hasUnsavedChanges == false else { return }
            saved = loaded
            preferences = loaded
            message = nil
        } catch {
            message = Self.loadFailedMessage
        }
    }

    /// Applies the edit and saves. Edits made while a save is in flight are
    /// sent after it, newest last.
    func update(
        _ change: (inout Preferences) -> Void,
        save: @escaping (Preferences) async throws -> Preferences
    ) async {
        guard var next = preferences else { return }
        change(&next)
        guard next != preferences else { return }
        preferences = next
        message = nil
        guard isSaving == false else { return }
        isSaving = true
        defer { isSaving = false }
        while let target = preferences, target != saved {
            do {
                let confirmed = try await save(target)
                saved = confirmed
                if preferences == target {
                    preferences = confirmed
                }
                message = Self.savedMessage
            } catch {
                preferences = saved
                message = Self.saveFailedMessage
                return
            }
        }
    }
}

struct AgentNotificationSettingsView: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.scenePhase) private var scenePhase
    @StateObject private var editor = AgentNotificationPreferencesEditor()
    @State private var agentID = ""

    var body: some View {
        Form {
            Section {
                if editor.isLoading && editor.preferences == nil { ProgressView("Loading settings…") }
                if let preferences = editor.preferences {
                    ForEach(preferences.agentUserIDs, id: \.self) { id in
                        HStack {
                            Text(id)
                                .textSelection(.enabled)
                            Spacer()
                            Button("Remove", role: .destructive) {
                                apply { $0.agentUserIDs.removeAll { $0 == id } }
                            }
                            .accessibilityLabel("Remove \(id)")
                        }
                    }
                    if preferences.agentUserIDs.isEmpty {
                        Text("No agents selected. All message categories continue to notify.")
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                    TextField("@hermes:example.org", text: $agentID)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .submitLabel(.done)
                        .onSubmit(addAgent)
                        .accessibilityLabel("Agent Matrix user ID")
                        .accessibilityIdentifier("AgentNotificationUserIDField")
                    Button("Add Agent", action: addAgent)
                        .disabled(agentID.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            } header: {
                Text("Agent Accounts")
            } footer: {
                Text("Add each agent’s full Matrix user ID. These rules apply only to the listed agents and sync with your account across Synara clients. Once you add an agent, only listed agents can raise urgent approval alerts, even in muted rooms; with no agents listed, approval alerts follow each room’s notification settings.")
            }

            if editor.preferences != nil {
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
                    Text("Synara recognizes messages starting with 🛠 Tool activity or 💬 Commentary, including Markdown headings. Other messages from the selected agents use the final response setting. Approval requests from listed agents remain eligible for urgent alerts. Messages remain in the conversation.")
                }
            }

            Section {
                if editor.isSaving {
                    HStack(spacing: SynaraSpacing.small) {
                        ProgressView()
                        Text("Saving…")
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                    .accessibilityElement(children: .combine)
                }
                Button("Refresh Settings") {
                    Task { await refresh() }
                }
                .disabled(editor.isSaving || editor.isLoading)
                .accessibilityIdentifier("RefreshAgentNotificationPreferencesButton")
                if let message = editor.message {
                    Text(message)
                        .foregroundStyle(
                            message == AgentNotificationPreferencesEditor.savedMessage
                                ? SynaraColor.secondaryText
                                : SynaraColor.criticalText
                        )
                        .accessibilityIdentifier("AgentNotificationPreferencesStatus")
                }
            } footer: {
                Text("Changes save to your account as you make them. Encrypted background alerts may still appear as generic activity unless the installed build supports Apple’s Notification Filtering capability. Hiding message previews does not suppress those alerts.")
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
            get: { editor.preferences?[keyPath: keyPath] ?? true },
            set: { value in
                apply { $0[keyPath: keyPath] = value }
            }
        )
    }

    private func addAgent() {
        let id = agentID.trimmingCharacters(in: .whitespacesAndNewlines)
        agentID = ""
        guard id.isEmpty == false else { return }
        apply { preferences in
            if preferences.agentUserIDs.contains(id) == false {
                preferences.agentUserIDs.append(id)
            }
        }
    }

    private func apply(_ change: @escaping (inout SynaraAgentNotificationPreferences) -> Void) {
        let matrix = environment.matrix
        Task {
            await editor.update(change) { try await matrix.setAgentNotificationPreferences($0) }
        }
    }

    private func refresh() async {
        let matrix = environment.matrix
        await editor.refresh { try await matrix.agentNotificationPreferences() }
    }
}
