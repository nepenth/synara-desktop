import Foundation
import SynaraCore

/// User room folders (`in.synara.room_folders`, `docs/design/room-folders.md`):
/// named sections of the Rooms list that sync across the user's devices.
/// Core owns every write (read-modify-write, validation, sync echoes); this
/// file only carries its commands to the room list. "Folder" here never means
/// a space.

struct RoomFolder: Identifiable, Equatable {
    let id: String
    let name: String
    let roomIDs: [String]
}

/// The folders as Core last wrote or read them, for management (names,
/// order, read-only state). The room list renders from its own snapshot.
struct RoomFoldersSnapshot: Equatable {
    /// A newer client's format is stored: folders show, management hides.
    let readOnly: Bool
    let folders: [RoomFolder]

    static let empty = RoomFoldersSnapshot(readOnly: false, folders: [])

    init(readOnly: Bool, folders: [RoomFolder]) {
        self.readOnly = readOnly
        self.folders = folders
    }

    init(_ dto: RoomFoldersSnapshotDto) {
        self.init(
            readOnly: dto.readOnly,
            folders: dto.folders.map { RoomFolder(id: $0.id, name: $0.name, roomIDs: $0.roomIds) }
        )
    }
}

struct RoomFolderCreateResult: Equatable {
    let folderID: String
    let snapshot: RoomFoldersSnapshot
}

/// Core's limits and name rule (`room_folders.rs`), mirrored for inline
/// feedback before the round trip. Core validates again.
enum RoomFolderName {
    static let maximumLength = 64
    static let maximumFolders = 100

    /// Trimmed, inner whitespace runs collapsed to one space.
    static func normalized(_ name: String) -> String {
        name.split(whereSeparator: { $0.isWhitespace }).joined(separator: " ")
    }

    /// 1–64 characters (Unicode scalars, as Core counts them) after
    /// normalizing.
    static func isValid(_ name: String) -> Bool {
        let length = normalized(name).unicodeScalars.count
        return length >= 1 && length <= maximumLength
    }
}

/// A failed folder command. `code` is Core's `v-room-folders.*` id when it
/// has one. The description is plain sentence-case copy, the same as
/// desktop's (`nativeRoomFolderOwner.ts`), never the raw error.
struct RoomFolderError: LocalizedError, Equatable {
    static let invalidName = "v-room-folders.invalid-name"
    static let tooManyFolders = "v-room-folders.too-many-folders"
    static let contentTooLarge = "v-room-folders.content-too-large"
    static let unsupportedVersion = "v-room-folders.unsupported-version"
    static let folderNotFound = "v-room-folders.folder-not-found"
    static let roomIsDirect = "v-room-folders.room-is-direct"
    static let roomIsSpace = "v-room-folders.room-is-space"
    static let roomNotJoined = "v-room-folders.room-not-joined"

    let code: String?

    init(code: String?) {
        self.code = code
    }

    init(_ error: Error) {
        if let error = error as? RoomFolderError {
            self = error
        } else if case let RoomFoldersCommandError.Failed(code, _) = error {
            self.init(code: code)
        } else {
            self.init(code: nil)
        }
    }

    /// A newer client owns the format: reload the snapshot so management hides.
    var isReadOnly: Bool {
        code == Self.unsupportedVersion
    }

    var errorDescription: String? {
        Self.message(forCode: code)
    }

    static func message(forCode code: String?) -> String {
        switch code {
        case invalidName:
            return String(localized: "Folder names need 1 to 64 characters.")
        case tooManyFolders:
            return String(localized: "You can have up to 100 folders. Delete one to add another.")
        case contentTooLarge:
            return String(localized: "Your folders are too large to save. Take some rooms out of folders and try again.")
        case unsupportedVersion:
            return String(localized: "Your folders were changed by a newer version of Synara. Update Synara to change them here.")
        case folderNotFound:
            return String(localized: "This folder no longer exists.")
        case roomIsDirect:
            return String(localized: "Direct messages can't go in a folder.")
        case roomIsSpace:
            return String(localized: "Spaces can't go in a folder.")
        case roomNotJoined:
            return String(localized: "Join this room before putting it in a folder.")
        default:
            return String(localized: "Couldn't save your folders. Try again.")
        }
    }
}

protocol RoomFoldersServicing: AnyObject {
    func snapshot() async throws -> RoomFoldersSnapshot
    /// Appends a folder; with `roomID`, files that room in the same write.
    func create(name: String, roomID: String?) async throws -> RoomFolderCreateResult
    func rename(folderID: String, name: String) async throws -> RoomFoldersSnapshot
    /// Its rooms become unfiled (they go back to Rooms).
    func delete(folderID: String) async throws -> RoomFoldersSnapshot
    func move(folderID: String, toIndex: Int) async throws -> RoomFoldersSnapshot
    /// Files `roomID` in `folderID`, or unfiles it with nil.
    func setRoom(roomID: String, folderID: String?) async throws -> RoomFoldersSnapshot
}

