import XCTest
@testable import Synara
import SynaraCore

final class SynaraCoreBindingsTests: XCTestCase {
    func testBindingScaffoldVersionExecutesGeneratedRustFFI() {
        let version = bindingScaffoldVersion()

        XCTAssertFalse(version.isEmpty)
    }

    func testSharedCoreConstructsOverGeneratedRustFFI() {
        let core = SharedCore()

        XCTAssertNotNil(core)
    }

    func testSharedCoreAcceptsInMemorySecretStore() {
        let core = SharedCore(store: InMemoryIosSecretVault())

        XCTAssertNotNil(core)
    }

    func testSharedCoreRestoreWithoutVaultFailsClosed() async {
        let core = SharedCore()
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3b-no-vault", isDirectory: true)

        do {
            _ = try await core.restorePersistedSession(
                userId: "@alice:example.org",
                homeserverUrl: "https://matrix.example.org",
                storeRoot: storeRoot.path
            )
            XCTFail("Fail-closed SharedCore must not restore")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3b-secret-vault-unavailable"))
            XCTAssertFalse(publicError.contains("p4-s3b-session-material-missing"))
            for forbidden in ["@alice:example.org", "matrix.example.org", "password", storeRoot.path] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRestoreRejectsHostileIdentityWithoutEcho() async {
        let core = SharedCore(store: InMemoryIosSecretVault())
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3b-hostile", isDirectory: true)
        let hostileURL = "https://user:secret@evil.example/?password=hunter2"

        do {
            _ = try await SharedCoreSessionRestore.restorePersistedSession(
                userID: "not-a-user",
                homeserverURL: hostileURL,
                storeRoot: storeRoot,
                core: core
            )
            XCTFail("Hostile identity must fail closed")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3b-identity-invalid"))
            for forbidden in [hostileURL, "secret", "hunter2", "evil.example", "password"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRestoreHoldsInstanceAcrossCalls() async {
        let vault = InMemoryIosSecretVault()
        let core = SharedCore(store: vault)
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3b-hold-core", isDirectory: true)

        do {
            _ = try await SharedCoreSessionRestore.restorePersistedSession(
                userID: "@alice:example.org",
                homeserverURL: "https://matrix.example.org",
                storeRoot: storeRoot,
                core: core
            )
            XCTFail("Empty vault must not restore")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3b-session-material-missing"))
        }

        do {
            _ = try await SharedCoreSessionRestore.restorePersistedSession(
                userID: "@alice:example.org",
                homeserverURL: "https://matrix.example.org",
                storeRoot: storeRoot,
                core: core
            )
            XCTFail("Second call on the same instance must still run")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3b-session-material-missing"))
            XCTAssertFalse(publicError.contains("p4-s3b-secret-vault-unavailable"))
        }
    }

    func testSharedCoreLoginWithoutVaultFailsClosed() async {
        let core = SharedCore()
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3c-no-vault", isDirectory: true)
        let password = "hunter2-s3c-secret"

        do {
            _ = try await SharedCoreSessionLogin.loginWithPassword(
                userID: "@alice:example.org",
                homeserverURL: "https://matrix.example.org",
                storeRoot: storeRoot,
                password: password,
                core: core
            )
            XCTFail("Fail-closed SharedCore must not login")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3c-secret-vault-unavailable"))
            for forbidden in [password, "hunter2", "@alice:example.org", "password"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreLoginRejectsHostileIdentityWithoutEchoingPassword() async {
        let core = SharedCore(store: InMemoryIosSecretVault())
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3c-hostile", isDirectory: true)
        let hostileURL = "https://user:secret@evil.example/?password=hunter2"
        let password = "s3c-password-must-not-leak"

        do {
            _ = try await SharedCoreSessionLogin.loginWithPassword(
                userID: "not-a-user",
                homeserverURL: hostileURL,
                storeRoot: storeRoot,
                password: password,
                core: core
            )
            XCTFail("Hostile identity must fail closed")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3c-identity-invalid"))
            for forbidden in [password, hostileURL, "secret", "hunter2", "evil.example"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreAttachWithoutSessionFailsClosed() async {
        let core = SharedCore()

        do {
            _ = try await SharedCoreSessionAttach.attachSessionOwners(core: core)
            XCTFail("Fail-closed SharedCore must not attach without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3d-session-missing"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRoomListWithoutSessionFailsClosed() async {
        let core = SharedCore()

        do {
            _ = try await SharedCoreRoomList.roomListSnapshot(core: core)
            XCTFail("Fail-closed SharedCore must not snapshot rooms without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-list-snapshot-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreInvitesWithoutSessionFailsClosed() async {
        let core = SharedCore()

        do {
            _ = try await SharedCoreInvites.invitesSnapshot(core: core)
            XCTFail("Fail-closed SharedCore must not snapshot invites without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-invites-snapshot-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreTimelineWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let position = TimelineOpenPositionDto(
            kind: "live_bottom",
            atBottom: false,
            restoredAnchorEventId: nil,
            liveTailEventId: nil,
            updatedAtMs: nil,
            eventId: nil
        )

        do {
            _ = try await SharedCoreTimeline.timelineOpen(
                core: core,
                roomId: "!missing:example.org",
                position: position
            )
            XCTFail("Fail-closed SharedCore must not open a timeline without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-open-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimeline.timelineClose(core: core, streamId: "view-1")
            XCTFail("Fail-closed SharedCore must not close a timeline without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-close-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimeline.timelinePaginate(
                core: core,
                streamId: "view-1",
                direction: "backwards"
            )
            XCTFail("Fail-closed SharedCore must not paginate a timeline without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-paginate-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreTypingPresenceWithoutSessionFailsClosed() async {
        let core = SharedCore()

        do {
            _ = try await SharedCoreTypingPresence.typingSnapshot(core: core)
            XCTFail("Fail-closed SharedCore must not snapshot typing without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-typing-snapshot-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            try await SharedCoreTypingPresence.typingSet(
                core: core,
                roomId: "!r:example.org",
                typing: true
            )
            XCTFail("Fail-closed SharedCore must not set typing without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-typing-set-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTypingPresence.presenceSnapshot(
                core: core,
                userId: "@bob:example.org"
            )
            XCTFail("Fail-closed SharedCore must not snapshot presence without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-presence-snapshot-no-session"))
            for forbidden in ["password", "syt_", "@bob:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTypingPresence.presenceSubscribe(
                core: core,
                userId: "@bob:example.org"
            )
            XCTFail("Fail-closed SharedCore must not subscribe presence without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-presence-subscribe-no-session"))
            for forbidden in ["password", "syt_", "@bob:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            try await SharedCoreTypingPresence.presenceUnsubscribe(
                core: core,
                subscriptionId: "presence-1-0"
            )
            XCTFail("Fail-closed SharedCore must not unsubscribe presence without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-presence-unsubscribe-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreVerificationListWithoutSessionFailsClosed() async {
        let core = SharedCore()

        do {
            _ = try await SharedCoreVerificationList.verificationList(core: core)
            XCTFail("Fail-closed SharedCore must not list verification without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-verification-list-no-session"))
            for forbidden in ["password", "syt_", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreVerificationSasWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let deviceId = "DEVICE_S9_BOB"
        let flowId = "$FLOW_S9_BOB"

        do {
            _ = try await SharedCoreVerificationSas.verificationStart(core: core, deviceId: deviceId)
            XCTFail("Fail-closed SharedCore must not start verification without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-verification-start-no-session"))
            for forbidden in ["password", "syt_", "token", deviceId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreVerificationSas.verificationAccept(core: core, flowId: flowId)
            XCTFail("Fail-closed SharedCore must not accept verification without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-verification-accept-no-session"))
            for forbidden in ["password", "syt_", "token", flowId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreVerificationSas.verificationBeginSas(core: core, flowId: flowId)
            XCTFail("Fail-closed SharedCore must not begin SAS without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-verification-begin-sas-no-session"))
            for forbidden in ["password", "syt_", "token", flowId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreVerificationSas.verificationConfirm(core: core, flowId: flowId)
            XCTFail("Fail-closed SharedCore must not confirm verification without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-verification-confirm-no-session"))
            for forbidden in ["password", "syt_", "token", flowId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreVerificationSas.verificationMismatch(core: core, flowId: flowId)
            XCTFail("Fail-closed SharedCore must not mismatch verification without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-verification-mismatch-no-session"))
            for forbidden in ["password", "syt_", "token", flowId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreVerificationSas.verificationCancel(core: core, flowId: flowId)
            XCTFail("Fail-closed SharedCore must not cancel verification without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-verification-cancel-no-session"))
            for forbidden in ["password", "syt_", "token", flowId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            try await SharedCoreVerificationSas.verificationDismiss(core: core, flowId: flowId)
            XCTFail("Fail-closed SharedCore must not dismiss verification without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-verification-dismiss-no-session"))
            for forbidden in ["password", "syt_", "token", flowId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreDevicesWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let deviceId = "DEVICE_S9_2_BOB"
        let displayName = "Bob Phone"

        do {
            _ = try await SharedCoreDevices.deviceSnapshot(core: core)
            XCTFail("Fail-closed SharedCore must not snapshot devices without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-device-snapshot-no-session"))
            for forbidden in ["syt_", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreDevices.deviceRename(core: core, deviceId: deviceId, displayName: displayName)
            XCTFail("Fail-closed SharedCore must not rename a device without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-device-rename-no-session"))
            for forbidden in ["syt_", "token", deviceId, displayName] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreDevices.deviceDeleteStart(core: core, deviceIds: [deviceId])
            XCTFail("Fail-closed SharedCore must not start device delete without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-device-delete-start-no-session"))
            for forbidden in ["syt_", "token", deviceId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            try await SharedCoreDevices.deviceDeleteCancel(core: core, operationId: 9, sessionGeneration: 1)
            XCTFail("Fail-closed SharedCore must not cancel device delete without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-device-delete-cancel-no-session"))
            for forbidden in ["syt_", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreJoinRulesWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s93join:example.org"

        do {
            _ = try await SharedCoreJoinRules.roomJoinRuleSnapshot(
                core: core,
                roomId: roomId,
                sessionGeneration: 1
            )
            XCTFail("Fail-closed SharedCore must not snapshot a join rule without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-join-rule-snapshot-no-session"))
            for forbidden in ["syt_", "token", roomId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreImagePacksWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s94pack:example.org"
        let stateKey = "s94state"
        let contentJson = #"{"pack":{"display_name":"S94"},"images":{"smile":{"url":"mxc://example.org/abc"}}}"#

        do {
            _ = try await SharedCoreImagePacks.getGlobalImagePacks(core: core)
            XCTFail("Fail-closed SharedCore must not snapshot global image packs without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-global-image-packs-no-session"))
            for forbidden in ["syt_", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreImagePacks.getUserImagePack(core: core)
            XCTFail("Fail-closed SharedCore must not snapshot the user image pack without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-user-image-pack-no-session"))
            for forbidden in ["syt_", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreImagePacks.getRoomImagePacks(core: core, roomId: roomId)
            XCTFail("Fail-closed SharedCore must not snapshot room image packs without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-image-packs-no-session"))
            for forbidden in ["syt_", "token", roomId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreImagePacks.setUserImagePack(core: core, contentJson: contentJson)
            XCTFail("Fail-closed SharedCore must not set the user image pack without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-set-user-image-pack-no-session"))
            for forbidden in ["syt_", "token", "mxc://", contentJson] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreImagePacks.setGlobalImagePacks(core: core, contentJson: contentJson)
            XCTFail("Fail-closed SharedCore must not set global image packs without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-set-global-image-packs-no-session"))
            for forbidden in ["syt_", "token", "mxc://", contentJson] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreImagePacks.setRoomImagePack(
                core: core,
                roomId: roomId,
                stateKey: stateKey,
                contentJson: contentJson
            )
            XCTFail("Fail-closed SharedCore must not set a room image pack without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-set-room-image-pack-no-session"))
            for forbidden in ["syt_", "token", roomId, stateKey, "mxc://", contentJson] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreLaterWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let item = LaterItemDto(
            id: "later-s95",
            kind: "saved",
            roomId: "!s95later:example.org",
            eventId: "$s95event",
            createdAt: 1_700_000_000_000,
            dueTs: nil,
            remindedAt: nil,
            completedAt: nil
        )

        do {
            _ = try await SharedCoreLater.laterSnapshot(core: core)
            XCTFail("Fail-closed SharedCore must not snapshot later without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-later-snapshot-no-session"))
            for forbidden in ["syt_", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreLater.laterUpsert(core: core, item: item)
            XCTFail("Fail-closed SharedCore must not upsert later without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-later-upsert-no-session"))
            for forbidden in ["syt_", "token", item.id, item.roomId, item.eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreLater.laterComplete(core: core, itemId: item.id, completedAt: 1_700_000_100_000)
            XCTFail("Fail-closed SharedCore must not complete later without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-later-complete-no-session"))
            for forbidden in ["syt_", "token", item.id] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreLater.laterSnooze(core: core, itemId: item.id, dueTs: 1_700_000_200_000)
            XCTFail("Fail-closed SharedCore must not snooze later without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-later-snooze-no-session"))
            for forbidden in ["syt_", "token", item.id] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreLater.laterClearCompleted(core: core)
            XCTFail("Fail-closed SharedCore must not clear completed later without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-later-clear-completed-no-session"))
            for forbidden in ["syt_", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreLater.laterMarkReminded(core: core, itemId: item.id, remindedAt: 1_700_000_300_000)
            XCTFail("Fail-closed SharedCore must not mark later reminded without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-later-mark-reminded-no-session"))
            for forbidden in ["syt_", "token", item.id] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreMDirectWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s96dm:example.org"
        let userId = "@bob:example.org"

        do {
            _ = try await SharedCoreMDirect.mdirectSnapshot(core: core)
            XCTFail("Fail-closed SharedCore must not snapshot m.direct without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-mdirect-snapshot-no-session"))
            for forbidden in ["syt_", "token", roomId, userId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreMDirect.mdirectAdd(core: core, roomId: roomId, userId: userId)
            XCTFail("Fail-closed SharedCore must not add m.direct without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-mdirect-add-no-session"))
            for forbidden in ["syt_", "token", roomId, userId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreMDirect.mdirectRemove(core: core, roomId: roomId)
            XCTFail("Fail-closed SharedCore must not remove m.direct without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-mdirect-remove-no-session"))
            for forbidden in ["syt_", "token", roomId, userId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRoomNotesWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let body = "secret note body text"
        let item = RoomNoteItemDto(
            id: "note-s97",
            kind: "note",
            roomId: "!s97notes:example.org",
            createdAt: 1_700_000_000_000,
            updatedAt: 1_700_000_000_000,
            body: body,
            completedAt: nil,
            order: nil,
            eventId: nil,
            eventTs: nil,
            sender: nil
        )

        do {
            _ = try await SharedCoreRoomNotes.roomNotesSnapshot(core: core)
            XCTFail("Fail-closed SharedCore must not snapshot room notes without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-notes-snapshot-no-session"))
            for forbidden in ["syt_", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomNotes.roomNotesUpsert(core: core, item: item)
            XCTFail("Fail-closed SharedCore must not upsert room notes without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-notes-upsert-no-session"))
            for forbidden in ["syt_", "token", item.id, item.roomId, body] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomNotes.roomNotesDelete(core: core, roomId: item.roomId, itemId: item.id)
            XCTFail("Fail-closed SharedCore must not delete room notes without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-notes-delete-no-session"))
            for forbidden in ["syt_", "token", item.id, item.roomId, body] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomNotes.roomNotesCompleteTodo(
                core: core,
                roomId: item.roomId,
                itemId: item.id,
                completed: true
            )
            XCTFail("Fail-closed SharedCore must not complete room-note todos without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-notes-complete-todo-no-session"))
            for forbidden in ["syt_", "token", item.id, item.roomId, body] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomNotes.roomNotesMoveTodo(
                core: core,
                roomId: item.roomId,
                itemId: item.id,
                direction: "up"
            )
            XCTFail("Fail-closed SharedCore must not move room-note todos without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-notes-move-todo-no-session"))
            for forbidden in ["syt_", "token", item.id, item.roomId, body] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testRegisterFlowsRejectsHostileURLWithStaticPrivacySafeError() async {
        let hostileURL = "https://user:secret@example.invalid"

        do {
            _ = try await SynaraCore.registerFlows(homeserverUrl: hostileURL)
            XCTFail("Hostile registration-flow URL must fail before a request")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p3.1-invalid-homeserver-url"))
            XCTAssertTrue(publicError.contains("The homeserver URL is invalid."))
            for forbidden in [hostileURL, "user:secret", "secret", "example.invalid"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSessionProjectionFacadeExecutesOpenSnapshotAndCloseOverGeneratedRustFFI() async throws {
        let core = SessionProjectionCore()
        let expected = SessionProjection(
            generation: 7,
            userId: "@alice:matrix.org",
            deviceId: "SYNARA-IOS-DEVICE",
            homeserverUrl: "https://matrix.org",
            lifecycle: .ready,
            cryptoReady: true
        )

        try await core.open(projection: expected)
        let openedSnapshot = try await core.sessionSnapshot()
        XCTAssertEqual(openedSnapshot, Optional(expected))

        try await core.close()
        let closedSnapshot = try await core.sessionSnapshot()
        XCTAssertNil(closedSnapshot)
    }

    func testSessionProjectionFacadeRejectsHostileValuesWithStaticError() async {
        let core = SessionProjectionCore()
        let hostileURL = "https://user:access-token@private.example/?password=secret"
        let invalid = SessionProjection(
            generation: 1,
            userId: "@alice:matrix.org",
            deviceId: "SYNARA-IOS-DEVICE",
            homeserverUrl: hostileURL,
            lifecycle: .ready,
            cryptoReady: true
        )

        do {
            try await core.open(projection: invalid)
            XCTFail("Hostile projection must fail before Core is opened")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4.3-session-projection-rejected"))
            XCTAssertTrue(publicError.contains("The session projection is invalid."))
            for forbidden in [hostileURL, "access-token", "password", "secret", "private.example"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testProductionMirrorReadsReadyCoreIdentityThenClearsOnClose() async {
        let mirror = MatrixSessionProjectionMirror()
        let expected = CoreSessionIdentity(
            userID: "@alice:matrix.org",
            deviceID: "SYNARA-IOS-DEVICE",
            homeserverURL: "https://matrix.org"
        )

        let beforeOpen = await mirror.coreSessionIdentity()
        XCTAssertNil(beforeOpen)

        await mirror.openAfterInstalledClient(
            userID: expected.userID,
            deviceID: expected.deviceID,
            homeserverURL: expected.homeserverURL,
            cryptoReady: true
        )

        let afterOpen = await mirror.coreSessionIdentity()
        XCTAssertEqual(afterOpen, expected)

        await mirror.closeBeforeSDKWipe()

        let afterClose = await mirror.coreSessionIdentity()
        XCTAssertNil(afterClose)
    }

    func testMirrorFailsClosedForMismatchedNonReadyAndMissingCoreSnapshots() async throws {
        let core = SessionProjectionCore()
        let mirror = MatrixSessionProjectionMirror(core: core)

        await mirror.openAfterInstalledClient(
            userID: "@alice:matrix.org",
            deviceID: "SYNARA-IOS-DEVICE",
            homeserverURL: "https://matrix.org",
            cryptoReady: true
        )

        try await core.open(
            projection: SessionProjection(
                generation: 2,
                userId: "@mallory:matrix.org",
                deviceId: "OTHER-DEVICE",
                homeserverUrl: "https://matrix.org",
                lifecycle: .ready,
                cryptoReady: true
            )
        )
        let mismatchedIdentity = await mirror.coreSessionIdentity()
        XCTAssertNil(mismatchedIdentity)

        try await core.open(
            projection: SessionProjection(
                generation: 3,
                userId: "@alice:matrix.org",
                deviceId: "SYNARA-IOS-DEVICE",
                homeserverUrl: "https://matrix.org",
                lifecycle: .syncing,
                cryptoReady: true
            )
        )
        let nonReadyIdentity = await mirror.coreSessionIdentity()
        XCTAssertNil(nonReadyIdentity)

        try await core.close()
        let missingIdentity = await mirror.coreSessionIdentity()
        XCTAssertNil(missingIdentity)
    }

    func testMirrorDoesNotPublishAnIdentityWhenCoreOpenFails() async {
        let mirror = MatrixSessionProjectionMirror()

        await mirror.openAfterInstalledClient(
            userID: "@alice:matrix.org",
            deviceID: "SYNARA-IOS-DEVICE",
            homeserverURL: "https://user:access-token@private.example/?password=secret",
            cryptoReady: true
        )

        let identity = await mirror.coreSessionIdentity()
        XCTAssertNil(identity)
    }
}
