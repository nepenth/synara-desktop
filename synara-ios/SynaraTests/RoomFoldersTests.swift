import SynaraCore
@testable import Synara
import XCTest

/// Room folders on the Rooms list (`docs/design/room-folders.md`, "iOS UI"):
/// the sections built from Core's folders and section rule, and the folder
/// commands' service layer over a fake core.
final class RoomFoldersTests: XCTestCase {
    private let now = RoomListFixtures.now
    private let workspace = SpaceSummary(id: "!workspace:example.org", name: "Workspace")
    private let ops = SpaceSummary(id: "!ops:example.org", name: "Ops")

    // MARK: Sections

    func testFavoriteWinsOverItsFolderAndKeepsItsFiling() {
        let favorite = room("!fav:example.org", folderID: "work", isFavorite: true)
        let filed = room("!filed:example.org", folderID: "work")
        let plain = room("!plain:example.org")
        let work = RoomFolderSection(id: "work", name: "Work", roomIDs: ["!filed:example.org"])

        let sections = RoomListFavorites.sections(from: [favorite, filed, plain], folders: [work])

        XCTAssertEqual(sections.favorites.map(\.id), ["!fav:example.org"])
        XCTAssertEqual(sections.favorites.first?.folderID, "work", "a favorite keeps its folder")
        XCTAssertEqual(sections.filed["work"]?.map(\.id), ["!filed:example.org"])
        XCTAssertEqual(sections.channels.map(\.id), ["!plain:example.org"], "a filed room leaves Rooms")

        let unfavorited = RoomListFavorites.sections(from: [favorite.withFavorite(false), filed, plain], folders: [work])
        XCTAssertEqual(
            Set(unfavorited.filed["work"]?.map(\.id) ?? []),
            ["!fav:example.org", "!filed:example.org"],
            "unfavoriting returns the room to its folder"
        )
    }

    func testFiledLowPriorityRoomStaysInItsFolderSortedLast() {
        let quiet = room("!quiet:example.org", name: "Aardvark", folderID: "work", isLowPriority: true)
        let busy = room("!busy:example.org", name: "Zulu", folderID: "work", activity: now.addingTimeInterval(-600))
        // Core's orders put low priority last in the folder.
        let work = RoomFolderSection(
            id: "work",
            name: "Work",
            roomIDs: ["!quiet:example.org", "!busy:example.org"],
            recentOrder: ["!busy:example.org", "!quiet:example.org"],
            nameOrder: ["!busy:example.org", "!quiet:example.org"]
        )

        let sections = RoomListFavorites.sections(from: [quiet, busy], folders: [work])
        XCTAssertEqual(sections.lowPriority, [], "a filed low-priority room is not in Low priority")
        let folders = RoomListFolders.sections(
            folders: [work], filed: sections.filed, order: .name, selectedSpaceID: nil, isFiltered: false
        )
        XCTAssertEqual(folders.first?.rooms.map(\.id), ["!busy:example.org", "!quiet:example.org"])
    }

    func testFiledRoomLeavesItsSpaceGroup() {
        let filed = room("!filed:example.org", folderID: "work", spaces: [workspace])
        let grouped = room("!grouped:example.org", spaces: [workspace])
        let work = RoomFolderSection(id: "work", name: "Work", roomIDs: ["!filed:example.org"])

        let sections = RoomListFavorites.sections(from: [filed, grouped], folders: [work])
        let groups = RoomListSpaceGrouping.spaceChannelGroups(from: sections.channels)

        XCTAssertEqual(groups.map(\.space.id), ["!workspace:example.org"])
        XCTAssertEqual(groups.first?.rooms.map(\.id), ["!grouped:example.org"])
        XCTAssertEqual(sections.filed["work"]?.map(\.id), ["!filed:example.org"])
    }

