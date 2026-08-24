import Foundation
import SynaraCore

enum RoomNoteKind: String, CaseIterable, Equatable {
    case note
    case todo
    case message

    var title: String {
        switch self {
        case .note: return "Note"
        case .todo: return "ToDo"
        case .message: return "Message"
        }
    }
}

enum RoomNoteMoveDirection: String, Equatable {
    case up
    case down
}

struct SynaraRoomNoteItem: Identifiable, Equatable {
    let id: String
    let kind: RoomNoteKind
    let roomID: String
    let createdAt: Date
    let updatedAt: Date
    let body: String?
    let completedAt: Date?
    let order: Double?
    let eventID: String?
    let eventTimestamp: Date?
    let senderID: String?

    var isCompleted: Bool { completedAt != nil }

    static func sorted(_ items: [SynaraRoomNoteItem]) -> [SynaraRoomNoteItem] {
        items.sorted { left, right in
            if left.isCompleted != right.isCompleted {
                return left.isCompleted == false
            }
            if left.kind != right.kind {
                let rank: [RoomNoteKind: Int] = [.todo: 0, .note: 1, .message: 2]
                return (rank[left.kind] ?? 3) < (rank[right.kind] ?? 3)
            }
            if left.kind == .todo {
                return (left.order ?? left.updatedAt.millisecondsSince1970)
                    > (right.order ?? right.updatedAt.millisecondsSince1970)
            }
            return left.updatedAt > right.updatedAt
        }
    }
}

enum RoomNotesError: Error, LocalizedError, Equatable {
    case noSession
    case invalidItem
    case unavailable

    var errorDescription: String? {
        switch self {
        case .noSession:
            return "Sign in to load your personal notes."
        case .invalidItem:
            return "This note could not be saved."
        case .unavailable:
            return "Personal notes could not be synced. Try again."
        }
    }
}

protocol RoomNotesServicing {
    func loadItems(roomID: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError>
    func addItem(roomID: String, kind: RoomNoteKind, body: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError>
    func updateItem(_ item: SynaraRoomNoteItem, body: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError>
    func pinMessage(roomID: String, item: TimelineItem) async -> Result<[SynaraRoomNoteItem], RoomNotesError>
    func deleteItem(roomID: String, itemID: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError>
    func setTodoCompleted(roomID: String, itemID: String, completed: Bool) async -> Result<[SynaraRoomNoteItem], RoomNotesError>
    func moveTodo(roomID: String, itemID: String, direction: RoomNoteMoveDirection) async -> Result<[SynaraRoomNoteItem], RoomNotesError>
}

final class SharedCoreRoomNotesService: RoomNotesServicing {
    private let host: SharedCoreProductHost
    private let now: () -> Date

    init(host: SharedCoreProductHost, now: @escaping () -> Date = Date.init) {
        self.host = host
        self.now = now
    }

    func loadItems(roomID: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        guard case .signedIn = host.sessionStore.currentState else {
            return .failure(.noSession)
        }
        do {
            return .success(items(from: try await SharedCoreRoomNotes.roomNotesSnapshot(core: host.core), roomID: roomID))
        } catch {
            return .failure(.unavailable)
        }
    }

    func addItem(
        roomID: String,
        kind: RoomNoteKind,
        body: String
    ) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        guard kind == .note || kind == .todo else { return .failure(.invalidItem) }
        let trimmed = String(body.trimmingCharacters(in: .whitespacesAndNewlines).prefix(4_000))
        guard trimmed.isEmpty == false else { return .failure(.invalidItem) }
        let timestamp = now().millisecondsSince1970
        let item = RoomNoteItemDto(
            id: "\(kind.rawValue):\(UUID().uuidString.lowercased())",
            kind: kind.rawValue,
            roomId: roomID,
            createdAt: timestamp,
            updatedAt: timestamp,
            body: trimmed,
            completedAt: nil,
            order: kind == .todo ? timestamp : nil,
            eventId: nil,
            eventTs: nil,
            sender: nil
        )
        return await upsert(item, roomID: roomID)
    }

    func pinMessage(roomID: String, item: TimelineItem) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        guard let eventID = item.serverEventID, eventID.isEmpty == false else {
            return .failure(.invalidItem)
        }
        let timestamp = now().millisecondsSince1970
        let preview = String(TimelineSearchFilter.searchableText(for: item).trimmingCharacters(in: .whitespacesAndNewlines).prefix(1_000))
        let note = RoomNoteItemDto(
            id: "message:\(UUID().uuidString.lowercased())",
            kind: RoomNoteKind.message.rawValue,
            roomId: roomID,
            createdAt: timestamp,
            updatedAt: timestamp,
            body: preview.isEmpty ? nil : preview,
            completedAt: nil,
            order: nil,
            eventId: eventID,
            eventTs: item.timestamp.millisecondsSince1970,
            sender: item.senderID
        )
        return await upsert(note, roomID: roomID)
    }

    func updateItem(
        _ item: SynaraRoomNoteItem,
        body: String
    ) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        guard item.kind == .note || item.kind == .todo else { return .failure(.invalidItem) }
        let trimmed = String(body.trimmingCharacters(in: .whitespacesAndNewlines).prefix(4_000))
        guard trimmed.isEmpty == false else { return .failure(.invalidItem) }
        let updated = RoomNoteItemDto(
            id: item.id,
            kind: item.kind.rawValue,
            roomId: item.roomID,
            createdAt: item.createdAt.millisecondsSince1970,
            updatedAt: now().millisecondsSince1970,
            body: trimmed,
            completedAt: item.completedAt?.millisecondsSince1970,
            order: item.order,
            eventId: nil,
            eventTs: nil,
            sender: nil
        )
        return await upsert(updated, roomID: item.roomID)
    }

    func deleteItem(roomID: String, itemID: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        do {
            let snapshot = try await SharedCoreRoomNotes.roomNotesDelete(core: host.core, roomId: roomID, itemId: itemID)
            return .success(items(from: snapshot, roomID: roomID))
        } catch {
            return .failure(.unavailable)
        }
    }

    func setTodoCompleted(
        roomID: String,
        itemID: String,
        completed: Bool
    ) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        do {
            let snapshot = try await SharedCoreRoomNotes.roomNotesCompleteTodo(
                core: host.core,
                roomId: roomID,
                itemId: itemID,
                completed: completed
            )
            return .success(items(from: snapshot, roomID: roomID))
        } catch {
            return .failure(.unavailable)
        }
    }

