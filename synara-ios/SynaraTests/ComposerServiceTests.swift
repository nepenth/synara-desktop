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

}
