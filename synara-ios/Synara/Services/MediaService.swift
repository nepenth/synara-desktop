import Foundation

struct MediaResource: Identifiable, Equatable {
    let id: String
    let filename: String
    let authenticatedURL: URL?
    let requiresAuthentication: Bool
    let isEncrypted: Bool

    init(
        id: String,
        filename: String,
        authenticatedURL: URL?,
        requiresAuthentication: Bool,
        isEncrypted: Bool = false
    ) {
        self.id = id
        self.filename = filename
        self.authenticatedURL = authenticatedURL
        self.requiresAuthentication = requiresAuthentication
        self.isEncrypted = isEncrypted
    }

    var safeDescription: String {
        let safeName = URL(fileURLWithPath: filename).lastPathComponent
        return safeName.isEmpty ? "Attachment" : safeName
    }
}

enum MediaLoadState: Equatable {
    case idle
    case loading
    case thumbnail(MediaResource)
    case failed(String)
}

protocol MediaLoading {
    func loadThumbnail(for resource: MediaResource) async -> MediaLoadState
}

enum MediaUploadSource: Equatable {
    case photoLibrary
    case camera
    case file
}

struct MediaUploadRequest: Equatable {
    let roomID: String
    let source: MediaUploadSource
    let displayName: String
    let data: Data
    let mimeType: String

    init(
        roomID: String,
        source: MediaUploadSource,
        displayName: String,
        data: Data = Data("Synara attachment".utf8),
        mimeType: String = "application/octet-stream"
    ) {
        self.roomID = roomID
        self.source = source
        self.displayName = displayName
        self.data = data
        self.mimeType = mimeType
    }
}

enum MediaUploadState: Equatable {
    case idle
    case uploading(progress: Double)
    case uploaded(TimelineItem)
    case failed(String)
}

protocol MediaUploading {
    func upload(_ request: MediaUploadRequest) async -> MediaUploadState
}

struct MockMediaLoader: MediaLoading {
    func loadThumbnail(for resource: MediaResource) async -> MediaLoadState {
        guard resource.isEncrypted == false else {
            return .failed("Encrypted media requires recovered keys before it can be opened.")
        }

        if resource.authenticatedURL == nil {
            return .failed("Media is unavailable.")
        }

        return .thumbnail(resource)
    }
}

struct MockMediaUploadService: MediaUploading {
    func upload(_ request: MediaUploadRequest) async -> MediaUploadState {
        let safeName = URL(fileURLWithPath: request.displayName).lastPathComponent
        let resource = MediaResource(
            id: "$upload-\(UUID().uuidString)",
            filename: safeName.isEmpty ? "Attachment" : safeName,
            authenticatedURL: URL(string: "mxc://local/upload"),
            requiresAuthentication: true
        )
        let item = TimelineItem(
            id: resource.id,
            eventID: resource.id,
            senderID: "@local:matrix.org",
            timestamp: Date(),
            kind: .mediaPlaceholder(resource),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        return .uploaded(item)
    }
}

final class MatrixMediaLoader: MediaLoading {
    private let sessionStore: AppSessionStore
    private let httpClient: AuthHTTPClient

    init(
        sessionStore: AppSessionStore,
        httpClient: AuthHTTPClient = URLSession.shared
    ) {
        self.sessionStore = sessionStore
        self.httpClient = httpClient
    }

    func loadThumbnail(for resource: MediaResource) async -> MediaLoadState {
        guard resource.isEncrypted == false else {
            return .failed("Encrypted media requires recovered keys before it can be opened.")
        }

        guard case .signedIn(let session) = sessionStore.currentState,
              let url = resource.authenticatedURL,
              let downloadURL = mediaURL(homeserverURL: session.homeserverURL, mxcURL: url, kind: .thumbnail) else {
            return .failed("Media is unavailable.")
        }

        var request = URLRequest(url: downloadURL)
        request.httpMethod = "GET"
        request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")

        do {
            let (_, response) = try await httpClient.data(for: request)
            guard let httpResponse = response as? HTTPURLResponse,
                  (200...299).contains(httpResponse.statusCode) else {
                return .failed("Media could not be loaded.")
            }

            return .thumbnail(resource)
        } catch {
            return .failed("Media could not be loaded.")
        }
    }

    private func mediaURL(homeserverURL: URL, mxcURL: URL, kind: MatrixMediaURLKind) -> URL? {
        guard mxcURL.scheme == "mxc",
              let serverName = mxcURL.host else {
            return nil
        }

        let mediaID = mxcURL.path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        guard mediaID.isEmpty == false else {
            return nil
        }

        var url = homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v1")
        url.appendPathComponent("media")
        url.appendPathComponent(kind.pathComponent)
        url.appendPathComponent(serverName)
        url.appendPathComponent(mediaID)

        if kind == .thumbnail {
            var components = URLComponents(url: url, resolvingAgainstBaseURL: false)
            components?.queryItems = [
                URLQueryItem(name: "width", value: "640"),
                URLQueryItem(name: "height", value: "480"),
                URLQueryItem(name: "method", value: "scale")
            ]
            return components?.url
        }

        return url
    }
}

final class MatrixMediaUploadService: MediaUploading {
    private let sessionStore: AppSessionStore
    private let httpClient: AuthHTTPClient
    private let jsonEncoder: JSONEncoder
    private let jsonDecoder: JSONDecoder