    func testDirectMessagesAndUnknownFoldersStayInTheirDefaultSections() {
        // Core never files a DM; a stale id on a row must not drop it.
        let direct = room("!dm:example.org", kind: .directMessage, folderID: "work")
        let unknown = room("!unknown:example.org", folderID: "gone")
        let work = RoomFolderSection(id: "work", name: "Work")

        XCTAssertEqual(RoomListFolders.placement(of: direct), .default)
        XCTAssertEqual(RoomListFolders.placement(of: unknown), .folder(folderId: "gone"))
        let sections = RoomListFavorites.sections(from: [direct, unknown], folders: [work])
        XCTAssertEqual(sections.directs.map(\.id), ["!dm:example.org"])
        XCTAssertEqual(sections.channels.map(\.id), ["!unknown:example.org"])
        XCTAssertEqual(sections.filed, [:])
        XCTAssertFalse(RoomListFolders.canFile(direct))
        XCTAssertTrue(RoomListFolders.canFile(room("!fav:example.org", isFavorite: true)), "favorites can be filed")
    }

    func testWithoutASpaceEveryFolderShowsAndAnEmptyOneHasNoRooms() {
        let folders = fixtureFolders()
        let rooms = fixtureRooms()
        let shown = folderSections(rooms: rooms, folders: folders, filter: .all, spaceID: nil)

        XCTAssertEqual(shown.map(\.id), ["work", "ops", "side"], "folder order is display order")
        XCTAssertEqual(shown.map(\.index), [0, 1, 2])
        XCTAssertEqual(shown.map(\.folderCount), [3, 3, 3])
        XCTAssertEqual(shown.last?.rooms, [], "the empty folder shows with no rows (\"No rooms\")")
        XCTAssertEqual(shown.map(\.key), ["folder:work", "folder:ops", "folder:side"])
    }

    func testASpaceShowsOnlyItsRoomsAndFoldersThatHoldNoRoomsAnywhere() {
        let shown = folderSections(rooms: fixtureRooms(), folders: fixtureFolders(), filter: .all, spaceID: ops.id)

        // Work holds only Workspace rooms: hidden in Ops. Side projects holds
        // none anywhere: it shows so a new folder is visible.
        XCTAssertEqual(shown.map(\.id), ["ops", "side"])
        XCTAssertEqual(shown.first?.rooms.map(\.id), ["!alerts:example.org"])
        XCTAssertEqual(shown.first?.index, 1, "Move up / Move down keep the position among all folders")
    }

    func testFilterChipsApplyInsideFoldersAndHideFoldersWithoutMatches() {
        let unread = folderSections(rooms: fixtureRooms(), folders: fixtureFolders(), filter: .unread, spaceID: nil)
        XCTAssertEqual(unread.map(\.id), ["work", "ops"], "the empty folder hides while a chip filters")
        XCTAssertEqual(unread.first?.rooms.map(\.id), ["!plan:example.org"], "only matching rows")

        let mentions = folderSections(rooms: fixtureRooms(), folders: fixtureFolders(), filter: .mentions, spaceID: nil)
        XCTAssertEqual(mentions.map(\.id), ["work"])

        let search = folderSections(
            rooms: fixtureRooms(), folders: fixtureFolders(), filter: .all, spaceID: nil, query: "Alerts"
        )
        XCTAssertEqual(search.map(\.id), ["ops"], "a search hides folders without matches too")
    }

    func testRoomsInAFolderFollowTheSharedSortOrder() {
        let folder = RoomFolderSection(
            id: "work",
            name: "Work",
            roomIDs: ["!b:example.org", "!a:example.org"],
            recentOrder: ["!b:example.org", "!a:example.org"],
            nameOrder: ["!a:example.org", "!b:example.org"]
        )
        let rows = [room("!a:example.org", folderID: "work"), room("!b:example.org", folderID: "work")]

        XCTAssertEqual(RoomListFolders.ordered(rows, in: folder, order: .recent).map(\.id), ["!b:example.org", "!a:example.org"])
        XCTAssertEqual(RoomListFolders.ordered(rows, in: folder, order: .name).map(\.id), ["!a:example.org", "!b:example.org"])
        // A row Core did not order still shows, after the ordered ones.
        let extra = room("!c:example.org", folderID: "work")
        XCTAssertEqual(
            RoomListFolders.ordered(rows + [extra], in: folder, order: .name).map(\.id),
            ["!a:example.org", "!b:example.org", "!c:example.org"]
        )
    }

