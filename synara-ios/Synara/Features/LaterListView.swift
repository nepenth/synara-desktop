import SwiftUI

struct LaterListView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: LaterInboxState = .idle
    @State private var roomNames: [String: String] = [:]

    var body: some View {
        Group {
            switch state {
            case .idle, .loading:
                List {
                    Section {
                        SynaraSkeletonList(rowCount: 6, showsAvatar: false)
                            .listRowSeparator(.hidden)
                            .listRowInsets(EdgeInsets(top: 3, leading: SynaraSpacing.large, bottom: 3, trailing: SynaraSpacing.large))
                    }
                }
                .listStyle(.insetGrouped)
                .accessibilityIdentifier("LaterLoading")
            case .empty:
                SynaraEmptyState(
                    title: "No Later Items",
                    systemImage: "clock",
                    message: "Items saved for later will appear here once synced."
                )
            case .failed(let error):
                SynaraErrorState(title: "Could Not Load Later", message: error.errorDescription ?? "Try again") {
                    load()
                }
            case .loaded(let activeItems, let completedItems):
                List {
                    if activeItems.isEmpty == false {
                        Section("Active") {
                            ForEach(activeItems) { item in
                                LaterListRow(
                                    item: item,
                                    roomName: roomDisplayName(for: item.roomID),
                                    onComplete: completeItem
                                )
                            }
                        }
                    }

                    if completedItems.isEmpty == false {
                        Section("Completed") {
                            ForEach(completedItems) { item in
                                LaterListRow(
                                    item: item,
                                    roomName: roomDisplayName(for: item.roomID),
                                    onComplete: nil
                                )
                            }
                        }
                    }
                }
                .listStyle(.insetGrouped)
                .refreshable {
                    load()
                }
                .accessibilityIdentifier("LaterList")
            }
        }
        .navigationTitle("Later")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: "Accounts") {
                    environment.router.present(.accountSwitcher)
                }
            }
        }
        .task {
            if case .idle = state {
                load()
            }
        }
    }

    private func roomDisplayName(for roomID: String) -> String {
        RoomDisplayNameLookup.resolve(roomID: roomID, names: roomNames)
    }

    private func load() {
        state = .loading

        Task {
            async let laterResult = environment.later.loadItems()
            async let roomListState = environment.roomList.loadRooms()
            let result = await laterResult
            let rooms = await roomListState

            await MainActor.run {
                roomNames = RoomDisplayNameLookup.names(from: rooms)

                guard case let .success((items, error)) = result else {
                    state = .failed(.networkFailure)
                    return
                }

                if items.isEmpty {
                    if let error {
                        state = .failed(error)
                    } else {
                        state = .empty
                    }
                    return
                }

                let active = items.filter { $0.isCompleted == false }
                let completed = items.filter { $0.isCompleted }
                state = .loaded(active: active, completed: completed)
            }
        }
    }

    private func completeItem(_ item: SynaraLaterListItem) {
        guard item.isCompleted == false else {
            return
        }

        Task {
            let result = await environment.later.completeItem(id: item.id)
            await MainActor.run {
                guard case .success(true) = result else {
                    return
                }

                load()
            }
        }
    }
}

private struct LaterListRow: View {
    let item: SynaraLaterListItem
    let roomName: String
    let onComplete: ((SynaraLaterListItem) -> Void)?

    var body: some View {
        Group {
            if item.canNavigate {
                NavigationLink(
                    value: AppRoute.room(
                        id: item.roomID,
                        eventID: item.eventID,
                        title: roomName
                    )
                ) {
                    rowContent
                }
            } else {
                rowContent
            }
        }
        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
            if let onComplete, item.isCompleted == false {
                Button {
                    onComplete(item)
                } label: {
                    Label("Complete", systemImage: "checkmark.circle")
                }
                .tint(SynaraColor.success)
                .accessibilityIdentifier("LaterComplete-\(item.id)")
            }
        }
        .accessibilityIdentifier(item.accessibilityRowIdentifier)
    }

    private var rowContent: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.small) {
            Image(systemName: item.icon)
                .foregroundStyle(item.statusTint)
                .padding(.top, 2)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                HStack {
                    Text(item.stateLabel)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(item.statusTint)

                    if item.isCompleted {
                        Text("Completed")
                            .font(.caption)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }

                    Spacer()

                    if item.showsDueBadge {
                        Text(item.dueLabel)
                            .font(.caption)
                            .padding(.horizontal, SynaraSpacing.xSmall)
                            .padding(.vertical, 2)
                            .foregroundStyle(item.dueBadgeForeground)
                            .background(item.dueBadgeBackground)
                            .clipShape(Capsule())
                            .accessibilityLabel(item.dueAccessibilityLabel)
                    }
                }

                Text(item.preview(roomName: roomName))
                    .font(SynaraTypography.body)
                    .foregroundStyle(item.canNavigate ? SynaraColor.primaryText : SynaraColor.secondaryText)
                    .lineLimit(2)

                if !item.canNavigate {
                    Text("Destination unavailable")
                        .font(.caption)
                        .foregroundStyle(.red)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 2)
    }
}

