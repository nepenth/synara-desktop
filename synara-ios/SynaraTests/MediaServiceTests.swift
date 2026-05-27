import XCTest
@testable import Synara

final class MediaServiceTests: XCTestCase {
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

    func testUploadSanitizesLocalFilePath() async {
        let request = MediaUploadRequest(
            roomID: "!room:matrix.org",
            source: .file,
            displayName: "/private/tmp/photo.png"
        )

        let state = await MockMediaUploadService().upload(request)

        guard case .uploaded(let item) = state, case .mediaPlaceholder(let resource) = item.kind else {
            XCTFail("Expected uploaded media item")
            return
        }

        XCTAssertEqual(resource.safeDescription, "photo.png")
        XCTAssertFalse(resource.safeDescription.contains("/private/tmp"))
    }
}