/// The six Core commands, so the service can be tested with a fake core.
protocol RoomFoldersCore: AnyObject {
    func roomFoldersSnapshot() async throws -> RoomFoldersSnapshotDto
    func roomFoldersCreate(name: String, roomId: String?) async throws -> RoomFolderCreateDto
    func roomFoldersRename(folderId: String, name: String) async throws -> RoomFoldersSnapshotDto
    func roomFoldersDelete(folderId: String) async throws -> RoomFoldersSnapshotDto
    func roomFoldersMove(folderId: String, toIndex: UInt32) async throws -> RoomFoldersSnapshotDto
    func roomFoldersSetRoom(roomId: String, folderId: String?) async throws -> RoomFoldersSnapshotDto
}

extension SharedCore: RoomFoldersCore {}

/// Sole write owner for room folders on iOS: every change goes through
/// Core's `room_folders_*` commands. A command's errors arrive as
/// `RoomFolderError`. The room list wakes from Core's room-list signal right
/// after a local change.
final class SharedCoreRoomFoldersService: RoomFoldersServicing {
    private let core: RoomFoldersCore

    init(core: RoomFoldersCore) {
        self.core = core
    }

    func snapshot() async throws -> RoomFoldersSnapshot {
        try await run { RoomFoldersSnapshot(try await self.core.roomFoldersSnapshot()) }
    }

    func create(name: String, roomID: String?) async throws -> RoomFolderCreateResult {
        try await run {
            let created = try await self.core.roomFoldersCreate(
                name: RoomFolderName.normalized(name),
                roomId: roomID
            )
            return RoomFolderCreateResult(folderID: created.folderId, snapshot: RoomFoldersSnapshot(created.snapshot))
        }
    }

    func rename(folderID: String, name: String) async throws -> RoomFoldersSnapshot {
        try await run {
            RoomFoldersSnapshot(try await self.core.roomFoldersRename(
                folderId: folderID,
                name: RoomFolderName.normalized(name)
            ))
        }
    }

    func delete(folderID: String) async throws -> RoomFoldersSnapshot {
        try await run { RoomFoldersSnapshot(try await self.core.roomFoldersDelete(folderId: folderID)) }
    }

    func move(folderID: String, toIndex: Int) async throws -> RoomFoldersSnapshot {
        guard let index = UInt32(exactly: toIndex) else {
            throw RoomFolderError(code: nil)
        }
        return try await run {
            RoomFoldersSnapshot(try await self.core.roomFoldersMove(folderId: folderID, toIndex: index))
        }
    }

    func setRoom(roomID: String, folderID: String?) async throws -> RoomFoldersSnapshot {
        try await run {
            RoomFoldersSnapshot(try await self.core.roomFoldersSetRoom(roomId: roomID, folderId: folderID))
        }
    }

    private func run<T>(_ command: () async throws -> T) async throws -> T {
        do {
            return try await command()
        } catch {
            throw RoomFolderError(error)
        }
    }
}

/// In-memory folders for previews and UI tests (`SYNARA_UI_TEST_ROOM_FOLDERS`).
/// The mock room list reads it to file its rows and build its folder
/// sections, so a mock change shows after the list reloads.
final class MockRoomFolderStore {
    private let lock = NSLock()
    private var storedFolders: [RoomFolder]
    private var nextID = 1
    let readOnly: Bool
    /// A Core error id every create and rename fails with (the name prompt's
    /// error state), or nil.
    var nameFailureCode: String?

    init(folders: [RoomFolder] = [], readOnly: Bool = false, nameFailureCode: String? = nil) {
        self.storedFolders = folders
        self.readOnly = readOnly
        self.nameFailureCode = nameFailureCode
    }

    var folders: [RoomFolder] {
        lock.withLock { storedFolders }
    }

    var snapshot: RoomFoldersSnapshot {
        RoomFoldersSnapshot(readOnly: readOnly, folders: folders)
    }

    func update(_ change: (inout [RoomFolder]) throws -> Void) throws -> RoomFoldersSnapshot {
        if readOnly {
            throw RoomFolderError(code: RoomFolderError.unsupportedVersion)
        }
        return try lock.withLock {
            try change(&storedFolders)
            return RoomFoldersSnapshot(readOnly: readOnly, folders: storedFolders)
        }
    }

    func makeID() -> String {
        lock.withLock {
            defer { nextID += 1 }
            return "mock-folder-\(nextID)"
        }
    }

    /// The rows with this store's filing applied (joined rooms that are not
    /// DMs or spaces, as Core files them).
    func filing(_ rooms: [RoomSummary]) -> [RoomSummary] {
        let folderByRoom = Dictionary(
            folders.flatMap { folder in folder.roomIDs.map { ($0, folder.id) } },
            uniquingKeysWith: { first, _ in first }
        )
        return rooms.map { room in
            RoomListFolders.canFile(room) ? room.withFolderID(folderByRoom[room.id]) : room.withFolderID(nil)
        }
    }

