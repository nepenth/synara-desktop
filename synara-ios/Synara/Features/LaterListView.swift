import SwiftUI

struct LaterListView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: LaterInboxState = .idle
    @State private var roomNames: [String: String] = [:]
    /// Sender and first line of each saved message, keyed by item id.
    @State private var excerpts: [String: LaterMessageExcerpt] = [:]

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
                    title: String(localized: "No Later Items"),
                    systemImage: "bookmark",
                    message: String(localized: "Items saved for later will appear here once synced.")
                )
            case .failed(let error):
                SynaraErrorState(
                    title: String(localized: "Could Not Load Later"),
                    message: error.errorDescription ?? String(localized: "Try again")
                ) {
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
                                    excerpt: excerpts[item.id],
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
                                    excerpt: excerpts[item.id],
                                    onComplete: nil
                                )
                            }
                        }
                    }
                }
                .listStyle(.insetGrouped)
                .refreshable {
                    // Awaited, and the list stays in place; skeletons are for
                    // the first load only.
                    await reload()
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

    /// First load (and retry after a failure): shows the skeletons.
    private func load() {
        state = .loading
        Task {
            await reload()
        }
    }

    /// Fetches items and room names; a loaded list keeps showing until the
    /// new snapshot replaces it.
    private func reload() async {
        async let laterResult = environment.later.loadItems()
        async let roomListState = environment.roomList.loadRooms()
        let result = await laterResult
        let rooms = await roomListState

        let nextState: LaterInboxState
        var items: [SynaraLaterListItem] = []
        if case let .success((loadedItems, error)) = result {
            if loadedItems.isEmpty {
                nextState = error.map(LaterInboxState.failed) ?? .empty
            } else {
                items = loadedItems
                nextState = .loaded(
                    active: loadedItems.filter { $0.isCompleted == false },
                    completed: loadedItems.filter(\.isCompleted)
                )
            }
        } else {
            nextState = .failed(.networkFailure)
        }

        await MainActor.run {
            roomNames = RoomDisplayNameLookup.names(from: rooms)
            state = nextState
        }

        await loadExcerpts(for: items)
    }

    /// Resolves each saved event's sender and first line from the room's
    /// loaded timeline, one lookup per room. Events outside the loaded window
    /// keep the kind label as their excerpt.
    private func loadExcerpts(for items: [SynaraLaterListItem]) async {
        let pending = items.filter { $0.canNavigate && excerpts[$0.id] == nil }
        let roomIDs = Set(pending.map(\.roomID))
        for roomID in roomIDs.sorted() {
            guard Task.isCancelled == false,
                  case .loaded(let timelineItems) = await environment.timeline.loadLatestTimeline(roomID: roomID) else {
                continue
            }
            var resolved: [String: LaterMessageExcerpt] = [:]
            for item in pending where item.roomID == roomID {
                if let match = timelineItems.first(where: { $0.eventID == item.eventID || $0.serverEventID == item.eventID }) {
                    resolved[item.id] = LaterMessageExcerpt(timelineItem: match)
                }
            }
            guard resolved.isEmpty == false else {
                continue
            }
            await MainActor.run {
                excerpts.merge(resolved) { _, new in new }
            }
        }
    }

    private func completeItem(_ item: SynaraLaterListItem) {
        guard item.isCompleted == false else {
            return
        }

        Task {
            let result = await environment.later.completeItem(id: item.id)
            guard case .success(true) = result else {
                return
            }
            await MainActor.run {
                SynaraHaptics.trigger(.success)
            }
            await reload()
        }
    }
}

/// Who wrote the saved message and how it starts, as Slack's Later shows.
struct LaterMessageExcerpt: Equatable {
    let sender: String
    let text: String

    init(sender: String, text: String) {
        self.sender = sender
        self.text = text
    }

    init(timelineItem: TimelineItem) {
        sender = timelineItem.senderProfileDisplayName
            ?? Self.localpart(of: timelineItem.senderID)
        text = Self.firstLine(of: timelineItem.kind)
    }

    static func localpart(of userID: String) -> String {
        let trimmed = userID.hasPrefix("@") ? String(userID.dropFirst()) : userID
        return trimmed.split(separator: ":", maxSplits: 1).first.map(String.init) ?? userID
    }

    static func firstLine(of kind: TimelineItem.Kind) -> String {
        switch kind {
        case .text(let body), .formattedText(let body, _), .systemNotice(let body):
            let line = body.split(whereSeparator: \.isNewline).first.map(String.init) ?? body
            return line.trimmingCharacters(in: .whitespaces)
        case .mediaPlaceholder:
            return String(localized: "Attachment")
        case .redacted:
            return String(localized: "Message removed")
        case .encryptedPlaceholder:
            return String(localized: "Encrypted message")
        case .agentCard(let card):
            return card.title
        case .unknown:
            return String(localized: "Message")
        }
    }
}

