import Foundation
#if canImport(UIKit)
    import UIKit
#endif

enum TimelineSearchFilter {
    static func searchableText(for item: TimelineItem) -> String {
        switch item.kind {
        case let .text(body):
            return body
        case let .formattedText(body, _):
            return body
        case let .mediaPlaceholder(resource):
            return resource.safeDescription
        case let .agentCard(card):
            return card.title
        case .redacted:
            return "Deleted message"
        case .encryptedPlaceholder:
            return "Encrypted message"
        case let .unknown(type):
            return type
        }
    }

    static func itemMatchesQuery(_ item: TimelineItem, query: String) -> Bool {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return true
        }

        return searchableText(for: item).localizedCaseInsensitiveContains(trimmed)
            || item.senderID.localizedCaseInsensitiveContains(trimmed)
    }

    static func applySearchQuery(_ query: String, to items: [TimelineItem]) -> [TimelineItem] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return items
        }

        return items.filter { itemMatchesQuery($0, query: trimmed) }
    }
}

enum TimelineDeliveryStatus: Equatable {
    case sending
    case queued
    case sent
    case failed
}

struct TimelineItem: Identifiable, Equatable {
    enum Kind: Equatable {
        case text(String)
        case formattedText(body: String, html: String)
        case mediaPlaceholder(MediaResource)
        case redacted
        case encryptedPlaceholder
        case agentCard(SynaraAgentCard)
        case unknown(type: String)
    }

    let id: String
    let eventID: String
    /// The homeserver event identifier that may safely be used for receipts and
    /// read markers. Local echoes and transaction identifiers intentionally leave
    /// this nil while continuing to use `eventID` as their stable presentation ID.
    let serverEventID: String?
    let senderID: String
    let senderAvatarURL: URL?
    let timestamp: Date
    let kind: Kind
    let replyToEventID: String?
    let isEdited: Bool
    let reactions: [String: Int]
    let isEncrypted: Bool
    let deliveryStatus: TimelineDeliveryStatus?
    /// True when matrix-rust-sdk attached the signed-in user's durable read
    /// receipt to this event in the active timeline provider.
    let hasCurrentUserReadReceipt: Bool

    init(
        id: String,
        eventID: String,
        serverEventID: String? = nil,
        senderID: String,
        senderAvatarURL: URL? = nil,
        timestamp: Date,
        kind: Kind,
        replyToEventID: String?,
        isEdited: Bool,
        reactions: [String: Int],
        isEncrypted: Bool = false,
        deliveryStatus: TimelineDeliveryStatus? = nil,
        hasCurrentUserReadReceipt: Bool = false
    ) {
        self.id = id
        self.eventID = eventID
        self.serverEventID = deliveryStatus == nil ? (serverEventID ?? eventID) : serverEventID
        self.senderID = senderID
        self.senderAvatarURL = senderAvatarURL
        self.timestamp = timestamp
        self.kind = kind
        self.replyToEventID = replyToEventID
        self.isEdited = isEdited
        self.reactions = reactions
        self.isEncrypted = isEncrypted
        self.deliveryStatus = deliveryStatus
        self.hasCurrentUserReadReceipt = hasCurrentUserReadReceipt
    }

    var isLocalPending: Bool {
        deliveryStatus != nil
    }

    func withDeliveryStatus(_ deliveryStatus: TimelineDeliveryStatus?) -> TimelineItem {
        TimelineItem(
            id: id,
            eventID: eventID,
            serverEventID: serverEventID,
            senderID: senderID,
            senderAvatarURL: senderAvatarURL,
            timestamp: timestamp,
            kind: kind,
            replyToEventID: replyToEventID,
            isEdited: isEdited,
            reactions: reactions,
            isEncrypted: isEncrypted,
            deliveryStatus: deliveryStatus,
            hasCurrentUserReadReceipt: hasCurrentUserReadReceipt
        )
    }

    func withSenderAvatarURL(_ senderAvatarURL: URL?) -> TimelineItem {
        TimelineItem(
            id: id,
            eventID: eventID,
            serverEventID: serverEventID,
            senderID: senderID,
            senderAvatarURL: senderAvatarURL,
            timestamp: timestamp,
            kind: kind,
            replyToEventID: replyToEventID,
            isEdited: isEdited,
            reactions: reactions,
            isEncrypted: isEncrypted,
            deliveryStatus: deliveryStatus,
            hasCurrentUserReadReceipt: hasCurrentUserReadReceipt
        )
    }

    static func pendingMessage(
        localID: String = "$pending-\(UUID().uuidString)",
        body: String,
        formattedBody: String? = nil,
        senderID: String,
        senderAvatarURL: URL? = nil,
        replyToEventID: String?,
        deliveryStatus: TimelineDeliveryStatus = .sending,
        timestamp: Date = Date()
    ) -> TimelineItem {
        TimelineItem(
            id: localID,
            eventID: localID,
            serverEventID: nil,
            senderID: senderID,
            senderAvatarURL: senderAvatarURL,
            timestamp: timestamp,
            kind: formattedBody.map { .formattedText(body: body, html: $0) } ?? .text(body),
            replyToEventID: replyToEventID,
            isEdited: false,
            reactions: [:],
            deliveryStatus: deliveryStatus
        )
    }
}

enum TimelineMessageCopy {
    static func payload(for item: TimelineItem) -> String? {
        guard let body = TimelinePendingReconciler.messageBody(for: item), body.isEmpty == false else {
            return nil
        }
        return body
    }

    static func copyToPasteboard(_ text: String) {
        #if canImport(UIKit)
            UIPasteboard.general.string = text
        #endif
    }
}

enum TimelinePendingReconciler {
    static func messageBody(for item: TimelineItem) -> String? {
        switch item.kind {
        case let .text(body):
            return body
        case let .formattedText(body, _):
            return body
        default:
            return nil
        }
    }

    static func formattedBody(for item: TimelineItem) -> String? {
        switch item.kind {
        case let .formattedText(_, html):
            return html
        default:
            return nil
        }
    }

    static func pendingItems(from items: [TimelineItem]) -> [TimelineItem] {
        items.filter(\.isLocalPending)
    }

    static func combining(localItems: [TimelineItem], storedPending: [TimelineItem]) -> [TimelineItem] {
        var combined = localItems
        var ids = Set(localItems.map(\.id))
        for pending in storedPending {
            if let index = combined.firstIndex(where: { $0.id == pending.id }) {
                combined[index] = combined[index].withDeliveryStatus(pending.deliveryStatus)
            } else if ids.insert(pending.id).inserted {
                combined.append(pending)
            }
        }
        return combined
    }

    static func matchesPending(_ pending: TimelineItem, serverItem: TimelineItem) -> Bool {
        guard pending.deliveryStatus == .sending || pending.deliveryStatus == .sent else {
            return false
        }
        guard pending.senderID == serverItem.senderID else {
            return false
        }
        guard pending.replyToEventID == serverItem.replyToEventID else {
            return false
        }
        guard let pendingBody = messageBody(for: pending),
              let serverBody = messageBody(for: serverItem),
              pendingBody == serverBody
        else {
            return false
        }
        return abs(serverItem.timestamp.timeIntervalSince(pending.timestamp)) < 5 * 60
    }

    static func merge(
        streamItems: [TimelineItem],
        localItems: [TimelineItem],
        currentUserID: String
    ) -> [TimelineItem] {
        let traceID = PerformanceTrace.begin("ReconcilerMerge")
        defer { PerformanceTrace.end("ReconcilerMerge", id: traceID) }
        let pendingItems = pendingItems(from: localItems)
        guard pendingItems.isEmpty == false else {
            return streamItems
        }

        var unmatchedPending = pendingItems
        for serverItem in streamItems where serverItem.senderID == currentUserID {
            if let index = unmatchedPending.firstIndex(where: { matchesPending($0, serverItem: serverItem) }) {
                unmatchedPending.remove(at: index)
            }
        }

        guard unmatchedPending.isEmpty == false else {
            return streamItems
        }

        // The SDK vector is authoritative. Insert local echoes by timestamp without
        // ever reordering server events relative to one another.
        var merged = streamItems
        for pending in unmatchedPending.sorted(by: { $0.timestamp < $1.timestamp }) {
            let insertionIndex = merged.firstIndex { item in
                item.isLocalPending == false && item.timestamp > pending.timestamp
            } ?? merged.endIndex
            merged.insert(pending, at: insertionIndex)
        }
        return merged
    }
}

enum TimelineWindowPolicy {
    static let stableEventLimit = 300

    static func replacingServerWindow(
        _ items: [TimelineItem],
        limit: Int = stableEventLimit
    ) -> [TimelineItem] {
        guard limit > 0 else {
            return []
        }
        let stableItems = deduplicated(items.filter { $0.isLocalPending == false })
        return Array(stableItems.suffix(limit))
    }

    static func prependingHistory(
        _ olderItems: [TimelineItem],
        to currentItems: [TimelineItem],
        limit: Int = stableEventLimit
    ) -> [TimelineItem] {
        guard limit > 0 else {
            return []
        }
        return Array(deduplicated(olderItems + currentItems).prefix(limit))
    }

    private static func deduplicated(_ items: [TimelineItem]) -> [TimelineItem] {
        var seen = Set<String>()
        return items.filter { item in
            let key = item.eventID.isEmpty ? item.id : item.eventID
            return seen.insert(key).inserted
        }
    }
}

protocol LaterServicing {
    func loadItems() async -> Result<([SynaraLaterListItem], LaterInboxError?), Never>
    func completeItem(id: String) async -> Result<Bool, LaterInboxError>
}

enum LaterInboxError: Error, LocalizedError, Equatable {
    case noSession
    case malformedPayload
    case networkFailure

    var errorDescription: String? {
        switch self {
        case .noSession:
            return "Sign in to load your Later items."
        case .malformedPayload:
            return "Later account data was not readable."
        case .networkFailure:
            return "Could not load Later account data."
        }
    }
}

struct SynaraLaterListItem: Identifiable, Equatable {
    let id: String
    let roomID: String
    let eventID: String
    let kind: SynaraLaterItem.Kind
    let dueTs: Int?
    let completedAt: Int?
    let createdAt: Int
    let isCompleted: Bool

