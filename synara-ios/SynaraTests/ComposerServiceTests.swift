import XCTest
@testable import Synara

final class ComposerServiceTests: XCTestCase {
    func testDraftStorePreservesDraftByRoom() {
        let store = DraftStore()

        store.setDraft("hello", roomID: "!room:matrix.org")

        XCTAssertEqual(store.draft(roomID: "!room:matrix.org"), "hello")
        XCTAssertEqual(store.draft(roomID: "!other:matrix.org"), "")
    }

    func testSendRejectsWhitespaceOnlyMessage() async throws {
        let service = MockMessageSendService()
        let request = MessageSendRequest(
            roomID: "!room:matrix.org",
            body: "   ",
            replyToEventID: nil,
            editEventID: nil
        )

        do {
            _ = try await service.send(request)
            XCTFail("Expected empty message error")
        } catch let error as MessageSendError {
            XCTAssertEqual(error, .emptyMessage)
        }
    }

    func testSendCreatesLocalEchoWithReplyMetadata() async throws {
        let service = MockMessageSendService()
        let request = MessageSendRequest(
            roomID: "!room:matrix.org",
            body: " reply body ",
            replyToEventID: "$parent:matrix.org",
            editEventID: nil
        )

        let item = try await service.send(request)

        XCTAssertEqual(item.kind, .text("reply body"))
        XCTAssertEqual(item.replyToEventID, "$parent:matrix.org")
        XCTAssertFalse(item.isEdited)
    }

    func testSendCreatesFormattedLocalEchoWhenMatrixHTMLIsPresent() async throws {
        let service = MockMessageSendService()
        let request = MessageSendRequest(
            roomID: "!room:matrix.org",
            body: "**ship it**",
            formattedBody: "<strong>ship it</strong>",
            replyToEventID: nil,
            editEventID: nil
        )

        let item = try await service.send(request)

        XCTAssertEqual(item.kind, .formattedText(body: "**ship it**", html: "<strong>ship it</strong>"))
    }

    func testBeginEditOnFailedTextItemLoadsComposer() {
        let failed = TimelineItem.pendingMessage(
            localID: "$pending-failed",
            body: "typo in this mesage",
            senderID: "@alice:matrix.org",
            replyToEventID: "$parent:matrix.org",
            deliveryStatus: .failed
        )

        let session = ComposerEditFlow.begin(
            item: failed,
            currentUserID: "@alice:matrix.org",
            currentDraft: "in progress"
        )

        XCTAssertEqual(session.draft, "typo in this mesage")
        XCTAssertEqual(session.previousDraft, "in progress")
        XCTAssertEqual(session.editTarget.kind, .edit)
        XCTAssertEqual(session.editTarget.eventID, failed.eventID)
        XCTAssertTrue(session.editTarget.isLocalPending)
        XCTAssertEqual(session.editTarget.bannerTitle, "Editing unsent message")
        XCTAssertEqual(session.retryingItem?.id, failed.id)
        XCTAssertNil(session.remoteEditEventID)
    }

    func testSendingFailedEditUpdatesBodyAndRetriesSamePendingID() throws {
        let failed = TimelineItem.pendingMessage(
            localID: "$pending-failed",
            body: "typo in this mesage",
            senderID: "@alice:matrix.org",
            replyToEventID: "$parent:matrix.org",
            deliveryStatus: .failed,
            timestamp: TimelineFixtures.baseDate
        )
        let session = ComposerEditFlow.begin(
            item: failed,
            currentUserID: "@alice:matrix.org",
            currentDraft: ""
        )

        let intent = ComposerEditFlow.sendIntent(
            body: "typo in this message, plus a note",
            replyToEventID: "$ignored-reply:matrix.org",
            session: session
        )

        XCTAssertNil(intent.editEventID)
        XCTAssertEqual(intent.retrying?.id, failed.id)
        XCTAssertEqual(intent.replyToEventID, "$parent:matrix.org")
        XCTAssertEqual(intent.body, "typo in this message, plus a note")

        let updated = TimelineItem.pendingMessage(
            localID: try XCTUnwrap(intent.retrying?.id),
            body: intent.body,
            senderID: failed.senderID,
            replyToEventID: intent.replyToEventID,
            deliveryStatus: .sending,
            timestamp: failed.timestamp
        )

        XCTAssertEqual(updated.id, failed.id)
        XCTAssertEqual(updated.eventID, failed.eventID)
        XCTAssertEqual(updated.kind, .text("typo in this message, plus a note"))
        XCTAssertEqual(updated.timestamp, failed.timestamp)
        XCTAssertEqual(updated.replyToEventID, failed.replyToEventID)
    }

    func testSentMessageEditStillUsesEditEventID() async throws {
        let sent = TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Hello world"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let session = ComposerEditFlow.begin(
            item: sent,
            currentUserID: "@alice:matrix.org",
            currentDraft: "scratch"
        )

        XCTAssertEqual(session.draft, "Hello world")
        XCTAssertNil(session.retryingItem)
        XCTAssertEqual(session.remoteEditEventID, "$event:matrix.org")
        XCTAssertEqual(session.editTarget.bannerTitle, "Editing your message")

        let intent = ComposerEditFlow.sendIntent(
            body: "Hello world, edited",
            replyToEventID: nil,
            session: session
        )

        XCTAssertEqual(intent.editEventID, "$event:matrix.org")
        XCTAssertNil(intent.retrying)

        let request = MessageSendRequest(
            roomID: "!room:matrix.org",
            body: intent.body,
            replyToEventID: intent.replyToEventID,
            editEventID: intent.editEventID
        )
        let item = try await MockMessageSendService().send(request)

        XCTAssertEqual(item.eventID, "$event:matrix.org")
        XCTAssertEqual(item.id, "$event:matrix.org")
        XCTAssertTrue(item.isEdited)
        XCTAssertEqual(item.kind, .text("Hello world, edited"))
    }

    func testCancelEditRestoresComposerDraft() {
        let sent = TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Hello world"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let session = ComposerEditFlow.begin(
            item: sent,
            currentUserID: "@alice:matrix.org",
            currentDraft: "unsent draft"
        )

        XCTAssertEqual(ComposerEditFlow.cancel(session), "unsent draft")
    }

}