    func testFolderUnreadIsCoresRollupOrTheShownRowsCounts() {
        let rows = [
            room("!m:example.org", folderID: "work", unread: 4, highlight: 2),
            room("!u:example.org", folderID: "work", unread: 1),
            room("!muted:example.org", folderID: "work", unread: 9, muted: true),
        ]
        let folder = RoomFolderSection(
            id: "work",
            name: "Work",
            roomIDs: rows.map(\.id),
            unreadHighlight: 2,
            unreadTotal: 3,
            unreadRoomIDs: ["!m:example.org", "!u:example.org"]
        )

        XCTAssertEqual(RoomListFolders.unread(of: folder, rows: rows), .init(highlight: 2, total: 3), "Core's rollup")
        let subset = RoomListFolders.unread(of: folder, rows: [rows[1], rows[2]])
        XCTAssertEqual(subset.highlight, 0)
        XCTAssertEqual(subset.total, 1, "Core's attention list leaves the muted room out")
        XCTAssertTrue(subset.hasUnread)
        XCTAssertFalse(RoomListFolders.unread(of: folder, rows: [rows[2]]).hasUnread)
    }

    func testFolderCollapseKeysPersistAndKeepUnreadAndSelectedRows() throws {
        let suite = "RoomFoldersCollapse-\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }

        XCTAssertEqual(RoomListSectionKey.folder("Xk2pQ9"), "folder:Xk2pQ9")
        RoomListSectionCollapse.persist([RoomListSectionKey.folder("work"), RoomListSectionKey.channels], defaults: defaults)
        XCTAssertEqual(RoomListSectionCollapse.stored(defaults: defaults), ["folder:work", "channels"])

        let rows = [
            room("!read:example.org"),
            room("!unread:example.org", unread: 1),
            room("!open:example.org"),
        ]
        XCTAssertEqual(
            RoomListSectionCollapse.visibleRooms(rows, isCollapsed: true, selectedRoomID: "!open:example.org").map(\.id),
            ["!unread:example.org", "!open:example.org"]
        )
        XCTAssertEqual(RoomListSectionCollapse.visibleRooms(rows, isCollapsed: false, selectedRoomID: nil), rows)
    }

    func testRowMappingCarriesFolderAndMentionCountFromCore() {
        let rooms = SharedCoreRoomListRows.rooms(
            rooms: [
                SharedCoreRoomListRows.RoomRow(
                    roomId: "!a:example.org", name: "A", avatarUrl: nil, membership: .join, isDirect: false,
                    unreadCount: 5, highlightCount: 3, markedUnread: false, lastActivityTs: nil,
                    lastMessagePreview: nil, lastMessageIsAgentApproval: false, isFavorite: false, isCall: false,
                    hasActiveCall: false, activeCallParticipantCount: 0, directUserId: nil,
                    encryptionStatus: .unknown, folderId: "work"
                ),
            ],
            invites: [],
            spaceParents: []
        )
        XCTAssertEqual(rooms.first?.folderID, "work")
        XCTAssertEqual(rooms.first?.highlightCount, 3)
        XCTAssertEqual(rooms.first?.hasHighlight, true)
        XCTAssertEqual(rooms.first?.withFolderID(nil).folderID, nil)
        XCTAssertEqual(rooms.first?.withFolderID(nil).highlightCount, 3)
    }

    func testFolderSectionMapsCoresDto() {
        let section = RoomFolderSection(RoomFolderSectionDto(
            id: "work", name: "Work", roomIds: ["!a:x", "!b:x"], recentOrder: ["!b:x", "!a:x"],
            nameOrder: ["!a:x", "!b:x"], unreadHighlight: 2, unreadTotal: 5, unreadRoomIds: ["!a:x"]
        ))
        XCTAssertEqual(section.roomIDs, ["!a:x", "!b:x"])
        XCTAssertEqual(section.recentOrder, ["!b:x", "!a:x"])
        XCTAssertEqual(section.nameOrder, ["!a:x", "!b:x"])
        XCTAssertEqual(section.unreadHighlight, 2)
        XCTAssertEqual(section.unreadTotal, 5)
        XCTAssertEqual(section.unreadRoomIDs, ["!a:x"])
    }

