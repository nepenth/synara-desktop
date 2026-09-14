import SwiftUI

struct ApprovalsView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var filter: ApprovalsFilter = .pending
    @State private var inboxItems: [AgentApprovalInboxRecord] = []
    @State private var historyItems: [AgentApprovalHistoryRecord] = []
    @State private var roomNames: [String: String] = [:]
    @State private var isLoading = true
    @State private var errorMessage: String?
    @State private var historyUpdatesTask: Task<Void, Never>?

    private var pendingItems: [AgentApprovalInboxRecord] {
        inboxItems
            .filter { $0.status == .pending }
            .sorted { $0.expiresAt < $1.expiresAt }
    }

    private var recentItems: [AgentApprovalInboxRecord] {
        AgentApprovalHistoryProjection.unionRecent(
            inboxItems: inboxItems,
            historyItems: historyItems
        )
    }

    private var visibleItems: [AgentApprovalInboxRecord] {
        filter == .pending ? pendingItems : recentItems
    }

    var body: some View {
        VStack(spacing: 0) {
            Picker("Approval request filter", selection: $filter) {
                Text("Pending · \(pendingItems.count)").tag(ApprovalsFilter.pending)
                Text("Recent · \(recentItems.count)").tag(ApprovalsFilter.recent)
            }
            .pickerStyle(.segmented)
            .padding(.horizontal, SynaraSpacing.large)
            .padding(.vertical, SynaraSpacing.small)
            .accessibilityIdentifier("ApprovalsFilter")

            approvalsContent
        }
        .navigationTitle("Approvals")
        .task {
            load(discoveryActive: true)
            startHistoryUpdates()
        }
        .onDisappear {
            historyUpdatesTask?.cancel()
            historyUpdatesTask = nil
            Task {
                _ = await environment.agentApprovalHistory.loadInbox(discoveryActive: false)
            }
        }
    }

    @ViewBuilder
    private var approvalsContent: some View {
        if isLoading && inboxItems.isEmpty && historyItems.isEmpty && errorMessage == nil {
                List {
                    Section {
                        SynaraSkeletonList(rowCount: 6, showsAvatar: false)
                            .listRowSeparator(.hidden)
                            .listRowInsets(
                                EdgeInsets(
                                    top: 3,
                                    leading: SynaraSpacing.large,
                                    bottom: 3,
                                    trailing: SynaraSpacing.large
                                )
                            )
                    }
                }
                .listStyle(.insetGrouped)
                .accessibilityIdentifier("ApprovalsLoading")
            } else if let errorMessage, visibleItems.isEmpty {
                SynaraErrorState(title: "Could Not Load Approvals", message: errorMessage) {
                    load(discoveryActive: true)
                }
            } else if visibleItems.isEmpty {
                SynaraEmptyState(
                    title: filter == .pending ? "No pending approvals" : "No recent requests",
                    systemImage: filter == .pending ? "checkmark.shield" : "clock",
                    message: filter == .pending
                        ? "New Hermes approval requests will appear here as they arrive."
                        : "Decided approvals will appear here after you approve or deny a request."
                )
            } else {
                List {
                    if filter == .recent {
                        Section {
                            Text("Decisions your account made, synced across devices. Inbox-only expired requests stay here until they age out of recent room activity.")
                                .font(SynaraTypography.supporting)
                                .foregroundStyle(SynaraColor.secondaryText)
                                .listRowBackground(Color.clear)
                        }
                    }
                    Section {
                        ForEach(visibleItems, id: \.identity) { item in
                            ApprovalsRow(
                                item: item,
                                roomName: roomDisplayName(for: item.roomId),
                                showsDecision: filter == .recent
                            )
                        }
                    }
                }
                .listStyle(.insetGrouped)
                .refreshable {
                    await loadAsync(discoveryActive: true)
                }
                .accessibilityIdentifier("ApprovalsList")
        }
    }

    private func roomDisplayName(for roomID: String) -> String {
        RoomDisplayNameLookup.resolve(roomID: roomID, names: roomNames)
    }

    private func load(discoveryActive: Bool) {
        isLoading = true
        Task {
            await loadAsync(discoveryActive: discoveryActive)
        }
    }

    private func loadAsync(discoveryActive: Bool) async {
        async let inboxResult = environment.agentApprovalHistory.loadInbox(discoveryActive: discoveryActive)
        async let historyResult = environment.agentApprovalHistory.loadHistory()
        async let roomListState = environment.roomList.loadRooms()
        let inbox = await inboxResult
        let history = await historyResult
        let rooms = await roomListState

        await MainActor.run {
            roomNames = RoomDisplayNameLookup.names(from: rooms)
            var nextError: String?
            switch inbox {
            case .success(let items):
                inboxItems = items
            case .failure(let error):
                inboxItems = []
                nextError = error.errorDescription
            }
            switch history {
            case .success(let items):
                historyItems = items
            case .failure(let error):
                historyItems = []
                if nextError == nil {
                    nextError = error.errorDescription
                }
            }
            errorMessage = visibleItems.isEmpty ? nextError : nil
            isLoading = false
        }
    }

    private func startHistoryUpdates() {
        historyUpdatesTask?.cancel()
        historyUpdatesTask = Task {
            for await _ in environment.agentApprovalHistory.historyUpdates() {
                guard Task.isCancelled == false else {
                    return
                }
                await loadAsync(discoveryActive: true)
            }
        }
    }
}

private enum ApprovalsFilter: Hashable {
    case pending
    case recent
}

private struct ApprovalsRow: View {
    let item: AgentApprovalInboxRecord
    let roomName: String
    let showsDecision: Bool

    var body: some View {
        NavigationLink(
            value: AppRoute.room(id: item.roomId, eventID: item.eventId, title: roomName)
        ) {
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                HStack {
                    Text(roomName)
                        .font(SynaraTypography.body.weight(.semibold))
                        .foregroundStyle(SynaraColor.headingText)
                        .lineLimit(1)
                    Spacer(minLength: SynaraSpacing.small)
                    Text(statusLabel)
                        .font(SynaraTypography.supporting.weight(.semibold))
                        .foregroundStyle(statusTint)
                }
                Text(item.sender)
                    .font(SynaraTypography.messageMeta)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(1)
                if let summary = item.summary, summary.isEmpty == false {
                    Text(summary)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(3)
                } else {
                    Text(item.body)
                        .font(SynaraTypography.roomPreview)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .lineLimit(3)
                }
                if let decidedAt = item.decidedAt {
                    Text("Decided \(formatted(decidedAt))")
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.tertiaryText)
                }
                Text("Open message")
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
            }
            .padding(.vertical, SynaraSpacing.xSmall)
        }
        .accessibilityLabel(accessibilityLabel)
        .accessibilityHint("Opens the original message")
        .accessibilityIdentifier("ApprovalsRow-\(item.roomId)-\(item.eventId)")
    }

    private var statusLabel: String {
        if showsDecision {
            return AgentApprovalHistoryProjection.decisionLabel(
                decision: item.decision,
                status: item.status
            )
        }
        return "Pending"
    }

    private var statusTint: Color {
        item.status == .expired ? SynaraColor.secondaryText : SynaraColor.primaryText
    }

    private var accessibilityLabel: String {
        let summary = item.summary?.isEmpty == false ? item.summary ?? item.body : item.body
        return "\(statusLabel) from \(item.sender) in \(roomName). \(summary)"
    }

    private func formatted(_ milliseconds: Double) -> String {
        let date = Date(timeIntervalSince1970: milliseconds / 1000)
        return date.formatted(date: .abbreviated, time: .shortened)
    }
}
