import Foundation
import UniformTypeIdentifiers
#if canImport(UIKit)
import UIKit
#endif

struct MediaResource: Identifiable, Equatable {
    let id: String
    let filename: String
    let caption: String?
    let formattedCaption: String?
    let authenticatedURL: URL?
    let requiresAuthentication: Bool
    let isEncrypted: Bool
    let mimeType: String?
    let byteSize: UInt64?

    init(
        id: String,
        filename: String,
        caption: String? = nil,
        formattedCaption: String? = nil,
        authenticatedURL: URL?,
        requiresAuthentication: Bool,
        isEncrypted: Bool = false,
        mimeType: String? = nil,
        byteSize: UInt64? = nil
    ) {
        self.id = id
        self.filename = filename
        self.caption = caption
        self.formattedCaption = formattedCaption
        self.authenticatedURL = authenticatedURL
        self.requiresAuthentication = requiresAuthentication
        self.isEncrypted = isEncrypted
        self.mimeType = mimeType
        self.byteSize = byteSize
    }

    var safeDescription: String {
        let safeName = URL(fileURLWithPath: filename).lastPathComponent
        return safeName.isEmpty ? "Attachment" : safeName
    }

    var resolvedMimeType: String? {
        if let mimeType {
            return mimeType
        }

        let fileExtension = URL(fileURLWithPath: safeDescription).pathExtension
        guard fileExtension.isEmpty == false,
              let type = UTType(filenameExtension: fileExtension),
              let resolved = type.preferredMIMEType else {
            return nil
        }

        return resolved
    }

    var isImageMedia: Bool {
        resolvedMimeType?.hasPrefix("image/") == true
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
    func loadMediaData(for resource: MediaResource) async -> Data?
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
    let caption: String?
    let formattedCaption: String?
    let replyToEventID: String?
    let threadRootEventID: String?
    let transactionID: String?
    let mentionUserIDs: [String]?
    let mentionRoom: Bool?

    init(
        roomID: String,
        source: MediaUploadSource,
        displayName: String,
        data: Data = Data("Synara attachment".utf8),
        mimeType: String = "application/octet-stream",
        caption: String? = nil,
        formattedCaption: String? = nil,
        replyToEventID: String? = nil,
        threadRootEventID: String? = nil,
        transactionID: String? = nil,
        mentionUserIDs: [String]? = nil,
        mentionRoom: Bool? = nil
    ) {
        self.roomID = roomID
        self.source = source
        self.displayName = displayName
        self.data = data
        self.mimeType = mimeType
        self.caption = caption
        self.formattedCaption = formattedCaption
        self.replyToEventID = replyToEventID
        self.threadRootEventID = threadRootEventID
        self.transactionID = transactionID
        self.mentionUserIDs = mentionUserIDs
        self.mentionRoom = mentionRoom
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

enum MediaFormatting {
    static func formattedFileSize(_ byteSize: UInt64?) -> String? {
        guard let byteSize else {
            return nil
        }

        return ByteCountFormatter.string(
            fromByteCount: Int64(byteSize),
            countStyle: .file
        )
    }
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

    func loadMediaData(for resource: MediaResource) async -> Data? {
        nil
    }
}

struct MockMediaUploadService: MediaUploading {
    func upload(_ request: MediaUploadRequest) async -> MediaUploadState {
        let safeName = URL(fileURLWithPath: request.displayName).lastPathComponent
        let resource = MediaResource(
            id: "$upload-\(UUID().uuidString)",
            filename: safeName.isEmpty ? "Attachment" : safeName,
            caption: request.caption,
            formattedCaption: request.formattedCaption,
            authenticatedURL: URL(string: "mxc://local/upload"),
            requiresAuthentication: true,
            mimeType: request.mimeType,
            byteSize: UInt64(request.data.count)
        )
        let item = TimelineItem(
            id: resource.id,
            eventID: resource.id,
            senderID: "@local:matrix.org",
            timestamp: Date(),
            kind: .mediaPlaceholder(resource),
            replyToEventID: request.replyToEventID,
            threadRootEventID: request.threadRootEventID,
            isEdited: false,
            reactions: [:]
        )
        return .uploaded(item)
    }
}