    // MARK: Names and copy

    func testFolderNamesFollowCoresRule() {
        XCTAssertEqual(RoomFolderName.normalized("  Side \n  projects  "), "Side projects")
        XCTAssertFalse(RoomFolderName.isValid("   "))
        XCTAssertTrue(RoomFolderName.isValid("W"))
        XCTAssertTrue(RoomFolderName.isValid(String(repeating: "a", count: 64)))
        XCTAssertFalse(RoomFolderName.isValid(String(repeating: "a", count: 65)))
        XCTAssertTrue(RoomFolderName.isValid("  " + String(repeating: "a", count: 64) + "  "), "trimmed first")
        // Core counts characters (Unicode scalars), as desktop does.
        XCTAssertFalse(RoomFolderName.isValid(String(repeating: "👩‍💻", count: 22)), "22 × 3 scalars is 66")
    }

    func testErrorCopyMatchesDesktop() {
        let expected: [String?: String] = [
            "v-room-folders.invalid-name": "Folder names need 1 to 64 characters.",
            "v-room-folders.too-many-folders": "You can have up to 100 folders. Delete one to add another.",
            "v-room-folders.content-too-large": "Your folders are too large to save. Take some rooms out of folders and try again.",
            "v-room-folders.unsupported-version": "Your folders were changed by a newer version of Synara. Update Synara to change them here.",
            "v-room-folders.folder-not-found": "This folder no longer exists.",
            "v-room-folders.room-is-direct": "Direct messages can't go in a folder.",
            "v-room-folders.room-is-space": "Spaces can't go in a folder.",
            "v-room-folders.room-not-joined": "Join this room before putting it in a folder.",
            "v-room-folders.set-failed": "Couldn't save your folders. Try again.",
            nil: "Couldn't save your folders. Try again.",
        ]
        for (code, message) in expected {
            XCTAssertEqual(RoomFolderError(code: code).localizedDescription, message, code ?? "nil")
        }
        XCTAssertTrue(RoomFolderError(code: "v-room-folders.unsupported-version").isReadOnly)
        XCTAssertFalse(RoomFolderError(code: "v-room-folders.invalid-name").isReadOnly)
    }

    // MARK: Service layer

    func testServiceSendsNormalizedNamesAndMapsSnapshots() async throws {
        let core = FakeRoomFoldersCore()
        let service = SharedCoreRoomFoldersService(core: core)

        let created = try await service.create(name: "  Side   projects ", roomID: "!a:example.org")
        XCTAssertEqual(created.folderID, "new")
        XCTAssertEqual(created.snapshot.folders.map(\.name), ["Work"])
        _ = try await service.create(name: "Plain", roomID: nil)
        _ = try await service.rename(folderID: "work", name: " Work  stuff ")
        _ = try await service.move(folderID: "work", toIndex: 2)
        _ = try await service.setRoom(roomID: "!a:example.org", folderID: "work")
        _ = try await service.setRoom(roomID: "!a:example.org", folderID: nil)
        let deleted = try await service.delete(folderID: "work")
        let snapshot = try await service.snapshot()

        XCTAssertEqual(core.calls, [
            "create(Side projects, !a:example.org)",
            "create(Plain, nil)",
            "rename(work, Work stuff)",
            "move(work, 2)",
            "setRoom(!a:example.org, work)",
            "setRoom(!a:example.org, nil)",
            "delete(work)",
            "snapshot",
        ])
        XCTAssertEqual(deleted, RoomFoldersSnapshot(
            readOnly: false,
            folders: [RoomFolder(id: "work", name: "Work", roomIDs: ["!a:example.org"])]
        ))
        XCTAssertFalse(snapshot.readOnly)
    }