private enum LaterInboxState: Equatable {
    case idle
    case loading
    case empty
    case failed(LaterInboxError)
    case loaded(active: [SynaraLaterListItem], completed: [SynaraLaterListItem])
}

enum LaterDueUrgency: Equatable {
    case none
    case future
    case dueSoon
    case overdue

    var tint: Color {
        switch self {
        case .none, .future:
            return SynaraColor.secondaryText
        case .dueSoon:
            return SynaraColor.warning
        case .overdue:
            return SynaraColor.critical
        }
    }

    var badgeBackground: Color {
        switch self {
        case .none:
            return SynaraColor.secondarySurface
        case .future:
            return SynaraColor.secondarySurface
        case .dueSoon:
            return SynaraColor.warning.opacity(0.16)
        case .overdue:
            return SynaraColor.critical.opacity(0.16)
        }
    }

    static func classify(dueTs: Int?, isCompleted: Bool, now: Int = Int(Date().timeIntervalSince1970 * 1_000)) -> LaterDueUrgency {
        guard isCompleted == false, let dueTs else {
            return .none
        }

        if dueTs <= now {
            return .overdue
        }

        if dueTs <= now + (24 * 60 * 60 * 1_000) {
            return .dueSoon
        }

        return .future
    }
}

private extension SynaraLaterListItem {
    var canNavigate: Bool {
        roomID.isEmpty == false && eventID.isEmpty == false
    }

    var stateLabel: String {
        guard isCompleted == false else {
            return kind == .saved ? "Saved" : "Reminder"
        }

        return kind == .saved ? "Saved" : "Reminder"
    }

    var dueUrgency: LaterDueUrgency {
        LaterDueUrgency.classify(dueTs: dueTs, isCompleted: isCompleted)
    }

    var dueLabel: String {
        guard let dueTs else {
            return "No due date"
        }

        let now = Date()
        let dueDate = Date(timeIntervalSince1970: TimeInterval(dueTs) / 1_000)

        if dueUrgency == .overdue {
            return "Overdue"
        }

        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .short
        return "Due \(formatter.localizedString(for: dueDate, relativeTo: now))"
    }

    var showsDueBadge: Bool {
        isCompleted == false && dueTs != nil
    }

    var dueBadgeForeground: Color {
        dueUrgency.tint
    }

    var dueBadgeBackground: Color {
        dueUrgency.badgeBackground
    }

    var dueAccessibilityLabel: String {
        switch dueUrgency {
        case .overdue:
            return "Overdue"
        case .dueSoon:
            return "Due soon"
        case .future:
            return "Due later"
        case .none:
            return "No due date"
        }
    }

    var icon: String {
        isCompleted ? "checkmark.seal.fill" : kind == .reminder ? "alarm" : "clock"
    }

    var statusTint: Color {
        if isCompleted {
            return SynaraColor.success
        }

        if kind == .reminder {
            return dueUrgency.tint
        }

        return SynaraColor.accent
    }

    func preview(roomName: String) -> String {
        switch kind {
        case .saved:
            return "Saved\nRoom: \(roomName)"
        case .reminder:
            if let dueTs {
                let formatter = RelativeDateTimeFormatter()
                formatter.unitsStyle = .full
                let dueDate = Date(timeIntervalSince1970: TimeInterval(dueTs) / 1_000)
                let relative = formatter.localizedString(for: dueDate, relativeTo: Date())
                return "Reminder: \(relative)\nRoom: \(roomName)"
            }

            return "Reminder\nRoom: \(roomName)"
        }
    }

    var accessibilityRowIdentifier: String {
        let stableID = eventID.isEmpty ? id : eventID
        let safeEventID = stableID.replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: ":", with: "_")
        return "LaterRow-\(safeEventID)"
    }
}