private struct LaterListRow: View {
    let item: SynaraLaterListItem
    let roomName: String
    var excerpt: LaterMessageExcerpt? = nil
    let onComplete: ((SynaraLaterListItem) -> Void)?
    @Environment(\.appEnvironment) private var environment
    @Environment(\.synaraCanvasLayout) private var canvasLayout

    var body: some View {
        Group {
            if item.canNavigate {
                if canvasLayout == .stacked {
                    NavigationLink(
                        value: AppRoute.room(id: item.roomID, eventID: item.eventID, title: roomName)
                    ) {
                        rowContent
                    }
                } else {
                    Button {
                        environment.router.route(
                            to: .room(id: item.roomID, eventID: item.eventID, title: roomName)
                        )
                    } label: {
                        rowContent
                    }
                    .buttonStyle(.plain)
                }
            } else {
                rowContent
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier(item.accessibilityRowIdentifier)
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

    /// Sender · excerpt · room, with the due pill on the first line. Without
    /// a resolved excerpt the kind ("Saved message" / "Reminder") leads.
    private var rowContent: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.medium) {
            Image(systemName: item.icon)
                .font(SynaraTypography.body.weight(.semibold))
                .foregroundStyle(item.statusTint)
                .frame(width: SynaraMetrics.roomGlyph, alignment: .center)
                .padding(.top, 2)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                    Text(excerpt?.sender ?? item.stateLabel)
                        .font(SynaraTypography.body.weight(.bold))
                        .foregroundStyle(item.canNavigate ? SynaraColor.headingText : SynaraColor.secondaryText)
                        .lineLimit(1)

                    Spacer(minLength: 0)

                    if item.isCompleted {
                        Text("Completed")
                            .font(SynaraTypography.messageMeta)
                            .foregroundStyle(SynaraColor.secondaryText)
                    } else if item.showsDueBadge {
                        Text(item.dueLabel)
                            .font(SynaraTypography.chipLabel)
                            .padding(.horizontal, SynaraSpacing.small)
                            .padding(.vertical, 2)
                            .foregroundStyle(item.dueBadgeForeground)
                            .background(item.dueBadgeBackground)
                            .clipShape(Capsule())
                            .accessibilityLabel(item.dueAccessibilityLabel)
                    }
                }

                Text(excerpt?.text ?? item.preview)
                    .font(SynaraTypography.roomPreview)
                    .foregroundStyle(item.canNavigate ? SynaraColor.primaryText : SynaraColor.secondaryText)
                    .lineLimit(2)

                HStack(spacing: SynaraSpacing.xSmall) {
                    if item.canNavigate {
                        Label(roomName, systemImage: "number")
                            .labelStyle(.titleAndIcon)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    if !item.canNavigate {
                        Text("Destination unavailable")
                            .foregroundStyle(SynaraColor.criticalText)
                    }
                }
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.tertiaryText)
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
        kind == .saved ? String(localized: "Saved message") : String(localized: "Reminder")
    }

    var dueUrgency: LaterDueUrgency {
        LaterDueUrgency.classify(dueTs: dueTs, isCompleted: isCompleted)
    }

    var dueLabel: String {
        guard let dueTs else {
            return String(localized: "No due date")
        }

        let now = Date()
        let dueDate = Date(timeIntervalSince1970: TimeInterval(dueTs) / 1_000)

        if dueUrgency == .overdue {
            return String(localized: "Overdue")
        }

        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .short
        let relative = formatter.localizedString(for: dueDate, relativeTo: now)
        return String(localized: "Due \(relative)")
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
            return String(localized: "Overdue")
        case .dueSoon:
            return String(localized: "Due soon")
        case .future:
            return String(localized: "Due later")
        case .none:
            return String(localized: "No due date")
        }
    }

    /// Bookmark for a saved message (desktop's Later glyph), alarm for a
    /// reminder, seal once completed.
    var icon: String {
        isCompleted ? "checkmark.seal.fill" : kind == .reminder ? "alarm" : "bookmark.fill"
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

    /// Fallback body while the message excerpt is unknown.
    var preview: String {
        switch kind {
        case .saved:
            return String(localized: "Saved for later")
        case .reminder:
            if let dueTs {
                let formatter = RelativeDateTimeFormatter()
                formatter.unitsStyle = .full
                let dueDate = Date(timeIntervalSince1970: TimeInterval(dueTs) / 1_000)
                let relative = formatter.localizedString(for: dueDate, relativeTo: Date())
                return String(localized: "Reminder \(relative)")
            }

            return String(localized: "Reminder")
        }
    }

    var accessibilityRowIdentifier: String {
        let stableID = eventID.isEmpty ? id : eventID
        let safeEventID = stableID.replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: ":", with: "_")
        return "LaterRow-\(safeEventID)"
    }
}