    func testServiceMapsCoreErrorsToTheirCodes() async {
        let core = FakeRoomFoldersCore()
        core.failure = RoomFoldersCommandError.Failed(code: "v-room-folders.too-many-folders", description: "raw")
        let service = SharedCoreRoomFoldersService(core: core)
        do {
            _ = try await service.create(name: "One more", roomID: nil)
            XCTFail("expected a failure")
        } catch {
            XCTAssertEqual(error as? RoomFolderError, RoomFolderError(code: "v-room-folders.too-many-folders"))
            XCTAssertEqual(error.localizedDescription, "You can have up to 100 folders. Delete one to add another.")
        }

        core.failure = URLError(.notConnectedToInternet)
        do {
            _ = try await service.delete(folderID: "work")
            XCTFail("expected a failure")
        } catch {
            XCTAssertEqual(error as? RoomFolderError, RoomFolderError(code: nil), "never the raw error")
        }
    }

    func testServiceRejectsANegativeMoveWithoutCallingCore() async {
        let core = FakeRoomFoldersCore()
        let service = SharedCoreRoomFoldersService(core: core)
        do {
            _ = try await service.move(folderID: "work", toIndex: -1)
            XCTFail("expected a failure")
        } catch {
            XCTAssertEqual(core.calls, [])
        }
    }

    func testServiceReportsReadOnlyFromCore() async throws {
        let core = FakeRoomFoldersCore()
        core.readOnly = true
        let snapshot = try await SharedCoreRoomFoldersService(core: core).snapshot()
        XCTAssertTrue(snapshot.readOnly)
    }

    // MARK: Fixture (previews and UI tests)

    func testMockStoreFilesRowsAndBuildsSectionsLikeCore() async throws {
        let store = MockRoomFolderStore.uiTestFixture()
        let list = MockRoomListService(folderStore: store)
        guard case .loaded(let rooms) = await list.loadRooms() else {
            return XCTFail("expected rooms")
        }
        XCTAssertEqual(rooms.first { $0.id == "!project:matrix.org" }?.folderID, "work")
        XCTAssertNil(rooms.first { $0.id == "!alice:matrix.org" }?.folderID)
        let sections = list.roomFolderSections()
        XCTAssertEqual(sections.map(\.name), ["Work", "Ops watch", "Side projects"])
        XCTAssertEqual(sections[0].unreadHighlight, 1, "Product has a mention")
        XCTAssertEqual(sections[2].roomIDs, [])

        let service = MockRoomFoldersService(store: store)
        _ = try await service.setRoom(roomID: "!general:matrix.org", folderID: "side")
        XCTAssertEqual(list.roomFolderSections()[2].roomIDs, ["!general:matrix.org"])
        _ = try await service.move(folderID: "side", toIndex: 0)
        XCTAssertEqual(list.roomFolderSections().map(\.id), ["side", "work", "ops"])
        _ = try await service.delete(folderID: "side")
        guard case .loaded(let after) = await list.loadRooms() else {
            return XCTFail("expected rooms")
        }
        XCTAssertNil(after.first { $0.id == "!general:matrix.org" }?.folderID, "its rooms go back to Rooms")
    }

    // MARK: Helpers

    private func fixtureFolders() -> [RoomFolderSection] {
        [
            RoomFolderSection(id: "work", name: "Work", roomIDs: ["!plan:example.org", "!docs:example.org"]),
            RoomFolderSection(id: "ops", name: "Ops", roomIDs: ["!alerts:example.org"]),
            RoomFolderSection(id: "side", name: "Side projects"),
        ]
    }

    private func fixtureRooms() -> [RoomSummary] {
        [
            room("!plan:example.org", name: "Plan", folderID: "work", spaces: [workspace], unread: 2, highlight: 1),
            room("!docs:example.org", name: "Docs", folderID: "work", spaces: [workspace]),
            room("!alerts:example.org", name: "Alerts", folderID: "ops", spaces: [ops], unread: 1),
            room("!general:example.org", name: "General", spaces: [ops]),
        ]
    }

