import SwiftUI

struct LaterListView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: LaterInboxState = .idle

    var body: some View {
        Group {
            switch state {
            case .idle, .loading:
                SynaraLoadingState(title: "Loading Later")
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
                                LaterListRow(item: item, onTap: openItem)
                            }
                        }
                    }

                    if completedItems.isEmpty == false {
                        Section("Completed") {
                            ForEach(completedItems) { item in
                                LaterListRow(item: item, onTap: openItem)
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

    private func load() {
        state = .loading

        Task {
            let result = await environment.later.loadItems()

            await MainActor.run {
                guard case let .success((items, error)) = result else {
                    state = .failed(.networkFailure)
                    return
                }

                if items.isEmpty {
                    state = error == nil ? .empty : .failed(error)
                    return
                }

                let active = items.filter { $0.isCompleted == false }
                let completed = items.filter { $0.isCompleted }
                state = .loaded(active: active, completed: completed)
            }
        }
    }

    private func openItem(_ item: SynaraLaterListItem) {
        guard item.canNavigate else {
            return
        }

        environment.router.route(to: .room(id: item.roomID, eventID: item.eventID, title: item.detailTitle))
    }
}

private struct LaterListRow: View {
    let item: SynaraLaterListItem
    let onTap: (SynaraLaterListItem) -> Void

    var body: some View {
        Button {
            onTap(item)
        } label: {
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

                if item.isDueSoon {
                    Text(item.dueLabel)
                        .font(.caption)
                                .padding(.horizontal, SynaraSpacing.xSmall)
                                .padding(.vertical, 2)
                                .background(SynaraColor.secondarySurface)
                                .clipShape(Capsule())
                                .accessibilityLabel("Due soon")
                        }
                    }

                    Text(item.preview)
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
        .disabled(!item.canNavigate)
        .buttonStyle(.plain)
        .accessibilityIdentifier(item.accessibilityRowIdentifier)
    }
}

private enum LaterInboxState: Equatable {
    case idle
    case loading
    case empty
    case failed(LaterInboxError)
    case loaded(active: [SynaraLaterListItem], completed: [SynaraLaterListItem])
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

    var detailTitle: String {
        roomID
    }

    var dueLabel: String {
        guard let dueTs else {
            return "No due date"
        }

        let now = Date()
        let dueDate = Date(timeIntervalSince1970: TimeInterval(dueTs) / 1_000)

        if dueTs <= Int(now.timeIntervalSince1970 * 1000) {
            return "Due now"
        }

        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .short
        return "Due \(formatter.localizedString(for: dueDate, relativeTo: now))"
    }

    var isDueSoon: Bool {
        guard isCompleted == false, let dueTs else {
            return false
        }

        let now = Int(Date().timeIntervalSince1970 * 1_000)
        return dueTs <= now + (24 * 60 * 60 * 1000)
    }

    var icon: String {
        isCompleted ? "checkmark.seal.fill" : kind == .reminder ? "alarm" : "clock"
    }

    var statusTint: Color {
        isCompleted ? .green : isDueSoon ? .orange : SynaraColor.accent
    }

    var preview: String {
        switch kind {
        case .saved:
            return "Saved\nRoom: \(roomID)"
        case .reminder:
            if let dueTs {
                let formatter = RelativeDateTimeFormatter()
                formatter.unitsStyle = .full
                let dueDate = Date(timeIntervalSince1970: TimeInterval(dueTs) / 1_000)
                let relative = formatter.localizedString(for: dueDate, relativeTo: Date())
                return "Reminder: \(relative)\nRoom: \(roomID)"
            }

            return "Reminder\nRoom: \(roomID)"
        }
    }

    var accessibilityRowIdentifier: String {
        let safeEventID = eventID.replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: ":", with: "_")
        return "LaterRow-\(safeEventID)"
    }
}