    var label: String {
        switch kind {
        case .saved:
            return "Saved"
        case .reminder:
            return "Reminder"
        }
    }

    var detail: String {
        if completedAt != nil {
            return "Completed"
        }

        if let dueTs {
            if dueTs < Int(Date().timeIntervalSince1970 * 1000) {
                return "Due"
            }

            return "Due soon"
        }

        return "No due date"
    }
}

enum SynaraLaterAccountDataCodec {
    static func decodeEnvelopeData(_ data: Data, jsonDecoder: JSONDecoder) -> SynaraLaterContent? {
        guard let content = extractAccountDataContent(from: data) else {
            return nil
        }

        return decode(content: content, jsonDecoder: jsonDecoder)
    }

    static func decodeContentString(_ content: String, jsonDecoder: JSONDecoder) -> SynaraLaterContent? {
        guard let data = content.data(using: .utf8) else {
            return nil
        }

        if let decoded = try? jsonDecoder.decode(SynaraLaterContent.self, from: data) {
            return decoded
        }

        return decodeEnvelopeData(data, jsonDecoder: jsonDecoder)
    }

    private static func extractAccountDataContent(from data: Data) -> [String: Any]? {
        let object: Any

        do {
            object = try JSONSerialization.jsonObject(with: data)
        } catch {
            return nil
        }

        guard let top = object as? [String: Any] else {
            return nil
        }

        if let content = top["content"] as? [String: Any] {
            return content
        }

        if top["items"] != nil || top["version"] != nil {
            return top
        }

        return nil
    }

    private static func decode(content: [String: Any], jsonDecoder: JSONDecoder) -> SynaraLaterContent? {
        do {
            let data = try JSONSerialization.data(withJSONObject: content)
            return try jsonDecoder.decode(SynaraLaterContent.self, from: data)
        } catch {
            return nil
        }
    }
}

extension SynaraLaterContent {
    func completingItem(id: String, at completedAt: Int) throws -> SynaraLaterContent {
        guard let item = items[id] else {
            return self
        }

        let completedItem = try SynaraLaterItem(
            id: item.id,
            kind: item.kind,
            roomId: item.roomId,
            eventId: item.eventId,
            createdAt: item.createdAt,
            dueTs: item.dueTs,
            remindedAt: item.remindedAt,
            completedAt: completedAt
        )

        var updatedItems = items
        updatedItems[id] = completedItem
        return try SynaraLaterContent(version: version, items: updatedItems)
    }
}

extension SynaraLaterListItem {
    static func sorted(items: SynaraLaterContent, now: Int) -> [SynaraLaterListItem] {
        return items.items.values
            .map {
                SynaraLaterListItem(
                    id: $0.id,
                    roomID: $0.roomId,
                    eventID: $0.eventId,
                    kind: $0.kind,
                    dueTs: $0.dueTs,
                    completedAt: $0.completedAt,
                    createdAt: $0.createdAt,
                    isCompleted: $0.completedAt != nil
                )
            }
            .sorted { left, right in
                if left.completedAt != nil, right.completedAt == nil {
                    return false
                }

                if left.completedAt == nil, right.completedAt != nil {
                    return true
                }

                let leftDue = left.dueTs ?? Int.max
                let rightDue = right.dueTs ?? Int.max
                let leftDueSoon = leftDue <= now
                let rightDueSoon = rightDue <= now

                if leftDueSoon != rightDueSoon {
                    return leftDueSoon
                }

                if leftDue != rightDue {
                    return leftDue < rightDue
                }

                return left.createdAt > right.createdAt
            }
    }

    static let empty = SynaraLaterListItem(
        id: "",
        roomID: "",
        eventID: "",
        kind: .saved,
        dueTs: nil,
        completedAt: nil,
        createdAt: 0,
        isCompleted: false
    )
}

final class MockLaterService: LaterServicing {
    private var items: [SynaraLaterListItem]
    private let now: () -> Int

    init(items: [SynaraLaterListItem] = [], now: @escaping () -> Int = { Int(Date().timeIntervalSince1970 * 1000) }) {
        self.items = items
        self.now = now
    }

    func loadItems() async -> Result<([SynaraLaterListItem], LaterInboxError?), Never> {
        .success((items, nil))
    }

    func completeItem(id: String) async -> Result<Bool, LaterInboxError> {
        guard let index = items.firstIndex(where: { $0.id == id }) else {
            return .success(false)
        }

        let item = items[index]
        guard item.isCompleted == false else {
            return .success(false)
        }

        items[index] = SynaraLaterListItem(
            id: item.id,
            roomID: item.roomID,
            eventID: item.eventID,
            kind: item.kind,
            dueTs: item.dueTs,
            completedAt: now(),
            createdAt: item.createdAt,
            isCompleted: true
        )
        return .success(true)
    }
}

enum SynaraAgentCardPayloadParser {
    private static let contentKeys = ["org.hermes.agent", "io.hermes.agent", "in.synara.agent", "m.custom.agent"]

    static func parse(raw: [String: Any] = [:], body: String? = nil) -> SynaraAgentCard? {
        if let directPayload = contentKeys.compactMap({ key in
            extractAgentCard(from: raw[key])
        }).first {
            return directPayload
        }

        guard let body,
              body.count <= 200_000,
              let bodyData = body.data(using: .utf8),
              let parsedBody = try? JSONSerialization.jsonObject(with: bodyData) as? [String: Any]
        else {
            return nil
        }

        if let directPayload = contentKeys.compactMap({ key in
            extractAgentCard(from: parsedBody[key])
        }).first {
            return directPayload
        }

        guard let hermes = parsedBody["hermes"] as? Bool, hermes else {
            return nil
        }

        return extractAgentCard(from: parsedBody["payload"]) ?? extractAgentCard(from: parsedBody["agent"])
    }

    static func parse(payloadJSON: String?) -> SynaraAgentCard? {
        guard let payloadJSON,
              payloadJSON.utf8.count <= 200_000,
              let data = payloadJSON.data(using: .utf8)
        else {
            return nil
        }
        return try? JSONDecoder().decode(SynaraAgentCard.self, from: data)
    }

    private static func extractAgentCard(from rawValue: Any?) -> SynaraAgentCard? {
        guard let raw = rawValue as? [String: Any] else {
            return nil
        }
        do {
            return try JSONDecoder().decode(SynaraAgentCard.self, from: JSONSerialization.data(withJSONObject: raw))
        } catch {
            return nil
        }
    }
}

struct RawTimelineEvent: Equatable {
    let eventID: String
    let senderID: String
    let senderAvatarURL: URL?
    let timestamp: Date
    let type: String
    let body: String?
    let formattedBody: String?
    let replyToEventID: String?
    let isEdited: Bool
    let mediaURL: URL?
    let mediaMimeType: String?
    let mediaByteSize: UInt64?
    let isEncrypted: Bool
    let agentCard: SynaraAgentCard?
    let reactions: [String: Int]

    init(
        eventID: String,
        senderID: String,
        senderAvatarURL: URL? = nil,
        timestamp: Date,
        type: String,
        body: String?,
        formattedBody: String? = nil,
        replyToEventID: String?,
        isEdited: Bool,
        mediaURL: URL?,
        mediaMimeType: String? = nil,
        mediaByteSize: UInt64? = nil,
        isEncrypted: Bool = false,
        agentCard: SynaraAgentCard? = nil,
        reactions: [String: Int] = [:]
    ) {
        self.eventID = eventID
        self.senderID = senderID
        self.senderAvatarURL = senderAvatarURL
        self.timestamp = timestamp
        self.type = type
        self.body = body
        self.formattedBody = formattedBody
        self.replyToEventID = replyToEventID
        self.isEdited = isEdited
        self.mediaURL = mediaURL
        self.mediaMimeType = mediaMimeType
        self.mediaByteSize = mediaByteSize
        self.isEncrypted = isEncrypted
        self.agentCard = agentCard
        self.reactions = reactions
    }
}

enum TimelineLoadOutcome: Equatable {
    case loaded([TimelineItem])
    case empty
    case failed(String)
}

protocol TimelineServicing: AnyObject {
    func loadInitialTimeline(roomID: String) async -> TimelineLoadOutcome
    func loadInitialTimeline(roomID: String, focusedEventID: String?) async -> TimelineLoadOutcome
    func loadLatestTimeline(roomID: String) async -> TimelineLoadOutcome
    func loadThreadTimeline(roomID: String, rootEventID: String) async -> TimelineLoadOutcome
    func loadOlderTimeline(roomID: String, before eventID: String) async -> TimelineLoadOutcome
    func timelineUpdates(roomID: String, focusedEventID: String?) -> AsyncStream<TimelineLoadOutcome>
    func typingUsers(roomID: String) -> AsyncStream<[String]>
    func threadTimelineUpdates(roomID: String, rootEventID: String) -> AsyncStream<TimelineLoadOutcome>
    func clearSessionCaches()
}

extension TimelineServicing {
    func clearSessionCaches() {}

    func loadInitialTimeline(roomID: String) async -> TimelineLoadOutcome {
        await loadInitialTimeline(roomID: roomID, focusedEventID: nil)
    }

    func loadLatestTimeline(roomID: String) async -> TimelineLoadOutcome {
        await loadInitialTimeline(roomID: roomID, focusedEventID: nil)
    }

    func loadThreadTimeline(roomID: String, rootEventID: String) async -> TimelineLoadOutcome {
        await loadInitialTimeline(roomID: roomID, focusedEventID: rootEventID)
    }

    func threadTimelineUpdates(roomID: String, rootEventID: String) -> AsyncStream<TimelineLoadOutcome> {
        timelineUpdates(roomID: roomID, focusedEventID: rootEventID)
    }

    func typingUsers(roomID _: String) -> AsyncStream<[String]> {
        AsyncStream { continuation in
            continuation.yield([])
            continuation.finish()
        }
    }
}

