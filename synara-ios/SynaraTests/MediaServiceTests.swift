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
            mimeType: "image/png"
        )

        let state = await MockMediaUploadService().upload(request)

        guard case .uploaded(let item) = state, case .mediaPlaceholder(let resource) = item.kind else {
            XCTFail("Expected uploaded media item")
            return
        }

        XCTAssertEqual(resource.safeDescription, "photo.png")
        XCTAssertFalse(resource.safeDescription.contains("/private/tmp"))
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