    /// Folder sections like Core's (`room_list/folders.rs`) for the fixture
    /// rows: favorites left out, Core's orders, and attention from unmuted
    /// rooms. A fixture stand-in only; the app reads Core's sections.
    func sections(rooms: [RoomSummary]) -> [RoomFolderSection] {
        let byID = Dictionary(filing(rooms).map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        return folders.map { folder in
            let shown = folder.roomIDs.compactMap { byID[$0] }.filter {
                RoomListFolders.placement(of: $0) == .folder(folderId: folder.id)
            }
            let attention = shown.filter { $0.isMuted == false && $0.hasUnreadActivity }
            return RoomFolderSection(
                id: folder.id,
                name: folder.name,
                roomIDs: shown.map(\.id),
                recentOrder: RoomListFavorites.sorted(shown, order: .recent).map(\.id),
                nameOrder: RoomListFavorites.sorted(shown, order: .name).map(\.id),
                unreadHighlight: attention.reduce(0) { $0 + $1.highlightCount },
                unreadTotal: attention.reduce(0) { $0 + $1.unreadBadgeCount },
                unreadRoomIDs: attention.map(\.id).sorted()
            )
        }
    }

    /// Three folders over `RoomListFixtures.small()`: Work (a mention),
    /// Ops watch (plain unread) and an empty Side projects.
    static func uiTestFixture(nameFailureCode: String? = nil, readOnly: Bool = false) -> MockRoomFolderStore {
        MockRoomFolderStore(
            folders: [
                RoomFolder(id: "work", name: "Work", roomIDs: ["!project:matrix.org", "!design:matrix.org"]),
                RoomFolder(id: "ops", name: "Ops watch", roomIDs: ["!security:matrix.org", "!agent-workflows:matrix.org"]),
                RoomFolder(id: "side", name: "Side projects", roomIDs: []),
            ],
            readOnly: readOnly,
            nameFailureCode: nameFailureCode
        )
    }
}

final class MockRoomFoldersService: RoomFoldersServicing {
    let store: MockRoomFolderStore

    init(store: MockRoomFolderStore = MockRoomFolderStore()) {
        self.store = store
    }

    func snapshot() async throws -> RoomFoldersSnapshot {
        store.snapshot
    }

    func create(name: String, roomID: String?) async throws -> RoomFolderCreateResult {
        let id = store.makeID()
        let snapshot = try store.update { folders in
            try checkName(name)
            guard folders.count < RoomFolderName.maximumFolders else {
                throw RoomFolderError(code: RoomFolderError.tooManyFolders)
            }
            if let roomID {
                folders = folders.map { RoomFolder(id: $0.id, name: $0.name, roomIDs: $0.roomIDs.filter { $0 != roomID }) }
            }
            folders.append(RoomFolder(id: id, name: RoomFolderName.normalized(name), roomIDs: roomID.map { [$0] } ?? []))
        }
        return RoomFolderCreateResult(folderID: id, snapshot: snapshot)
    }

    func rename(folderID: String, name: String) async throws -> RoomFoldersSnapshot {
        try store.update { folders in
            try checkName(name)
            guard let index = folders.firstIndex(where: { $0.id == folderID }) else {
                throw RoomFolderError(code: RoomFolderError.folderNotFound)
            }
            let folder = folders[index]
            folders[index] = RoomFolder(id: folder.id, name: RoomFolderName.normalized(name), roomIDs: folder.roomIDs)
        }
    }

    func delete(folderID: String) async throws -> RoomFoldersSnapshot {
        try store.update { folders in
            folders.removeAll { $0.id == folderID }
        }
    }

    func move(folderID: String, toIndex: Int) async throws -> RoomFoldersSnapshot {
        try store.update { folders in
            guard let index = folders.firstIndex(where: { $0.id == folderID }) else {
                throw RoomFolderError(code: RoomFolderError.folderNotFound)
            }
            let folder = folders.remove(at: index)
            folders.insert(folder, at: min(max(0, toIndex), folders.count))
        }
    }

    func setRoom(roomID: String, folderID: String?) async throws -> RoomFoldersSnapshot {
        try store.update { folders in
            if let folderID, folders.contains(where: { $0.id == folderID }) == false {
                throw RoomFolderError(code: RoomFolderError.folderNotFound)
            }
            folders = folders.map { folder in
                if folder.id == folderID {
                    return folder.roomIDs.contains(roomID)
                        ? folder
                        : RoomFolder(id: folder.id, name: folder.name, roomIDs: folder.roomIDs + [roomID])
                }
                return RoomFolder(id: folder.id, name: folder.name, roomIDs: folder.roomIDs.filter { $0 != roomID })
            }
        }
    }

    private func checkName(_ name: String) throws {
        if let code = store.nameFailureCode {
            throw RoomFolderError(code: code)
        }
        guard RoomFolderName.isValid(name) else {
            throw RoomFolderError(code: RoomFolderError.invalidName)
        }
    }
}