    func moveTodo(
        roomID: String,
        itemID: String,
        direction: RoomNoteMoveDirection
    ) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        do {
            let snapshot = try await SharedCoreRoomNotes.roomNotesMoveTodo(
                core: host.core,
                roomId: roomID,
                itemId: itemID,
                direction: direction.rawValue
            )
            return .success(items(from: snapshot, roomID: roomID))
        } catch {
            return .failure(.unavailable)
        }
    }

    private func upsert(_ item: RoomNoteItemDto, roomID: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        do {
            let snapshot = try await SharedCoreRoomNotes.roomNotesUpsert(core: host.core, item: item)
            return .success(items(from: snapshot, roomID: roomID))
        } catch {
            return .failure(.unavailable)
        }
    }

    private func items(from snapshot: RoomNotesSnapshotDto, roomID: String) -> [SynaraRoomNoteItem] {
        let mapped = snapshot.items.compactMap { item -> SynaraRoomNoteItem? in
            guard item.roomId == roomID, let kind = RoomNoteKind(rawValue: item.kind) else { return nil }
            return SynaraRoomNoteItem(
                id: item.id,
                kind: kind,
                roomID: item.roomId,
                createdAt: Date(timeIntervalSince1970: item.createdAt / 1_000),
                updatedAt: Date(timeIntervalSince1970: item.updatedAt / 1_000),
                body: item.body,
                completedAt: item.completedAt.map { Date(timeIntervalSince1970: $0 / 1_000) },
                order: item.order,
                eventID: item.eventId,
                eventTimestamp: item.eventTs.map { Date(timeIntervalSince1970: $0 / 1_000) },
                senderID: item.sender
            )
        }
        return SynaraRoomNoteItem.sorted(mapped)
    }
}

final class MockRoomNotesService: RoomNotesServicing {
    private var items: [SynaraRoomNoteItem]
    private let now: () -> Date

    init(items: [SynaraRoomNoteItem] = [], now: @escaping () -> Date = Date.init) {
        self.items = items
        self.now = now
    }

    func loadItems(roomID: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        .success(roomItems(roomID))
    }

    func addItem(roomID: String, kind: RoomNoteKind, body: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        let trimmed = String(body.trimmingCharacters(in: .whitespacesAndNewlines).prefix(4_000))
        guard trimmed.isEmpty == false, kind != .message else { return .failure(.invalidItem) }
        let timestamp = now()
        items.append(SynaraRoomNoteItem(
            id: "\(kind.rawValue):\(UUID().uuidString.lowercased())",
            kind: kind,
            roomID: roomID,
            createdAt: timestamp,
            updatedAt: timestamp,
            body: trimmed,
            completedAt: nil,
            order: kind == .todo ? timestamp.millisecondsSince1970 : nil,
            eventID: nil,
            eventTimestamp: nil,
            senderID: nil
        ))
        return .success(roomItems(roomID))
    }

