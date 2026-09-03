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

    func testSharedCoreLeftoverStatusWithoutAttachFailsClosed() async {
        let core = SharedCore()

        do {
            _ = try await SharedCoreLeftovers.backupStatus(core: core)
            XCTFail("Fail-closed SharedCore must not read leftover backup status without attach")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-backup-status-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreLeftovers.roomKeyTransferStatus(core: core)
            XCTFail("Fail-closed SharedCore must not read leftover room-key status without attach")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-key-transfer-status-no-session"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreLeftoversWithoutSessionFailClosed() async {
        let core = SharedCore.newWithSecretStore(store: InMemoryIosSecretVault())
        let recoveryKey = "s10-secret-recovery-key"
        let roomId = "!s10SecretRoom:example.org"
        let actionTitle = "s10-secret-action-title"
        let mxc = "mxc://example.org/s10SecretMedia"

        do {
            _ = try await SharedCoreLeftovers.recover(core: core, recoveryKey: recoveryKey)
            XCTFail("Fail-closed SharedCore must not recover without leftover I/O")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s10-leftover-unavailable"))
            for forbidden in ["syt_", "token", recoveryKey] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreAgentApprovals.send(
                core: core,
                roomId: roomId,
                actionId: "approve-s10",
                actionTitle: actionTitle,
                decision: "approve",
                sourceEventId: "$source:example.org",
                createdAt: 1
            )
            XCTFail("Fail-closed SharedCore must not send an agent approval without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s34-agent-approval-no-session"))
            for forbidden in ["syt_", "token", roomId, actionTitle] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreLeftovers.mediaDownload(core: core, mxc: mxc)
            XCTFail("Fail-closed SharedCore must not download leftover media without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s10-leftover-no-session"))
            for forbidden in ["syt_", "token", mxc] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
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

    func testSharedCoreSessionBootstrapWithoutSessionDoesNotStart() async {
        let core = SharedCore()
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s13-no-session", isDirectory: true)
        let userID = "@alice:example.org"
        let homeserver = "https://matrix.example.org"

        let outcome = await SharedCoreSessionBootstrap.prepareLiveSession(
            userID: userID,
            homeserverURL: homeserver,
            storeRoot: storeRoot,
            core: core
        )

        XCTAssertFalse(outcome.restored)
        XCTAssertFalse(outcome.skippedRestore)
        XCTAssertFalse(outcome.attached)
        XCTAssertFalse(outcome.skippedAttach)
        XCTAssertFalse(outcome.started)
        XCTAssertNil(outcome.readiness)
        XCTAssertEqual(outcome.failure, .restoreFailed)
        let publicError = String(describing: outcome)
        for forbidden in ["password", "syt_", userID, "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreResumeFromForegroundWithoutSessionStaysStoppedWithoutEcho() async throws {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreMatrixClientService(host: host)
        let session = AuthenticatedSession(
            userID: "@alice:example.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.example.org")),
            accessToken: "syt_secret_token"
        )
        await service.resumeFromForeground(session: session)
        XCTAssertEqual(service.syncStatus, .restoreFailed)
        let publicError = String(describing: service.syncStatus)
        for forbidden in ["password", "syt_secret_token", "@alice:example.org", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreColdBackgroundStartDoesNotOpenStoresBeforeForegroundAuthority() async throws {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreMatrixClientService(host: host)
        let session = AuthenticatedSession(
            userID: "@alice:example.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.example.org")),
            accessToken: "syt_secret_token"
        )

        await service.start(session: session)
        XCTAssertEqual(service.syncStatus, .stopped)

        await service.resumeFromForeground(session: session)
        XCTAssertEqual(service.syncStatus, .restoreFailed)
    }

    func testSharedCoreColdBackgroundPauseWithoutOwnersStaysStopped() async {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreMatrixClientService(host: host)

        await service.pauseForBackground()

        XCTAssertEqual(service.syncStatus, .stopped)
    }

    func testSharedCoreSessionStopDoesNotRevokeForegroundAuthorityForRelogin() async throws {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreMatrixClientService(host: host)
        let session = AuthenticatedSession(
            userID: "@alice:example.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.example.org")),
            accessToken: "syt_secret_token"
        )

        service.setForegroundActive(true)
        await service.start(session: session)
        XCTAssertEqual(service.syncStatus, .restoreFailed)

        await service.stop()
        await service.start(session: session)
        XCTAssertEqual(service.syncStatus, .restoreFailed)
    }

    func testSharedCoreStartSyncWithoutAttachFailsClosed() async {
        let core = SharedCore()

        do {
            _ = try await SharedCoreSyncStart.startSync(core: core)
            XCTFail("Fail-closed SharedCore must not start sync without attach")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s12-sync-not-attached"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCorePollRoomListUpdatesWithoutSessionReturnsEmpty() async {
        let core = SharedCore()

        do {
            let updates = try await SharedCoreRoomListUpdates.poll(core: core)
            XCTAssertTrue(updates.isEmpty)
            let publicError = String(describing: updates)
            for forbidden in ["password", "syt_", "@alice:example.org", "token", "!room"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        } catch {
            XCTFail("Empty room-list update queue must not fail: \(error)")
        }
    }

    func testSharedCorePollOwnerUpdatesWithoutSessionReturnsEmpty() async {
        let core = SharedCore()

        do {
            let updates = try await SharedCoreOwnerUpdates.poll(core: core)
            XCTAssertTrue(updates.isEmpty)
            let publicError = String(describing: updates)
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        } catch {
            XCTFail("Empty owner-update queue must not fail: \(error)")
        }
    }

    func testSharedCorePollTimelineViewUpdatesWithoutSessionReturnsEmpty() async {
        let core = SharedCore()

        do {
            let updates = try await SharedCoreTimelineViewUpdates.poll(core: core)
            XCTAssertTrue(updates.isEmpty)
            let publicError = String(describing: updates)
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        } catch {
            XCTFail("Empty timeline view-update queue must not fail: \(error)")
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

    func testSharedCoreTimelineRowsRequireExhaustionBeforeEmpty() {
        XCTAssertNil(
            SharedCoreTimelineRows.authoritativeOutcome(
                from: [],
                paginationBackward: "available"
            )
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.authoritativeOutcome(
                from: [],
                paginationBackward: "exhausted"
            ),
            .empty
        )
    }

    func testSharedCoreTimelinePaginationStopsAfterConsecutiveNoProgressPages() {
        var progress = SharedCoreTimelinePaginationProgress()
        progress.observeInitial(rowIDs: ["row-1"])

        XCTAssertTrue(progress.canRequestPage)
        progress.recordPage(rowIDs: ["row-1"])
        XCTAssertTrue(progress.canRequestPage)
        progress.recordPage(rowIDs: ["row-1"])

        XCTAssertFalse(progress.canRequestPage)
        XCTAssertEqual(progress.pageRequestCount, 2)
        XCTAssertEqual(progress.consecutiveNoProgressPages, 2)
    }

    func testSharedCoreTimelinePaginationHasAnAbsolutePageBound() {
        var progress = SharedCoreTimelinePaginationProgress()
        progress.observeInitial(rowIDs: [])

        for page in 0 ..< SharedCoreTimelinePaginationProgress.maximumPageRequests {
            XCTAssertTrue(progress.canRequestPage)
            progress.recordPage(rowIDs: ["row-\(page)"])
        }

        XCTAssertFalse(progress.canRequestPage)
        XCTAssertEqual(
            progress.pageRequestCount,
            SharedCoreTimelinePaginationProgress.maximumPageRequests
        )
    }

    func testSharedCoreTimelineErrorsStayTypedAndPrivacySafe() {
        let noSession = SharedCoreTimelineService.failure(
            from: TimelineError.Failed(
                code: "p2-timeline-open-no-session",
                description: "ignored owner text"
            )
        )
        XCTAssertEqual(noSession.kind, .sessionUnavailable)
        XCTAssertEqual(noSession.diagnosticCode, "p2-timeline-open-no-session")
        XCTAssertEqual(noSession.userMessage, "Sign in again to load this timeline.")

        let unavailable = SharedCoreTimelineService.failure(
            from: NSError(
                domain: "https://user:secret@example.org/?access_token=syt_secret",
                code: 1
            )
        )
        XCTAssertEqual(unavailable.kind, .temporarilyUnavailable)
        XCTAssertEqual(unavailable.diagnosticCode, "timeline-temporarily-unavailable")
        for forbidden in ["secret", "example.org", "access_token", "syt_"] {
            XCTAssertFalse(unavailable.userMessage.contains(forbidden))
            XCTAssertFalse(unavailable.diagnosticCode.contains(forbidden))
        }

        let hostileCode = SharedCoreTimelineService.failure(
            from: TimelineError.Failed(
                code: "https://example.org/?access_token=syt_secret",
                description: "ignored owner text"
            )
        )
        XCTAssertEqual(hostileCode.kind, .temporarilyUnavailable)
        XCTAssertEqual(hostileCode.diagnosticCode, "timeline-invalid-diagnostic-code")
    }

    func testSharedCoreTimelineSenderAvatarAcceptsOnlyValidMXCMetadata() {
        XCTAssertEqual(
            SharedCoreTimelineRows.senderAvatarURL("mxc://example.org/alice")?.absoluteString,
            "mxc://example.org/alice"
        )
        XCTAssertNil(SharedCoreTimelineRows.senderAvatarURL(nil))
        XCTAssertNil(SharedCoreTimelineRows.senderAvatarURL("https://example.org/alice.png"))
        XCTAssertNil(SharedCoreTimelineRows.senderAvatarURL("mxc://example.org"))
        XCTAssertNil(SharedCoreTimelineRows.senderAvatarURL("mxc:///alice"))
    }

    func testSharedCoreTimelineRowsMapsNonMessageBodiesWithoutEcho() {
        XCTAssertEqual(
            SharedCoreTimelineRows.displayKind(
                rowKind: "poll",
                body: "Lunch?",
                formattedBody: nil
            ),
            .text("Lunch?")
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.displayKind(
                rowKind: "membership",
                body: "@alex joined",
                formattedBody: nil
            ),
            .text("@alex joined")
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.displayKind(
                rowKind: "state",
                body: "Topic changed",
                formattedBody: nil
            ),
            .text("Topic changed")
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.displayKind(
                rowKind: "call",
                body: "voice",
                formattedBody: nil
            ),
            .text("voice")
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.displayKind(
                rowKind: "sticker",
                body: "",
                formattedBody: nil
            ),
            .unknown(type: "sticker")
        )
        if case let .mediaPlaceholder(resource) = SharedCoreTimelineRows.displayKind(
            rowKind: "sticker",
            body: "",
            formattedBody: nil,
            messageType: "m.sticker",
            mediaHandleId: "incoming-sticker-handle",
            mediaMimeType: "image/webp"
        ) {
            XCTAssertEqual(resource.id, "incoming-sticker-handle")
            XCTAssertEqual(resource.filename, "m.sticker")
            XCTAssertEqual(resource.mimeType, "image/webp")
            XCTAssertEqual(
                SharedCoreTimelineMedia.handleId(from: resource.authenticatedURL),
                "incoming-sticker-handle"
            )
        } else {
            XCTFail("An incoming Matrix sticker with native media must remain renderable")
        }
        XCTAssertNil(
            SharedCoreTimelineRows.displayKind(
                rowKind: "date_separator",
                body: "",
                formattedBody: nil
            )
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.displayKind(
                rowKind: "encrypted",
                body: "unable_to_decrypt",
                formattedBody: nil
            ),
            .encryptedPlaceholder
        )
        let projectedAgentJSON = #"{"title":"Approval required","status":"pending","summary":"Review","actions":[]}"#
        if case let .agentCard(card) = SharedCoreTimelineRows.displayKind(
            rowKind: "message",
            body: "fallback",
            formattedBody: nil,
            agentCardJSON: projectedAgentJSON
        ) {
            XCTAssertEqual(card.title, "Approval required")
            XCTAssertEqual(card.status, "pending")
        } else {
            XCTFail("Recognized SharedCore agent payload must map to an agent card")
        }
        XCTAssertEqual(
            SharedCoreTimelineRows.reactionCounts([("👍", 2), ("🎉", 1)]),
            ["👍": 2, "🎉": 1]
        )
        if case let .mediaPlaceholder(resource) = SharedCoreTimelineRows.displayKind(
            rowKind: "message",
            body: "A sunset",
            formattedBody: "<strong>A sunset</strong>",
            messageType: "image",
            mediaHandleId: "timeline-media-s32",
            mediaMimeType: "image/jpeg",
            mediaFilename: "photo.jpg",
            mediaCaption: "A sunset"
        ) {
            XCTAssertEqual(resource.filename, "photo.jpg")
            XCTAssertEqual(resource.caption, "A sunset")
            XCTAssertEqual(resource.formattedCaption, "<strong>A sunset</strong>")
            XCTAssertEqual(SharedCoreTimelineMedia.handleId(from: resource.authenticatedURL), "timeline-media-s32")
        } else {
            XCTFail("Image rows with a handle must map to a media placeholder")
        }
        XCTAssertEqual(
            SharedCoreDevicesLive.devices(
                deviceId: "DEVICE1",
                displayName: "iPhone",
                isCurrent: true,
                trust: "verified"
            ).displayName,
            "iPhone"
        )
        XCTAssertEqual(
            SharedCorePresenceLive.presence(
                userId: "@alice:example.org",
                state: "online",
                currentlyActive: true,
                statusMsg: "On a call"
            ).displayName,
            "Online"
        )
        let stickers = SharedCoreImagePackRows.stickers(
            packId: "user",
            packName: "Mine",
            contentJSON: #"{"pack":{"display_name":"Mine"},"images":{"smile":{"url":"mxc://example.org/abc","body":":)"},"bad":{"url":"https://example.org/x"}}}"#
        )
        XCTAssertEqual(stickers.map(\.body), [":)"])
        XCTAssertEqual(stickers.first?.mxc, "mxc://example.org/abc")
        XCTAssertEqual(stickers.first?.packName, "Mine")
        let publicError = String(describing: SharedCoreTimelineRows.displayKind(
            rowKind: "poll",
            body: "Lunch?",
            formattedBody: nil
        ))
        for forbidden in ["password", "syt_", "token", "mxc://"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreTimelineRowsPreservesRelationsPollsCapabilitiesAndReactionOwnership() {
        let mappedRow = SharedCoreTimelineRows.item(
            from: TimelineViewRowDto(
                kind: "message",
                itemId: "item-1",
                eventId: "$message:example.org",
                sender: "@alice:example.org",
                senderName: "Alice Example",
                senderAvatarUrl: nil,
                body: "Hello",
                originServerTs: 1_700_000_000_000,
                edited: false,
                replyToEventId: nil,
                replyPreview: nil,
                threadRootEventId: "$thread-root:example.org",
                threadSummary: nil,
                poll: nil,
                capabilities: nil,
                decryptionState: nil,
                messageType: "m.text",
                forwardTransport: "text",
                formattedBody: nil,
                agentCardJson: nil,
                isAgentApproval: false,
                mediaFilename: nil,
                mediaCaption: nil,
                reactions: [TimelineViewReactionDto(key: "👍", count: 2, own: true)],
                mediaHandleId: nil,
                mediaMimeType: nil,
                mediaWidth: nil,
                mediaHeight: nil,
                mediaDurationMs: nil
            )
        )
        XCTAssertEqual(mappedRow?.senderProfileDisplayName, "Alice Example")
        XCTAssertEqual(mappedRow?.senderDisplayName, "Alice Example")
        XCTAssertEqual(mappedRow?.threadRootEventID, "$thread-root:example.org")
        XCTAssertEqual(mappedRow?.reactions, ["👍": 2])
        XCTAssertEqual(mappedRow?.reactionOwnership, .known(["👍"]))

        let reply = SharedCoreTimelineRows.replyPreview(
            from: TimelineViewReplyPreviewDto(
                eventId: "$root:example.org",
                senderId: "@alice:example.org",
                senderName: "Alice",
                body: String(repeating: "reply ", count: 30)
            )
        )
        XCTAssertEqual(reply?.eventID, "$root:example.org")
        XCTAssertEqual(reply?.senderID, "@alice:example.org")
        XCTAssertEqual(reply?.senderName, "Alice")
        XCTAssertEqual(reply?.snippet.count, TimelineReplyPreview.maxSnippetLength)
        XCTAssertTrue(reply?.snippet.hasSuffix("…") == true)
        XCTAssertNil(SharedCoreTimelineRows.replyPreview(from: nil))

        let thread = SharedCoreTimelineRows.threadSummary(
            from: TimelineViewThreadSummaryDto(
                rootEventId: "$root:example.org",
                replyCount: 7,
                latestEventId: "$latest:example.org"
            )
        )
        XCTAssertEqual(
            thread,
            TimelineThreadSummary(
                rootEventID: "$root:example.org",
                replyCount: 7,
                latestEventID: "$latest:example.org"
            )
        )

        let openPoll = SharedCoreTimelineRows.poll(
            from: TimelineViewPollDto(
                question: "Choose two",
                closed: false,
                maxSelections: 2,
                answers: [
                    TimelineViewPollAnswerDto(id: "a", text: "Alpha", voteCount: 3, own: true),
                    TimelineViewPollAnswerDto(id: "b", text: "Beta", voteCount: 1, own: false),
                ]
            )
        )
        XCTAssertEqual(openPoll?.question, "Choose two")
        XCTAssertEqual(openPoll?.maximumSelections, 2)
        XCTAssertEqual(openPoll?.isClosed, false)
        XCTAssertEqual(openPoll?.answers.map(\.voteCount), [3, 1])
        XCTAssertEqual(openPoll?.answers.map(\.isOwn), [true, false])

        let closedPoll = SharedCoreTimelineRows.poll(
            from: TimelineViewPollDto(
                question: "Finished",
                closed: true,
                maxSelections: 0,
                answers: []
            )
        )
        XCTAssertEqual(closedPoll?.isClosed, true)
        XCTAssertEqual(closedPoll?.maximumSelections, 0)

        let capabilities = SharedCoreTimelineRows.actionCapabilities(
            from: TimelineViewRowCapabilitiesDto(
                react: true,
                reply: true,
                edit: false,
                redact: true,
                report: true,
                pin: true,
                forward: true,
                vote: false,
                declineCall: false
            )
        )
        XCTAssertEqual(capabilities?.canReply, true)
        XCTAssertEqual(capabilities?.canReact, true)
        XCTAssertEqual(capabilities?.canEdit, false)
        XCTAssertEqual(capabilities?.canRedact, true)
        XCTAssertEqual(capabilities?.canReport, true)
        XCTAssertEqual(capabilities?.canPin, true)
        XCTAssertEqual(capabilities?.canForward, true)
        XCTAssertEqual(capabilities?.canVote, false)
        XCTAssertEqual(capabilities?.canDeclineCall, false)

        XCTAssertEqual(
            SharedCoreTimelineRows.reactionOwnership(from: [
                TimelineViewReactionDto(key: "👍", count: 2, own: true),
                TimelineViewReactionDto(key: "🎉", count: 1, own: false),
            ]),
            .known(["👍"])
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.reactionOwnership(from: [
                TimelineViewReactionDto(key: "👍", count: 2, own: nil),
            ]),
            .unknown
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.forwardTransport("text"),
            .text
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.forwardTransport("media"),
            .media
        )
        XCTAssertEqual(
            SharedCoreTimelineRows.forwardTransport(nil),
            .unavailable
        )
    }

    func testSharedCoreTimelineRowsMapsNonMessageEventMetadata() throws {
        let capabilities = TimelineViewRowCapabilitiesDto(
            react: false,
            reply: false,
            edit: false,
            redact: true,
            report: true,
            pin: true,
            forward: false,
            vote: false,
            declineCall: false
        )
        let row = { (kind: String) in
            TimelineViewRowDto(
                kind: kind,
                itemId: "\(kind)-item",
                eventId: "$\(kind):example.org",
                sender: "@alice:example.org",
                senderName: "Alice Example",
                senderAvatarUrl: "mxc://example.org/alice",
                body: "\(kind) event",
                originServerTs: 1_700_000_000_123,
                edited: false,
                replyToEventId: nil,
                replyPreview: nil,
                threadRootEventId: nil,
                threadSummary: nil,
                poll: nil,
                capabilities: capabilities,
                decryptionState: nil,
                messageType: nil,
                forwardTransport: nil,
                formattedBody: nil,
                agentCardJson: nil,
                isAgentApproval: false,
                mediaFilename: nil,
                mediaCaption: nil,
                reactions: [],
                mediaHandleId: nil,
                mediaMimeType: nil,
                mediaWidth: nil,
                mediaHeight: nil,
                mediaDurationMs: nil
            )
        }

        for kind in ["membership", "state", "call"] {
            let item = try XCTUnwrap(SharedCoreTimelineRows.item(from: row(kind)))
            XCTAssertEqual(item.eventID, "$\(kind):example.org")
            XCTAssertEqual(item.senderID, "@alice:example.org")
            XCTAssertEqual(item.senderProfileDisplayName, "Alice Example")
            XCTAssertEqual(item.senderAvatarURL?.absoluteString, "mxc://example.org/alice")
            XCTAssertEqual(
                item.timestamp.timeIntervalSince1970,
                1_700_000_000.123,
                accuracy: 0.001
            )
            XCTAssertEqual(item.actionCapabilities?.canRedact, true)
            XCTAssertEqual(item.actionCapabilities?.canReport, true)
            XCTAssertEqual(item.actionCapabilities?.canPin, true)
        }
    }

    func testSharedCoreTimelineLiveRefreshMatchesRoomAndStream() {
        XCTAssertTrue(
            SharedCoreTimelineLiveRefresh.shouldRefresh(
                watchingRoomID: "!s18:example.org",
                watchingStreamId: "view-1",
                updateRoomId: "!s18:example.org",
                updateStreamId: "view-1"
            )
        )
        XCTAssertTrue(
            SharedCoreTimelineLiveRefresh.shouldRefresh(
                watchingRoomID: "!s18:example.org",
                watchingStreamId: "view-1",
                updateRoomId: "!s18:example.org",
                updateStreamId: ""
            )
        )
        XCTAssertFalse(
            SharedCoreTimelineLiveRefresh.shouldRefresh(
                watchingRoomID: "!s18:example.org",
                watchingStreamId: "view-1",
                updateRoomId: "!other:example.org",
                updateStreamId: "view-1"
            )
        )
        XCTAssertFalse(
            SharedCoreTimelineLiveRefresh.shouldRefresh(
                watchingRoomID: "!s18:example.org",
                watchingStreamId: "view-1",
                updateRoomId: "!s18:example.org",
                updateStreamId: "view-2"
            )
        )
    }

    func testSharedCoreTimelineUpdateBootstrapRefreshesOnlyLiveStream() {
        XCTAssertTrue(
            SharedCoreTimelineUpdateBootstrap.shouldRefreshOpenStream(focusedEventID: nil)
        )
        XCTAssertFalse(
            SharedCoreTimelineUpdateBootstrap.shouldRefreshOpenStream(
                focusedEventID: "$event:example.org"
            )
        )
    }

    func testTimelineSignalBatchKeepsOnlyNewestInvalidationPerStream() {
        let updates = [
            TimelineViewUpdateDto(
                schemaVersion: 1,
                sessionGeneration: 7,
                streamId: "view-a",
                roomId: "!a:example.org",
                revision: 3,
                opCount: 1
            ),
            TimelineViewUpdateDto(
                schemaVersion: 1,
                sessionGeneration: 7,
                streamId: "view-a",
                roomId: "!a:example.org",
                revision: 5,
                opCount: 2
            ),
            TimelineViewUpdateDto(
                schemaVersion: 1,
                sessionGeneration: 7,
                streamId: "view-b",
                roomId: "!b:example.org",
                revision: 4,
                opCount: 1
            ),
            TimelineViewUpdateDto(
                schemaVersion: 1,
                sessionGeneration: 7,
                streamId: "view-a",
                roomId: "!a:example.org",
                revision: 4,
                opCount: 1
            ),
        ]

        let coalesced = SharedCoreTimelineSignalBatch.coalesced(updates)

        XCTAssertEqual(coalesced.count, 2)
        XCTAssertEqual(coalesced.first(where: { $0.streamId == "view-a" })?.revision, 5)
        XCTAssertEqual(coalesced.first(where: { $0.streamId == "view-b" })?.revision, 4)
    }

    func testSharedCoreTimelineUpdatesWithoutSessionFailsClosedWithoutEcho() async {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreTimelineService(host: host)
        var outcomes: [TimelineLoadOutcome] = []
        for await outcome in service.timelineUpdates(roomID: "!s18:example.org", focusedEventID: nil) {
            outcomes.append(outcome)
            let publicError = String(describing: outcome)
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
            break
        }
        XCTAssertEqual(
            outcomes,
            [
                .failed(
                    TimelineLoadFailure(
                        kind: .sessionUnavailable,
                        diagnosticCode: "p2-timeline-open-no-session"
                    )
                ),
            ]
        )
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
            _ = try await SharedCoreTimeline.timelineSnapshot(core: core, streamId: "view-1")
            XCTFail("Fail-closed SharedCore must not snapshot a timeline without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-snapshot-no-session"))
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

    func testSharedCoreTypingLiveMatchesRoomWithoutEcho() {
        let users = SharedCoreTypingLive.users(
            roomID: "!s21:example.org",
            rooms: [
                (roomId: "!other:example.org", userIds: ["@carol:example.org"]),
                (roomId: "!s21:example.org", userIds: ["@bob:example.org"]),
            ]
        )
        XCTAssertEqual(users, ["@bob:example.org"])
        XCTAssertTrue(
            SharedCoreTypingLive.shouldRefresh(watchingRoomID: "!s21:example.org", updateRoomId: "!s21:example.org")
        )
        XCTAssertFalse(
            SharedCoreTypingLive.shouldRefresh(watchingRoomID: "!s21:example.org", updateRoomId: "!other:example.org")
        )
        let publicError = String(describing: users)
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreTypingUsersWithoutSessionYieldsEmptyWithoutEcho() async {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreTimelineService(host: host)
        var batches: [[String]] = []
        for await users in service.typingUsers(roomID: "!s21:example.org") {
            batches.append(users)
            let publicError = String(describing: users)
            for forbidden in ["password", "syt_", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
            break
        }
        XCTAssertEqual(batches, [[]])
    }

    func testSharedCoreReadMarkersPrefersOwnReadWithoutEcho() {
        let acknowledged = SharedCoreReadMarkers.acknowledgedEventID(
            ownReadEventID: "$s24-own:example.org",
            rowEventIDs: ["$local-1", "$s24-row:example.org"]
        )
        XCTAssertEqual(acknowledged, "$s24-own:example.org")
        XCTAssertEqual(
            SharedCoreReadMarkers.acknowledgedEventID(
                ownReadEventID: "$pending-1",
                rowEventIDs: ["$local-1", "$s24-row:example.org"]
            ),
            "$s24-row:example.org"
        )
        let publicError = String(describing: acknowledged)
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreReadMarkersWithoutSessionStayEmptyWithoutEcho() async {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreRoomReadMarkerService(host: host)
        let marked = await service.markRoomAsRead(roomID: "!s24:example.org")
        let fullyRead = await service.fullyReadEventID(roomID: "!s24:example.org")
        XCTAssertNil(marked)
        XCTAssertNil(fullyRead)
        let publicError = String(describing: (marked, fullyRead))
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreRoomDetailsMapsSnapshotsWithoutEcho() {
        let powerJSON = """
        {"users_default":0,"events_default":0,"state_default":50,"invite":50,"kick":50,"ban":50,"redact":50,"events":{"m.room.name":50,"m.room.topic":50,"m.room.avatar":50,"m.room.canonical_alias":50},"users":{"@alice:example.org":100}}
        """
        let details = SharedCoreRoomDetails.details(
            roomID: "!s22:example.org",
            ownUserID: "@alice:example.org",
            room: SharedCoreRoomDetails.RoomRow(
                roomId: "!s22:example.org",
                name: "Ops",
                canonicalAlias: "#ops:example.org",
                avatarUrl: "mxc://example.org/roomAvatar"
            ),
            members: [
                SharedCoreRoomDetails.MemberRow(
                    userId: "@alice:example.org",
                    membership: "join",
                    powerLevel: 100
                ),
                SharedCoreRoomDetails.MemberRow(
                    userId: "@bob:example.org",
                    membership: "leave",
                    powerLevel: 0
                ),
            ],
            powerLevelsJSON: powerJSON,
            joinRule: "public",
            topic: "Invite topic",
            encryptionStatus: .encrypted
        )
        XCTAssertEqual(details.name, "Ops")
        XCTAssertEqual(details.topic, "Invite topic")
        XCTAssertEqual(details.aliases, ["#ops:example.org"])
        XCTAssertEqual(details.avatarURL, "mxc://example.org/roomAvatar")
        XCTAssertEqual(details.memberCount, 1)
        XCTAssertEqual(details.members.map(\.userID), ["@alice:example.org", "@bob:example.org"])
        XCTAssertEqual(details.isPublic, true)
        XCTAssertEqual(details.isEncrypted, true)
        XCTAssertEqual(details.encryptionStatus, .encrypted)
        XCTAssertEqual(details.encryptionLabel, "Encrypted")
        XCTAssertEqual(SynaraRoomEncryptionStatus.notEncrypted.roomDetailsLabel, "Not encrypted")
        XCTAssertEqual(SynaraRoomEncryptionStatus.unknown.roomDetailsLabel, "Unknown")
        XCTAssertEqual(SynaraRoomEncryptionStatus.unavailable.roomDetailsLabel, "Unavailable")
        XCTAssertEqual(
            SharedCoreRoomDetails.notificationMode("mentions"),
            .mentionsOnly
        )
        XCTAssertEqual(SharedCoreRoomDetails.notificationMode("mute"), .mute)
        XCTAssertEqual(SharedCoreRoomDetails.notificationMode("all"), .allMessages)
        XCTAssertEqual(SharedCoreRoomDetails.notificationMode("default"), .default)
        XCTAssertEqual(SharedCoreRoomDetails.notificationMode(nil), .default)
        XCTAssertEqual(SharedCoreRoomDetails.notificationMode("unknown"), .default)
        XCTAssertEqual(SharedCoreRoomDetails.wireNotificationMode(.allMessages), "all")
        XCTAssertEqual(SharedCoreRoomDetails.wireNotificationMode(.mentionsOnly), "mentions")
        XCTAssertEqual(SharedCoreRoomDetails.wireNotificationMode(.mute), "mute")
        XCTAssertEqual(SharedCoreRoomDetails.wireNotificationMode(.default), "default")
        XCTAssertEqual(details.canInvite, true)
        XCTAssertEqual(details.canEditName, true)
        XCTAssertEqual(details.canEditAliases, true)
        XCTAssertEqual(details.powerLevels?.ownUserLevel, 100)
        XCTAssertEqual(details.notificationMode, .default)
        let publicError = String(describing: details)
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreRoomDetailsWithoutSessionFallsBackWithoutEcho() async {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreRoomManagementService(host: host)
        let details = await service.roomDetails(roomID: "!s22:example.org")
        XCTAssertEqual(details?.name, "!s22:example.org")
        XCTAssertEqual(details?.canInvite, false)
        XCTAssertNil(details?.powerLevels)
        let publicError = String(describing: details)
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreRoomListRowsMapsInviteAndSpaceWithoutEcho() {
        XCTAssertEqual(SharedCoreRoomListRows.encryptionStatus(.encrypted), .encrypted)
        XCTAssertEqual(SharedCoreRoomListRows.encryptionStatus(.notEncrypted), .notEncrypted)
        XCTAssertEqual(SharedCoreRoomListRows.encryptionStatus(.unknown), .unknown)
        let unknownRoom = RoomSummary(
            id: "!unknown:example.org",
            name: "Unknown",
            lastMessagePreview: "",
            unreadCount: 0,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: .distantPast
        )
        XCTAssertEqual(unknownRoom.encryptionStatus, .unknown)
        XCTAssertFalse(unknownRoom.isEncrypted)
        let rooms = SharedCoreRoomListRows.rooms(
            rooms: [
                SharedCoreRoomListRows.RoomRow(
                    roomId: "!s25:example.org",
                    name: "Ops",
                    avatarUrl: "mxc://example.org/room",
                    membership: "invite",
                    isDirect: false,
                    unreadCount: 0,
                    highlightCount: 0,
                    markedUnread: true,
                    lastActivityTs: 1_700_000_000_000,
                    lastMessagePreview: nil,
                    isFavorite: false,
                    encryptionStatus: .encrypted
                ),
                SharedCoreRoomListRows.RoomRow(
                    roomId: "!space:example.org",
                    name: "Team",
                    avatarUrl: nil,
                    membership: "join",
                    isDirect: false,
                    unreadCount: 0,
                    highlightCount: 0,
                    markedUnread: false,
                    lastActivityTs: 0,
                    lastMessagePreview: "Hello from Alice",
                    isFavorite: true,
                    encryptionStatus: .notEncrypted
                ),
            ],
            invites: [
                SharedCoreRoomListRows.InviteRow(
                    roomId: "!s25:example.org",
                    roomName: "Ops",
                    roomTopic: "On-call",
                    senderName: "Alex",
                    reason: nil
                ),
            ],
            spaceParents: [
                SharedCoreRoomListRows.SpaceParentRow(
                    roomId: "!s25:example.org",
                    parentIds: ["!space:example.org"]
                ),
            ]
        )
        XCTAssertEqual(rooms.first?.lastMessagePreview, "Invited by Alex")
        XCTAssertEqual(rooms.last?.lastMessagePreview, "Hello from Alice")
        XCTAssertEqual(rooms.first?.parentSpaces, [SpaceSummary(id: "!space:example.org", name: "Team")])
        XCTAssertEqual(rooms.first?.membership, .invited)
        XCTAssertEqual(rooms.first?.isFavorite, false)
        XCTAssertEqual(rooms.last?.isFavorite, true)
        XCTAssertEqual(rooms.first?.isEncrypted, true)
        XCTAssertEqual(rooms.last?.isEncrypted, false)
        XCTAssertEqual(rooms.first?.encryptionStatus, .encrypted)
        XCTAssertEqual(rooms.last?.encryptionStatus, .notEncrypted)
        XCTAssertEqual(rooms.first?.hasHighlight, false)
        XCTAssertEqual(rooms.first?.isMarkedUnread, true)
        XCTAssertEqual(rooms.last?.lastActivityAt, .distantPast)
        let publicError = String(describing: rooms)
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreRoomListWithoutSessionIsEmptyWithoutEcho() async {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreRoomListService(host: host)
        let state = await service.loadRooms()
        XCTAssertEqual(state, .empty)
        let publicError = String(describing: state)
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreRoomListUnreadLookupUsesSnapshotWithoutEcho() {
        XCTAssertTrue(SharedCoreRoomListRows.hasUnreadMessages(unreadCount: 2, hasHighlight: false))
        XCTAssertTrue(SharedCoreRoomListRows.hasUnreadMessages(unreadCount: 0, hasHighlight: true))
        XCTAssertTrue(SharedCoreRoomListRows.hasUnreadMessages(
            unreadCount: 0,
            hasHighlight: false,
            isMarkedUnread: true
        ))
        XCTAssertFalse(SharedCoreRoomListRows.hasUnreadMessages(unreadCount: 0, hasHighlight: false))
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreRoomListService(host: host)
        XCTAssertFalse(service.hasUnreadMessages(roomID: "!s26:example.org"))
        XCTAssertFalse(service.isAgentRoom(roomID: "!s26:example.org"))
        let publicError = String(describing: service.hasUnreadMessages(roomID: "!s26:example.org"))
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreSessionCryptoMapsLeftoverStatusWithoutEcho() {
        let ready = SharedCoreSessionCrypto.status(
            crossSigningState: "ready",
            backupEnabled: true,
            backupAvailability: "available",
            backupDeviceState: "ready",
            recoveryState: "ready",
            secretStorageState: "ready"
        )
        XCTAssertEqual(ready.verification, .verified)
        XCTAssertEqual(ready.recovery, .enabled)
        XCTAssertEqual(ready.backup, .enabled)
        XCTAssertNil(ready.hasDevicesToVerifyAgainst)
        XCTAssertNil(ready.isLastDevice)
        XCTAssertEqual(ready.unableToDecryptCount, 0)

        let attention = SharedCoreSessionCrypto.status(
            crossSigningState: "not_set_up",
            backupEnabled: false,
            backupAvailability: "available",
            backupDeviceState: "downloading",
            recoveryState: "incomplete",
            secretStorageState: "locked"
        )
        XCTAssertEqual(attention.verification, .unverified)
        XCTAssertEqual(attention.recovery, .incomplete)
        XCTAssertEqual(attention.backup, .syncing)

        let missing = SharedCoreSessionCrypto.status(
            crossSigningState: "unavailable",
            backupEnabled: false,
            backupAvailability: "missing",
            backupDeviceState: "unavailable",
            recoveryState: "not_set_up",
            secretStorageState: "unavailable"
        )
        XCTAssertEqual(missing.verification, .unverified)
        XCTAssertEqual(missing.recovery, .disabled)
        XCTAssertEqual(missing.backup, .unavailable)

        let secretStorageFallback = SharedCoreSessionCrypto.status(
            crossSigningState: nil,
            backupEnabled: nil,
            backupAvailability: nil,
            backupDeviceState: nil,
            recoveryState: nil,
            secretStorageState: "locked"
        )
        XCTAssertEqual(secretStorageFallback.verification, .unknown)
        XCTAssertEqual(secretStorageFallback.recovery, .incomplete)
        XCTAssertEqual(secretStorageFallback.backup, .unknown)

        let publicError = String(describing: ready)
        for forbidden in ["password", "syt_", "token", "missing_secrets", "recovery_key"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreSessionStatusWithoutSessionIsUnknownWithoutEcho() async {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreCryptoStatusService(host: host)
        let status = await service.sessionStatus()
        XCTAssertEqual(status, .unknown)
        let publicError = String(describing: status)
        for forbidden in ["password", "syt_", "token", "missing_secrets", "recovery_key"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreRoomCryptoMapsInviteAndSessionWithoutEcho() {
        let session = SharedCoreSessionCrypto.status(
            crossSigningState: "not_set_up",
            backupEnabled: false,
            backupAvailability: "available",
            backupDeviceState: "ready",
            recoveryState: "incomplete",
            secretStorageState: "locked"
        )
        let invited = SharedCoreSessionCrypto.roomStatus(isEncrypted: true, session: session)
        XCTAssertEqual(invited.encryption, .encrypted)
        XCTAssertEqual(invited.verification, .unverified)
        XCTAssertEqual(invited.recovery, .incomplete)
        XCTAssertEqual(invited.unableToDecryptCount, 0)
        XCTAssertTrue(invited.needsCryptoActionBanner)

        let joinedUnknown = SharedCoreSessionCrypto.roomStatus(isEncrypted: nil, session: session)
        XCTAssertEqual(joinedUnknown.encryption, .unknown)
        XCTAssertEqual(joinedUnknown.verification, .unverified)

        let clearInvite = SharedCoreSessionCrypto.roomStatus(isEncrypted: false, session: .unknown)
        XCTAssertEqual(clearInvite.encryption, .notEncrypted)
        XCTAssertEqual(clearInvite.verification, .unknown)

        let publicError = String(describing: invited)
        for forbidden in ["password", "syt_", "token", "missing_secrets", "recovery_key"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreRoomStatusWithoutSessionIsUnknownWithoutEcho() async {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreCryptoStatusService(host: host)
        let status = await service.roomStatus(roomID: "!s28:example.org")
        XCTAssertEqual(status, .unknown)
        let publicError = String(describing: status)
        for forbidden in ["password", "syt_", "token", "missing_secrets", "recovery_key"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testSharedCoreForwardSourceUsesOnlyJoinedRoomEncryptionTriState() {
        let joinedRows = [
            SharedCoreCryptoStatusService.JoinedRoomEncryptionRow(
                roomID: "!encrypted:example.org",
                membership: "join",
                encryption: .encrypted
            ),
            SharedCoreCryptoStatusService.JoinedRoomEncryptionRow(
                roomID: "!clear:example.org",
                membership: "join",
                encryption: .notEncrypted
            ),
            SharedCoreCryptoStatusService.JoinedRoomEncryptionRow(
                roomID: "!unknown:example.org",
                membership: "join",
                encryption: .unknown
            ),
            SharedCoreCryptoStatusService.JoinedRoomEncryptionRow(
                roomID: "!invite:example.org",
                membership: "invite",
                encryption: .notEncrypted
            ),
        ]

        XCTAssertEqual(
            SharedCoreCryptoStatusService.joinedRoomEncryption(
                roomID: "!encrypted:example.org",
                rows: joinedRows
            ),
            .encrypted
        )
        XCTAssertEqual(
            SharedCoreCryptoStatusService.joinedRoomEncryption(
                roomID: "!clear:example.org",
                rows: joinedRows
            ),
            .notEncrypted
        )
        XCTAssertEqual(
            SharedCoreCryptoStatusService.joinedRoomEncryption(
                roomID: "!unknown:example.org",
                rows: joinedRows
            ),
            .unknown
        )
        XCTAssertEqual(
            SharedCoreCryptoStatusService.joinedRoomEncryption(
                roomID: "!invite:example.org",
                rows: joinedRows
            ),
            .unknown,
            "An invite Bool must never authorize a joined-room forward"
        )
        XCTAssertEqual(
            SharedCoreCryptoStatusService.joinedRoomEncryption(
                roomID: "!missing:example.org",
                rows: nil
            ),
            .unknown,
            "A failed or absent joined-room snapshot must fail closed"
        )
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

        do {
            _ = try await SharedCoreTypingPresence.presenceSet(
                core: core,
                state: "unavailable",
                statusMsg: "coffee-status-secret"
            )
            XCTFail("Fail-closed SharedCore must not set presence without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-presence-set-no-session"))
            for forbidden in ["password", "syt_", "coffee-status-secret", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreVerificationLiveMapsPhasesWithoutEcho() {
        let incoming = SharedCoreVerificationLive.state(
            phase: "requested",
            direction: "incoming",
            flowId: "flow-1",
            otherUserId: "@bob:example.org",
            otherDeviceId: "DEVICE1"
        )
        guard case let .requestReceived(request) = incoming else {
            XCTFail("Incoming requested must map to requestReceived")
            return
        }
        XCTAssertEqual(request.flowID, "flow-1")
        XCTAssertEqual(request.userID, "@bob:example.org")
        let publicError = String(describing: incoming)
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "requested",
                direction: "outgoing",
                flowId: "flow-2",
                otherUserId: "@bob:example.org",
                otherDeviceId: nil
            ),
            .requestSent
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "sas_ready",
                direction: "outgoing",
                flowId: "flow-3",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1",
                decimals: [1, 2, 3]
            ),
            .decimals([1, 2, 3])
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "done",
                direction: "outgoing",
                flowId: "flow-4",
                otherUserId: "@bob:example.org",
                otherDeviceId: nil
            ),
            .finished
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "confirmed",
                direction: "outgoing",
                flowId: "flow-5",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1"
            ),
            .confirmed
        )
        XCTAssertNotEqual(
            SharedCoreVerificationLive.state(
                phase: "confirmed",
                direction: "outgoing",
                flowId: "flow-5",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1"
            ),
            .finished
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "mismatched",
                direction: "outgoing",
                flowId: "flow-6",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1"
            ),
            .mismatched
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "sas_ready",
                direction: "outgoing",
                flowId: "flow-7",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1"
            ),
            .failed
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "failed",
                direction: "incoming",
                flowId: "flow-8",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1"
            ),
            .failed
        )
        XCTAssertTrue(SharedCoreVerificationLive.needsSasStart(phase: "ready", direction: "outgoing"))
        XCTAssertFalse(SharedCoreVerificationLive.needsSasStart(phase: "started", direction: "incoming"))
        XCTAssertFalse(SharedCoreVerificationLive.needsSasStart(phase: "ready", direction: "incoming"))
        XCTAssertFalse(SharedCoreVerificationLive.needsSasStart(phase: "started", direction: "outgoing"))
        XCTAssertFalse(SharedCoreVerificationLive.needsSasStart(phase: "sas_ready", direction: "incoming"))
        XCTAssertTrue(SharedCoreVerificationLive.isTerminal(phase: "done"))
        XCTAssertTrue(SharedCoreVerificationLive.isTerminal(phase: "cancelled"))
        XCTAssertTrue(SharedCoreVerificationLive.isTerminal(phase: "failed"))
        XCTAssertFalse(SharedCoreVerificationLive.isTerminal(phase: "sas_ready"))
        XCTAssertEqual(
            SharedCoreVerificationLive.selectedFlowId(
                requests: [("incoming", "requested"), ("sas", "sas_ready")],
                preferring: "sas"
            ),
            "sas"
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.selectedFlowId(
                requests: [("incoming", "requested"), ("sas", "sas_ready")],
                preferring: nil
            ),
            "incoming"
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.selectedFlowId(
                requests: [("done", "done"), ("incoming", "requested")],
                preferring: "done"
            ),
            "done"
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.selectedFlowId(
                requests: [("done", "done")],
                preferring: nil
            ),
            "done"
        )
        XCTAssertNil(
            SharedCoreVerificationLive.selectedFlowId(requests: [], preferring: "missing")
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "ready",
                direction: "outgoing",
                flowId: "flow-8",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1"
            ),
            .accepted
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "ready",
                direction: "incoming",
                flowId: "flow-9",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1"
            ),
            .sasStarted
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "started",
                direction: "incoming",
                flowId: "flow-10",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1"
            ),
            .sasStarted,
            "Incoming Started is observation-only because Rust owns protocol acceptance"
        )
        XCTAssertEqual(
            SharedCoreVerificationLive.state(
                phase: "started",
                direction: "outgoing",
                flowId: "flow-11",
                otherUserId: "@bob:example.org",
                otherDeviceId: "DEVICE1"
            ),
            .sasStarted
        )
    }

    func testSharedCoreVerificationActionsWithoutSessionFailClosedWithoutEcho() async {
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let service = SharedCoreCryptoStatusService(host: host)
        let started = await service.requestDeviceVerification()
        let publicError = String(describing: started)
        XCTAssertEqual(started, .failed("Device verification is unavailable."))
        for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
        let accepted = await service.acceptVerificationRequest()
        XCTAssertEqual(accepted, .unavailable("Device verification is unavailable."))
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

        let password = "not-a-secret"
        do {
            _ = try await SharedCoreDevices.deviceDeletePassword(
                core: core,
                operationId: 9,
                sessionGeneration: 1,
                password: password
            )
            XCTFail("Fail-closed SharedCore must not finish device delete with a password without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-device-delete-password-no-session"))
            for forbidden in ["syt_", "token", password] {
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

        do {
            _ = try await SharedCoreRoomLeaveJoin.roomSetFavorite(
                core: core,
                roomId: roomId,
                favorite: true
            )
            XCTFail("Fail-closed SharedCore must not favorite a room without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-room-set-favorite-no-session"))
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
                action: "mark_read",
                intent: "explicit_user"
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

    func testSharedCoreSessionStatusWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let userId = "@alice:example.org"
        let homeserver = "https://matrix.example.org"
        let deviceId = "DEVICEABC"

        do {
            let snapshot = try await SharedCoreSessionStatus.sessionSnapshot(core: core)
            XCTAssertEqual(snapshot.status, "logged_out")
            XCTAssertNil(snapshot.userId)
            XCTAssertNil(snapshot.deviceId)
            XCTAssertNil(snapshot.homeserverUrl)
            XCTAssertNil(snapshot.sessionGeneration)
        } catch {
            XCTFail("Fail-closed SharedCore must return the registered logged-out session snapshot")
        }

        do {
            _ = try await SharedCoreSessionStatus.syncStatus(core: core)
            XCTFail("Fail-closed SharedCore must not read sync status from the iOS platform")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-sync-status-platform-unavailable"))
            for forbidden in ["syt_", "token", userId, homeserver, deviceId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreSessionStatus.mediaConfig(core: core)
            XCTFail("Fail-closed SharedCore must not read media config without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-media-config-no-session"))
            for forbidden in ["syt_", "token", userId, homeserver, deviceId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreSessionStatus.secretStorageStatus(core: core)
            XCTFail("Fail-closed SharedCore must not read secret-storage status without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("v-crypto.4-secret-storage-requires-session"))
            for forbidden in ["syt_", "token", userId, homeserver, deviceId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreNseStoreWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let userId = "@alice:example.org"
        let homeserver = "https://matrix.example.org"
        let roomId = "!s11SecretRoom:example.org"
        let eventId = "$s11SecretEvent:example.org"

        do {
            _ = try await SharedCoreNseStore.storeStatus(core: core)
            XCTFail("Fail-closed SharedCore must not report NSE store status before open")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s11-nse-store-not-open"))
            for forbidden in ["syt_", "token", userId, homeserver, roomId, eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreNseStore.eventPreview(
                core: core,
                roomId: roomId,
                eventId: eventId
            )
            XCTFail("Fail-closed SharedCore must not read an NSE preview before open")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s11-nse-store-not-open"))
            for forbidden in ["syt_", "token", userId, homeserver, roomId, eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreTimelineForwardWithoutSessionFailsClosed() async {
        let core = SharedCore()
        let sourceRoomId = "!s930SecretSource:example.org"
        let targetRoomId = "!s930SecretTarget:example.org"
        let eventId = "$s930SecretEvent:example.org"

        do {
            _ = try await SharedCoreTimelineForward.timelineForwardText(
                core: core,
                sourceRoomId: sourceRoomId,
                eventId: eventId,
                targetRoomId: targetRoomId,
                asQuote: false,
                confirmedEncryptionDowngrade: false
            )
            XCTFail("Fail-closed SharedCore must not forward timeline text without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-forward-text-no-session"))
            for forbidden in ["syt_", "token", sourceRoomId, targetRoomId, eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }

        do {
            _ = try await SharedCoreTimelineForward.timelineForwardMedia(
                core: core,
                sourceRoomId: sourceRoomId,
                eventId: eventId,
                targetRoomId: targetRoomId,
                confirmedEncryptionDowngrade: true
            )
            XCTFail("Fail-closed SharedCore must not forward timeline media without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p2-timeline-forward-media-no-session"))
            for forbidden in ["syt_", "token", sourceRoomId, targetRoomId, eventId] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreTimelineForwardWrapperForwardsDowngradeAuthorizationExactly() throws {
        let testsURL = URL(fileURLWithPath: #filePath)
        let serviceURL = testsURL
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("Synara/Services/SharedCoreTimelineForward.swift")
        let source = try String(contentsOf: serviceURL, encoding: .utf8)
        XCTAssertEqual(
            source.components(separatedBy: "confirmedEncryptionDowngrade: confirmedEncryptionDowngrade").count - 1,
            2
        )
        XCTAssertFalse(source.contains("confirmedEncryptionDowngrade: false"))
        XCTAssertFalse(source.contains("confirmedEncryptionDowngrade: true"))
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

    func testLiveCoreStoreMigratesToSharedContainerBeforeOpen() throws {
        let fileManager = FileManager.default
        let root = fileManager.temporaryDirectory
            .appendingPathComponent("synara-core-store-migration-\(UUID().uuidString)", isDirectory: true)
        defer { try? fileManager.removeItem(at: root) }
        let legacy = root.appendingPathComponent("legacy", isDirectory: true)
        let shared = root.appendingPathComponent("group/SynaraCore", isDirectory: true)
        try fileManager.createDirectory(at: legacy, withIntermediateDirectories: true)
        let marker = legacy.appendingPathComponent("persisted-session-marker")
        try Data("present".utf8).write(to: marker)

        let resolved = SharedCoreProductHost.resolvedLiveStoreRoot(
            legacyRoot: legacy,
            sharedRoot: shared,
            fileManager: fileManager
        )

        XCTAssertEqual(resolved, shared)
        XCTAssertFalse(fileManager.fileExists(atPath: legacy.path))
        XCTAssertTrue(
            fileManager.fileExists(
                atPath: shared.appendingPathComponent("persisted-session-marker").path
            )
        )
        XCTAssertFalse(SynaraSharedConstants.sharedCoreStoreIsReady(at: shared, fileManager: fileManager))
        XCTAssertTrue(
            try SharedCoreProductHost.publishNseStoreReady(
                at: shared,
                fileManager: fileManager,
                expectedSharedRoot: shared
            )
        )
        XCTAssertTrue(SynaraSharedConstants.sharedCoreStoreIsReady(at: shared, fileManager: fileManager))
    }

    func testLiveCoreStoreReplacesEmptySharedDirectoryBeforeMigration() throws {
        let fileManager = FileManager.default
        let root = fileManager.temporaryDirectory
            .appendingPathComponent("synara-core-empty-destination-\(UUID().uuidString)", isDirectory: true)
        defer { try? fileManager.removeItem(at: root) }
        let legacy = root.appendingPathComponent("legacy", isDirectory: true)
        let shared = root.appendingPathComponent("group/SynaraCore", isDirectory: true)
        try fileManager.createDirectory(at: legacy, withIntermediateDirectories: true)
        try fileManager.createDirectory(at: shared, withIntermediateDirectories: true)
        try Data("present".utf8).write(
            to: legacy.appendingPathComponent("persisted-session-marker")
        )

        let resolved = SharedCoreProductHost.resolvedLiveStoreRoot(
            legacyRoot: legacy,
            sharedRoot: shared,
            fileManager: fileManager
        )

        XCTAssertEqual(resolved, shared)
        XCTAssertFalse(fileManager.fileExists(atPath: legacy.path))
        XCTAssertTrue(
            fileManager.fileExists(
                atPath: shared.appendingPathComponent("persisted-session-marker").path
            )
        )
        XCTAssertFalse(SynaraSharedConstants.sharedCoreStoreIsReady(at: shared, fileManager: fileManager))
        XCTAssertTrue(
            try SharedCoreProductHost.publishNseStoreReady(
                at: shared,
                fileManager: fileManager,
                expectedSharedRoot: shared
            )
        )
        XCTAssertTrue(SynaraSharedConstants.sharedCoreStoreIsReady(at: shared, fileManager: fileManager))
    }

    func testLiveCoreStoreFailsBackWhenBothRootsContainData() throws {
        let fileManager = FileManager.default
        let root = fileManager.temporaryDirectory
            .appendingPathComponent("synara-core-ambiguous-destination-\(UUID().uuidString)", isDirectory: true)
        defer { try? fileManager.removeItem(at: root) }
        let legacy = root.appendingPathComponent("legacy", isDirectory: true)
        let shared = root.appendingPathComponent("group/SynaraCore", isDirectory: true)
        try fileManager.createDirectory(at: legacy, withIntermediateDirectories: true)
        try fileManager.createDirectory(at: shared, withIntermediateDirectories: true)
        try Data("legacy".utf8).write(to: legacy.appendingPathComponent("legacy-store"))
        try Data("shared".utf8).write(to: shared.appendingPathComponent("unexpected-store"))

        let resolved = SharedCoreProductHost.resolvedLiveStoreRoot(
            legacyRoot: legacy,
            sharedRoot: shared,
            fileManager: fileManager
        )

        XCTAssertEqual(resolved, legacy)
        XCTAssertTrue(fileManager.fileExists(atPath: legacy.appendingPathComponent("legacy-store").path))
        XCTAssertTrue(fileManager.fileExists(atPath: shared.appendingPathComponent("unexpected-store").path))
        XCTAssertFalse(SynaraSharedConstants.sharedCoreStoreIsReady(at: shared, fileManager: fileManager))
    }

    func testLiveNseNotificationClientResolvesRealEventWhenConfigured() async throws {
        let environment = ProcessInfo.processInfo.environment
        let enabled = environment["SYNARA_LIVE_NSE_RESOLVE_SMOKE"]
            ?? environment["TEST_RUNNER_SYNARA_LIVE_NSE_RESOLVE_SMOKE"]
        guard enabled == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_NSE_RESOLVE_SMOKE=1 for the local NSE resolver smoke.")
        }
        guard let roomID = environment["SYNARA_LIVE_NSE_ROOM_ID"]
                ?? environment["TEST_RUNNER_SYNARA_LIVE_NSE_ROOM_ID"],
              let eventID = environment["SYNARA_LIVE_NSE_EVENT_ID"]
                ?? environment["TEST_RUNNER_SYNARA_LIVE_NSE_EVENT_ID"] else {
            throw XCTSkip("The local NSE resolver smoke needs a room ID and event ID.")
        }

        let session = try XCTUnwrap(KeychainSecureSessionStore().load())
        let storeRoot = SharedCoreProductHost.liveStoreRoot()
        XCTAssertTrue(SynaraSharedConstants.sharedCoreStoreIsReady(at: storeRoot))
        let core = SharedCore.newWithSecretStore(store: KeychainIosSecretVault())
        let started = Date()

        let preview: NseEventPreviewDto
        do {
            preview = try await core.nseResolveEventPreview(
                userId: session.userID,
                homeserverUrl: session.homeserverURL.absoluteString,
                storeRoot: storeRoot.path,
                roomId: roomID,
                eventId: eventID
            )
        } catch {
            try? await core.nseCloseReadOnlyStore()
            if case let NseStoreError.Failed(code, _) = error {
                XCTFail("NSE resolver failed with static diagnostic \(code).")
            } else {
                XCTFail("NSE resolver failed with unexpected error type \(String(reflecting: type(of: error))).")
            }
            return
        }
        do {
            try await core.nseCloseReadOnlyStore()
        } catch {
            if case let NseStoreError.Failed(code, _) = error {
                XCTFail("NSE teardown failed with static diagnostic \(code).")
            } else {
                XCTFail("NSE teardown failed with unexpected error type \(String(reflecting: type(of: error))).")
            }
            return
        }

        XCTAssertLessThan(Date().timeIntervalSince(started), 21)
        XCTAssertEqual(preview.eventType, "m.room.message")
        XCTAssertFalse(preview.body?.isEmpty ?? true)
    }
}