    init(
        sessionStore: AppSessionStore,
        httpClient: AuthHTTPClient = URLSession.shared,
        jsonEncoder: JSONEncoder = JSONEncoder(),
        jsonDecoder: JSONDecoder = JSONDecoder()
    ) {
        self.sessionStore = sessionStore
        self.httpClient = httpClient
        self.jsonEncoder = jsonEncoder
        self.jsonDecoder = jsonDecoder
    }

    func upload(_ request: MediaUploadRequest) async -> MediaUploadState {
        let safeName = URL(fileURLWithPath: request.displayName).lastPathComponent
        let filename = safeName.isEmpty ? "Attachment" : safeName

        guard case .signedIn(let session) = sessionStore.currentState else {
            return .failed("Sign in before uploading media.")
        }

        guard request.data.isEmpty == false else {
            return .failed("Attachment is empty.")
        }

        do {
            let contentURI = try await uploadContent(request: request, filename: filename, session: session)
            let eventID = try await sendMediaEvent(
                roomID: request.roomID,
                filename: filename,
                contentURI: contentURI,
                mimeType: request.mimeType,
                session: session
            )
            let resource = MediaResource(
                id: eventID,
                filename: filename,
                authenticatedURL: URL(string: contentURI),
                requiresAuthentication: true
            )
            let item = TimelineItem(
                id: eventID,
                eventID: eventID,
                senderID: session.userID,
                timestamp: Date(),
                kind: .mediaPlaceholder(resource),
                replyToEventID: nil,
                isEdited: false,
                reactions: [:]
            )
            return .uploaded(item)
        } catch {
            return .failed("Attachment could not be uploaded. Try again.")
        }
    }

    private func uploadContent(
        request: MediaUploadRequest,
        filename: String,
        session: AuthenticatedSession
    ) async throws -> String {
        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("media")
        url.appendPathComponent("v3")
        url.appendPathComponent("upload")

        var components = URLComponents(url: url, resolvingAgainstBaseURL: false)
        components?.queryItems = [URLQueryItem(name: "filename", value: filename)]

        var urlRequest = URLRequest(url: components?.url ?? url)
        urlRequest.httpMethod = "POST"
        urlRequest.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
        urlRequest.setValue(request.mimeType, forHTTPHeaderField: "Content-Type")
        urlRequest.httpBody = request.data

        let (data, response) = try await httpClient.data(for: urlRequest)
        guard let httpResponse = response as? HTTPURLResponse,
              (200...299).contains(httpResponse.statusCode) else {
            throw MediaServiceError.failed
        }

        return try jsonDecoder.decode(MatrixMediaUploadResponse.self, from: data).contentURI
    }

    private func sendMediaEvent(
        roomID: String,
        filename: String,
        contentURI: String,
        mimeType: String,
        session: AuthenticatedSession
    ) async throws -> String {
        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("rooms")
        url.appendPathComponent(roomID)
        url.appendPathComponent("send")
        url.appendPathComponent("m.room.message")
        url.appendPathComponent(UUID().uuidString)

        var request = URLRequest(url: url)
        request.httpMethod = "PUT"
        request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try jsonEncoder.encode(
            MatrixMediaMessageRequest(filename: filename, contentURI: contentURI, mimeType: mimeType)
        )

        let (data, response) = try await httpClient.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse,
              (200...299).contains(httpResponse.statusCode) else {
            throw MediaServiceError.failed
        }

        return try jsonDecoder.decode(MatrixMediaSendResponse.self, from: data).eventID
    }
}

private enum MatrixMediaURLKind {
    case download
    case thumbnail

    var pathComponent: String {
        switch self {
        case .download:
            return "download"
        case .thumbnail:
            return "thumbnail"
        }
    }
}

private enum MediaServiceError: Error {
    case failed
}

private struct MatrixMediaUploadResponse: Decodable {
    let contentURI: String

    enum CodingKeys: String, CodingKey {
        case contentURI = "content_uri"
    }
}

private struct MatrixMediaSendResponse: Decodable {
    let eventID: String

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
    }
}

private struct MatrixMediaMessageRequest: Encodable {
    let msgtype: String
    let body: String
    let url: String
    let info: MatrixMediaMessageInfo

    init(filename: String, contentURI: String, mimeType: String) {
        msgtype = mimeType.hasPrefix("image/") ? "m.image" : "m.file"
        body = filename
        url = contentURI
        info = MatrixMediaMessageInfo(mimeType: mimeType)
    }
}

private struct MatrixMediaMessageInfo: Encodable {
    let mimeType: String

    enum CodingKeys: String, CodingKey {
        case mimeType = "mimetype"
    }
}