extension TimelineServicing {
    func timelineUpdates(roomID: String, focusedEventID: String?) -> AsyncStream<TimelineLoadOutcome> {
        AsyncStream { continuation in
            let task = Task {
                let outcome = await loadInitialTimeline(roomID: roomID, focusedEventID: focusedEventID)
                continuation.yield(outcome)
                continuation.finish()
            }

            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }
}

enum RoomTimelineMode: Equatable {
    case live
    case unread(markerEventID: String)
    case focused(eventID: String)

    var focusedEventID: String? {
        switch self {
        case .live:
            return nil
        case let .unread(markerEventID):
            return markerEventID
        case let .focused(eventID):
            return eventID
        }
    }

    var isLive: Bool {
        if case .live = self {
            return true
        }
        return false
    }
}

struct RoomTimelineSessionFeed {
    let generation: UInt64
    let mode: RoomTimelineMode
    let providerIsLive: Bool
    let initialOutcome: TimelineLoadOutcome
    let updates: AsyncStream<TimelineLoadOutcome>

    func presenting(mode: RoomTimelineMode) -> RoomTimelineSessionFeed {
        RoomTimelineSessionFeed(
            generation: generation,
            mode: mode,
            providerIsLive: providerIsLive,
            initialOutcome: initialOutcome,
            updates: updates
        )
    }
}

enum RoomTimelineProviderPresentationPolicy {
    static func modeWhenPinned(providerIsLive: Bool, currentMode: RoomTimelineMode) -> RoomTimelineMode {
        providerIsLive ? .live : currentMode
    }

    static func focusedEventID(providerIsLive: Bool, currentMode: RoomTimelineMode) -> String? {
        providerIsLive ? nil : currentMode.focusedEventID
    }
}

enum RoomTimelineLiveTransition {
    case succeeded(RoomTimelineSessionFeed)
    case empty
    case failed(String)
    case superseded
}

actor RoomTimelineSession {
    private let roomID: String
    private let service: TimelineServicing
    private var generation: UInt64 = 0
    private var mode: RoomTimelineMode = .live
    private var serverItems: [TimelineItem] = []

    init(roomID: String, service: TimelineServicing) {
        self.roomID = roomID
        self.service = service
    }

    func open(mode: RoomTimelineMode) async -> RoomTimelineSessionFeed? {
        generation &+= 1
        let requestedGeneration = generation
        self.mode = mode
        serverItems = []

        let outcome = await service.loadInitialTimeline(
            roomID: roomID,
            focusedEventID: mode.focusedEventID
        )
        guard requestedGeneration == generation else {
            return nil
        }

        let accepted = accept(outcome, generation: requestedGeneration)
        return RoomTimelineSessionFeed(
            generation: requestedGeneration,
            mode: mode,
            providerIsLive: mode.isLive,
            initialOutcome: accepted,
            updates: makeUpdateStream(mode: mode, generation: requestedGeneration)
        )
    }

    func transitionToLive() async -> RoomTimelineLiveTransition {
        let originGeneration = generation
        let outcome = await service.loadLatestTimeline(roomID: roomID)
        guard originGeneration == generation else {
            return .superseded
        }

        switch outcome {
        case let .loaded(items) where items.isEmpty == false:
            generation &+= 1
            mode = .live
            serverItems = TimelineWindowPolicy.replacingServerWindow(items)
            let nextGeneration = generation
            return .succeeded(
                RoomTimelineSessionFeed(
                    generation: nextGeneration,
                    mode: .live,
                    providerIsLive: true,
                    initialOutcome: .loaded(serverItems),
                    updates: makeUpdateStream(mode: .live, generation: nextGeneration)
                )
            )
        case .loaded, .empty:
            return .empty
        case let .failed(message):
            return .failed(message)
        }
    }

    func loadOlder(before eventID: String) async -> TimelineLoadOutcome? {
        let originGeneration = generation
        let outcome = await service.loadOlderTimeline(roomID: roomID, before: eventID)
        guard originGeneration == generation else {
            return nil
        }

        switch outcome {
        case let .loaded(olderItems) where olderItems.isEmpty == false:
            let existingIDs = Set(serverItems.map { $0.eventID.isEmpty ? $0.id : $0.eventID })
            let hasNewStableItem = olderItems.contains { item in
                let key = item.eventID.isEmpty ? item.id : item.eventID
                return item.isLocalPending == false && existingIDs.contains(key) == false
            }
            guard hasNewStableItem else {
                return .empty
            }
            generation &+= 1
            mode = .focused(eventID: eventID)
            serverItems = TimelineWindowPolicy.prependingHistory(olderItems, to: serverItems)
            return .loaded(serverItems)
        case .loaded, .empty:
            return .empty
        case let .failed(message):
            return .failed(message)
        }
    }

    func invalidate() {
        generation &+= 1
        serverItems = []
    }

    func currentGeneration() -> UInt64 {
        generation
    }

    private func accept(_ outcome: TimelineLoadOutcome, generation requestedGeneration: UInt64) -> TimelineLoadOutcome {
        guard requestedGeneration == generation else {
            return .empty
        }
        switch outcome {
        case let .loaded(items):
            serverItems = TimelineWindowPolicy.replacingServerWindow(items)
            return serverItems.isEmpty ? .empty : .loaded(serverItems)
        case .empty:
            return .empty
        case let .failed(message):
            return .failed(message)
        }
    }

    private func acceptedUpdate(
        _ outcome: TimelineLoadOutcome,
        generation requestedGeneration: UInt64
    ) -> TimelineLoadOutcome? {
        guard requestedGeneration == generation else {
            return nil
        }
        return accept(outcome, generation: requestedGeneration)
    }

    private func makeUpdateStream(
        mode: RoomTimelineMode,
        generation requestedGeneration: UInt64
    ) -> AsyncStream<TimelineLoadOutcome> {
        let service = self.service
        let roomID = self.roomID
        return AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let task = Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }
                for await outcome in service.timelineUpdates(
                    roomID: roomID,
                    focusedEventID: mode.focusedEventID
                ) {
                    guard Task.isCancelled == false else {
                        break
                    }
                    guard let accepted = await self.acceptedUpdate(
                        outcome,
                        generation: requestedGeneration
                    ) else {
                        break
                    }
                    continuation.yield(accepted)
                }
                continuation.finish()
            }
            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }
}

enum TimelineMapper {
    static func map(_ event: RawTimelineEvent) -> TimelineItem {
        let kind: TimelineItem.Kind

        if let agentCard = event.agentCard {
            kind = .agentCard(agentCard)
        } else {
            switch event.type {
            case "m.room.message":
                if let formattedBody = event.formattedBody, formattedBody.isEmpty == false {
                    kind = .formattedText(body: event.body ?? "", html: formattedBody)
                } else {
                    kind = .text(event.body ?? "")
                }
            case "m.room.encrypted":
                kind = .encryptedPlaceholder
            case "m.room.redaction":
                kind = .redacted
            case "m.room.media":
                kind = .mediaPlaceholder(
                    MediaResource(
                        id: event.eventID,
                        filename: event.body ?? "Attachment",
                        authenticatedURL: event.mediaURL,
                        requiresAuthentication: true,
                        isEncrypted: event.isEncrypted,
                        mimeType: event.mediaMimeType,
                        byteSize: event.mediaByteSize
                    )
                )
            default:
                kind = .unknown(type: event.type)
            }
        }

        return TimelineItem(
            id: event.eventID,
            eventID: event.eventID,
            senderID: event.senderID,
            senderAvatarURL: event.senderAvatarURL,
            timestamp: event.timestamp,
            kind: kind,
            replyToEventID: event.replyToEventID,
            isEdited: event.isEdited,
            reactions: event.reactions,
            isEncrypted: event.type == "m.room.encrypted" || event.isEncrypted
        )
    }
}

enum TimelineFixtures {
    static let baseDate = Date(timeIntervalSince1970: 1_700_000_000)

    static func commonEvents(roomID: String = "!project:matrix.org") -> [RawTimelineEvent] {
        [
            RawTimelineEvent(
                eventID: "$text:\(roomID)",
                senderID: "@mina:matrix.org",
                timestamp: baseDate,
                type: "m.room.message",
                body: "Here's the latest spec for the new permissions model.",
                replyToEventID: nil,
                isEdited: false,
                mediaURL: nil,
                reactions: ["👍": 3, "👏": 2]
            ),
            RawTimelineEvent(
                eventID: "$media:\(roomID)",
                senderID: "@mina:matrix.org",
                timestamp: baseDate.addingTimeInterval(8),
                type: "m.room.media",
                body: "permissions-spec.pdf",
                replyToEventID: nil,
                isEdited: false,
                mediaURL: URL(string: "mxc://matrix.org/media-id")
            ),
            RawTimelineEvent(
                eventID: "$alex:\(roomID)",
                senderID: "@alex:matrix.org",
                timestamp: baseDate.addingTimeInterval(60),
                type: "m.room.message",
                body: "Thanks! I'll take a look and drop feedback.",
                replyToEventID: nil,
                isEdited: false,
                mediaURL: nil
            ),
            RawTimelineEvent(
                eventID: "$security:\(roomID)",
                senderID: "@ravi:matrix.org",
                timestamp: baseDate.addingTimeInterval(300),
                type: "m.room.message",
                body: "We should also update the role matrix while we're at it.",
                replyToEventID: nil,
                isEdited: false,
                mediaURL: nil,
                reactions: ["👍": 2]
            ),
            RawTimelineEvent(
                eventID: "$thread-reply:\(roomID)",
                senderID: "@mina:matrix.org",
                timestamp: baseDate.addingTimeInterval(360),
                type: "m.room.message",
                body: "+1 — I'll update the doc and share a draft.",
                replyToEventID: "$security:\(roomID)",
                isEdited: false,
                mediaURL: nil,
                reactions: ["👍": 1]
            ),
            RawTimelineEvent(
                eventID: "$alex-thread:\(roomID)",
                senderID: "@alex:matrix.org",
                timestamp: baseDate.addingTimeInterval(365),
                type: "m.room.message",
                body: "Can I take a pass on the reviewer roles?",
                replyToEventID: "$security:\(roomID)",
                isEdited: false,
                mediaURL: nil
            ),
        ]
    }

