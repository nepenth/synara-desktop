import UIKit
import XCTest
@testable import Synara

final class MediaServiceTests: XCTestCase {
    func testMediaLoaderBlocksEncryptedMedia() async throws {
        let resource = MediaResource(
            id: "$encrypted-media",
            filename: "secret.png",
            authenticatedURL: try XCTUnwrap(URL(string: "mxc://matrix.org/secret")),
            requiresAuthentication: true,
            isEncrypted: true
        )

        let state = await MockMediaLoader().loadThumbnail(for: resource)

        XCTAssertEqual(state, .failed("Encrypted media requires recovered keys before it can be opened."))
    }

    func testMediaLoaderDoesNotExposeAuthenticatedURLInDescription() async throws {
        let resource = MediaResource(
            id: "$media:matrix.org",
            filename: "private.png",
            authenticatedURL: try XCTUnwrap(URL(string: "mxc://matrix.org/private-id")),
            requiresAuthentication: true
        )

        let state = await MockMediaLoader().loadThumbnail(for: resource)

        XCTAssertEqual(state, .thumbnail(resource))
        XCTAssertFalse(resource.safeDescription.contains("matrix.org"))
    }

    func testMediaResourceDetectsImageFromFilename() throws {
        let resource = MediaResource(
            id: "$image",
            filename: "photo.jpg",
            authenticatedURL: try XCTUnwrap(URL(string: "mxc://matrix.org/photo")),
            requiresAuthentication: true
        )

        XCTAssertTrue(resource.isImageMedia)
        XCTAssertEqual(resource.resolvedMimeType, "image/jpeg")
    }

    func testMediaFormattingOmitsMissingFileSize() {
        XCTAssertNil(MediaFormatting.formattedFileSize(nil))
    }

    func testMediaFormattingFormatsFileSize() {
        XCTAssertEqual(MediaFormatting.formattedFileSize(1_024), "1 KB")
    }

    func testMediaAttachmentSupportResolvesMimeTypeFromExtension() throws {
        let url = try XCTUnwrap(URL(fileURLWithPath: "/tmp/report.pdf"))
        XCTAssertEqual(MediaAttachmentSupport.mimeType(for: url), "application/pdf")
        XCTAssertEqual(MediaAttachmentSupport.displayName(for: url), "report.pdf")
    }

    func testUploadSanitizesLocalFilePath() async {
        let request = MediaUploadRequest(
            roomID: "!room:matrix.org",
            source: .file,
            displayName: "/private/tmp/photo.png",
            data: Data("image".utf8),
            mimeType: "image/png",
            caption: "A cat",
            formattedCaption: "<strong>A cat</strong>",
            replyToEventID: "$reply:matrix.org",
            threadRootEventID: "$root:matrix.org",
            transactionID: "stable-media-transaction",
            mentionUserIDs: ["@alice:matrix.org"],
            mentionRoom: true
        )

        XCTAssertEqual(request.transactionID, "stable-media-transaction")
        XCTAssertEqual(request.mentionUserIDs, ["@alice:matrix.org"])
        XCTAssertEqual(request.mentionRoom, true)

        let state = await MockMediaUploadService().upload(request)

        guard case .uploaded(let item) = state, case .mediaPlaceholder(let resource) = item.kind else {
            XCTFail("Expected uploaded media item")
            return
        }

        XCTAssertEqual(resource.safeDescription, "photo.png")
        XCTAssertEqual(resource.caption, "A cat")
        XCTAssertEqual(resource.formattedCaption, "<strong>A cat</strong>")
        XCTAssertEqual(item.replyToEventID, "$reply:matrix.org")
        XCTAssertEqual(item.threadRootEventID, "$root:matrix.org")
        XCTAssertFalse(resource.safeDescription.contains("/private/tmp"))
    }

    func testQueuedUploadIsPendingUnderItsTransactionIDAndShowsLocalBytes() throws {
        let previews = LocalUploadPreviewStore()
        let request = imageUploadRequest(transactionID: "synara-attachment-a")

        let item = MediaUploadAcknowledgement.item(
            eventID: "",
            transactionID: "synara-attachment-a",
            isQueued: true,
            request: request,
            senderID: "@alice:matrix.org",
            previews: previews
        )

        XCTAssertEqual(item.id, "$pending-synara-attachment-a")
        XCTAssertEqual(item.eventID, item.id)
        XCTAssertNil(item.serverEventID, "a queued upload has no server event to receipt")
        XCTAssertEqual(item.deliveryStatus, .sending)
        XCTAssertTrue(item.isLocalPending)
        XCTAssertEqual(item.transactionID, "synara-attachment-a")
        guard case let .mediaPlaceholder(resource) = item.kind else {
            return XCTFail("Expected a media row")
        }
        let url = try XCTUnwrap(resource.authenticatedURL)
        XCTAssertEqual(url.scheme, LocalUploadPreviewStore.urlScheme)
        XCTAssertNotEqual(url.scheme, "mxc", "a pending row must never request a server thumbnail")
        XCTAssertEqual(previews.data(for: url), request.data)
    }

