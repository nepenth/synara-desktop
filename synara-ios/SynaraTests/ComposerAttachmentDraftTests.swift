import XCTest
@testable import Synara

final class ComposerAttachmentDraftTests: XCTestCase {
    func testCanSendAllowsAttachmentOnlyMessage() {
        let drafts = [makeDraft(name: "one.jpg")]

        XCTAssertTrue(ComposerAttachmentDraftList.canSend(text: "   ", drafts: drafts))
        XCTAssertTrue(ComposerAttachmentDraftList.canSend(text: "caption", drafts: drafts))
        XCTAssertTrue(ComposerAttachmentDraftList.canSend(text: "caption", drafts: []))
        XCTAssertFalse(ComposerAttachmentDraftList.canSend(text: "   ", drafts: []))
    }

    func testAppendingStopsAtTenImages() {
        let existing = (0 ..< 8).map { makeDraft(name: "existing-\($0).jpg") }
        let incoming = (0 ..< 4).map { makeDraft(name: "incoming-\($0).jpg") }

        let outcome = ComposerAttachmentDraftList.appending(incoming, to: existing)

        XCTAssertEqual(outcome.addedCount, 2)
        XCTAssertEqual(outcome.drafts.count, ComposerAttachmentDraftList.maxCount)
        XCTAssertEqual(outcome.rejection, .limitReached)
        XCTAssertEqual(outcome.drafts.last?.displayName, "incoming-1.jpg")
    }

    func testEleventhImageIsRejectedWithoutMutatingEarlierDrafts() {
        let existing = (0 ..< ComposerAttachmentDraftList.maxCount).map { makeDraft(name: "full-\($0).jpg") }

        let outcome = ComposerAttachmentDraftList.appending([makeDraft(name: "overflow.jpg")], to: existing)

        XCTAssertEqual(outcome.addedCount, 0)
        XCTAssertEqual(outcome.drafts, existing)
        XCTAssertEqual(outcome.rejection, .limitReached)
        XCTAssertEqual(
            ComposerAttachmentDraftList.userMessage(for: .limitReached),
            "You can attach up to 10 images."
        )
    }

    func testRemoveDropsOnlyTheRequestedDraft() {
        let first = makeDraft(name: "keep.jpg")
        let second = makeDraft(name: "drop.jpg")

        let remaining = ComposerAttachmentDraftList.remove(id: second.id, from: [first, second])

        XCTAssertEqual(remaining.map(\.id), [first.id])
    }

    func testRejectsEmptyPayload() {
        let draft = makeDraft(name: "empty.jpg", data: Data())

        XCTAssertEqual(
            ComposerAttachmentDraftList.validate(draft, against: []),
            .empty
        )
    }

    func testRejectsPayloadOverNativeUploadBound() {
        let oversize = Data(count: ComposerAttachmentDraftList.maxBytesPerItem + 1)
        let draft = makeDraft(name: "huge.jpg", data: oversize)

        XCTAssertEqual(
            ComposerAttachmentDraftList.validate(draft, against: []),
            .tooLarge
        )
        XCTAssertEqual(ComposerAttachmentDraftList.maxBytesPerItem, 100 * 1024 * 1024)
    }

    func testAcceptsImageAndVideoMimeTypesOnly() {
        XCTAssertTrue(ComposerAttachmentDraftList.isAllowedMimeType("image/jpeg"))
        XCTAssertTrue(ComposerAttachmentDraftList.isAllowedMimeType("image/heic"))
        XCTAssertTrue(ComposerAttachmentDraftList.isAllowedMimeType("video/mp4"))
        XCTAssertFalse(ComposerAttachmentDraftList.isAllowedMimeType("application/pdf"))
        XCTAssertFalse(ComposerAttachmentDraftList.isAllowedMimeType("application/octet-stream"))

        let pdf = makeDraft(name: "notes.pdf", mimeType: "application/pdf")
        XCTAssertEqual(
            ComposerAttachmentDraftList.validate(pdf, against: []),
            .unsupportedType
        )
    }

    func testAcceptsItemAtExactNativeByteBound() {
        let exact = makeDraft(
            name: "exact.jpg",
            data: Data(count: ComposerAttachmentDraftList.maxBytesPerItem)
        )

        XCTAssertNil(ComposerAttachmentDraftList.validate(exact, against: []))
    }

    private func makeDraft(
        name: String,
        mimeType: String = "image/jpeg",
        data: Data = Data("synara-draft".utf8)
    ) -> ComposerAttachmentDraft {
        ComposerAttachmentDraft(
            displayName: name,
            mimeType: mimeType,
            data: data,
            source: .photoLibrary
        )
    }
}
