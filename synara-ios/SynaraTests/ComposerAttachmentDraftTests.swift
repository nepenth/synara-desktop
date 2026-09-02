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

    func testSingleAttachmentUsesComposerTextAsItsCaption() {
        let draft = makeDraft(name: "one.jpg")

        XCTAssertEqual(
            ComposerAttachmentSendPlan.make(drafts: [draft], body: "  A useful caption.  "),
            [.attachment(id: draft.id, caption: "A useful caption.")]
        )
    }

    func testSingleAttachmentEditKeepsAttachmentAndEditAsDistinctSteps() {
        let draft = makeDraft(name: "one.jpg")
        let intent = ComposerSendIntent(
            body: "Corrected text",
            replyToEventID: nil,
            editEventID: "$original:example.org",
            retrying: nil
        )

        let transaction = ComposerAttachmentSendTransaction.reusableOrNew(
            existing: nil,
            drafts: [draft],
            body: intent.body,
            proposedIntent: intent
        )

        XCTAssertEqual(
            transaction.steps,
            [
                .attachment(id: draft.id, caption: nil),
                .text(body: "Corrected text"),
            ]
        )
        XCTAssertEqual(transaction.intent.editEventID, "$original:example.org")
    }

    func testSingleAttachmentFailedMessageRetryKeepsRetryAsDistinctTextStep() {
        let draft = makeDraft(name: "one.jpg")
        let failed = TimelineItem.pendingMessage(
            localID: "$pending-failed",
            body: "Retry this",
            senderID: "@alice:example.org",
            replyToEventID: "$parent:example.org",
            threadRootEventID: "$root:example.org",
            deliveryStatus: .failed
        )
        let intent = ComposerSendIntent(
            body: "Retry this",
            replyToEventID: failed.replyToEventID,
            threadRootEventID: failed.threadRootEventID,
            editEventID: nil,
            retrying: failed
        )

        let transaction = ComposerAttachmentSendTransaction.reusableOrNew(
            existing: nil,
            drafts: [draft],
            body: intent.body,
            proposedIntent: intent
        )

        XCTAssertEqual(
            transaction.steps,
            [
                .attachment(id: draft.id, caption: nil),
                .text(body: "Retry this"),
            ]
        )
        XCTAssertEqual(transaction.intent.retrying?.id, "$pending-failed")
        XCTAssertEqual(transaction.intent.replyToEventID, "$parent:example.org")
        XCTAssertEqual(transaction.intent.threadRootEventID, "$root:example.org")
    }

    func testSingleAttachmentWithoutTextHasNoCaption() {
        let draft = makeDraft(name: "one.jpg")

        XCTAssertEqual(
            ComposerAttachmentSendPlan.make(drafts: [draft], body: " \n "),
            [.attachment(id: draft.id, caption: nil)]
        )
    }

    func testMultipleAttachmentsCarryTextOnceAsTrailingMessage() {
        let first = makeDraft(name: "one.jpg")
        let second = makeDraft(name: "two.jpg")

        XCTAssertEqual(
            ComposerAttachmentSendPlan.make(drafts: [first, second], body: "Context"),
            [
                .attachment(id: first.id, caption: nil),
                .attachment(id: second.id, caption: nil),
                .text(body: "Context"),
            ]
        )
    }

    func testMultipleAttachmentsWithoutTextHaveNoTrailingMessage() {
        let first = makeDraft(name: "one.jpg")
        let second = makeDraft(name: "two.jpg")

        XCTAssertEqual(
            ComposerAttachmentSendPlan.make(drafts: [first, second], body: ""),
            [
                .attachment(id: first.id, caption: nil),
                .attachment(id: second.id, caption: nil),
            ]
        )
    }

    func testTextOnlyPlanRemainsOneTextStep() {
        XCTAssertEqual(
            ComposerAttachmentSendPlan.make(drafts: [], body: "  Hello  "),
            [.text(body: "Hello")]
        )
    }

    func testTrailingTextIsPresentOnlyForMultiAttachmentPlan() {
        let first = makeDraft(name: "one.jpg")
        let second = makeDraft(name: "two.jpg")

        XCTAssertNil(
            ComposerAttachmentSendPlan.trailingText(
                in: ComposerAttachmentSendPlan.make(drafts: [first], body: "Caption")
            )
        )
        XCTAssertEqual(
            ComposerAttachmentSendPlan.trailingText(
                in: ComposerAttachmentSendPlan.make(drafts: [first, second], body: "Context")
            ),
            "Context"
        )
    }

    func testSingleAttachmentUploadCarriesCaptionAndRelations() async {
        let draft = makeDraft(name: "one.jpg")
        let uploader = RecordingMediaUploader()
        let plan = ComposerAttachmentSendPlan.make(drafts: [draft], body: "**Caption**")

        let uploaded = await ComposerAttachmentSend.uploadAll(
            [draft],
            steps: plan,
            roomID: "!room:example.org",
            replyToEventID: "$reply:example.org",
            threadRootEventID: "$root:example.org",
            uploader: uploader,
            onState: { _ in },
            onUploaded: { _, _ in }
        )
        let requests = await uploader.recordedRequests()

        XCTAssertTrue(uploaded)
        XCTAssertEqual(requests.count, 1)
        XCTAssertEqual(requests.first?.caption, "**Caption**")
        XCTAssertNotNil(requests.first?.formattedCaption)
        XCTAssertEqual(requests.first?.replyToEventID, "$reply:example.org")
        XCTAssertEqual(requests.first?.threadRootEventID, "$root:example.org")
        XCTAssertEqual(requests.first?.transactionID, draft.transactionID)
    }

    func testPartialUploadStopsAndDoesNotAttemptLaterDrafts() async {
        let first = makeDraft(name: "one.jpg")
        let second = makeDraft(name: "two.jpg")
        let third = makeDraft(name: "three.jpg")
        let uploader = RecordingMediaUploader(failingRequestIndex: 1)
        let plan = ComposerAttachmentSendPlan.make(
            drafts: [first, second, third],
            body: "Context"
        )

        let uploaded = await ComposerAttachmentSend.uploadAll(
            [first, second, third],
            steps: plan,
            roomID: "!room:example.org",
            replyToEventID: nil,
            threadRootEventID: nil,
            uploader: uploader,
            onState: { _ in },
            onUploaded: { _, _ in }
        )
        let requests = await uploader.recordedRequests()

        XCTAssertFalse(uploaded)
        XCTAssertEqual(requests.map(\.displayName), ["one.jpg", "two.jpg"])
        XCTAssertTrue(requests.allSatisfy { $0.caption == nil })
    }

    func testPartialRetryPreservesOriginalTrailingTextSemantics() {
        let first = makeDraft(name: "one.jpg")
        let second = makeDraft(name: "two.jpg")
        let initial = ComposerAttachmentSendPlan.make(
            drafts: [first, second],
            body: "Context"
        )
        let afterFirstSuccess = ComposerAttachmentSendPlan.removingAttachment(
            id: first.id,
            from: initial
        )

        let retry = ComposerAttachmentSendPlan.reusableOrNew(
            existing: afterFirstSuccess,
            drafts: [second],
            body: "Context"
        )

        XCTAssertEqual(
            retry,
            [
                .attachment(id: second.id, caption: nil),
                .text(body: "Context"),
            ]
        )
    }

    func testPartialRetryRetainsOriginalRelationSnapshotWhileUpdatingUnsentText() {
        let first = makeDraft(name: "one.jpg")
        let second = makeDraft(name: "two.jpg")
        let originalIntent = ComposerSendIntent(
            body: "Original context",
            replyToEventID: "$original-parent:example.org",
            threadRootEventID: "$original-root:example.org",
            editEventID: nil,
            retrying: nil
        )
        let initial = ComposerAttachmentSendTransaction.reusableOrNew(
            existing: nil,
            drafts: [first, second],
            body: originalIntent.body,
            proposedIntent: originalIntent
        )
        let afterFirstSuccess = initial.removingAttachment(id: first.id)
        let changedUIIntent = ComposerSendIntent(
            body: "Updated context",
            replyToEventID: "$mutable-ui-parent:example.org",
            threadRootEventID: "$mutable-ui-root:example.org",
            editEventID: nil,
            retrying: nil
        )

        let retry = ComposerAttachmentSendTransaction.reusableOrNew(
            existing: afterFirstSuccess,
            drafts: [second],
            body: changedUIIntent.body,
            proposedIntent: changedUIIntent
        )

        XCTAssertEqual(retry.intent.replyToEventID, "$original-parent:example.org")
        XCTAssertEqual(retry.intent.threadRootEventID, "$original-root:example.org")
        XCTAssertEqual(retry.intent.body, "Updated context")
        XCTAssertEqual(
            retry.steps,
            [
                .attachment(id: second.id, caption: nil),
                .text(body: "Updated context"),
            ]
        )
    }

    func testPartialRetryPreservesTrailingRoleWhenRetainedTextIsEdited() {
        let first = makeDraft(name: "one.jpg")
        let second = makeDraft(name: "two.jpg")
        let initial = ComposerAttachmentSendPlan.make(
            drafts: [first, second],
            body: "Original context"
        )
        let afterFirstSuccess = ComposerAttachmentSendPlan.removingAttachment(
            id: first.id,
            from: initial
        )

        let retry = ComposerAttachmentSendPlan.reusableOrNew(
            existing: afterFirstSuccess,
            drafts: [second],
            body: "Updated context"
        )

        XCTAssertEqual(
            retry,
            [
                .attachment(id: second.id, caption: nil),
                .text(body: "Updated context"),
            ]
        )
    }

    func testPartialRetryAssignsNewTextOnceWhenOriginalPlanHadNone() {
        let first = makeDraft(name: "one.jpg")
        let second = makeDraft(name: "two.jpg")
        let third = makeDraft(name: "three.jpg")
        let initial = ComposerAttachmentSendPlan.make(
            drafts: [first, second, third],
            body: ""
        )
        let afterFirstSuccess = ComposerAttachmentSendPlan.removingAttachment(
            id: first.id,
            from: initial
        )

        let withTrailingText = ComposerAttachmentSendPlan.reusableOrNew(
            existing: afterFirstSuccess,
            drafts: [second, third],
            body: "New context"
        )
        XCTAssertEqual(
            withTrailingText,
            [
                .attachment(id: second.id, caption: nil),
                .attachment(id: third.id, caption: nil),
                .text(body: "New context"),
            ]
        )

        let afterSecondSuccess = ComposerAttachmentSendPlan.removingAttachment(
            id: second.id,
            from: withTrailingText
        )
        XCTAssertEqual(
            ComposerAttachmentSendPlan.reusableOrNew(
                existing: afterSecondSuccess,
                drafts: [third],
                body: "New context"
            ),
            [
                .attachment(id: third.id, caption: nil),
                .text(body: "New context"),
            ]
        )
    }

    func testPartialRetryUsesNewTextAsCaptionWithOneAttachmentRemaining() {
        let first = makeDraft(name: "one.jpg")
        let second = makeDraft(name: "two.jpg")
        let initial = ComposerAttachmentSendPlan.make(
            drafts: [first, second],
            body: ""
        )
        let afterFirstSuccess = ComposerAttachmentSendPlan.removingAttachment(
            id: first.id,
            from: initial
        )

        XCTAssertEqual(
            ComposerAttachmentSendPlan.reusableOrNew(
                existing: afterFirstSuccess,
                drafts: [second],
                body: "New caption"
            ),
            [.attachment(id: second.id, caption: "New caption")]
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
        XCTAssertEqual(ComposerAttachmentDraftList.maxBytesPerItem, 32 * 1024 * 1024)
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

private actor RecordingMediaUploader: MediaUploading {
    private var requests: [MediaUploadRequest] = []
    private let failingRequestIndex: Int?

    init(failingRequestIndex: Int? = nil) {
        self.failingRequestIndex = failingRequestIndex
    }

    func upload(_ request: MediaUploadRequest) async -> MediaUploadState {
        let requestIndex = requests.count
        requests.append(request)
        if requestIndex == failingRequestIndex {
            return .failed("Media could not be uploaded.")
        }
        return await MockMediaUploadService().upload(request)
    }

    func recordedRequests() -> [MediaUploadRequest] {
        requests
    }
}