    func testQueuedUploadsNeverShareAnEmptyIdentity() {
        let previews = LocalUploadPreviewStore()
        let first = MediaUploadAcknowledgement.item(
            eventID: "", transactionID: "synara-attachment-a", isQueued: true,
            request: imageUploadRequest(transactionID: "synara-attachment-a"),
            senderID: "@alice:matrix.org", previews: previews
        )
        let second = MediaUploadAcknowledgement.item(
            eventID: "", transactionID: "synara-attachment-b", isQueued: true,
            request: imageUploadRequest(transactionID: "synara-attachment-b"),
            senderID: "@alice:matrix.org", previews: previews
        )
        // Core omitted the transaction id: fall back to the request's, then
        // to a generated one. An empty id is never a row identity.
        let fromRequest = MediaUploadAcknowledgement.item(
            eventID: "", transactionID: "", isQueued: true,
            request: imageUploadRequest(transactionID: "synara-attachment-c"),
            senderID: "@alice:matrix.org", previews: previews
        )
        let generated = MediaUploadAcknowledgement.item(
            eventID: "", transactionID: "", isQueued: false,
            request: imageUploadRequest(transactionID: nil),
            senderID: "@alice:matrix.org", previews: previews
        )

        XCTAssertNotEqual(first.id, second.id)
        XCTAssertEqual(fromRequest.id, "$pending-synara-attachment-c")
        XCTAssertTrue(generated.id.hasPrefix("$pending-synara-attachment-"))
        XCTAssertEqual(generated.deliveryStatus, .sending, "an empty event id is pending whatever the status")
        for item in [first, second, fromRequest, generated] {
            XCTAssertFalse(item.id.isEmpty)
            XCTAssertFalse(item.eventID.isEmpty)
            XCTAssertNotEqual(item.transactionID ?? "", "")
        }
    }

    func testSentUploadKeepsItsServerEventAndTransactionID() throws {
        let item = MediaUploadAcknowledgement.item(
            eventID: "$sent:matrix.org",
            transactionID: "synara-attachment-a",
            isQueued: false,
            request: imageUploadRequest(transactionID: "synara-attachment-a"),
            senderID: "@alice:matrix.org",
            previews: LocalUploadPreviewStore()
        )

        XCTAssertEqual(item.id, "$sent:matrix.org")
        XCTAssertEqual(item.serverEventID, "$sent:matrix.org")
        XCTAssertEqual(item.deliveryStatus, .sent)
        XCTAssertEqual(item.transactionID, "synara-attachment-a")
        guard case let .mediaPlaceholder(resource) = item.kind else {
            return XCTFail("Expected a media row")
        }
        XCTAssertNotEqual(try XCTUnwrap(resource.authenticatedURL).scheme, "mxc")
    }

    func testLocalUploadPreviewsAreBoundedOldestFirst() throws {
        let previews = LocalUploadPreviewStore(maxEntries: 2, maxBytes: 10)
        previews.store(Data("aaaa".utf8), key: "a")
        previews.store(Data("bbbb".utf8), key: "b")
        previews.store(Data("cccc".utf8), key: "c")
        previews.store(Data(repeating: 0, count: 11), key: "too-large")

        XCTAssertNil(previews.data(for: LocalUploadPreviewStore.url(for: "a")))
        XCTAssertEqual(previews.data(for: LocalUploadPreviewStore.url(for: "b")), Data("bbbb".utf8))
        XCTAssertEqual(previews.data(for: LocalUploadPreviewStore.url(for: "c")), Data("cccc".utf8))
        XCTAssertNil(previews.data(for: LocalUploadPreviewStore.url(for: "too-large")))
        XCTAssertNil(previews.data(for: try XCTUnwrap(URL(string: "mxc://matrix.org/b"))))
    }

    func testLocalUploadPreviewsClearOnRemoveAllAndMemoryWarning() throws {
        let previews = LocalUploadPreviewStore()
        let first = try XCTUnwrap(LocalUploadPreviewStore.url(for: "a"))
        previews.store(Data("aaaa".utf8), key: "a")
        previews.removeAll()
        XCTAssertNil(previews.data(for: first))

        // Byte accounting restarts at zero: a full-budget preview fits again.
        let bounded = LocalUploadPreviewStore(maxEntries: 2, maxBytes: 8)
        bounded.store(Data("aaaa".utf8), key: "a")
        bounded.removeAll()
        bounded.store(Data("bbbbbbbb".utf8), key: "b")
        XCTAssertEqual(bounded.data(for: LocalUploadPreviewStore.url(for: "b")), Data("bbbbbbbb".utf8))

        bounded.store(Data("cc".utf8), key: "c")
        NotificationCenter.default.post(name: UIApplication.didReceiveMemoryWarningNotification, object: nil)
        XCTAssertNil(bounded.data(for: LocalUploadPreviewStore.url(for: "c")))
    }

    private func imageUploadRequest(transactionID: String?) -> MediaUploadRequest {
        MediaUploadRequest(
            roomID: "!room:matrix.org",
            source: .photoLibrary,
            displayName: "photo.jpg",
            data: Data("jpeg bytes".utf8),
            mimeType: "image/jpeg",
            transactionID: transactionID
        )
    }

    private func makeSession() throws -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "token"
        )
    }
}
