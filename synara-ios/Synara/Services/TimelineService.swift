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

enum TimelineMessageGroupingPolicy {
    static let maximumInterval: TimeInterval = 2 * 60 * 60

    static func shouldGroup(previous: TimelineItem?, current: TimelineItem) -> Bool {
        guard let previous, previous.senderID == current.senderID else {
            return false
        }
        let elapsed = current.timestamp.timeIntervalSince(previous.timestamp)
        return elapsed >= 0
            && elapsed < maximumInterval
            && Calendar.current.isDate(previous.timestamp, inSameDayAs: current.timestamp)
    }
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

struct TimelineLoadFailure: Equatable {
    enum Kind: Equatable {
        case sessionUnavailable
        case roomUnavailable
        case viewUnavailable
        case temporarilyUnavailable
    }

    let kind: Kind
    let diagnosticCode: String

    init(kind: Kind, diagnosticCode: String) {
        self.kind = kind
        self.diagnosticCode = Self.privacySafeDiagnosticCode(diagnosticCode)
    }

    private static func privacySafeDiagnosticCode(_ candidate: String) -> String {
        guard candidate.isEmpty == false,
              candidate.utf8.count <= 80,
              candidate.utf8.allSatisfy({ byte in
                  switch byte {
                  case 45, 46, 48 ... 57, 65 ... 90, 95, 97 ... 122:
                      return true
                  default:
                      return false
                  }
              })
        else {
            return "timeline-invalid-diagnostic-code"
        }
        return candidate
    }

    var userMessage: String {
        switch kind {
        case .sessionUnavailable:
            return "Sign in again to load this timeline."
        case .roomUnavailable:
            return "This room is not available."
        case .viewUnavailable, .temporarilyUnavailable:
            return "Messages are temporarily unavailable. Try again."
        }
    }
}

enum TimelineLoadOutcome: Equatable {
    case loaded([TimelineItem])
    /// The owning timeline opened successfully and backward pagination proved
    /// there are no displayable events before the start of the room history.
    case empty
    case failed(TimelineLoadFailure)
}

struct RoomTimelineAvailabilityState: Equatable {
    private(set) var failure: TimelineLoadFailure?

    mutating func recordSuccess() {
        failure = nil
    }

    mutating func recordFailure(_ failure: TimelineLoadFailure, preservingRows: Bool) {
        self.failure = preservingRows ? failure : nil
    }
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
    case failed(TimelineLoadFailure)
    case superseded
}

actor RoomTimelineSession {
    private enum UpdateAcceptance {
        case accepted(TimelineLoadOutcome)
        case unchanged
        case superseded
    }

    private let roomID: String
    private let service: TimelineServicing
    private var generation: UInt64 = 0
    private var mode: RoomTimelineMode = .live
    private var serverItems: [TimelineItem] = []
    private var updateFailureOutstanding = false

    init(roomID: String, service: TimelineServicing) {
        self.roomID = roomID
        self.service = service
    }

    func open(mode: RoomTimelineMode) async -> RoomTimelineSessionFeed? {
        generation &+= 1
        let requestedGeneration = generation
        self.mode = mode
        serverItems = []
        updateFailureOutstanding = false

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
            updateFailureOutstanding = false
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
            updateFailureOutstanding = true
            return .empty
        case let .failed(failure):
            updateFailureOutstanding = true
            return .failed(failure)
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
            updateFailureOutstanding = false
            return .loaded(serverItems)
        case .loaded, .empty:
            updateFailureOutstanding = false
            return .empty
        case let .failed(failure):
            updateFailureOutstanding = true
            return .failed(failure)
        }
    }

