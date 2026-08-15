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
        // UniFFI 0.28 Swift emits the named UDL constructor as a static
        // factory, not a second init(store:).
        let core = SharedCore.newWithSecretStore(store: InMemoryIosSecretVault())

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
        let core = SharedCore.newWithSecretStore(store: InMemoryIosSecretVault())
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
        let core = SharedCore.newWithSecretStore(store: vault)
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
        let core = SharedCore.newWithSecretStore(store: InMemoryIosSecretVault())
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

    func testSharedCoreOwnProfileWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let displayName = "S98 Secret Display Name"
        let mxc = "mxc://example.org/s98SecretAvatarId"

        do {
            _ = try await SharedCoreOwnProfile.setOwnDisplayName(core: core, displayName: displayName)
            XCTFail("Fail-closed SharedCore must not set own display name without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-set-own-display-name-no-session"))
            for forbidden in ["syt_", "token", displayName] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreOwnProfile.setOwnAvatar(core: core, mxc: mxc)
            XCTFail("Fail-closed SharedCore must not set own avatar without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-set-own-avatar-no-session"))
            for forbidden in ["syt_", "token", mxc] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRoomProfileWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s99SecretRoom:example.org"
        let name = "S99 Secret Room Name"
        let topic = "S99 Secret Room Topic"
        let mxc = "mxc://example.org/s99SecretRoomAvatarId"

        do {
            _ = try await SharedCoreRoomProfile.setRoomName(core: core, roomId: roomId, name: name)
            XCTFail("Fail-closed SharedCore must not set room name without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-set-room-name-no-session"))
            for forbidden in ["syt_", "token", roomId, name] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomProfile.setRoomTopic(core: core, roomId: roomId, topic: topic)
            XCTFail("Fail-closed SharedCore must not set room topic without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-set-room-topic-no-session"))
            for forbidden in ["syt_", "token", roomId, topic] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomProfile.setRoomAvatar(core: core, roomId: roomId, mxc: mxc)
            XCTFail("Fail-closed SharedCore must not set room avatar without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-set-room-avatar-no-session"))
            for forbidden in ["syt_", "token", roomId, mxc] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreDirectoryVisibilityWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s910SecretRoom:example.org"
        let visibility = "public"

        do {
            _ = try await SharedCoreDirectoryVisibility.getRoomDirectoryVisibility(
                core: core,
                roomId: roomId,
                sessionGeneration: 1
            )
            XCTFail("Fail-closed SharedCore must not get directory visibility without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-get-room-directory-visibility-no-session"))
            for forbidden in ["syt_", "token", roomId, visibility] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreDirectoryVisibility.setRoomDirectoryVisibility(
                core: core,
                roomId: roomId,
                sessionGeneration: 1,
                visibility: visibility
            )
            XCTFail("Fail-closed SharedCore must not set directory visibility without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-set-room-directory-visibility-no-session"))
            for forbidden in ["syt_", "token", roomId, visibility] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreDirectorySearchWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let term = "s911SecretTerm"
        let server = "s911.secret.example.org"

        do {
            _ = try await SharedCoreDirectorySearch.roomDirectoryProtocols(core: core)
            XCTFail("Fail-closed SharedCore must not list directory protocols without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-directory-protocols-no-session"))
            for forbidden in ["syt_", "token", term, server] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreDirectorySearch.roomDirectorySearch(
                core: core,
                sessionGeneration: 1,
                requestId: 1,
                serverName: server,
                term: term,
                roomType: nil,
                thirdPartyInstanceId: nil,
                limit: 20,
                since: nil
            )
            XCTFail("Fail-closed SharedCore must not search the room directory without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-directory-search-no-session"))
            for forbidden in ["syt_", "token", term, server] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreDirectorySearch.roomDirectoryCancel(
                core: core,
                sessionGeneration: 1,
                requestId: 1
            )
            XCTFail("Fail-closed SharedCore must not cancel directory search without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-directory-cancel-no-session"))
            for forbidden in ["syt_", "token", term, server] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRoomLeaveJoinWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s912SecretRoom:example.org"
        let alias = "#s912SecretAlias:example.org"
        let via = "s912.secret.example.org"

        do {
            _ = try await SharedCoreRoomLeaveJoin.roomLeave(core: core, roomId: roomId)
            XCTFail("Fail-closed SharedCore must not leave a room without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-leave-no-session"))
            for forbidden in ["syt_", "token", roomId, alias, via] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomLeaveJoin.roomJoin(
                core: core,
                roomIdOrAlias: alias,
                viaServers: [via]
            )
            XCTFail("Fail-closed SharedCore must not join a room without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-join-no-session"))
            for forbidden in ["syt_", "token", roomId, alias, via] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRoomModerationWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s913SecretRoom:example.org"
        let userId = "@s913SecretUser:example.org"
        let reason = "s913SecretReason"

        do {
            _ = try await SharedCoreRoomModeration.roomInvite(
                core: core,
                roomId: roomId,
                userId: userId,
                reason: reason
            )
            XCTFail("Fail-closed SharedCore must not invite without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-invite-no-session"))
            for forbidden in ["syt_", "token", roomId, userId, reason] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomModeration.roomKick(
                core: core,
                roomId: roomId,
                userId: userId,
                reason: reason
            )
            XCTFail("Fail-closed SharedCore must not kick without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-kick-no-session"))
            for forbidden in ["syt_", "token", roomId, userId, reason] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomModeration.roomBan(
                core: core,
                roomId: roomId,
                userId: userId,
                reason: reason
            )
            XCTFail("Fail-closed SharedCore must not ban without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-ban-no-session"))
            for forbidden in ["syt_", "token", roomId, userId, reason] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomModeration.roomUnban(
                core: core,
                roomId: roomId,
                userId: userId
            )
            XCTFail("Fail-closed SharedCore must not unban without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-unban-no-session"))
            for forbidden in ["syt_", "token", roomId, userId, reason] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRoomPowerLevelsWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s914SecretRoom:example.org"
        let userId = "@s914SecretUser:example.org"
        let content = "{\"users\":{\"@s914SecretUser:example.org\":50}}"
        let tags = "{\"50\":{\"name\":\"s914SecretTag\"}}"

        do {
            _ = try await SharedCoreRoomPowerLevels.roomSetPowerLevel(
                core: core,
                roomId: roomId,
                userId: userId,
                powerLevel: 914
            )
            XCTFail("Fail-closed SharedCore must not set a power level without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-set-power-level-no-session"))
            for forbidden in ["syt_", "token", roomId, userId, "914"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomPowerLevels.roomSetPowerLevels(
                core: core,
                roomId: roomId,
                contentJson: content
            )
            XCTFail("Fail-closed SharedCore must not set power levels without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-set-power-levels-no-session"))
            for forbidden in ["syt_", "token", roomId, userId, content] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomPowerLevels.roomSetPowerLevelTags(
                core: core,
                roomId: roomId,
                contentJson: tags
            )
            XCTFail("Fail-closed SharedCore must not set power-level tags without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-set-power-level-tags-no-session"))
            for forbidden in ["syt_", "token", roomId, tags] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRoomCreateWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let name = "s915SecretName"
        let topic = "s915SecretTopic"
        let alias = "s915secretalias"
        let invite = "s915SecretInvite"
        let parent = "!s915SecretParent:example.org"
        let request = RoomCreateRequestDto(
            name: name,
            topic: topic,
            roomAliasName: alias,
            visibility: "private",
            preset: "private_chat",
            isDirect: false,
            encryption: false,
            invite: [invite],
            roomVersion: nil,
            joinRule: nil,
            knock: false,
            parentRoomId: parent
        )

        do {
            _ = try await SharedCoreRoomCreate.roomCreate(core: core, request: request)
            XCTFail("Fail-closed SharedCore must not create a room without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-create-no-session"))
            for forbidden in ["syt_", "token", name, topic, alias, invite, parent] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRoomMembersSnapshotsWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s916SecretRoom:example.org"
        let member = "@s916SecretMember:example.org"

        do {
            _ = try await SharedCoreRoomMembersSnapshots.roomMembersSnapshot(
                core: core,
                roomId: roomId
            )
            XCTFail("Fail-closed SharedCore must not load members without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-members-snapshot-no-session"))
            for forbidden in ["syt_", "token", roomId, member] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomMembersSnapshots.roomPowerLevelsSnapshot(
                core: core,
                roomId: roomId
            )
            XCTFail("Fail-closed SharedCore must not load power levels without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-power-levels-snapshot-no-session"))
            for forbidden in ["syt_", "token", roomId, member] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomMembersSnapshots.roomCreatorsSnapshot(
                core: core,
                roomId: roomId
            )
            XCTFail("Fail-closed SharedCore must not load creators without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-creators-snapshot-no-session"))
            for forbidden in ["syt_", "token", roomId, member] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreRoomMembersSnapshots.roomPowerLevelTagsSnapshot(
                core: core,
                roomId: roomId
            )
            XCTFail("Fail-closed SharedCore must not load power-level tags without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-power-level-tags-snapshot-no-session"))
            for forbidden in ["syt_", "token", roomId, member] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreSpacesWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s917SecretRoom:example.org"
        let parentId = "!s917SecretParent:example.org"
        let childId = "!s917SecretChild:example.org"
        let via = "s917.secret.example.org"
        let order = "s917SecretOrder"

        do {
            _ = try await SharedCoreSpaces.spaceParentsSnapshot(core: core)
            XCTFail("Fail-closed SharedCore must not load space parents without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-space-parents-snapshot-no-session"))
            for forbidden in ["syt_", "token", roomId, parentId, childId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreSpaces.spaceHierarchySnapshot(core: core, roomId: roomId)
            XCTFail("Fail-closed SharedCore must not load space hierarchy without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-space-hierarchy-snapshot-no-session"))
            for forbidden in ["syt_", "token", roomId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreSpaces.spaceChildrenSnapshot(core: core)
            XCTFail("Fail-closed SharedCore must not load space children without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-space-children-snapshot-no-session"))
            for forbidden in ["syt_", "token", roomId, parentId, childId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreSpaces.spaceChildSet(
                core: core,
                parentId: parentId,
                childId: childId,
                via: [via],
                order: order,
                suggested: true
            )
            XCTFail("Fail-closed SharedCore must not set a space child without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-space-child-set-no-session"))
            for forbidden in ["syt_", "token", parentId, childId, via, order] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreSpaces.spaceChildRemove(
                core: core,
                parentId: parentId,
                childId: childId
            )
            XCTFail("Fail-closed SharedCore must not remove a space child without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-space-child-remove-no-session"))
            for forbidden in ["syt_", "token", parentId, childId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreSpaces.restrictedJoinReparent(
                core: core,
                roomId: roomId,
                removeParentId: parentId,
                addParentId: childId
            )
            XCTFail("Fail-closed SharedCore must not reparent restricted join without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-restricted-join-reparent-no-session"))
            for forbidden in ["syt_", "token", roomId, parentId, childId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreInviteActionsWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s918SecretRoom:example.org"
        let senderId = "@s918SecretSender:example.org"

        do {
            _ = try await SharedCoreInviteActions.invitesAccept(core: core, roomId: roomId)
            XCTFail("Fail-closed SharedCore must not accept an invite without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-invites-accept-no-session"))
            for forbidden in ["syt_", "token", roomId, senderId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreInviteActions.invitesDecline(core: core, roomId: roomId)
            XCTFail("Fail-closed SharedCore must not decline an invite without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-invites-decline-no-session"))
            for forbidden in ["syt_", "token", roomId, senderId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreInviteActions.invitesReportSpam(core: core, roomId: roomId)
            XCTFail("Fail-closed SharedCore must not report invite spam without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-invites-report-spam-no-session"))
            for forbidden in ["syt_", "token", roomId, senderId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreInviteActions.invitesBlockSender(core: core, roomId: roomId)
            XCTFail("Fail-closed SharedCore must not block an invite sender without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-invites-block-sender-no-session"))
            for forbidden in ["syt_", "token", roomId, senderId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreTimelineReadStateWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s919SecretRoom:example.org"
        let eventId = "$s919SecretEvent"
        let streamId = "s919SecretStream"

        do {
            _ = try await SharedCoreTimelineReadState.timelineEventReadback(
                core: core,
                roomId: roomId,
                eventId: eventId
            )
            XCTFail("Fail-closed SharedCore must not read back a timeline event without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-event-readback-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, streamId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimelineReadState.timelineSetReadState(
                core: core,
                streamId: streamId,
                action: "mark_read"
            )
            XCTFail("Fail-closed SharedCore must not set timeline read-state without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-set-read-state-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, streamId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimelineReadState.timelineJumpLatest(
                core: core,
                streamId: streamId
            )
            XCTFail("Fail-closed SharedCore must not jump a timeline to latest without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-jump-latest-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, streamId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreTimelineReactionsWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s920SecretRoom:example.org"
        let eventId = "$s920SecretEvent"
        let reactionEventId = "$s920SecretReaction"
        let key = "s920SecretKey"

        do {
            _ = try await SharedCoreTimelineReactions.reactionEnsure(
                core: core,
                roomId: roomId,
                eventId: eventId,
                key: key
            )
            XCTFail("Fail-closed SharedCore must not ensure a reaction without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-reaction-ensure-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, reactionEventId, key] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimelineReactions.reactionRedact(
                core: core,
                roomId: roomId,
                targetEventId: eventId,
                reactionEventId: reactionEventId,
                key: key
            )
            XCTFail("Fail-closed SharedCore must not redact a reaction without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-reaction-redact-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, reactionEventId, key] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimelineReactions.timelineReactionToggle(
                core: core,
                roomId: roomId,
                eventId: eventId,
                key: key
            )
            XCTFail("Fail-closed SharedCore must not toggle a reaction without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-reaction-toggle-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, reactionEventId, key] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreComposerReplyDraftWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s921SecretRoom:example.org"
        let eventId = "$s921SecretEvent"

        do {
            _ = try await SharedCoreComposerReplyDraft.composerSetReplyDraft(
                core: core,
                roomId: roomId,
                eventId: eventId,
                startThread: false
            )
            XCTFail("Fail-closed SharedCore must not set a composer reply draft without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-composer-set-reply-draft-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreComposerReplyDraft.composerGetReplyDraft(
                core: core,
                roomId: roomId
            )
            XCTFail("Fail-closed SharedCore must not get a composer reply draft without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-composer-get-reply-draft-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreComposerReplyDraft.composerClearReplyDraft(
                core: core,
                roomId: roomId
            )
            XCTFail("Fail-closed SharedCore must not clear a composer reply draft without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-composer-clear-reply-draft-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreSendTextWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s922SecretRoom:example.org"
        let body = "s922SecretBody"

        do {
            _ = try await SharedCoreSendText.sendText(
                core: core,
                roomId: roomId,
                body: body,
                msgType: nil,
                formattedBody: nil,
                mentionUserIds: nil,
                mentionRoom: nil,
                replyTo: nil,
                threadRoot: nil,
                txnId: nil
            )
            XCTFail("Fail-closed SharedCore must not send text without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-send-text-no-session"))
            for forbidden in ["syt_", "token", roomId, body] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreSendStickerWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s923SecretRoom:example.org"
        let body = "s923SecretBody"
        let mxc = "mxc://example.org/s923SecretMxc"

        do {
            _ = try await SharedCoreSendSticker.sendSticker(
                core: core,
                roomId: roomId,
                body: body,
                mxc: mxc,
                width: nil,
                height: nil,
                mimetype: nil,
                size: nil,
                replyTo: nil,
                threadRoot: nil
            )
            XCTFail("Fail-closed SharedCore must not send a sticker without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-send-sticker-no-session"))
            for forbidden in ["syt_", "token", roomId, body, mxc] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreSendPollWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s924SecretRoom:example.org"
        let question = "s924SecretQuestion"
        let optionA = "s924SecretOptionA"
        let optionB = "s924SecretOptionB"

        do {
            _ = try await SharedCoreSendPoll.sendPoll(
                core: core,
                roomId: roomId,
                question: question,
                answers: [optionA, optionB],
                maxSelections: 1,
                threadRoot: nil,
                replyTo: nil
            )
            XCTFail("Fail-closed SharedCore must not send a poll without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-send-poll-no-session"))
            for forbidden in ["syt_", "token", roomId, question, optionA, optionB] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreEditMessageWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s925SecretRoom:example.org"
        let eventId = "$s925SecretEvent:example.org"
        let body = "s925SecretBody"

        do {
            _ = try await SharedCoreEditMessage.editMessage(
                core: core,
                roomId: roomId,
                eventId: eventId,
                body: body,
                msgType: nil,
                formattedBody: nil,
                mentionUserIds: nil,
                mentionRoom: nil,
                txnId: nil
            )
            XCTFail("Fail-closed SharedCore must not edit a message without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-edit-message-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, body] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCorePollRespondWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s926SecretRoom:example.org"
        let pollEventId = "$s926SecretEvent:example.org"
        let answer = "s926SecretAnswer"

        do {
            _ = try await SharedCorePollRespond.pollRespond(
                core: core,
                roomId: roomId,
                pollEventId: pollEventId,
                answerIds: [answer]
            )
            XCTFail("Fail-closed SharedCore must not respond to a poll without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-poll-respond-no-session"))
            for forbidden in ["syt_", "token", roomId, pollEventId, answer] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreTimelineMutateWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s927SecretRoom:example.org"
        let eventId = "$s927SecretEvent:example.org"
        let body = "s927SecretBody"
        let reason = "s927SecretReason"

        do {
            _ = try await SharedCoreTimelineMutate.timelineEditText(
                core: core,
                roomId: roomId,
                eventId: eventId,
                body: body,
                formattedBody: nil
            )
            XCTFail("Fail-closed SharedCore must not edit timeline text without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-edit-text-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, body, reason] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimelineMutate.timelineRedact(
                core: core,
                roomId: roomId,
                eventId: eventId,
                reason: reason
            )
            XCTFail("Fail-closed SharedCore must not redact a timeline event without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-redact-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, body, reason] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimelineMutate.timelineReport(
                core: core,
                roomId: roomId,
                eventId: eventId,
                reason: reason
            )
            XCTFail("Fail-closed SharedCore must not report a timeline event without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-report-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, body, reason] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreTimelinePinWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s928SecretRoom:example.org"
        let eventId = "$s928SecretEvent:example.org"

        do {
            _ = try await SharedCoreTimelinePin.timelinePin(
                core: core,
                roomId: roomId,
                eventId: eventId
            )
            XCTFail("Fail-closed SharedCore must not pin a timeline event without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-pin-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimelinePin.timelineUnpin(
                core: core,
                roomId: roomId,
                eventId: eventId
            )
            XCTFail("Fail-closed SharedCore must not unpin a timeline event without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-unpin-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreTimelineVoteDeclineWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let roomId = "!s929SecretRoom:example.org"
        let eventId = "$s929SecretEvent:example.org"
        let answer = "s929SecretAnswer"

        do {
            _ = try await SharedCoreTimelineVoteDecline.timelinePollVote(
                core: core,
                roomId: roomId,
                eventId: eventId,
                answerIds: [answer]
            )
            XCTFail("Fail-closed SharedCore must not vote on a timeline poll without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-poll-vote-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, answer] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimelineVoteDecline.timelineCallDecline(
                core: core,
                roomId: roomId,
                eventId: eventId
            )
            XCTFail("Fail-closed SharedCore must not decline a timeline call without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-call-decline-no-session"))
            for forbidden in ["syt_", "token", roomId, eventId, answer] {
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