    static func largeTimeline(count: Int = 10000) -> [TimelineItem] {
        largeTimeline(indices: 0 ..< count)
    }

    static func largeTimeline(
        indices: Range<Int>,
        expandedMessageIndex: Int? = nil,
        expandedLineCount: Int = 180
    ) -> [TimelineItem] {
        var items: [TimelineItem] = []
        items.reserveCapacity(indices.count)

        for index in indices {
            let body: String
            if index == expandedMessageIndex {
                let lines = (1 ... max(1, expandedLineCount)).map { "Variable height line \($0)" }
                body = (["Expanded variable-height message \(index)"] + lines).joined(separator: "\n")
            } else {
                body = "Synthetic message \(index)"
            }
            let item = TimelineItem(
                id: "$synthetic-\(index):matrix.org",
                eventID: "$synthetic-\(index):matrix.org",
                senderID: index % 2 == 0 ? "@alice:matrix.org" : "@bob:matrix.org",
                timestamp: baseDate.addingTimeInterval(TimeInterval(index)),
                kind: .text(body),
                replyToEventID: nil,
                isEdited: false,
                reactions: [:]
            )
            items.append(item)
        }

        return items
    }
}

enum TimelineReplyCounter {
    static func replyCounts(for items: [TimelineItem]) -> [String: Int] {
        var counts: [String: Int] = [:]
        counts.reserveCapacity(min(items.count, 512))

        for item in items {
            guard let replyToEventID = item.replyToEventID else { continue }
            counts[replyToEventID, default: 0] += 1
        }

        return counts
    }
}

final class MockTimelineService: TimelineServicing {
    var events: [RawTimelineEvent]
    var itemFixture: [TimelineItem]?
    var updateOutcomes: [TimelineLoadOutcome] = []
    var typingUserUpdates: [[String]] = []
    var latestOutcome: TimelineLoadOutcome?
    var olderOutcome: TimelineLoadOutcome?
    var loadDelayNanoseconds: UInt64 = 0
    var updateDelayNanoseconds: UInt64 = 0
    private(set) var clearSessionCachesCallCount = 0
    private let usesRoomSpecificCommonEvents: Bool

    init() {
        events = TimelineFixtures.commonEvents()
        itemFixture = nil
        usesRoomSpecificCommonEvents = true
    }

    init(events: [RawTimelineEvent]) {
        self.events = events
        itemFixture = nil
        usesRoomSpecificCommonEvents = false
    }

    init(items: [TimelineItem]) {
        events = []
        itemFixture = items
        usesRoomSpecificCommonEvents = false
    }

    func clearSessionCaches() {
        clearSessionCachesCallCount += 1
    }

    func loadInitialTimeline(roomID: String) async -> TimelineLoadOutcome {
        await loadInitialTimeline(roomID: roomID, focusedEventID: nil)
    }

