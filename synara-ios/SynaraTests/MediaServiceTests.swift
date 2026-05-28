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

    func testMatrixMediaUploadUploadsContentThenSendsMediaEvent() async throws {
        let client = MockMediaHTTPClient(responses: [
            .success(statusCode: 200, body: #"{"content_uri":"mxc://matrix.org/uploaded"}"#),
            .success(statusCode: 200, body: #"{"event_id":"$media-event"}"#)
        ])
        let service = MatrixMediaUploadService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )

        let state = await service.upload(
            MediaUploadRequest(
                roomID: "!room:matrix.org",
                source: .photoLibrary,
                displayName: "/private/tmp/photo.png",
                data: Data("image".utf8),
                mimeType: "image/png"
            )
        )

        guard case .uploaded(let item) = state, case .mediaPlaceholder(let resource) = item.kind else {
            XCTFail("Expected uploaded media item")
            return
        }

        XCTAssertEqual(item.eventID, "$media-event")
        XCTAssertEqual(resource.filename, "photo.png")
        XCTAssertEqual(resource.authenticatedURL?.absoluteString, "mxc://matrix.org/uploaded")
        XCTAssertEqual(client.requests.count, 2)
        XCTAssertEqual(client.requests[0].httpMethod, "POST")
        XCTAssertEqual(
            client.requests[0].url?.absoluteString,
            "https://matrix.org/_matrix/media/v3/upload?filename=photo.png"
        )
        XCTAssertEqual(client.requests[0].value(forHTTPHeaderField: "Content-Type"), "image/png")
        XCTAssertEqual(client.requests[1].httpMethod, "PUT")

        let payloadData = try XCTUnwrap(client.requests[1].httpBody)
        let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: payloadData) as? [String: Any])
        XCTAssertEqual(payload["msgtype"] as? String, "m.image")
        XCTAssertEqual(payload["body"] as? String, "photo.png")
        XCTAssertEqual(payload["url"] as? String, "mxc://matrix.org/uploaded")
    }

    func testMatrixMediaUploadFailureReturnsRetryableError() async throws {
        let client = MockMediaHTTPClient(responses: [
            .success(statusCode: 500, body: #"{}"#)
        ])
        let service = MatrixMediaUploadService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )

        let state = await service.upload(
            MediaUploadRequest(
                roomID: "!room:matrix.org",
                source: .photoLibrary,
                displayName: "photo.png",
                data: Data("image".utf8),
                mimeType: "image/png"
            )
        )

        XCTAssertEqual(state, .failed("Attachment could not be uploaded. Try again."))
    }

    func testMatrixMediaLoaderRequestsAuthenticatedThumbnail() async throws {
        let client = MockMediaHTTPClient(responses: [
            .success(statusCode: 200, body: "thumbnail")
        ])
        let service = MatrixMediaLoader(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )
        let resource = MediaResource(
            id: "$media:matrix.org",
            filename: "private.png",
            authenticatedURL: try XCTUnwrap(URL(string: "mxc://remote.example/media-id")),
            requiresAuthentication: true
        )

        let state = await service.loadThumbnail(for: resource)

        XCTAssertEqual(state, .thumbnail(resource))
        XCTAssertEqual(
            client.requests.first?.url?.absoluteString,
            "https://matrix.org/_matrix/client/v1/media/thumbnail/remote.example/media-id?width=640&height=480&method=scale"
        )
        XCTAssertEqual(client.requests.first?.value(forHTTPHeaderField: "Authorization"), "Bearer token")
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

private final class MockMediaHTTPClient: AuthHTTPClient {
    enum Response {
        case success(statusCode: Int, body: String)
        case failure(Error)
    }

    private var responses: [Response]
    private(set) var requests: [URLRequest] = []

    init(responses: [Response] = []) {
        self.responses = responses
    }

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        requests.append(request)

        guard responses.isEmpty == false else {
            throw LoginError.networkFailure
        }

        let response = responses.removeFirst()
        switch response {
        case .success(let statusCode, let body):
            let url = try XCTUnwrap(request.url)
            let httpResponse = try XCTUnwrap(
                HTTPURLResponse(
                    url: url,
                    statusCode: statusCode,
                    httpVersion: nil,
                    headerFields: nil
                )
            )
            return (Data(body.utf8), httpResponse)
        case .failure(let error):
            throw error
        }
    }
}
