import Foundation
import UniformTypeIdentifiers
#if canImport(UIKit)
import UIKit
#endif

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
    func loadThumbnailData(for resource: MediaResource, width: UInt64, height: UInt64) async -> Data?
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

enum MediaAttachmentSupport {
    static func mimeType(for url: URL) -> String {
        if let type = try? url.resourceValues(forKeys: [.contentTypeKey]).contentType,
           let mimeType = type.preferredMIMEType {
            return mimeType
        }

        let fileExtension = url.pathExtension
        if fileExtension.isEmpty == false,
           let type = UTType(filenameExtension: fileExtension),
           let mimeType = type.preferredMIMEType {
            return mimeType
        }

        return "application/octet-stream"
    }

    static func displayName(for url: URL) -> String {
        let name = url.lastPathComponent
        return name.isEmpty ? "Attachment" : name
    }

    #if canImport(UIKit)
    static func jpegData(from image: UIImage, compressionQuality: CGFloat = 0.85) -> Data? {
        image.jpegData(compressionQuality: compressionQuality)
    }
    #endif
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

    func loadThumbnailData(for resource: MediaResource, width: UInt64, height: UInt64) async -> Data? {
        nil
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
    private let clientStore: MatrixRustSDKClientStore

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
    }

    func loadThumbnail(for resource: MediaResource) async -> MediaLoadState {
        guard resource.isEncrypted == false else {
            return .failed("Encrypted media requires recovered keys before it can be opened.")
        }

        guard case .signedIn(let session) = sessionStore.currentState,
              let url = resource.authenticatedURL else {
            return .failed("Media is unavailable.")
        }

        do {
            _ = try await clientStore.mediaThumbnailData(mxcURL: url, session: session)
            return .thumbnail(resource)
        } catch {
            return .failed("Media could not be loaded.")
        }
    }

    func loadThumbnailData(for resource: MediaResource, width: UInt64, height: UInt64) async -> Data? {
        guard resource.isEncrypted == false,
              case .signedIn(let session) = sessionStore.currentState,
              let url = resource.authenticatedURL,
              url.scheme == "mxc" else {
            return nil
        }

        return try? await clientStore.mediaThumbnailData(
            mxcURL: url,
            width: width,
            height: height,
            session: session
        )
    }
}

final class MatrixMediaUploadService: MediaUploading {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
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
            let contentURI = try await clientStore.uploadMedia(
                data: request.data,
                mimeType: request.mimeType,
                session: session
            )
            try await clientStore.sendMediaMessage(
                roomID: request.roomID,
                filename: filename,
                contentURI: contentURI,
                mimeType: request.mimeType,
                size: UInt64(request.data.count),
                session: session
            )
            try? await clientStore.syncOnce(session: session, fullState: false)
            let eventID = "$local-media-\(UUID().uuidString)"
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
}