    private func folderSections(
        rooms: [RoomSummary],
        folders: [RoomFolderSection],
        filter: RoomListScopeFilter.Kind,
        spaceID: String?,
        query: String = ""
    ) -> [RoomListFolders.Section] {
        let filtered = RoomListScopeFilter.filteredRooms(
            from: rooms, filter: filter, selectedSpaceID: spaceID, searchQuery: query
        )
        let sections = RoomListFavorites.sections(from: filtered, folders: folders)
        return RoomListFolders.sections(
            folders: folders,
            filed: sections.filed,
            order: .recent,
            selectedSpaceID: spaceID,
            isFiltered: filter != .all || query.isEmpty == false
        )
    }

    private func room(
        _ id: String,
        name: String? = nil,
        kind: RoomSummary.RoomKind = .room,
        folderID: String? = nil,
        isFavorite: Bool = false,
        isLowPriority: Bool = false,
        spaces: [SpaceSummary] = [],
        activity: Date? = nil,
        unread: Int = 0,
        highlight: Int = 0,
        muted: Bool = false
    ) -> RoomSummary {
        RoomSummary(
            id: id,
            name: name ?? id,
            lastMessagePreview: "",
            unreadCount: unread,
            hasHighlight: highlight > 0,
            kind: kind,
            membership: .joined,
            lastActivityAt: activity ?? now,
            parentSpaces: spaces,
            isFavorite: isFavorite,
            isLowPriority: isLowPriority,
            notificationMode: muted ? .mute : .default,
            folderID: folderID,
            highlightCount: highlight
        )
    }
}

private extension RoomSummary {
    func withFavorite(_ isFavorite: Bool) -> RoomSummary {
        RoomSummary(
            id: id,
            name: name,
            lastMessagePreview: lastMessagePreview,
            unreadCount: unreadCount,
            hasHighlight: hasHighlight,
            kind: kind,
            membership: membership,
            lastActivityAt: lastActivityAt,
            parentSpaces: parentSpaces,
            isFavorite: isFavorite,
            isLowPriority: isLowPriority,
            folderID: folderID,
            highlightCount: highlightCount
        )
    }
}

/// Records each folder command and answers with a one-folder snapshot.
private final class FakeRoomFoldersCore: RoomFoldersCore {
    var calls: [String] = []
    var failure: Error?
    var readOnly = false

    private var snapshotDto: RoomFoldersSnapshotDto {
        RoomFoldersSnapshotDto(
            sessionGeneration: 1,
            version: readOnly ? 2 : 1,
            readOnly: readOnly,
            folders: [RoomFolderDto(id: "work", name: "Work", roomIds: ["!a:example.org"])]
        )
    }

    private func answer(_ call: String) throws -> RoomFoldersSnapshotDto {
        if let failure {
            throw failure
        }
        calls.append(call)
        return snapshotDto
    }

    func roomFoldersSnapshot() async throws -> RoomFoldersSnapshotDto {
        try answer("snapshot")
    }

    func roomFoldersCreate(name: String, roomId: String?) async throws -> RoomFolderCreateDto {
        RoomFolderCreateDto(folderId: "new", snapshot: try answer("create(\(name), \(roomId ?? "nil"))"))
    }

    func roomFoldersRename(folderId: String, name: String) async throws -> RoomFoldersSnapshotDto {
        try answer("rename(\(folderId), \(name))")
    }

    func roomFoldersDelete(folderId: String) async throws -> RoomFoldersSnapshotDto {
        try answer("delete(\(folderId))")
    }

    func roomFoldersMove(folderId: String, toIndex: UInt32) async throws -> RoomFoldersSnapshotDto {
        try answer("move(\(folderId), \(toIndex))")
    }

    func roomFoldersSetRoom(roomId: String, folderId: String?) async throws -> RoomFoldersSnapshotDto {
        try answer("setRoom(\(roomId), \(folderId ?? "nil"))")
    }
}