    func invalidate() {
        generation &+= 1
        serverItems = []
        updateFailureOutstanding = false
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
        case let .failed(failure):
            return .failed(failure)
        }
    }

    private func acceptedUpdate(
        _ outcome: TimelineLoadOutcome,
        generation requestedGeneration: UInt64
    ) -> UpdateAcceptance {
        guard requestedGeneration == generation else {
            return .superseded
        }
        switch outcome {
        case let .loaded(items):
            let nextItems = TimelineWindowPolicy.replacingServerWindow(items)
            let isRecoveryHeartbeat = updateFailureOutstanding
            updateFailureOutstanding = false
            guard nextItems != serverItems else {
                return isRecoveryHeartbeat
                    ? .accepted(nextItems.isEmpty ? .empty : .loaded(nextItems))
                    : .unchanged
            }
            serverItems = nextItems
            return .accepted(nextItems.isEmpty ? .empty : .loaded(nextItems))
        case .empty:
            updateFailureOutstanding = false
            return .accepted(.empty)
        case let .failed(failure):
            updateFailureOutstanding = true
            return .accepted(.failed(failure))
        }
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
                    let acceptance = await self.acceptedUpdate(
                        outcome,
                        generation: requestedGeneration
                    )
                    switch acceptance {
                    case let .accepted(accepted):
                        continuation.yield(accepted)
                    case .unchanged:
                        continue
                    case .superseded:
                        continuation.finish()
                        return
                    }
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
        expandedLineCount: Int = 180,
        usesFormattedHTML: Bool = false
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
            let kind: TimelineItem.Kind = usesFormattedHTML
                ? .formattedText(
                    body: "\(body) & live ©",
                    html: "<p>\(body) &amp; <strong>live</strong> &copy;</p>"
                )
                : .text(body)
            let item = TimelineItem(
                id: "$synthetic-\(index):matrix.org",
                eventID: "$synthetic-\(index):matrix.org",
                senderID: index % 2 == 0 ? "@alice:matrix.org" : "@bob:matrix.org",
                timestamp: baseDate.addingTimeInterval(TimeInterval(index)),
                kind: kind,
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
    var updateIntervalNanoseconds: UInt64 = 0
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
                    if updateIntervalNanoseconds > 0 {
                        try? await Task.sleep(nanoseconds: updateIntervalNanoseconds)
                    }
                    guard Task.isCancelled == false else {
                        break
                    }
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
    private indirect enum HTMLNode {
        case text(String)
        case element(name: String, attributes: [String: String], children: [HTMLNode])
    }

    private final class HTMLNodeBuilder {
        let name: String
        let attributes: [String: String]
        var children: [HTMLNode] = []

        init(name: String, attributes: [String: String] = [:]) {
            self.name = name
            self.attributes = attributes
        }

        func frozen() -> HTMLNode {
            .element(name: name, attributes: attributes, children: children)
        }
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
            let rawValue: UInt16

            static let bold = Style(rawValue: 1 << 0)
            static let italic = Style(rawValue: 1 << 1)
            static let strikethrough = Style(rawValue: 1 << 2)
            static let underline = Style(rawValue: 1 << 3)
            static let code = Style(rawValue: 1 << 4)
            static let superscript = Style(rawValue: 1 << 5)
            static let subscriptText = Style(rawValue: 1 << 6)
            static let heading1 = Style(rawValue: 1 << 7)
            static let heading2 = Style(rawValue: 1 << 8)
            static let heading3 = Style(rawValue: 1 << 9)
            static let heading4 = Style(rawValue: 1 << 10)
            static let heading5 = Style(rawValue: 1 << 11)
            static let heading6 = Style(rawValue: 1 << 12)

            static func heading(_ level: Int) -> Style {
                switch level {
                case 1: return .heading1
                case 2: return .heading2
                case 3: return .heading3
                case 4: return .heading4
                case 5: return .heading5
                default: return .heading6
                }
            }
        }

        struct Run: Equatable {
            let text: String
            let style: Style
            let link: URL?
            let foregroundColorHex: String?
            let backgroundColorHex: String?

            init(
                text: String,
                style: Style,
                link: URL?,
                foregroundColorHex: String? = nil,
                backgroundColorHex: String? = nil
            ) {
                self.text = text
                self.style = style
                self.link = link
                self.foregroundColorHex = foregroundColorHex
                self.backgroundColorHex = backgroundColorHex
            }
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

    struct HeadingBlock: Equatable {
        let level: Int
        let content: RichText
    }

    struct DetailsBlock: Equatable {
        let summaryContent: RichText
        let content: [Segment]

        var summary: String { summaryContent.plainText }

        // Compatibility projections for callers that need a compact preview.
        // The timeline renders `content` and never uses these lossy views.
        var code: CodeBlock? {
            content.lazy.compactMap { segment in
                if case let .code(block) = segment { return block }
                return nil
            }.first
        }

        var body: String {
            content.compactMap(\.plainTextExcludingCode)
                .filter { $0.isEmpty == false }
                .joined(separator: "\n")
                .trimmingCharacters(in: .whitespacesAndNewlines)
        }
    }

    struct SpoilerBlock: Equatable {
        let content: RichText
        let reason: String?
    }

    enum InlinePiece: Equatable {
        case richText(RichText)
        case spoiler(SpoilerBlock)
    }

    struct InlineGroup: Equatable {
        let pieces: [InlinePiece]
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

    indirect enum Segment: Equatable {
        case richText(RichText)
        case inline(InlineGroup)
        case heading(HeadingBlock)
        case code(CodeBlock)
        case quote(RichText)
        case spoiler(SpoilerBlock)
        case details(DetailsBlock)
        case table(TableBlock)

        fileprivate var plainTextExcludingCode: String? {
            switch self {
            case let .richText(text), let .quote(text):
                return text.plainText
            case let .heading(block):
                return block.content.plainText
            case let .inline(group):
                return group.pieces.map { piece in
                    switch piece {
                    case let .richText(text): text.plainText
                    case let .spoiler(block): block.content.plainText
                    }
                }.joined()
            case .code:
                return nil
            case let .spoiler(block):
                return block.content.plainText
            case let .details(block):
                return [block.summary, block.body].filter { $0.isEmpty == false }.joined(separator: "\n")
            case let .table(block):
                return block.rows.map { $0.cells.map(\.plainText).joined(separator: "\t") }.joined(separator: "\n")
            }
        }

        fileprivate var isInlinePresentation: Bool {
            switch self {
            case .richText, .inline, .spoiler: return true
            default: return false
            }
        }
    }

    static func segments(body: String, html: String) -> [Segment] {
        guard let nodes = parsedNodes(html: html) else {
            let fallback = fallbackRichText(body)
            return fallback.runs.isEmpty ? [] : [.richText(fallback)]
        }
        var segments: [Segment] = []
        appendSegments(nodes, to: &segments)

        if segments.isEmpty {
            let text = richText(nodes: nodes)
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
        let cacheKey = "\(body)\u{0}\(html)" as NSString
        if let cached = richTextCache.object(forKey: cacheKey) {
            return cached.value
        }
        guard let nodes = parsedNodes(html: html) else {
            let fallback = fallbackRichText(body)
            cacheRichText(fallback, forKey: cacheKey, body: body, html: html)
            return fallback
        }
        let rendered = richText(nodes: nodes)
        let result = rendered.runs.isEmpty ? fallbackRichText(body) : rendered
        cacheRichText(result, forKey: cacheKey, body: body, html: html)
        return result
    }

    private static func parsedNodes(html: String) -> [HTMLNode]? {
        guard html.utf8.count <= maximumRichHTMLBytes else { return nil }
        let sanitized = html.sanitizingMatrixHTMLForNativeImport()
        let characters = Array(sanitized)
        let root = HTMLNodeBuilder(name: "__root__")
        var stack = [root]
        var index = 0
        var textStart = 0

        func appendText(until end: Int) {
            guard end > textStart else { return }
            stack.last?.children.append(.text(String(characters[textStart ..< end])))
        }

        while index < characters.count {
            guard characters[index] == "<",
                  let end = sanitized.htmlTagEnd(in: characters, after: index + 1)
            else {
                index += 1
                continue
            }
            appendText(until: index)
            let raw = String(characters[(index + 1) ..< end])
            index = end + 1
            textStart = index
            guard let tag = MatrixHTMLTag(raw: raw) else { continue }

            if tag.isClosing {
                guard let matching = stack.lastIndex(where: { $0.name == tag.name }), matching > 0 else {
                    continue
                }
                while stack.count - 1 >= matching {
                    let completed = stack.removeLast()
                    stack.last?.children.append(completed.frozen())
                }
                continue
            }

            let node = HTMLNodeBuilder(name: tag.name, attributes: tag.attributes)
            if tag.isSelfClosing || ["br", "hr", "img"].contains(tag.name) {
                stack.last?.children.append(node.frozen())
            } else {
                // The sanitizer enforces the same bound. Keep the parser's
                // ownership explicit so a future sanitizer change cannot
                // reintroduce an unbounded recursive tree.
                guard stack.count <= 100 else { return nil }
                stack.append(node)
            }
        }
        appendText(until: characters.count)
        while stack.count > 1 {
            let completed = stack.removeLast()
            stack.last?.children.append(completed.frozen())
        }
        return root.children
    }

    private static func appendSegments(
        _ nodes: [HTMLNode],
        style: RichText.Style = [],
        link: URL? = nil,
        to output: inout [Segment]
    ) {
        var inlineNodes: [HTMLNode] = []
        var emittedInlineInScope = false
        func flushInline(preserveTrailingEdge: Bool = false) {
            let containsBlock = inlineNodes.contains { node in
                guard case let .element(name, _, _) = node else { return false }
                return ["p", "div", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "hr"].contains(name)
            }
            let text = richText(
                nodes: inlineNodes,
                style: style,
                link: link,
                trimLeadingEdge: emittedInlineInScope == false || containsBlock,
                trimTrailingEdge: preserveTrailingEdge == false,
                assumesVisiblePredecessor: emittedInlineInScope && containsBlock == false
            )
            if text.runs.isEmpty == false {
                appendInlinePiece(
                    .richText(text),
                    mergeWithPrevious: emittedInlineInScope && containsBlock == false,
                    to: &output
                )
                emittedInlineInScope = containsBlock == false
            }
            inlineNodes.removeAll(keepingCapacity: true)
        }

        for node in nodes {
            guard case let .element(name, attributes, children) = node else {
                inlineNodes.append(node)
                continue
            }
            let isSpoiler = name == "span" && attributes["data-mx-spoiler"] != nil
            if ["h1", "h2", "h3", "h4", "h5", "h6"].contains(name) {
                flushInline()
                let level = Int(name.dropFirst()) ?? 6
                let content = richText(
                    nodes: children,
                    style: style.union(.bold).union(.heading(level)),
                    link: link
                )
                if content.runs.isEmpty == false {
                    output.append(.heading(.init(level: level, content: content)))
                }
                emittedInlineInScope = false
            } else if name == "pre" {
                flushInline()
                if let block = codeBlock(node: node) { output.append(.code(block)) }
            } else if name == "blockquote" {
                flushInline()
                let quote = richText(nodes: children)
                if quote.runs.isEmpty == false { output.append(.quote(quote)) }
            } else if name == "table" {
                flushInline()
                if let block = tableBlock(node: node) {
                    output.append(.table(block))
                } else {
                    appendReadableFallback(
                        children,
                        style: style,
                        link: link,
                        to: &output
                    )
                }
            } else if name == "details" {
                flushInline()
                if let block = detailsBlock(node: node) {
                    output.append(.details(block))
                } else {
                    appendReadableFallback(
                        children,
                        style: style,
                        link: link,
                        to: &output
                    )
                }
            } else if isSpoiler {
                // Whitespace immediately before an inline spoiler separates
                // two visible siblings and must not be mistaken for message-
                // edge whitespace.
                flushInline(preserveTrailingEdge: true)
                if let block = spoilerBlock(node: node, style: style, link: link) {
                    appendInlinePiece(.spoiler(block), mergeWithPrevious: emittedInlineInScope, to: &output)
                    emittedInlineInScope = true
                }
            } else if containsSegmentBoundary(children) {
                // A semantic block can legally be nested inside a permitted
                // container. Walk the actual tree instead of pairing tags by
                // regex, while retaining the exact sibling order.
                flushInline()
                let nestedStyle: RichText.Style
                switch name {
                case "strong", "b":
                    nestedStyle = style.union(.bold)
                case "h1", "h2", "h3", "h4", "h5", "h6":
                    let level = Int(name.dropFirst()) ?? 6
                    nestedStyle = style.union(.bold).union(.heading(level))
                case "em", "i": nestedStyle = style.union(.italic)
                case "del", "s": nestedStyle = style.union(.strikethrough)
                case "u": nestedStyle = style.union(.underline)
                case "code": nestedStyle = style.union(.code)
                case "sup": nestedStyle = style.subtracting(.subscriptText).union(.superscript)
                case "sub": nestedStyle = style.subtracting(.superscript).union(.subscriptText)
                default: nestedStyle = style
                }
                let nestedLink: URL?
                if name == "a", let candidate = attributes["href"]?.decodingBasicHTMLEntities(),
                   candidate.isSafeMatrixHTMLLink
                {
                    nestedLink = URL(string: candidate)
                } else {
                    nestedLink = link
                }
                appendSegments(children, style: nestedStyle, link: nestedLink, to: &output)
                emittedInlineInScope = false
            } else {
                inlineNodes.append(node)
            }
        }
        flushInline()
    }

    /// A malformed semantic container must not make otherwise safe message
    /// content disappear. Preserve any nested semantic blocks when possible,
    /// then fall back to the typed rich-text projection of its descendants.
    private static func appendReadableFallback(
        _ children: [HTMLNode],
        style: RichText.Style,
        link: URL?,
        to output: inout [Segment]
    ) {
        let start = output.count
        appendSegments(children, style: style, link: link, to: &output)
        guard output.count == start else { return }

        let text = richText(nodes: children, style: style, link: link)
        if text.runs.isEmpty == false {
            output.append(.richText(text))
        }
    }

    private static func appendInlinePiece(
        _ piece: InlinePiece,
        mergeWithPrevious: Bool,
        to output: inout [Segment]
    ) {
        guard mergeWithPrevious else {
            switch piece {
            case let .richText(text): output.append(.richText(text))
            case let .spoiler(block): output.append(.spoiler(block))
            }
            return
        }
        switch output.last {
        case let .inline(group):
            output[output.count - 1] = .inline(.init(pieces: group.pieces + [piece]))
        case let .richText(text):
            output[output.count - 1] = .inline(.init(pieces: [.richText(text), piece]))
        case let .spoiler(block):
            output[output.count - 1] = .inline(.init(pieces: [.spoiler(block), piece]))
        default:
            switch piece {
            case let .richText(text): output.append(.richText(text))
            case let .spoiler(block): output.append(.spoiler(block))
            }
        }
    }

    private static func containsSegmentBoundary(_ nodes: [HTMLNode]) -> Bool {
        nodes.contains { node in
            guard case let .element(name, attributes, children) = node else { return false }
            if ["pre", "blockquote", "table", "details"].contains(name) { return true }
            if name == "span", attributes["data-mx-spoiler"] != nil { return true }
            return containsSegmentBoundary(children)
        }
    }

    private static func richText(
        nodes: [HTMLNode],
        style: RichText.Style = [],
        link: URL? = nil,
        trimLeadingEdge: Bool = true,
        trimTrailingEdge: Bool = true,
        assumesVisiblePredecessor: Bool = false
    ) -> RichText {
        let sentinel = "\u{E000}"
        var runs: [RichText.Run] = assumesVisiblePredecessor
            ? [.init(text: sentinel, style: [], link: nil)]
            : []
        render(nodes: nodes, style: style, link: link, listDepth: 0, to: &runs)
        if assumesVisiblePredecessor, let first = runs.first, first.text.hasPrefix(sentinel) {
            let remainder = String(first.text.dropFirst())
            if remainder.isEmpty {
                runs.removeFirst()
            } else {
                runs[0] = .init(
                    text: remainder,
                    style: first.style,
                    link: first.link,
                    foregroundColorHex: first.foregroundColorHex,
                    backgroundColorHex: first.backgroundColorHex
                )
            }
        }
        let collapsed = collapsingExcessiveNewlines(in: runs)
        var bounded = collapsed
        if trimLeadingEdge { bounded = trimmingLeadingRichTextRuns(bounded) }
        if trimTrailingEdge { bounded = trimmingTrailingRichTextRuns(bounded) }
        return RichText(runs: bounded)
    }

    private static func render(
        nodes: [HTMLNode],
        style: RichText.Style,
        link: URL?,
        listDepth: Int,
        to runs: inout [RichText.Run]
    ) {
        for node in nodes {
            switch node {
            case let .text(raw):
                appendNormalizedHTMLText(raw, style: style, link: link, to: &runs)
            case let .element(name, attributes, children):
                switch name {
                case "strong", "b":
                    render(
                        nodes: children,
                        style: style.union(.bold),
                        link: link,
                        listDepth: listDepth,
                        to: &runs
                    )
                case "h1", "h2", "h3", "h4", "h5", "h6":
                    let level = Int(name.dropFirst()) ?? 6
                    ensureNewlines(2, in: &runs)
                    render(
                        nodes: children,
                        style: style.union(.bold).union(.heading(level)),
                        link: link,
                        listDepth: listDepth,
                        to: &runs
                    )
                    ensureNewlines(2, in: &runs)
                case "em", "i":
                    render(nodes: children, style: style.union(.italic), link: link, listDepth: listDepth, to: &runs)
                case "del", "s":
                    render(nodes: children, style: style.union(.strikethrough), link: link, listDepth: listDepth, to: &runs)
                case "u":
                    render(nodes: children, style: style.union(.underline), link: link, listDepth: listDepth, to: &runs)
                case "sup":
                    render(
                        nodes: children,
                        style: style.subtracting(.subscriptText).union(.superscript),
                        link: link,
                        listDepth: listDepth,
                        to: &runs
                    )
                case "sub":
                    render(
                        nodes: children,
                        style: style.subtracting(.superscript).union(.subscriptText),
                        link: link,
                        listDepth: listDepth,
                        to: &runs
                    )
                case "code":
                    render(nodes: children, style: style.union(.code), link: link, listDepth: listDepth, to: &runs)
                case "a":
                    let candidate = attributes["href"]?.decodingBasicHTMLEntities()
                    let safeLink = candidate.flatMap { $0.isSafeMatrixHTMLLink ? URL(string: $0) : nil }
                    render(nodes: children, style: style, link: safeLink, listDepth: listDepth, to: &runs)
                case "br":
                    ensureNewlines(1, in: &runs)
                case "p", "div":
                    ensureNewlines(2, in: &runs)
                    if children.isEmpty, let math = attributes["data-mx-maths"]?.decodingBasicHTMLEntities() {
                        appendRichTextRun(.init(text: math, style: style.union(.code), link: nil), to: &runs)
                    } else {
                        render(nodes: children, style: style, link: link, listDepth: listDepth, to: &runs)
                    }
                    ensureNewlines(2, in: &runs)
                case "ul", "ol":
                    renderList(
                        name: name,
                        attributes: attributes,
                        children: children,
                        style: style,
                        link: link,
                        depth: listDepth,
                        to: &runs
                    )
                case "blockquote":
                    ensureNewlines(1, in: &runs)
                    appendQuoted(
                        children,
                        style: style,
                        link: link,
                        listDepth: listDepth,
                        to: &runs
                    )
                    ensureNewlines(1, in: &runs)
                case "pre":
                    ensureNewlines(1, in: &runs)
                    appendRichTextRun(
                        .init(text: exactText(nodes: children), style: style.union(.code), link: nil),
                        to: &runs
                    )
                    ensureNewlines(1, in: &runs)
                case "hr":
                    ensureNewlines(1, in: &runs)
                    appendRichTextRun(.init(text: "────────", style: style, link: nil), to: &runs)
                    ensureNewlines(1, in: &runs)
                case "table":
                    renderTableAsText(node: node, style: style, to: &runs)
                case "details":
                    ensureNewlines(1, in: &runs)
                    render(nodes: children, style: style, link: link, listDepth: listDepth, to: &runs)
                    ensureNewlines(1, in: &runs)
                case "summary":
                    render(nodes: children, style: style.union(.bold), link: link, listDepth: listDepth, to: &runs)
                    ensureNewlines(1, in: &runs)
                case "span":
                    let boundary = (index: runs.count - 1, characterCount: runs.last?.text.count ?? 0)
                    if children.isEmpty, let math = attributes["data-mx-maths"]?.decodingBasicHTMLEntities() {
                        appendRichTextRun(.init(text: math, style: style.union(.code), link: link), to: &runs)
                    } else {
                        render(nodes: children, style: style, link: link, listDepth: listDepth, to: &runs)
                    }
                    let foreground = attributes["data-mx-color"]
                    let background = attributes["data-mx-bg-color"]
                    if foreground != nil || background != nil {
                        applyMatrixColors(
                            foreground: foreground,
                            background: background,
                            after: boundary,
                            to: &runs
                        )
                    }
                default:
                    render(nodes: children, style: style, link: link, listDepth: listDepth, to: &runs)
                }
            }
        }
    }

    private static func appendQuoted(
        _ children: [HTMLNode],
        style: RichText.Style,
        link: URL?,
        listDepth: Int,
        to runs: inout [RichText.Run]
    ) {
        var content: [RichText.Run] = []
        render(nodes: children, style: style, link: link, listDepth: listDepth, to: &content)
        content = trimmingRichTextRuns(collapsingExcessiveNewlines(in: content))
        guard content.isEmpty == false else { return }

        var needsPrefix = true
        for run in content {
            var fragment = ""
            for character in run.text {
                if needsPrefix {
                    if fragment.isEmpty == false {
                        appendRichTextRun(.init(
                            text: fragment,
                            style: run.style,
                            link: run.link,
                            foregroundColorHex: run.foregroundColorHex,
                            backgroundColorHex: run.backgroundColorHex
                        ), to: &runs)
                        fragment = ""
                    }
                    appendRichTextRun(.init(text: "> ", style: style, link: link), to: &runs)
                    needsPrefix = false
                }
                fragment.append(character)
                if character == "\n" { needsPrefix = true }
            }
            appendRichTextRun(.init(
                text: fragment,
                style: run.style,
                link: run.link,
                foregroundColorHex: run.foregroundColorHex,
                backgroundColorHex: run.backgroundColorHex
            ), to: &runs)
        }
    }

    private static func renderList(
        name: String,
        attributes: [String: String],
        children: [HTMLNode],
        style: RichText.Style,
        link: URL?,
        depth: Int,
        to runs: inout [RichText.Run]
    ) {
        let items = children.compactMap { node -> [HTMLNode]? in
            guard case let .element(itemName, _, itemChildren) = node, itemName == "li" else { return nil }
            return itemChildren
        }
        guard items.isEmpty == false else { return }
        let start = attributes["start"].flatMap(Int.init) ?? 1
        ensureNewlines(1, in: &runs)
        for (offset, itemChildren) in items.enumerated() {
            if offset > 0 { ensureNewlines(1, in: &runs) }
            let marker = name == "ol" ? "\(start + offset). " : "• "
            appendRichTextRun(
                .init(text: String(repeating: "  ", count: depth) + marker, style: style, link: link),
                to: &runs
            )
            var inline: [HTMLNode] = []
            for child in itemChildren {
                if case let .element(childName, childAttributes, nested) = child,
                   childName == "ol" || childName == "ul"
                {
                    render(nodes: inline, style: style, link: link, listDepth: depth, to: &runs)
                    inline.removeAll(keepingCapacity: true)
                    ensureNewlines(1, in: &runs)
                    renderList(
                        name: childName,
                        attributes: childAttributes,
                        children: nested,
                        style: style,
                        link: link,
                        depth: depth + 1,
                        to: &runs
                    )
                } else if case let .element(childName, _, paragraphChildren) = child,
                          childName == "p"
                {
                    // `<li><p>…</p></li>` is common Matrix HTML. A paragraph
                    // wrapper must not insert a blank line between the bullet
                    // and its text. Preserve separation only between multiple
                    // paragraphs inside the same list item.
                    if inline.isEmpty == false {
                        inline.append(.element(name: "br", attributes: [:], children: []))
                        inline.append(.element(name: "br", attributes: [:], children: []))
                    }
                    inline.append(contentsOf: paragraphChildren)
                } else {
                    inline.append(child)
                }
            }
            render(nodes: inline, style: style, link: link, listDepth: depth, to: &runs)
        }
        ensureNewlines(1, in: &runs)
    }

    private static func appendNormalizedHTMLText(
        _ raw: String,
        style: RichText.Style,
        link: URL?,
        to runs: inout [RichText.Run]
    ) {
        let decoded = decodeHTMLText(raw)
        var normalized = ""
        var pendingSpace = false
        let hasVisiblePredecessor = runs.last.map {
            $0.text.last.map { $0.isWhitespace == false } ?? false
        } ?? false
        for scalar in decoded.unicodeScalars {
            if scalar == " " || scalar == "\t" || scalar == "\r" || scalar == "\n" || scalar == "\u{000C}" {
                pendingSpace = true
            } else {
                if pendingSpace, normalized.isEmpty == false || hasVisiblePredecessor {
                    normalized.append(" ")
                }
                normalized.unicodeScalars.append(scalar)
                pendingSpace = false
            }
        }
        if pendingSpace, normalized.isEmpty == false || hasVisiblePredecessor { normalized.append(" ") }
        guard normalized.isEmpty == false else { return }
        if normalized == " ", runs.isEmpty || runs.last?.text.hasSuffix("\n") == true { return }
        if runs.last?.text.hasSuffix(" ") == true, normalized.hasPrefix(" ") {
            normalized.removeFirst()
        }
        appendRichTextRun(.init(text: normalized, style: style, link: link), to: &runs)
    }

    private static func ensureNewlines(_ requested: Int, in runs: inout [RichText.Run]) {
        guard runs.isEmpty == false else { return }
        while let last = runs.last, last.text.allSatisfy({ $0 == " " || $0 == "\t" }) {
            runs.removeLast()
        }
        if let last = runs.last {
            let trimmed = last.text.trimmingTrailingCharacters(in: CharacterSet(charactersIn: " \t"))
            if trimmed != last.text {
                runs[runs.count - 1] = .init(
                    text: trimmed,
                    style: last.style,
                    link: last.link,
                    foregroundColorHex: last.foregroundColorHex,
                    backgroundColorHex: last.backgroundColorHex
                )
            }
        }
        var existing = 0
        trailingNewlines: for run in runs.reversed() {
            for character in run.text.reversed() {
                guard character == "\n" else { break trailingNewlines }
                existing += 1
            }
        }
        let needed = max(0, requested - existing)
        if needed > 0 {
            appendRichTextRun(.init(text: String(repeating: "\n", count: needed), style: [], link: nil), to: &runs)
        }
    }

    private static func exactText(nodes: [HTMLNode]) -> String {
        nodes.map { node in
            switch node {
            case let .text(text):
                return decodeHTMLText(text)
            case let .element(name, _, children):
                return name == "br" ? "\n" : exactText(nodes: children)
            }
        }.joined()
    }

    private static func decodeHTMLText(_ value: String) -> String {
        // Decode one character reference at a time. Importing the complete
        // text node through WebKit/Foundation applies HTML whitespace
        // collapsing and corrupts byte-significant pre/code content.
        var output = ""
        output.reserveCapacity(value.count)
        var cursor = value.startIndex
        while cursor < value.endIndex {
            guard value[cursor] == "&",
                  let semicolon = value[cursor...].firstIndex(of: ";"),
                  value.distance(from: cursor, to: semicolon) <= 64
            else {
                output.append(value[cursor])
                cursor = value.index(after: cursor)
                continue
            }
            let end = value.index(after: semicolon)
            let entity = String(value[cursor ..< end])
            if let decoded = decodeSingleHTMLEntity(entity), decoded != entity {
                output.append(decoded)
                cursor = end
            } else {
                output.append(value[cursor])
                cursor = value.index(after: cursor)
            }
        }
        return output
    }

    private static func decodeSingleHTMLEntity(_ entity: String) -> String? {
        guard entity.first == "&", entity.last == ";" else { return nil }
        let name = entity.dropFirst().dropLast()
        let isNamed = name.first?.isLetter == true
            && name.allSatisfy { $0.isASCII && ($0.isLetter || $0.isNumber) }
        let isDecimal = name.first == "#"
            && name.dropFirst().isEmpty == false
            && name.dropFirst().allSatisfy(\.isNumber)
        let isHex = name.count > 2
            && name.prefix(2).lowercased() == "#x"
            && name.dropFirst(2).allSatisfy { $0.isHexDigit }
        guard isNamed || isDecimal || isHex else { return nil }

        if isDecimal || isHex {
            let digits = isHex ? name.dropFirst(2) : name.dropFirst()
            let radix = isHex ? 16 : 10
            guard let value = UInt32(digits, radix: radix),
                  let scalar = UnicodeScalar(value)
            else { return nil }
            return String(Character(scalar))
        }

        return namedHTMLEntities[String(name)]
    }

    // HTML 4's complete named-character-reference set plus the HTML5 apos
    // reference. Matrix formatted bodies use semicolon-terminated references;
    // decoding them directly keeps rendering deterministic and avoids the
    // WebKit-backed NSAttributedString HTML importer, which can spin the main
    // run loop while a diffable snapshot is being applied.
    private static let namedHTMLEntities: [String: String] = [
            "AElig": "Æ",
            "Aacute": "Á",
            "Acirc": "Â",
            "Agrave": "À",
            "Alpha": "Α",
            "Aring": "Å",
            "Atilde": "Ã",
            "Auml": "Ä",
            "Beta": "Β",
            "Ccedil": "Ç",
            "Chi": "Χ",
            "Dagger": "‡",
            "Delta": "Δ",
            "ETH": "Ð",
            "Eacute": "É",
            "Ecirc": "Ê",
            "Egrave": "È",
            "Epsilon": "Ε",
            "Eta": "Η",
            "Euml": "Ë",
            "Gamma": "Γ",
            "Iacute": "Í",
            "Icirc": "Î",
            "Igrave": "Ì",
            "Iota": "Ι",
            "Iuml": "Ï",
            "Kappa": "Κ",
            "Lambda": "Λ",
            "Mu": "Μ",
            "Ntilde": "Ñ",
            "Nu": "Ν",
            "OElig": "Œ",
            "Oacute": "Ó",
            "Ocirc": "Ô",
            "Ograve": "Ò",
            "Omega": "Ω",
            "Omicron": "Ο",
            "Oslash": "Ø",
            "Otilde": "Õ",
            "Ouml": "Ö",
            "Phi": "Φ",
            "Pi": "Π",
            "Prime": "″",
            "Psi": "Ψ",
            "Rho": "Ρ",
            "Scaron": "Š",
            "Sigma": "Σ",
            "THORN": "Þ",
            "Tau": "Τ",
            "Theta": "Θ",
            "Uacute": "Ú",
            "Ucirc": "Û",
            "Ugrave": "Ù",
            "Upsilon": "Υ",
            "Uuml": "Ü",
            "Xi": "Ξ",
            "Yacute": "Ý",
            "Yuml": "Ÿ",
            "Zeta": "Ζ",
            "aacute": "á",
            "acirc": "â",
            "acute": "´",
            "aelig": "æ",
            "agrave": "à",
            "alefsym": "ℵ",
            "alpha": "α",
            "amp": "&",
            "and": "∧",
            "ang": "∠",
            "aring": "å",
            "asymp": "≈",
            "atilde": "ã",
            "auml": "ä",
            "bdquo": "„",
            "beta": "β",
            "brvbar": "¦",
            "bull": "•",
            "cap": "∩",
            "ccedil": "ç",
            "cedil": "¸",
            "cent": "¢",
            "chi": "χ",
            "circ": "ˆ",
            "clubs": "♣",
            "cong": "≅",
            "copy": "©",
            "crarr": "↵",
            "cup": "∪",
            "curren": "¤",
            "dArr": "⇓",
            "dagger": "†",
            "darr": "↓",
            "deg": "°",
            "delta": "δ",
            "diams": "♦",
            "divide": "÷",
            "eacute": "é",
            "ecirc": "ê",
            "egrave": "è",
            "empty": "∅",
            "emsp": " ",
            "ensp": " ",
            "epsilon": "ε",
            "equiv": "≡",
            "eta": "η",
            "eth": "ð",
            "euml": "ë",
            "euro": "€",
            "exist": "∃",
            "fnof": "ƒ",
            "forall": "∀",
            "frac12": "½",
            "frac14": "¼",
            "frac34": "¾",
            "frasl": "⁄",
            "gamma": "γ",
            "ge": "≥",
            "gt": ">",
            "hArr": "⇔",
            "harr": "↔",
            "hearts": "♥",
            "hellip": "…",
            "iacute": "í",
            "icirc": "î",
            "iexcl": "¡",
            "igrave": "ì",
            "image": "ℑ",
            "infin": "∞",
            "int": "∫",
            "iota": "ι",
            "iquest": "¿",
            "isin": "∈",
            "iuml": "ï",
            "kappa": "κ",
            "lArr": "⇐",
            "lambda": "λ",
            "lang": "〈",
            "laquo": "«",
            "larr": "←",
            "lceil": "⌈",
            "ldquo": "“",
            "le": "≤",
            "lfloor": "⌊",
            "lowast": "∗",
            "loz": "◊",
            "lrm": "‎",
            "lsaquo": "‹",
            "lsquo": "‘",
            "lt": "<",
            "macr": "¯",
            "mdash": "—",
            "micro": "µ",
            "middot": "·",
            "minus": "−",
            "mu": "μ",
            "nabla": "∇",
            "nbsp": " ",
            "ndash": "–",
            "ne": "≠",
            "ni": "∋",
            "not": "¬",
            "notin": "∉",
            "nsub": "⊄",
            "ntilde": "ñ",
            "nu": "ν",
            "oacute": "ó",
            "ocirc": "ô",
            "oelig": "œ",
            "ograve": "ò",
            "oline": "‾",
            "omega": "ω",
            "omicron": "ο",
            "oplus": "⊕",
            "or": "∨",
            "ordf": "ª",
            "ordm": "º",
            "oslash": "ø",
            "otilde": "õ",
            "otimes": "⊗",
            "ouml": "ö",
            "para": "¶",
            "part": "∂",
            "permil": "‰",
            "perp": "⊥",
            "phi": "φ",
            "pi": "π",
            "piv": "ϖ",
            "plusmn": "±",
            "pound": "£",
            "prime": "′",
            "prod": "∏",
            "prop": "∝",
            "psi": "ψ",
            "quot": "\"",
            "rArr": "⇒",
            "radic": "√",
            "rang": "〉",
            "raquo": "»",
            "rarr": "→",
            "rceil": "⌉",
            "rdquo": "”",
            "real": "ℜ",
            "reg": "®",
            "rfloor": "⌋",
            "rho": "ρ",
            "rlm": "‏",
            "rsaquo": "›",
            "rsquo": "’",
            "sbquo": "‚",
            "scaron": "š",
            "sdot": "⋅",
            "sect": "§",
            "shy": "­",
            "sigma": "σ",
            "sigmaf": "ς",
            "sim": "∼",
            "spades": "♠",
            "sub": "⊂",
            "sube": "⊆",
            "sum": "∑",
            "sup": "⊃",
            "sup1": "¹",
            "sup2": "²",
            "sup3": "³",
            "supe": "⊇",
            "szlig": "ß",
            "tau": "τ",
            "there4": "∴",
            "theta": "θ",
            "thetasym": "ϑ",
            "thinsp": " ",
            "thorn": "þ",
            "tilde": "˜",
            "times": "×",
            "trade": "™",
            "uArr": "⇑",
            "uacute": "ú",
            "uarr": "↑",
            "ucirc": "û",
            "ugrave": "ù",
            "uml": "¨",
            "upsih": "ϒ",
            "upsilon": "υ",
            "uuml": "ü",
            "weierp": "℘",
            "xi": "ξ",
            "yacute": "ý",
            "yen": "¥",
            "yuml": "ÿ",
            "zeta": "ζ",
            "zwj": "‍",
            "zwnj": "‌",

            "apos": "'",
    ]

    private static func renderTableAsText(
        node: HTMLNode,
        style: RichText.Style,
        to runs: inout [RichText.Run]
    ) {
        guard let table = tableBlock(node: node) else { return }
        ensureNewlines(1, in: &runs)
        if let caption = table.caption {
            caption.runs.forEach { appendRichTextRun($0, to: &runs) }
            ensureNewlines(1, in: &runs)
        }
        for (rowIndex, row) in table.rows.enumerated() {
            if rowIndex > 0 { ensureNewlines(1, in: &runs) }
            for (cellIndex, cell) in row.cells.enumerated() {
                if cellIndex > 0 { appendRichTextRun(.init(text: "\t", style: style, link: nil), to: &runs) }
                cell.content.runs.forEach { appendRichTextRun($0, to: &runs) }
            }
        }
        ensureNewlines(1, in: &runs)
    }

    static func detailsBlocks(html: String) -> [DetailsBlock] {
        guard let nodes = parsedNodes(html: html) else { return [] }
        return descendantElements(named: "details", in: nodes).compactMap(detailsBlock(node:))
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

    static func codeLineCount(_ code: String) -> Int {
        let normalized = code.hasSuffix("\n") ? String(code.dropLast()) : code
        if normalized.isEmpty {
            return 1
        }
        return normalized.split(separator: "\n", omittingEmptySubsequences: false).count
    }

    static func tableBlock(html: String) -> TableBlock? {
        guard let nodes = parsedNodes(html: html),
              let table = descendantElements(named: "table", in: nodes).first
        else { return nil }
        return tableBlock(node: table)
    }

    private static func spoilerBlock(
        node: HTMLNode,
        style: RichText.Style = [],
        link: URL? = nil
    ) -> SpoilerBlock? {
        guard case let .element(name, attributes, children) = node,
              name == "span",
              let rawReason = attributes["data-mx-spoiler"]
        else { return nil }
        let content = richText(nodes: children, style: style, link: link)
        guard content.runs.isEmpty == false else { return nil }
        let reason = rawReason.decodingBasicHTMLEntities()
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return SpoilerBlock(
            content: content,
            reason: reason.isEmpty ? nil : String(reason.prefix(160))
        )
    }

    private static func codeBlock(html: String) -> CodeBlock? {
        guard let nodes = parsedNodes(html: html),
              let pre = descendantElements(named: "pre", in: nodes).first
        else { return nil }
        return codeBlock(node: pre)
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
        guard let nodes = parsedNodes(html: html),
              case let .element(_, _, children)? = descendantElements(named: "blockquote", in: nodes).first
        else { return nil }
        let quote = richText(nodes: children)
        return quote.runs.isEmpty ? nil : quote
    }

    private static func codeBlock(node: HTMLNode) -> CodeBlock? {
        guard case let .element(name, _, children) = node, name == "pre" else { return nil }
        let code = exactText(nodes: children)
        guard code.isEmpty == false else { return nil }
        let language = descendantElements(named: "code", in: children).lazy.compactMap { node -> String? in
            guard case let .element(_, attributes, _) = node,
                  let classes = attributes["class"]?.decodingBasicHTMLEntities()
            else { return nil }
            return classes.split(whereSeparator: \.isWhitespace)
                .map(String.init)
                .first(where: { $0.isSafeMatrixLanguageClass })
                .map { String($0.dropFirst("language-".count)) }
        }.first
        return CodeBlock(code: code, language: language)
    }

    private static func detailsBlock(node: HTMLNode) -> DetailsBlock? {
        guard case let .element(name, _, children) = node, name == "details",
              let summaryNode = children.first(where: {
                  if case let .element(childName, _, _) = $0 { return childName == "summary" }
                  return false
              }),
              case let .element(_, _, summaryChildren) = summaryNode
        else { return nil }
        let summary = richText(nodes: summaryChildren)
        guard summary.runs.isEmpty == false else { return nil }
        let content = children.filter { child in
            if case let .element(childName, _, _) = child { return childName != "summary" }
            return true
        }
        var segments: [Segment] = []
        appendSegments(content, to: &segments)
        return DetailsBlock(summaryContent: summary, content: segments)
    }

    private static func tableBlock(node: HTMLNode) -> TableBlock? {
        guard case let .element(name, _, children) = node, name == "table" else { return nil }
        let caption = children.compactMap { child -> RichText? in
            guard case let .element(childName, _, captionChildren) = child, childName == "caption" else { return nil }
            let text = richText(nodes: captionChildren)
            return text.runs.isEmpty ? nil : text
        }.first
        var rowNodes: [HTMLNode] = []
        collectTableRows(in: children, into: &rowNodes)
        let rows = rowNodes.compactMap { rowNode -> TableRow? in
            guard case let .element(_, _, rowChildren) = rowNode else { return nil }
            let cells = rowChildren.compactMap { cellNode -> TableCell? in
                guard case let .element(cellName, _, cellChildren) = cellNode,
                      cellName == "th" || cellName == "td"
                else { return nil }
                return TableCell(content: richText(nodes: cellChildren), isHeader: cellName == "th")
            }
            guard cells.isEmpty == false else { return nil }
            return TableRow(cells: cells, isHeader: cells.allSatisfy(\.isHeader))
        }
        return rows.isEmpty ? nil : TableBlock(caption: caption, rows: rows)
    }

    private static func collectTableRows(in nodes: [HTMLNode], into rows: inout [HTMLNode]) {
        for node in nodes {
            guard case let .element(name, _, children) = node else { continue }
            if name == "tr" {
                rows.append(node)
            } else if name != "table" {
                collectTableRows(in: children, into: &rows)
            }
        }
    }

    private static func descendantElements(named target: String, in nodes: [HTMLNode]) -> [HTMLNode] {
        var matches: [HTMLNode] = []
        for node in nodes {
            guard case let .element(name, _, children) = node else { continue }
            if name == target { matches.append(node) }
            matches.append(contentsOf: descendantElements(named: target, in: children))
        }
        return matches
    }

    private static func removingElements(named target: String, from nodes: [HTMLNode]) -> [HTMLNode] {
        nodes.compactMap { node in
            guard case let .element(name, attributes, children) = node else { return node }
            guard name != target else { return nil }
            return .element(
                name: name,
                attributes: attributes,
                children: removingElements(named: target, from: children)
            )
        }
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

    private static func appendRichTextRun(_ run: RichText.Run, to runs: inout [RichText.Run]) {
        guard run.text.isEmpty == false else {
            return
        }
        if let last = runs.last,
           last.style == run.style,
           last.link == run.link,
           last.foregroundColorHex == run.foregroundColorHex,
           last.backgroundColorHex == run.backgroundColorHex
        {
            runs[runs.count - 1] = .init(
                text: last.text + run.text,
                style: run.style,
                link: run.link,
                foregroundColorHex: run.foregroundColorHex,
                backgroundColorHex: run.backgroundColorHex
            )
        } else {
            runs.append(run)
        }
    }

    private static func applyMatrixColors(
        foreground: String?,
        background: String?,
        after boundary: (index: Int, characterCount: Int),
        to runs: inout [RichText.Run]
    ) {
        let foreground = foreground?
            .decodingBasicHTMLEntities()
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let background = background?
            .decodingBasicHTMLEntities()
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let safeForeground = foreground?.isSafeMatrixColor == true ? foreground?.uppercased() : nil
        let safeBackground = background?.isSafeMatrixColor == true ? background?.uppercased() : nil
        guard safeForeground != nil || safeBackground != nil else { return }
        var firstColoredIndex = max(0, boundary.index + 1)
        if boundary.index >= 0,
           boundary.index < runs.count,
           boundary.characterCount < runs[boundary.index].text.count
        {
            let run = runs[boundary.index]
            let split = run.text.index(run.text.startIndex, offsetBy: boundary.characterCount)
            let prefix = String(run.text[..<split])
            let suffix = String(run.text[split...])
            runs[boundary.index] = .init(
                text: prefix,
                style: run.style,
                link: run.link,
                foregroundColorHex: run.foregroundColorHex,
                backgroundColorHex: run.backgroundColorHex
            )
            runs.insert(
                .init(
                    text: suffix,
                    style: run.style,
                    link: run.link,
                    foregroundColorHex: run.foregroundColorHex,
                    backgroundColorHex: run.backgroundColorHex
                ),
                at: boundary.index + 1
            )
            firstColoredIndex = boundary.index + 1
        }
        guard firstColoredIndex < runs.count else { return }
        for index in firstColoredIndex ..< runs.count {
            let run = runs[index]
            runs[index] = .init(
                text: run.text,
                style: run.style,
                link: run.link,
                foregroundColorHex: run.foregroundColorHex ?? safeForeground,
                backgroundColorHex: run.backgroundColorHex ?? safeBackground
            )
        }
    }

    private static func trimmingLeadingRichTextRuns(_ source: [RichText.Run]) -> [RichText.Run] {
        var runs = source.filter { $0.text.isEmpty == false }
        let edgeWhitespace = CharacterSet(charactersIn: " \r\n")
        while let first = runs.first {
            let text = first.text.trimmingLeadingCharacters(in: edgeWhitespace)
            if text.isEmpty {
                runs.removeFirst()
            } else {
                runs[0] = .init(
                    text: text,
                    style: first.style,
                    link: first.link,
                    foregroundColorHex: first.foregroundColorHex,
                    backgroundColorHex: first.backgroundColorHex
                )
                break
            }
        }
        return runs
    }

    private static func trimmingTrailingRichTextRuns(_ source: [RichText.Run]) -> [RichText.Run] {
        var runs = source.filter { $0.text.isEmpty == false }
        let edgeWhitespace = CharacterSet(charactersIn: " \r\n")
        while let last = runs.last {
            let text = last.text.trimmingTrailingCharacters(in: edgeWhitespace)
            if text.isEmpty {
                runs.removeLast()
            } else {
                runs[runs.count - 1] = .init(
                    text: text,
                    style: last.style,
                    link: last.link,
                    foregroundColorHex: last.foregroundColorHex,
                    backgroundColorHex: last.backgroundColorHex
                )
                break
            }
        }
        return runs
    }

    private static func trimmingRichTextRuns(_ source: [RichText.Run]) -> [RichText.Run] {
        trimmingTrailingRichTextRuns(trimmingLeadingRichTextRuns(source))
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
            appendRichTextRun(.init(
                text: text,
                style: run.style,
                link: run.link,
                foregroundColorHex: run.foregroundColorHex,
                backgroundColorHex: run.backgroundColorHex
            ), to: &output)
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
                    while cursor < characters.count, characters[cursor].isWhitespace == false {
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
                let fallback = tag.attributes["alt"] ?? tag.attributes["title"] ?? "[Inline image]"
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
                for attribute in ["data-mx-color", "data-mx-bg-color"] {
                    if let color = tag.attributes[attribute]?.decodingBasicHTMLEntities(),
                       color.isSafeMatrixColor
                    {
                        output.append(" \(attribute)=\"\(color.uppercased())\"")
                    }
                }
                if let maths = tag.attributes["data-mx-maths"]?.decodingBasicHTMLEntities(),
                   maths.isEmpty == false
                {
                    output.append(" data-mx-maths=\"\(maths.escapingHTMLAttributeValue())\"")
                }
            case "div":
                if let maths = tag.attributes["data-mx-maths"]?.decodingBasicHTMLEntities(),
                   maths.isEmpty == false
                {
                    output.append(" data-mx-maths=\"\(maths.escapingHTMLAttributeValue())\"")
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

    func htmlTagEnd(in characters: [Character], after start: Int) -> Int? {
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

    var isSafeMatrixLanguageClass: Bool {
        guard lowercased().hasPrefix("language-"), count > "language-".count, count <= 41 else {
            return false
        }
        return dropFirst("language-".count).unicodeScalars.allSatisfy {
            CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "_+.-")).contains($0)
        }
    }

    var isSafeMatrixColor: Bool {
        guard count == 7, first == "#" else { return false }
        return dropFirst().allSatisfy(\.isHexDigit)
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
