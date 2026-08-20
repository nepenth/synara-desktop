import XCTest
@testable import Synara

final class ComposerAttachmentDraftTests: XCTestCase {
    func testCanSendAllowsAttachmentOnlyMessage() {
        let drafts = [makeDraft(name: "one.jpg")]

        XCTAssertTrue(ComposerAttachmentDraftList.canSend(text: "   ", drafts: drafts))
        XCTAssertTrue(ComposerAttachmentDraftList.canSend(text: "caption", drafts: drafts))
        XCTAssertTrue(ComposerAttachmentDraftList.canSend(text: "caption", drafts: []))
        XCTAssertFalse(ComposerAttachmentDraftList.canSend(text: "   ", drafts: []))
        XCTAssertFalse(
            ComposerAttachmentDraftList.canBeginSend(isSending: true, text: "caption", drafts: drafts)
        )
        XCTAssertTrue(
            ComposerAttachmentDraftList.canBeginSend(isSending: false, text: "   ", drafts: drafts)
        )
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
            "You can attach up to 10 attachments."
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

    func testAcceptsImageVideoAndFileMimeTypes() {
        XCTAssertTrue(ComposerAttachmentDraftList.isAllowedMimeType("image/jpeg"))
        XCTAssertTrue(ComposerAttachmentDraftList.isAllowedMimeType("image/heic"))
        XCTAssertTrue(ComposerAttachmentDraftList.isAllowedMimeType("video/mp4"))
        XCTAssertTrue(ComposerAttachmentDraftList.isAllowedMimeType("application/pdf"))
        XCTAssertTrue(ComposerAttachmentDraftList.isAllowedMimeType("application/octet-stream"))
        XCTAssertFalse(ComposerAttachmentDraftList.isAllowedMimeType("   "))

        let pdf = makeDraft(name: "notes.pdf", mimeType: "application/pdf")
        XCTAssertNil(ComposerAttachmentDraftList.validate(pdf, against: []))

        let unnamed = makeDraft(name: "blank.bin", mimeType: " ")
        XCTAssertEqual(
            ComposerAttachmentDraftList.validate(unnamed, against: []),
            .unsupportedType
        )
    }

    func testDraftFromFileURLRespectsSizeBoundAndLoadsPayload() throws {
        let directory = FileManager.default.temporaryDirectory
        let fileURL = directory.appendingPathComponent("synara-draft-file.pdf")
        let payload = Data("Synara file draft".utf8)
        try payload.write(to: fileURL)
        defer {
            try? FileManager.default.removeItem(at: fileURL)
        }

        let draft = try ComposerAttachmentDraftList.draft(fromFileURL: fileURL).get()

        XCTAssertEqual(draft.displayName, "synara-draft-file.pdf")
        XCTAssertEqual(draft.mimeType, "application/pdf")
        XCTAssertEqual(draft.data, payload)
        XCTAssertEqual(draft.source, .file)
        XCTAssertEqual(draft.previewSystemImage, "doc")
    }

    func testDraftFromMissingFileURLFailsClosed() {
        let missing = FileManager.default.temporaryDirectory.appendingPathComponent("synara-missing-draft.bin")

        let result = ComposerAttachmentDraftList.draft(fromFileURL: missing)

        XCTAssertEqual(result, .failure(.couldNotLoad))
    }

    func testMixedImageAndFileDraftsShareTheTenItemCap() {
        let existing = (0 ..< 9).map { makeDraft(name: "photo-\($0).jpg") }
        let pdf = makeDraft(name: "notes.pdf", mimeType: "application/pdf")

        let accepted = ComposerAttachmentDraftList.appending([pdf], to: existing)
        XCTAssertEqual(accepted.addedCount, 1)
        XCTAssertNil(accepted.rejection)

        let overflow = ComposerAttachmentDraftList.appending(
            [makeDraft(name: "overflow.jpg")],
            to: accepted.drafts
        )
        XCTAssertEqual(overflow.addedCount, 0)
        XCTAssertEqual(overflow.rejection, .limitReached)
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