    func loadInitialTimeline(roomID: String, focusedEventID: String?) async -> TimelineLoadOutcome {
        if loadDelayNanoseconds > 0 {
            try? await Task.sleep(nanoseconds: loadDelayNanoseconds)
        }
        let items = timelineItems(roomID: roomID)
        if let focusedEventID,
           let focusedIndex = items.firstIndex(where: {
               $0.eventID == focusedEventID || $0.id == focusedEventID
           })
        {
            let lowerBound = max(items.startIndex, focusedIndex - 50)
            let upperBound = min(items.endIndex, focusedIndex + 51)
            return .loaded(Array(items[lowerBound ..< upperBound]))
        }
        return items.isEmpty ? .empty : .loaded(items)
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> TimelineLoadOutcome {
        if let olderOutcome {
            return olderOutcome
        }
        if let itemFixture {
            let older = Array(itemFixture.prefix(50))
            return older.isEmpty ? .empty : .loaded(older)
        }
        let older = timelineItems(roomID: roomID).filter { $0.eventID != eventID }
        return older.isEmpty ? .empty : .loaded(older)
    }

    func loadLatestTimeline(roomID: String) async -> TimelineLoadOutcome {
        if let latestOutcome {
            return latestOutcome
        }
        return await loadInitialTimeline(roomID: roomID, focusedEventID: nil)
    }

    func loadThreadTimeline(roomID: String, rootEventID: String) async -> TimelineLoadOutcome {
        let items = timelineItems(roomID: roomID)
        let threadItems = threadTimelineItems(from: items, rootEventID: rootEventID)
        return threadItems.isEmpty ? .empty : .loaded(threadItems)
    }

    func threadTimelineUpdates(roomID: String, rootEventID: String) -> AsyncStream<TimelineLoadOutcome> {
        timelineUpdates(roomID: roomID, focusedEventID: rootEventID)
    }

    private func threadTimelineItems(from items: [TimelineItem], rootEventID: String) -> [TimelineItem] {
        let root = items.first { $0.eventID == rootEventID }
        let replies = items
            .filter { $0.replyToEventID == rootEventID }
            .sorted { $0.timestamp < $1.timestamp }

        if let root {
            return [root] + replies
        }

        return replies
    }

    private func timelineItems(roomID: String) -> [TimelineItem] {
        if let itemFixture {
            return itemFixture
        }
        let sourceEvents = usesRoomSpecificCommonEvents
            ? TimelineFixtures.commonEvents(roomID: roomID)
            : events
        return sourceEvents.map(TimelineMapper.map)
    }

    func timelineUpdates(roomID: String, focusedEventID: String?) -> AsyncStream<TimelineLoadOutcome> {
        AsyncStream { continuation in
            let task = Task {
                let outcomes: [TimelineLoadOutcome]
                if updateOutcomes.isEmpty {
                    outcomes = [await loadInitialTimeline(roomID: roomID, focusedEventID: focusedEventID)]
                } else {
                    outcomes = updateOutcomes
                }

                if updateDelayNanoseconds > 0 {
                    try? await Task.sleep(nanoseconds: updateDelayNanoseconds)
                }
                for outcome in outcomes {
                    continuation.yield(outcome)
                }
                continuation.finish()
            }

            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    func typingUsers(roomID _: String) -> AsyncStream<[String]> {
        AsyncStream { continuation in
            let updates = typingUserUpdates
            for userIDs in updates {
                continuation.yield(userIDs)
            }
            continuation.finish()
        }
    }
}

enum MatrixHTMLRenderer {
    private struct NativeHTMLLinkProjection {
        let html: String
        let originalBySentinel: [String: URL]
    }

    private final class RichTextCacheEntry: NSObject {
        let value: RichText

        init(_ value: RichText) {
            self.value = value
        }
    }

    private static let richTextCache: NSCache<NSString, RichTextCacheEntry> = {
        let cache = NSCache<NSString, RichTextCacheEntry>()
        cache.countLimit = 600
        cache.totalCostLimit = 8 * 1_024 * 1_024
        return cache
    }()
    private static let maximumRichHTMLBytes = 256 * 1_024

    struct RichText: Equatable {
        struct Style: OptionSet, Hashable {
            let rawValue: UInt8

            static let bold = Style(rawValue: 1 << 0)
            static let italic = Style(rawValue: 1 << 1)
            static let strikethrough = Style(rawValue: 1 << 2)
            static let underline = Style(rawValue: 1 << 3)
            static let code = Style(rawValue: 1 << 4)
        }

        struct Run: Equatable {
            let text: String
            let style: Style
            let link: URL?
        }

        let runs: [Run]

        var plainText: String {
            runs.map(\.text).joined()
        }
    }

    struct CodeBlock: Equatable {
        let code: String
        let language: String?
    }

    struct DetailsBlock: Equatable {
        let summary: String
        let code: CodeBlock?
        let body: String
    }

    struct SpoilerBlock: Equatable {
        let content: RichText
        let reason: String?
    }

    struct TableCell: Equatable {
        let content: RichText
        let isHeader: Bool

        var plainText: String {
            content.plainText
        }
    }

    struct TableRow: Equatable {
        let cells: [TableCell]
        let isHeader: Bool
    }

    struct TableBlock: Equatable {
        let caption: RichText?
        let rows: [TableRow]
    }

    enum Segment: Equatable {
        case richText(RichText)
        case code(CodeBlock)
        case quote(RichText)
        case spoiler(SpoilerBlock)
        case details(DetailsBlock)
        case table(TableBlock)
    }

    static func segments(body: String, html: String) -> [Segment] {
        guard html.utf8.count <= maximumRichHTMLBytes else {
            let fallback = fallbackRichText(body)
            return fallback.runs.isEmpty ? [] : [.richText(fallback)]
        }
        let sanitized = html.sanitizingMatrixHTMLForNativeImport()
        let pattern = #"<details(?:\s+[^>]*)?>[\s\S]*?</details\s*>|<pre(?:\s+[^>]*)?>[\s\S]*?</pre\s*>|<blockquote(?:\s+[^>]*)?>[\s\S]*?</blockquote\s*>|<table(?:\s+[^>]*)?>[\s\S]*?</table\s*>|<span\b[^>]*\bdata-mx-spoiler(?:\s*=\s*(?:\"[^\"]*\"|'[^']*'|[^\s>]+))?[^>]*>[\s\S]*?</span\s*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            let text = richText(body: body, html: html)
            return text.runs.isEmpty ? [] : [.richText(text)]
        }

        let nsRange = NSRange(sanitized.startIndex ..< sanitized.endIndex, in: sanitized)
        let matches = regex.matches(in: sanitized, range: nsRange)
        guard matches.isEmpty == false else {
            let text = richText(body: body, html: html)
            return text.runs.isEmpty ? [] : [.richText(text)]
        }

        var segments: [Segment] = []
        var cursor = sanitized.startIndex

        for match in matches {
            guard let range = Range(match.range(at: 0), in: sanitized) else {
                continue
            }

            appendRichTextSegment(from: String(sanitized[cursor ..< range.lowerBound]), to: &segments)

            let blockHTML = String(sanitized[range])
            if blockHTML.range(of: #"^\s*<details"#, options: [.regularExpression, .caseInsensitive]) != nil {
                if let block = detailsBlocks(html: blockHTML).first {
                    segments.append(.details(block))
                }
            } else if blockHTML.range(of: #"^\s*<blockquote"#, options: [.regularExpression, .caseInsensitive]) != nil,
                      let quote = quoteBlock(html: blockHTML)
            {
                segments.append(.quote(quote))
            } else if blockHTML.range(of: #"^\s*<table"#, options: [.regularExpression, .caseInsensitive]) != nil,
                      let table = tableBlock(html: blockHTML)
            {
                segments.append(.table(table))
            } else if blockHTML.range(of: #"^\s*<span\b[^>]*\bdata-mx-spoiler"#, options: [.regularExpression, .caseInsensitive]) != nil,
                      let spoiler = spoilerBlock(html: blockHTML)
            {
                segments.append(.spoiler(spoiler))
            } else if let code = codeBlock(html: blockHTML) {
                segments.append(.code(code))
            }

            cursor = range.upperBound
        }

        appendRichTextSegment(from: String(sanitized[cursor...]), to: &segments)

        if segments.isEmpty {
            let text = richText(body: body, html: html)
            return text.runs.isEmpty ? [] : [.richText(text)]
        }
        return segments
    }

    /// Imports the SDK-sanitized Matrix HTML as HTML, never as Markdown. This
    /// distinction is essential: literal `**`, `~~`, and backticks in an HTML
    /// text node are user content and must not acquire formatting on a second
    /// parse. We project only the small set of semantic attributes SwiftUI
    /// needs and validate every link again before it reaches the view.
    static func richText(body: String, html: String) -> RichText {
        #if canImport(UIKit)
            let cacheKey = "\(body)\u{0}\(html)" as NSString
            if let cached = richTextCache.object(forKey: cacheKey) {
                return cached.value
            }
            guard html.utf8.count <= maximumRichHTMLBytes else {
                let fallback = fallbackRichText(body)
                cacheRichText(fallback, forKey: cacheKey, body: body, html: html)
                return fallback
            }

            let preparedHTML = html
                .sanitizingMatrixHTMLForNativeImport()
                .replacingHTMLListsForRichText()
                // UIKit's HTML importer emits only a single line break between
                // adjacent paragraphs. Matrix paragraphs are semantic blocks,
                // so retain the visible paragraph separation explicitly.
                .replacingHTMLPattern(#"</p\s*>"#, with: "</p><br>")
            let linkProjection = projectingSafeLinksForNativeImport(preparedHTML)
            let safeHTML = linkProjection.html

            let options: [NSAttributedString.DocumentReadingOptionKey: Any] = [
                .documentType: NSAttributedString.DocumentType.html,
                .characterEncoding: String.Encoding.utf8.rawValue,
            ]
            guard let imported = try? NSAttributedString(
                data: Data(safeHTML.utf8),
                options: options,
                documentAttributes: nil
            ) else {
                let fallback = fallbackRichText(body)
                cacheRichText(fallback, forKey: cacheKey, body: body, html: html)
                return fallback
            }

            var runs: [RichText.Run] = []
            imported.enumerateAttributes(
                in: NSRange(location: 0, length: imported.length),
                options: []
            ) { attributes, range, _ in
                guard range.length > 0 else {
                    return
                }

                var text = imported.attributedSubstring(from: range).string
                    .replacingOccurrences(of: "\u{2028}", with: "\n")
                    .replacingOccurrences(of: "\u{2029}", with: "\n")
                if let paragraph = attributes[.paragraphStyle] as? NSParagraphStyle,
                   paragraph.paragraphSpacing > 0,
                   paragraph.textLists.isEmpty,
                   text.hasSuffix("\n"),
                   text.hasSuffix("\n\n") == false
                {
                    text.append("\n")
                }

                var style: RichText.Style = []
                if let font = attributes[.font] as? UIFont {
                    let traits = font.fontDescriptor.symbolicTraits
                    if traits.contains(.traitBold) {
                        style.insert(.bold)
                    }
                    if traits.contains(.traitItalic) {
                        style.insert(.italic)
                    }
                    if traits.contains(.traitMonoSpace) {
                        style.insert(.code)
                    }
                }
                if (attributes[.strikethroughStyle] as? NSNumber)?.intValue ?? 0 != 0 {
                    style.insert(.strikethrough)
                }
                if (attributes[.underlineStyle] as? NSNumber)?.intValue ?? 0 != 0 {
                    style.insert(.underline)
                }

                let rawLink = (attributes[.link] as? URL)?.absoluteString
                    ?? attributes[.link] as? String
                let link = rawLink.flatMap { candidate in
                    if let original = linkProjection.originalBySentinel[candidate] {
                        return original
                    }
                    return candidate.isSafeMatrixHTMLLink ? URL(string: candidate) : nil
                }
                appendRichTextRun(.init(text: text, style: style, link: link), to: &runs)
            }

            let trimmed = trimmingRichTextRuns(collapsingExcessiveNewlines(in: runs))
            let result = trimmed.isEmpty ? fallbackRichText(body) : RichText(runs: trimmed)
            cacheRichText(result, forKey: cacheKey, body: body, html: html)
            return result
        #else
            return fallbackRichText(body)
        #endif
    }

    static func detailsBlocks(html: String) -> [DetailsBlock] {
        let sanitized = html
            .removingHTMLBlocks(named: "script")
            .removingHTMLBlocks(named: "style")
        let pattern = #"<details(?:\s+[^>]*)?>([\s\S]*?)</details\s*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return []
        }

        let source = sanitized as NSString
        let nsRange = NSRange(sanitized.startIndex ..< sanitized.endIndex, in: sanitized)
        return regex.matches(in: sanitized, range: nsRange).compactMap { match in
            guard match.numberOfRanges == 2 else {
                return nil
            }

            let content = source.substring(with: match.range(at: 1))
            guard let summary = firstHTMLCapture(
                in: content,
                pattern: #"<summary(?:\s+[^>]*)?>([\s\S]*?)</summary\s*>"#
            )?.strippingHTMLTagsAndDecoding(),
                summary.isEmpty == false
            else {
                return nil
            }

            let code = codeBlock(html: content)

            let body = content
                .replacingHTMLPattern(#"<summary(?:\s+[^>]*)?>[\s\S]*?</summary\s*>"#, with: "")
                .replacingHTMLPattern(#"<pre(?:\s+[^>]*)?>[\s\S]*?</pre\s*>"#, with: "")
                .strippingHTMLTagsAndDecoding()
                .trimmingCharacters(in: .whitespacesAndNewlines)

            return DetailsBlock(summary: summary, code: code, body: body)
        }
    }

    static func markdownExcludingDetails(body: String, html: String) -> String {
        let htmlWithoutDetails = html.replacingHTMLPattern(#"<details(?:\s+[^>]*)?>[\s\S]*?</details\s*>"#, with: "")
        let markdown = sanitizedMarkdown(body: "", html: htmlWithoutDetails)
        return markdown.isEmpty ? body : markdown
    }

    static func sanitizedMarkdown(body: String, html: String) -> String {
        var output = html
            .removingHTMLBlocks(named: "script")
            .removingHTMLBlocks(named: "style")

        output = output.replacingPreformattedBlocks()
        output = output.replacingAnchorTags()
        output = output.replacingHeadingTags()
        output = output.replacingTag("strong", with: "**")
        output = output.replacingTag("b", with: "**")
        output = output.replacingTag("em", with: "*")
        output = output.replacingTag("i", with: "*")
        output = output.replacingTag("code", with: "`")
        output = output.replacingTag("del", with: "~~")
        output = output.replacingTag("s", with: "~~")
        output = output.replacingTables()
        // Pretty-printed Matrix HTML commonly places a source newline after
        // `<br>` (Hermes does this for approval choices). That source newline
        // is formatting around the element, not a second user-visible break.
        output = output.replacingHTMLPattern(#"<br\s*/?>[ \t]*(?:\r?\n)?"#, with: "\n")
        output = output.replacingHTMLPattern(#"</p\s*>"#, with: "\n\n")
        output = output.replacingHTMLPattern(#"<p(?:\s+[^>]*)?>"#, with: "")
        output = output.replacingOrderedLists()
        output = output.replacingUnorderedLists()
        output = output.replacingHTMLPattern(#"<li(?:\s+[^>]*)?>"#, with: "\n- ")
        output = output.replacingHTMLPattern(#"</li\s*>"#, with: "")
        output = output.replacingHTMLPattern(#"</?(ul|ol)(?:\s+[^>]*)?>"#, with: "\n")
        output = output.replacingHTMLPattern(#"<blockquote(?:\s+[^>]*)?>"#, with: "\n> ")
        output = output.replacingHTMLPattern(#"</blockquote\s*>"#, with: "\n")
        output = output.replacingHTMLPattern(#"<hr(?:\s+[^>]*)?/?>"#, with: "\n---\n")
        output = output.replacingHTMLPattern(#"<span[^>]*data-mx-spoiler[^>]*>"#, with: "")
        output = output.replacingHTMLPattern(#"</?span(?:\s+[^>]*)?>"#, with: "")
        output = output.replacingHTMLPattern(#"</div\s*>"#, with: "\n")
        output = output.replacingHTMLPattern(#"<div(?:\s+[^>]*)?>"#, with: "")
        output = output.replacingHTMLPattern(#"</?[^>]+>"#, with: "")
        output = output.decodingBasicHTMLEntities()
        output = output.replacingHTMLPattern(#"\n{3,}"#, with: "\n\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        return output.isEmpty ? body : output
    }

    private static func appendRichTextSegment(from html: String, to segments: inout [Segment]) {
        let text = richText(body: "", html: html)
        guard text.runs.isEmpty == false else {
            return
        }
        segments.append(.richText(text))
    }

    static func codeLineCount(_ code: String) -> Int {
        let normalized = code.hasSuffix("\n") ? String(code.dropLast()) : code
        if normalized.isEmpty {
            return 1
        }
        return normalized.split(separator: "\n", omittingEmptySubsequences: false).count
    }

    static func tableBlock(html: String) -> TableBlock? {
        let caption = firstHTMLCapture(
            in: html,
            pattern: #"<caption(?:\s+[^>]*)?>([\s\S]*?)</caption\s*>"#
        ).map { richText(body: "", html: $0) }
            .flatMap { $0.runs.isEmpty ? nil : $0 }
        let rows = html.htmlTableRowFragments().map { row in
            TableRow(
                cells: row.cells.map {
                    TableCell(
                        content: richText(body: "", html: $0.html),
                        isHeader: $0.isHeader
                    )
                },
                isHeader: row.isHeader
            )
        }
        return rows.isEmpty ? nil : TableBlock(caption: caption, rows: rows)
    }

    private static func spoilerBlock(html: String) -> SpoilerBlock? {
        guard let openingTag = firstHTMLCapture(
            in: html,
            pattern: #"^(<span\b[^>]*\bdata-mx-spoiler(?:\s*=\s*(?:\"[^\"]*\"|'[^']*'|[^\s>]+))?[^>]*>)"#
        ),
            let contentHTML = firstHTMLCapture(
                in: html,
                pattern: #"^<span\b[^>]*\bdata-mx-spoiler(?:\s*=\s*(?:\"[^\"]*\"|'[^']*'|[^\s>]+))?[^>]*>([\s\S]*)</span\s*>$"#
            )
        else {
            return nil
        }

        let content = richText(body: "", html: contentHTML)
        guard content.runs.isEmpty == false else { return nil }
        let reason = MatrixHTMLTag(raw: String(openingTag.dropFirst().dropLast()))?
            .attributes["data-mx-spoiler"]?
            .decodingBasicHTMLEntities()
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return SpoilerBlock(
            content: content,
            reason: reason.flatMap { $0.isEmpty ? nil : String($0.prefix(160)) }
        )
    }

    private static func codeBlock(html: String) -> CodeBlock? {
        let rawCode = firstHTMLCapture(
            in: html,
            pattern: #"<pre(?:\s+[^>]*)?>\s*<code(?:\s+[^>]*)?>([\s\S]*?)</code\s*>\s*</pre\s*>"#
        ) ?? firstHTMLCapture(
            in: html,
            pattern: #"<pre(?:\s+[^>]*)?>([\s\S]*?)</pre\s*>"#
        )
        let code = rawCode?
            .replacingHTMLPattern(#"</?code(?:\s+[^>]*)?>"#, with: "")
            .decodingBasicHTMLEntities()

        guard let code, code.isEmpty == false else {
            return nil
        }
        return CodeBlock(code: code, language: codeLanguage(html: html))
    }

    private static func codeLanguage(html: String) -> String? {
        guard let classes = firstHTMLCapture(
            in: html,
            pattern: #"<code[^>]*\bclass\s*=\s*[\"']([^\"']*)[\"'][^>]*>"#
        ) else {
            return nil
        }
        let language = classes
            .split(whereSeparator: \.isWhitespace)
            .map(String.init)
            .first { $0.lowercased().hasPrefix("language-") }?
            .dropFirst("language-".count)
        guard let language, language.isEmpty == false, language.count <= 32,
              language.unicodeScalars.allSatisfy({
                  CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "_+.-")).contains($0)
              })
        else {
            return nil
        }
        return String(language)
    }

    private static func quoteBlock(html: String) -> RichText? {
        let rawQuote = firstHTMLCapture(
            in: html,
            pattern: #"<blockquote(?:\s+[^>]*)?>([\s\S]*?)</blockquote\s*>"#
        )
        let quote = rawQuote.map { richText(body: "", html: $0) }

        return quote?.runs.isEmpty == false ? quote : nil
    }

    private static func fallbackRichText(_ body: String) -> RichText {
        let text = body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard text.isEmpty == false else {
            return RichText(runs: [])
        }
        return RichText(runs: [.init(text: text, style: [], link: nil)])
    }

    private static func cacheRichText(
        _ text: RichText,
        forKey key: NSString,
        body: String,
        html: String
    ) {
        let cost = body.utf8.count + html.utf8.count + text.plainText.utf8.count
        richTextCache.setObject(RichTextCacheEntry(text), forKey: key, cost: cost)
    }

    /// Foundation's HTML importer silently drops some safe Matrix schemes
    /// (notably `magnet:` and `matrix:`). Replace every already-validated href
    /// with a local HTTPS sentinel for import, then restore the exact URL on
    /// the typed run. This also prevents importer-specific URL rewriting.
    private static func projectingSafeLinksForNativeImport(_ html: String) -> NativeHTMLLinkProjection {
        let pattern = #"<a href=\"([^\"]+)\">"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return NativeHTMLLinkProjection(html: html, originalBySentinel: [:])
        }
        let range = NSRange(html.startIndex ..< html.endIndex, in: html)
        let matches = regex.matches(in: html, range: range)
        guard matches.isEmpty == false else {
            return NativeHTMLLinkProjection(html: html, originalBySentinel: [:])
        }

        let source = html as NSString
        let output = NSMutableString(string: html)
        var links: [String: URL] = [:]
        for (offset, match) in matches.enumerated().reversed() {
            guard match.numberOfRanges == 2 else { continue }
            let raw = source.substring(with: match.range(at: 1)).decodingBasicHTMLEntities()
            guard raw.isSafeMatrixHTMLLink, let original = URL(string: raw) else { continue }
            let sentinel = "https://synara.invalid/__matrix_link/\(offset)"
            links[sentinel] = original
            let replacement = "<a href=\"\(sentinel)\">"
            output.replaceCharacters(in: match.range(at: 0), with: replacement)
        }
        return NativeHTMLLinkProjection(html: output as String, originalBySentinel: links)
    }

    private static func appendRichTextRun(_ run: RichText.Run, to runs: inout [RichText.Run]) {
        guard run.text.isEmpty == false else {
            return
        }
        if let last = runs.last, last.style == run.style, last.link == run.link {
            runs[runs.count - 1] = .init(text: last.text + run.text, style: run.style, link: run.link)
        } else {
            runs.append(run)
        }
    }

    private static func trimmingRichTextRuns(_ source: [RichText.Run]) -> [RichText.Run] {
        var runs = source.filter { $0.text.isEmpty == false }
        let edgeWhitespace = CharacterSet(charactersIn: " \r\n")
        while let first = runs.first {
            let text = first.text.trimmingLeadingCharacters(in: edgeWhitespace)
            if text.isEmpty {
                runs.removeFirst()
            } else {
                runs[0] = .init(text: text, style: first.style, link: first.link)
                break
            }
        }
        while let last = runs.last {
            let text = last.text.trimmingTrailingCharacters(in: edgeWhitespace)
            if text.isEmpty {
                runs.removeLast()
            } else {
                runs[runs.count - 1] = .init(text: text, style: last.style, link: last.link)
                break
            }
        }
        return runs
    }

    private static func collapsingExcessiveNewlines(in source: [RichText.Run]) -> [RichText.Run] {
        var output: [RichText.Run] = []
        var newlineCount = 0
        for run in source {
            var text = ""
            for character in run.text {
                if character == "\n" {
                    guard newlineCount < 2 else {
                        continue
                    }
                    newlineCount += 1
                } else {
                    newlineCount = 0
                }
                text.append(character)
            }
            appendRichTextRun(.init(text: text, style: run.style, link: run.link), to: &output)
        }
        return output
    }

    private static func firstHTMLCapture(in html: String, pattern: String) -> String? {
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return nil
        }

        let nsRange = NSRange(html.startIndex ..< html.endIndex, in: html)
        guard let match = regex.firstMatch(in: html, range: nsRange),
              match.numberOfRanges == 2,
              let range = Range(match.range(at: 1), in: html)
        else {
            return nil
        }

        return String(html[range])
    }
}

private struct MatrixHTMLTag {
    let name: String
    let attributes: [String: String]
    let isClosing: Bool
    let isSelfClosing: Bool

    init?(raw: String) {
        var characters = Array(raw)
        while characters.first?.isWhitespace == true { characters.removeFirst() }
        while characters.last?.isWhitespace == true { characters.removeLast() }
        guard characters.isEmpty == false,
              characters.first != "!",
              characters.first != "?"
        else {
            return nil
        }

        var cursor = 0
        let closing = characters[cursor] == "/"
        if closing { cursor += 1 }
        while cursor < characters.count, characters[cursor].isWhitespace { cursor += 1 }
        let nameStart = cursor
        while cursor < characters.count, characters[cursor].isASCIIHTMLNameCharacter {
            cursor += 1
        }
        guard cursor > nameStart else { return nil }
        name = String(characters[nameStart..<cursor]).lowercased()

        var selfClosing = false
        var parsed: [String: String] = [:]
        while cursor < characters.count {
            while cursor < characters.count, characters[cursor].isWhitespace { cursor += 1 }
            guard cursor < characters.count else { break }
            if characters[cursor] == "/" {
                selfClosing = true
                cursor += 1
                continue
            }
            let attributeStart = cursor
            while cursor < characters.count, characters[cursor].isASCIIHTMLAttributeNameCharacter {
                cursor += 1
            }
            guard cursor > attributeStart else {
                cursor += 1
                continue
            }
            let attributeName = String(characters[attributeStart..<cursor]).lowercased()
            while cursor < characters.count, characters[cursor].isWhitespace { cursor += 1 }
            var value = ""
            if cursor < characters.count, characters[cursor] == "=" {
                cursor += 1
                while cursor < characters.count, characters[cursor].isWhitespace { cursor += 1 }
                if cursor < characters.count, characters[cursor] == "\"" || characters[cursor] == "'" {
                    let quote = characters[cursor]
                    cursor += 1
                    let valueStart = cursor
                    while cursor < characters.count, characters[cursor] != quote { cursor += 1 }
                    value = String(characters[valueStart..<cursor])
                    if cursor < characters.count { cursor += 1 }
                } else {
                    let valueStart = cursor
                    while cursor < characters.count,
                          characters[cursor].isWhitespace == false,
                          characters[cursor] != "/"
                    {
                        cursor += 1
                    }
                    value = String(characters[valueStart..<cursor])
                }
            }
            // Match HTML's first-attribute-wins behavior and prevent a later
            // duplicate from changing a value already validated by the scanner.
            if parsed[attributeName] == nil {
                parsed[attributeName] = value
            }
        }

        attributes = parsed
        isClosing = closing
        isSelfClosing = selfClosing
    }
}

private extension Character {
    var isASCIIHTMLNameCharacter: Bool {
        isASCII && (isLetter || isNumber || self == "-" || self == ":")
    }

    var isASCIIHTMLAttributeNameCharacter: Bool {
        isASCII && isWhitespace == false && self != "=" && self != "/" && self != ">" && self != "<"
    }
}

private extension String {
    /// Rebuilds Matrix rich HTML from a strict allowlist before it reaches
    /// Foundation's HTML importer. Unknown tags keep their textual children,
    /// but executable/resource-owning containers are removed with their
    /// contents. Permitted tags are reconstructed without event handlers,
    /// CSS, remote-resource attributes, or any other unapproved attribute.
    ///
    /// The scanner is quote-aware instead of using `<[^>]+>` so an attacker
    /// cannot smuggle a second tag through a `>` inside an attribute value.
    func sanitizingMatrixHTMLForNativeImport() -> String {
        struct OpenTag {
            let name: String
            let emitted: Bool
        }

        let maximumTagNesting = 100
        let allowedTags: Set<String> = [
            "del", "h1", "h2", "h3", "h4", "h5", "h6", "blockquote", "p", "a",
            "ul", "ol", "sup", "sub", "li", "b", "i", "u", "strong", "em", "s",
            "code", "hr", "br", "div", "table", "thead", "tbody", "tr", "th", "td",
            "caption", "pre", "span", "img", "details", "summary",
        ]
        let contentDroppingTags: Set<String> = [
            "script", "style", "iframe", "object", "embed", "svg", "math", "template",
            "audio", "video", "source", "track", "canvas", "mx-reply",
        ]
        let voidTags: Set<String> = ["br", "hr", "img"]
        let characters = Array(self)
        var output = ""
        output.reserveCapacity(utf8.count)
        var index = 0
        var suppressedTag: String?
        var suppressedDepth = 0
        var openTags: [OpenTag] = []

        while index < characters.count {
            guard characters[index] == "<" else {
                if suppressedTag == nil {
                    output.append(characters[index])
                }
                index += 1
                continue
            }

            if characters[index...].starts(with: ["<", "!", "-", "-"]) {
                if let end = closingCommentIndex(in: characters, after: index + 4) {
                    index = end
                    continue
                }
                // An unterminated comment owns the remainder of the input.
                break
            }

            guard let end = htmlTagEnd(in: characters, after: index + 1) else {
                if suppressedTag == nil {
                    output.append("&lt;")
                    output.append(contentsOf: characters[(index + 1)...])
                }
                break
            }

            let raw = String(characters[(index + 1)..<end])
            index = end + 1
            guard let tag = MatrixHTMLTag(raw: raw) else {
                continue
            }

            if let currentSuppressedTag = suppressedTag {
                if tag.name == currentSuppressedTag {
                    if tag.isClosing {
                        suppressedDepth -= 1
                        if suppressedDepth == 0 {
                            suppressedTag = nil
                        }
                    } else if tag.isSelfClosing == false {
                        suppressedDepth += 1
                    }
                }
                continue
            }

            if contentDroppingTags.contains(tag.name) {
                if tag.isClosing == false, tag.isSelfClosing == false {
                    suppressedTag = tag.name
                    suppressedDepth = 1
                }
                continue
            }
            guard allowedTags.contains(tag.name) else {
                continue
            }

            if tag.name == "img" {
                guard tag.isClosing == false else { continue }
                guard openTags.last?.emitted != false, openTags.count < maximumTagNesting else {
                    continue
                }
                let fallback = tag.attributes["alt"] ?? tag.attributes["title"] ?? ""
                output.append(fallback.escapingHTMLTextAttributePayload())
                continue
            }

            if tag.isClosing {
                guard voidTags.contains(tag.name) == false else { continue }
                guard let matchingIndex = openTags.lastIndex(where: { $0.name == tag.name }) else {
                    continue
                }
                for openTag in openTags[matchingIndex...].reversed() where openTag.emitted {
                    output.append("</\(openTag.name)>")
                }
                openTags.removeSubrange(matchingIndex...)
                continue
            }

            let shouldEmit = openTags.last?.emitted != false && openTags.count < maximumTagNesting
            if voidTags.contains(tag.name) == false, tag.isSelfClosing == false {
                openTags.append(OpenTag(name: tag.name, emitted: shouldEmit))
            }
            guard shouldEmit else { continue }

            output.append("<\(tag.name)")
            switch tag.name {
            case "a":
                if let rawHref = tag.attributes["href"]?.decodingBasicHTMLEntities(),
                   rawHref.isSafeMatrixHTMLLink
                {
                    output.append(" href=\"\(rawHref.escapingHTMLAttributeValue())\"")
                }
            case "ol":
                if let rawStart = tag.attributes["start"]?.decodingBasicHTMLEntities(),
                   let start = Int(rawStart),
                   (-1_000_000 ... 1_000_000).contains(start)
                {
                    output.append(" start=\"\(start)\"")
                }
            case "code":
                if let classes = tag.attributes["class"]?.decodingBasicHTMLEntities(),
                   let languageClass = classes.split(whereSeparator: \.isWhitespace)
                    .map(String.init)
                    .first(where: { $0.isSafeMatrixLanguageClass })
                {
                    output.append(" class=\"\(languageClass.escapingHTMLAttributeValue())\"")
                }
            case "span":
                if let spoiler = tag.attributes["data-mx-spoiler"] {
                    output.append(
                        " data-mx-spoiler=\"\(spoiler.decodingBasicHTMLEntities().escapingHTMLAttributeValue())\""
                    )
                }
            default:
                break
            }
            output.append(">")

            if tag.isSelfClosing, voidTags.contains(tag.name) == false {
                output.append("</\(tag.name)>")
            }
        }

        for openTag in openTags.reversed() where openTag.emitted {
            output.append("</\(openTag.name)>")
        }

        return output
    }

    private func htmlTagEnd(in characters: [Character], after start: Int) -> Int? {
        var cursor = start
        var quote: Character?
        while cursor < characters.count {
            let character = characters[cursor]
            if let activeQuote = quote {
                if character == activeQuote {
                    quote = nil
                }
            } else if character == "\"" || character == "'" {
                quote = character
            } else if character == ">" {
                return cursor
            }
            cursor += 1
        }
        return nil
    }

    private func closingCommentIndex(in characters: [Character], after start: Int) -> Int? {
        var cursor = start
        while cursor + 2 < characters.count {
            if characters[cursor] == "-",
               characters[cursor + 1] == "-",
               characters[cursor + 2] == ">"
            {
                return cursor + 3
            }
            cursor += 1
        }
        return nil
    }

    private func escapingHTMLAttributeValue() -> String {
        replacingOccurrences(of: "&", with: "&amp;")
            .replacingOccurrences(of: "\"", with: "&quot;")
            .replacingOccurrences(of: "<", with: "&lt;")
            .replacingOccurrences(of: ">", with: "&gt;")
    }

    private func escapingHTMLTextAttributePayload() -> String {
        replacingOccurrences(of: "<", with: "&lt;")
            .replacingOccurrences(of: ">", with: "&gt;")
    }

    private var isSafeMatrixLanguageClass: Bool {
        guard lowercased().hasPrefix("language-"), count > "language-".count, count <= 41 else {
            return false
        }
        return dropFirst("language-".count).unicodeScalars.allSatisfy {
            CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "_+.-")).contains($0)
        }
    }

    func trimmingLeadingCharacters(in set: CharacterSet) -> String {
        var output = self
        while let scalar = output.unicodeScalars.first, set.contains(scalar) {
            output.removeFirst()
        }
        return output
    }

    func trimmingTrailingCharacters(in set: CharacterSet) -> String {
        var output = self
        while let scalar = output.unicodeScalars.last, set.contains(scalar) {
            output.removeLast()
        }
        return output
    }

    func removingHTMLBlocks(named tagName: String) -> String {
        replacingHTMLPattern(#"<\#(tagName)(?:\s+[^>]*)?>[\s\S]*?</\#(tagName)\s*>"#, with: "")
    }

    func replacingTag(_ tagName: String, with marker: String) -> String {
        replacingHTMLPattern(#"<\#(tagName)(?:\s+[^>]*)?>"#, with: marker)
            .replacingHTMLPattern(#"</\#(tagName)\s*>"#, with: marker)
    }

    func replacingHeadingTags() -> String {
        var output = self
        for level in 1 ... 6 {
            output = output
                .replacingHTMLPattern(#"<h\#(level)(?:\s+[^>]*)?>"#, with: "\n\n**")
                .replacingHTMLPattern(#"</h\#(level)\s*>"#, with: "**\n\n")
        }
        return output
    }

    func replacingAnchorTags() -> String {
        let pattern = #"<a\s+[^>]*href\s*=\s*['"]([^'"]+)['"][^>]*>([\s\S]*?)</a\s*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return self
        }

        let nsRange = NSRange(startIndex ..< endIndex, in: self)
        let matches = regex.matches(in: self, range: nsRange).reversed()
        let source = self as NSString
        let output = NSMutableString(string: self)

        for match in matches {
            guard match.numberOfRanges == 3 else {
                continue
            }

            let href = source.substring(with: match.range(at: 1)).decodingBasicHTMLEntities()
            let label = source.substring(with: match.range(at: 2))
                .replacingHTMLPattern(#"</?[^>]+>"#, with: "")
                .decodingBasicHTMLEntities()
            if href.isSafeMatrixHTMLLink {
                output.replaceCharacters(in: match.range(at: 0), with: "[\(label)](\(href))")
            } else {
                output.replaceCharacters(in: match.range(at: 0), with: label)
            }
        }

        return output as String
    }

    /// Native attributed-string HTML import may resolve image resources. Matrix
    /// inline images are represented by their accessible `alt` text instead so
    /// timeline rendering remains deterministic and performs no network I/O.
    func replacingImageTagsWithAltText() -> String {
        let pattern = #"<img\b[^>]*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return self
        }

        let nsRange = NSRange(startIndex ..< endIndex, in: self)
        let source = self as NSString
        let output = NSMutableString(string: self)
        for match in regex.matches(in: self, range: nsRange).reversed() {
            let tag = source.substring(with: match.range(at: 0))
            let alt = tag.firstQuotedHTMLAttribute(named: "alt") ?? ""
            output.replaceCharacters(in: match.range(at: 0), with: alt)
        }
        return output as String
    }

    func replacingHTMLListsForRichText() -> String {
        // Resolve innermost lists first. A single non-greedy regex pairs an
        // outer opening tag with the first nested closing tag and corrupts
        // valid nested Markdown output.
        let pattern = #"<(ol|ul)\b([^>]*)>((?:(?!<(?:ol|ul)\b|</(?:ol|ul)\s*>)[\s\S])*)</\1\s*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return self
        }

        var value = self
        for _ in 0 ..< 128 {
            let range = NSRange(value.startIndex ..< value.endIndex, in: value)
            let matches = regex.matches(in: value, range: range)
            guard matches.isEmpty == false else { break }
            let source = value as NSString
            let output = NSMutableString(string: value)
            var changed = false
            for match in matches.reversed() {
                guard match.numberOfRanges == 4 else { continue }
                let tagName = source.substring(with: match.range(at: 1)).lowercased()
                let attributes = source.substring(with: match.range(at: 2))
                let items = source.substring(with: match.range(at: 3)).htmlListItems()
                guard items.isEmpty == false else { continue }
                let start = tagName == "ol"
                    ? (attributes.firstQuotedHTMLAttribute(named: "start").flatMap(Int.init) ?? 1)
                    : 1
                var replacement = items.enumerated().map { offset, item in
                    let marker = tagName == "ol" ? "\(start + offset). " : "• "
                    return marker + item
                }.joined(separator: "<br>")
                let before = source.substring(to: match.range(at: 0).location)
                if before.hasUnclosedHTMLListItem {
                    replacement = "<br>" + replacement
                }
                output.replaceCharacters(in: match.range(at: 0), with: replacement)
                changed = true
            }
            guard changed else { break }
            value = output as String
        }
        return value
    }

    func firstQuotedHTMLAttribute(named name: String) -> String? {
        let pattern = #"\b\#(name)\s*=\s*([\"'])([\s\S]*?)\1"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]),
              let match = regex.firstMatch(
                  in: self,
                  range: NSRange(startIndex ..< endIndex, in: self)
              ),
              match.numberOfRanges == 3,
              let range = Range(match.range(at: 2), in: self)
        else {
            return nil
        }
        return String(self[range])
    }

    private var hasUnclosedHTMLListItem: Bool {
        let open = range(of: #"<li(?:\s+[^>]*)?>"#, options: [.regularExpression, .caseInsensitive, .backwards])
        let close = range(of: #"</li\s*>"#, options: [.regularExpression, .caseInsensitive, .backwards])
        guard let open else { return false }
        guard let close else { return true }
        return open.lowerBound > close.lowerBound
    }

    func replacingOrderedLists() -> String {
        replacingList(named: "ol") { index, item in
            "\(index + 1). \(item)"
        }
    }

    func replacingUnorderedLists() -> String {
        replacingList(named: "ul") { _, item in
            "- \(item)"
        }
    }

    func replacingTables() -> String {
        let pattern = #"<table(?:\s+[^>]*)?>([\s\S]*?)</table\s*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return self
        }

        let nsRange = NSRange(startIndex ..< endIndex, in: self)
        let matches = regex.matches(in: self, range: nsRange).reversed()
        let source = self as NSString
        let output = NSMutableString(string: self)

        for match in matches {
            guard match.numberOfRanges == 2 else {
                continue
            }

            let tableHTML = source.substring(with: match.range(at: 1))
            let rows = tableHTML.htmlTableRows()
            guard rows.isEmpty == false else {
                continue
            }

            var markdownRows: [String] = []
            for (index, row) in rows.enumerated() {
                markdownRows.append("| \(row.cells.joined(separator: " | ")) |")
                if index == 0, row.isHeader {
                    markdownRows.append("| \(Array(repeating: "---", count: row.cells.count).joined(separator: " | ")) |")
                }
            }

            output.replaceCharacters(in: match.range(at: 0), with: "\n\(markdownRows.joined(separator: "\n"))\n")
        }

        return output as String
    }

    private func replacingList(named tagName: String, itemPrefix: (Int, String) -> String) -> String {
        let pattern = #"<\#(tagName)(?:\s+[^>]*)?>([\s\S]*?)</\#(tagName)\s*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return self
        }

        let nsRange = NSRange(startIndex ..< endIndex, in: self)
        let matches = regex.matches(in: self, range: nsRange).reversed()
        let source = self as NSString
        let output = NSMutableString(string: self)

        for match in matches {
            guard match.numberOfRanges == 2 else {
                continue
            }

            let listHTML = source.substring(with: match.range(at: 1))
            let items = listHTML.htmlListItems()
            guard items.isEmpty == false else {
                continue
            }

            let markdown = items.enumerated()
                .map { itemPrefix($0.offset, $0.element) }
                .joined(separator: "\n")
            output.replaceCharacters(in: match.range(at: 0), with: "\n\(markdown)\n")
        }

        return output as String
    }

    func replacingPreformattedBlocks() -> String {
        let pattern = #"<pre(?:\s+[^>]*)?>\s*(?:<code(?:\s+[^>]*)?>)?([\s\S]*?)(?:</code\s*>)?\s*</pre\s*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return self
        }

        let nsRange = NSRange(startIndex ..< endIndex, in: self)
        let matches = regex.matches(in: self, range: nsRange).reversed()
        let source = self as NSString
        let output = NSMutableString(string: self)

        for match in matches {
            guard match.numberOfRanges == 2 else {
                continue
            }
            let code = source.substring(with: match.range(at: 1))
                .decodingBasicHTMLEntities()
                .trimmingCharacters(in: .whitespacesAndNewlines)
            output.replaceCharacters(in: match.range(at: 0), with: "\n```\n\(code)\n```\n")
        }

        return output as String
    }

    func htmlListItems() -> [String] {
        htmlCaptures(pattern: #"<li(?:\s+[^>]*)?>([\s\S]*?)</li\s*>"#)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { $0.isEmpty == false }
    }

    func htmlTableRows() -> [(cells: [String], isHeader: Bool)] {
        htmlTableRowFragments().map { row in
            (
                cells: row.cells.map { $0.html.strippingHTMLTagsAndDecoding() },
                isHeader: row.isHeader
            )
        }
    }

    func htmlTableRowFragments() -> [(cells: [(html: String, isHeader: Bool)], isHeader: Bool)] {
        htmlCaptures(pattern: #"<tr(?:\s+[^>]*)?>([\s\S]*?)</tr\s*>"#)
            .compactMap { rowHTML in
                guard let regex = try? NSRegularExpression(
                    pattern: #"<(th|td)(?:\s+[^>]*)?>([\s\S]*?)</\1\s*>"#,
                    options: [.caseInsensitive]
                ) else {
                    return nil
                }
                let range = NSRange(rowHTML.startIndex ..< rowHTML.endIndex, in: rowHTML)
                let source = rowHTML as NSString
                let parsed = regex.matches(in: rowHTML, range: range).compactMap { match -> (html: String, isHeader: Bool)? in
                    guard match.numberOfRanges == 3 else { return nil }
                    return (
                        source.substring(with: match.range(at: 2)),
                        source.substring(with: match.range(at: 1)).lowercased() == "th"
                    )
                }
                guard parsed.isEmpty == false else { return nil }
                return (parsed, parsed.allSatisfy { $0.isHeader })
            }
    }

    func htmlCaptures(pattern: String) -> [String] {
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return []
        }

        let nsRange = NSRange(startIndex ..< endIndex, in: self)
        let source = self as NSString
        return regex.matches(in: self, range: nsRange).compactMap { match in
            guard match.numberOfRanges == 2 else {
                return nil
            }
            return source.substring(with: match.range(at: 1))
        }
    }

    func replacingHTMLPattern(_ pattern: String, with replacement: String) -> String {
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return self
        }
        let nsRange = NSRange(startIndex ..< endIndex, in: self)
        return regex.stringByReplacingMatches(in: self, range: nsRange, withTemplate: replacement)
    }

    func strippingHTMLTagsAndDecoding() -> String {
        replacingHTMLPattern(#"</?[^>]+>"#, with: "")
            .decodingBasicHTMLEntities()
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    func decodingBasicHTMLEntities() -> String {
        replacingOccurrences(of: "&amp;", with: "&")
            .replacingOccurrences(of: "&lt;", with: "<")
            .replacingOccurrences(of: "&gt;", with: ">")
            .replacingOccurrences(of: "&quot;", with: "\"")
            .replacingOccurrences(of: "&#39;", with: "'")
            .replacingOccurrences(of: "&apos;", with: "'")
            .decodingNumericHTMLEntities()
    }

    func decodingNumericHTMLEntities() -> String {
        let pattern = #"&#(x?[0-9A-Fa-f]+);"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else {
            return self
        }

        let nsRange = NSRange(startIndex ..< endIndex, in: self)
        let matches = regex.matches(in: self, range: nsRange).reversed()
        let source = self as NSString
        let output = NSMutableString(string: self)

        for match in matches {
            guard match.numberOfRanges == 2 else {
                continue
            }

            let rawValue = source.substring(with: match.range(at: 1))
            let radix = rawValue.hasPrefix("x") || rawValue.hasPrefix("X") ? 16 : 10
            let digits = radix == 16 ? String(rawValue.dropFirst()) : rawValue
            guard let scalarValue = UInt32(digits, radix: radix),
                  let scalar = UnicodeScalar(scalarValue)
            else {
                continue
            }

            output.replaceCharacters(in: match.range(at: 0), with: String(Character(scalar)))
        }

        return output as String
    }

    var isSafeMatrixHTMLLink: Bool {
        guard let components = URLComponents(string: self),
              let scheme = components.scheme?.lowercased()
        else {
            return false
        }

        return ["https", "http", "ftp", "mailto", "magnet"].contains(scheme)
    }
}
