import Foundation

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

    static func pendingItems(from items: [TimelineItem]) -> [TimelineItem] {
        items.filter(\.isLocalPending)
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
    struct DetailsBlock: Equatable {
        let summary: String
        let code: String?
        let body: String
    }

    enum Segment: Equatable {
        case markdown(String)
        case code(String)
        case quote(String)
        case details(DetailsBlock)
    }

    static func attributedString(body: String, html: String) -> AttributedString {
        let markdown = sanitizedMarkdown(body: body, html: html)
        if let attributed = try? AttributedString(
            markdown: markdown,
            options: AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        ) {
            return attributed
        }

        return AttributedString(body)
    }

    static func segments(body: String, html: String) -> [Segment] {
        let sanitized = html
            .removingHTMLBlocks(named: "script")
            .removingHTMLBlocks(named: "style")
        let pattern = #"<details(?:\s+[^>]*)?>[\s\S]*?</details\s*>|<pre(?:\s+[^>]*)?>[\s\S]*?</pre\s*>|<blockquote(?:\s+[^>]*)?>[\s\S]*?</blockquote\s*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            let markdown = sanitizedMarkdown(body: body, html: html)
            return markdown.isEmpty ? [] : [.markdown(markdown)]
        }

        let nsRange = NSRange(sanitized.startIndex ..< sanitized.endIndex, in: sanitized)
        let matches = regex.matches(in: sanitized, range: nsRange)
        guard matches.isEmpty == false else {
            let markdown = sanitizedMarkdown(body: body, html: html)
            return markdown.isEmpty ? [] : [.markdown(markdown)]
        }

        var segments: [Segment] = []
        var cursor = sanitized.startIndex

        for match in matches {
            guard let range = Range(match.range(at: 0), in: sanitized) else {
                continue
            }

            appendMarkdownSegment(from: String(sanitized[cursor ..< range.lowerBound]), to: &segments)

            let blockHTML = String(sanitized[range])
            if blockHTML.range(of: #"^\s*<details"#, options: [.regularExpression, .caseInsensitive]) != nil {
                if let block = detailsBlocks(html: blockHTML).first {
                    segments.append(.details(block))
                }
            } else if blockHTML.range(of: #"^\s*<blockquote"#, options: [.regularExpression, .caseInsensitive]) != nil,
                      let quote = quoteBlock(html: blockHTML)
            {
                segments.append(.quote(quote))
            } else if let code = codeBlock(html: blockHTML) {
                segments.append(.code(code))
            }

            cursor = range.upperBound
        }

        appendMarkdownSegment(from: String(sanitized[cursor...]), to: &segments)

        if segments.isEmpty {
            let markdown = sanitizedMarkdown(body: body, html: html)
            return markdown.isEmpty ? [] : [.markdown(markdown)]
        }
        return segments
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

            let code = firstHTMLCapture(
                in: content,
                pattern: #"<pre(?:\s+[^>]*)?>\s*<code(?:\s+[^>]*)?>([\s\S]*?)</code\s*>\s*</pre\s*>"#
            )?.decodingBasicHTMLEntities()
                .trimmingCharacters(in: .whitespacesAndNewlines)

            let body = content
                .replacingHTMLPattern(#"<summary(?:\s+[^>]*)?>[\s\S]*?</summary\s*>"#, with: "")
                .replacingHTMLPattern(#"<pre(?:\s+[^>]*)?>[\s\S]*?</pre\s*>"#, with: "")
                .strippingHTMLTagsAndDecoding()
                .trimmingCharacters(in: .whitespacesAndNewlines)

            return DetailsBlock(summary: summary, code: code?.isEmpty == false ? code : nil, body: body)
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
        output = output.replacingHTMLPattern(#"<br\s*/?>"#, with: "\n")
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

    private static func appendMarkdownSegment(from html: String, to segments: inout [Segment]) {
        let markdown = sanitizedMarkdown(body: "", html: html)
        guard markdown.isEmpty == false else {
            return
        }
        segments.append(.markdown(markdown))
    }

    private static func codeBlock(html: String) -> String? {
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
            .trimmingCharacters(in: .whitespacesAndNewlines)

        return code?.isEmpty == false ? code : nil
    }

    private static func quoteBlock(html: String) -> String? {
        let rawQuote = firstHTMLCapture(
            in: html,
            pattern: #"<blockquote(?:\s+[^>]*)?>([\s\S]*?)</blockquote\s*>"#
        )
        let quote = rawQuote.map { sanitizedMarkdown(body: "", html: $0) }
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }

        return quote?.isEmpty == false ? quote : nil
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

enum MatrixDisplayMarkdown {
    static func normalize(_ markdown: String) -> String {
        let normalizedNewlines = markdown
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\r", with: "\n")

        var lines = normalizedNewlines
            .components(separatedBy: "\n")
            .map { $0.trimmingCharacters(in: .whitespaces) }

        while lines.first?.isEmpty == true {
            lines.removeFirst()
        }
        while lines.last?.isEmpty == true {
            lines.removeLast()
        }

        var output: [String] = []
        var sawBlankLine = false

        for line in lines {
            if line.isEmpty {
                sawBlankLine = output.isEmpty == false
                continue
            }

            if sawBlankLine,
               let previous = output.last,
               isListLine(line) == false,
               isListLine(previous) == false,
               isDivider(line) == false,
               isDivider(previous) == false
            {
                output.append("")
            }

            output.append(line)
            sawBlankLine = false
        }

        return output.joined(separator: "\n")
    }

    private static func isListLine(_ line: String) -> Bool {
        if line.hasPrefix("- ") || line.hasPrefix("* ") || line.hasPrefix("+ ") {
            return true
        }

        guard let dotIndex = line.firstIndex(of: ".") else {
            return false
        }
        let prefix = line[..<dotIndex]
        let suffix = line[line.index(after: dotIndex)...]
        return prefix.isEmpty == false
            && prefix.allSatisfy(\.isNumber)
            && suffix.first == " "
    }

    private static func isDivider(_ line: String) -> Bool {
        line == "---" || line == "***" || line == "___"
    }
}

private extension String {
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
        htmlCaptures(pattern: #"<tr(?:\s+[^>]*)?>([\s\S]*?)</tr\s*>"#)
            .compactMap { rowHTML in
                let headerCells = rowHTML.htmlCaptures(pattern: #"<th(?:\s+[^>]*)?>([\s\S]*?)</th\s*>"#)
                let dataCells = rowHTML.htmlCaptures(pattern: #"<td(?:\s+[^>]*)?>([\s\S]*?)</td\s*>"#)
                let cells = headerCells.isEmpty ? dataCells : headerCells
                let trimmedCells = cells
                    .map { $0.replacingHTMLPattern(#"</?[^>]+>"#, with: "").decodingBasicHTMLEntities().trimmingCharacters(in: .whitespacesAndNewlines) }
                    .filter { $0.isEmpty == false }
                return trimmedCells.isEmpty ? nil : (trimmedCells, headerCells.isEmpty == false)
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

        return ["https", "http", "matrix"].contains(scheme)
    }
}