    func pinMessage(roomID: String, item: TimelineItem) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        guard let eventID = item.serverEventID else { return .failure(.invalidItem) }
        let timestamp = now()
        items.append(SynaraRoomNoteItem(
            id: "message:\(UUID().uuidString.lowercased())",
            kind: .message,
            roomID: roomID,
            createdAt: timestamp,
            updatedAt: timestamp,
            body: String(TimelineSearchFilter.searchableText(for: item).prefix(1_000)),
            completedAt: nil,
            order: nil,
            eventID: eventID,
            eventTimestamp: item.timestamp,
            senderID: item.senderID
        ))
        return .success(roomItems(roomID))
    }

    func updateItem(_ item: SynaraRoomNoteItem, body: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        guard item.kind == .note || item.kind == .todo,
              let index = items.firstIndex(where: { $0.roomID == item.roomID && $0.id == item.id })
        else {
            return .failure(.invalidItem)
        }
        let trimmed = String(body.trimmingCharacters(in: .whitespacesAndNewlines).prefix(4_000))
        guard trimmed.isEmpty == false else { return .failure(.invalidItem) }
        let existing = items[index]
        items[index] = SynaraRoomNoteItem(
            id: existing.id,
            kind: existing.kind,
            roomID: existing.roomID,
            createdAt: existing.createdAt,
            updatedAt: now(),
            body: trimmed,
            completedAt: existing.completedAt,
            order: existing.order,
            eventID: existing.eventID,
            eventTimestamp: existing.eventTimestamp,
            senderID: existing.senderID
        )
        return .success(roomItems(item.roomID))
    }

    func deleteItem(roomID: String, itemID: String) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        items.removeAll { $0.roomID == roomID && $0.id == itemID }
        return .success(roomItems(roomID))
    }

    func setTodoCompleted(roomID: String, itemID: String, completed: Bool) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        guard let index = items.firstIndex(where: { $0.roomID == roomID && $0.id == itemID && $0.kind == .todo }) else {
            return .failure(.invalidItem)
        }
        let item = items[index]
        let timestamp = now()
        items[index] = SynaraRoomNoteItem(
            id: item.id, kind: item.kind, roomID: item.roomID, createdAt: item.createdAt,
            updatedAt: timestamp, body: item.body, completedAt: completed ? timestamp : nil,
            order: item.order, eventID: item.eventID, eventTimestamp: item.eventTimestamp, senderID: item.senderID
        )
        return .success(roomItems(roomID))
    }

    func moveTodo(roomID: String, itemID: String, direction: RoomNoteMoveDirection) async -> Result<[SynaraRoomNoteItem], RoomNotesError> {
        let ordered = roomItems(roomID).filter { $0.kind == .todo }
        guard let current = ordered.firstIndex(where: { $0.id == itemID }) else { return .failure(.invalidItem) }
        let target = direction == .up ? current - 1 : current + 1
        guard ordered.indices.contains(target), ordered[current].isCompleted == ordered[target].isCompleted else {
            return .success(roomItems(roomID))
        }
        let currentIndex = items.firstIndex(where: { $0.id == ordered[current].id })!
        let targetIndex = items.firstIndex(where: { $0.id == ordered[target].id })!
        let timestamp = now()
        items[currentIndex] = items[currentIndex].with(order: ordered[target].order ?? ordered[target].updatedAt.millisecondsSince1970, updatedAt: timestamp)
        items[targetIndex] = items[targetIndex].with(order: ordered[current].order ?? ordered[current].updatedAt.millisecondsSince1970, updatedAt: timestamp)
        return .success(roomItems(roomID))
    }

    private func roomItems(_ roomID: String) -> [SynaraRoomNoteItem] {
        SynaraRoomNoteItem.sorted(items.filter { $0.roomID == roomID })
    }
}

private extension SynaraRoomNoteItem {
    func with(order: Double, updatedAt: Date) -> SynaraRoomNoteItem {
        SynaraRoomNoteItem(
            id: id, kind: kind, roomID: roomID, createdAt: createdAt, updatedAt: updatedAt,
            body: body, completedAt: completedAt, order: order, eventID: eventID,
            eventTimestamp: eventTimestamp, senderID: senderID
        )
    }
}

private extension Date {
    var millisecondsSince1970: Double { timeIntervalSince1970 * 1_000 }
}
