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
        return safeName.isEmpty ? String(localized: "Attachment") : safeName
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

/// Bytes of attachments the user just sent, shown by their own upload row
/// until the timeline delivers the event. Rows point here with a
/// `synara-local-upload` URL, which no loader ever sends to the homeserver
/// (an `mxc://` placeholder did, and failed with 403). Bounded; the oldest
/// previews go first, and a row without one shows the media placeholder.
final class LocalUploadPreviewStore: @unchecked Sendable {
    static let shared = LocalUploadPreviewStore()
    static let urlScheme = "synara-local-upload"

    private let lock = NSLock()
    private let maxEntries: Int
    private let maxBytes: Int
    private var entries: [String: Data] = [:]
    private var order: [String] = []
    private var totalBytes = 0
    private var memoryWarningObserver: NSObjectProtocol?

    // Kept small: these are full-size originals held in memory next to Core's
    // own media cache, and only rows still waiting for their event use them.
    init(maxEntries: Int = 12, maxBytes: Int = 24 * 1024 * 1024) {
        self.maxEntries = maxEntries
        self.maxBytes = maxBytes
        #if canImport(UIKit)
        memoryWarningObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.didReceiveMemoryWarningNotification,
            object: nil,
            queue: nil
        ) { [weak self] _ in
            self?.removeAll()
        }
        #endif
    }

    deinit {
        if let memoryWarningObserver {
            NotificationCenter.default.removeObserver(memoryWarningObserver)
        }
    }

    /// Drops every preview: on sign-out and under memory pressure. Rows that
    /// still point here fall back to the media placeholder.
    func removeAll() {
        lock.lock()
        defer { lock.unlock() }
        entries.removeAll()
        order.removeAll()
        totalBytes = 0
    }

    static func url(for key: String) -> URL? {
        guard key.isEmpty == false else {
            return nil
        }
        var components = URLComponents()
        components.scheme = urlScheme
        components.host = "upload"
        components.path = "/" + key
        return components.url
    }

    static func isLocalPreview(_ url: URL?) -> Bool {
        url?.scheme == urlScheme
    }

    func store(_ data: Data, key: String) {
        guard key.isEmpty == false, data.isEmpty == false, data.count <= maxBytes else {
            return
        }
        lock.lock()
        defer { lock.unlock() }
        if let previous = entries.removeValue(forKey: key) {
            totalBytes -= previous.count
            order.removeAll { $0 == key }
        }
        while order.isEmpty == false, order.count >= maxEntries || totalBytes + data.count > maxBytes {
            let oldest = order.removeFirst()
            totalBytes -= entries.removeValue(forKey: oldest)?.count ?? 0
        }
        entries[key] = data
        order.append(key)
        totalBytes += data.count
    }

    func data(for url: URL?) -> Data? {
        guard let url, Self.isLocalPreview(url) else {
            return nil
        }
        let key = String(url.path.dropFirst())
        lock.lock()
        defer { lock.unlock() }
        return entries[key]
    }
}

/// Builds the timeline row for a finished `MediaUploading` call.
///
/// Core reports `queued` with an empty event id when the upload is still in
/// the SDK send queue (slow link, 5xx, 429). That row is pending, named by
/// its transaction id, and is replaced when the timeline shows the SDK local
/// echo or the server event for the same transaction. An empty id must never
/// become a row identity: two such rows crashed the timeline.
enum MediaUploadAcknowledgement {
    static func item(
        eventID: String,
        transactionID: String,
        isQueued: Bool,
        request: MediaUploadRequest,
        senderID: String,
        timestamp: Date = Date(),
        previews: LocalUploadPreviewStore = .shared
    ) -> TimelineItem {
        let transactionID = [transactionID, request.transactionID ?? ""]
            .first { $0.isEmpty == false }
            ?? "synara-attachment-\(UUID().uuidString.lowercased())"
        let isPending = isQueued || eventID.isEmpty
        let rowID = isPending ? "$pending-\(transactionID)" : eventID
        if request.mimeType.hasPrefix("image/") {
            previews.store(request.data, key: transactionID)
        }
        let safeName = URL(fileURLWithPath: request.displayName).lastPathComponent
        let resource = MediaResource(
            id: rowID,
            filename: safeName.isEmpty ? "Attachment" : safeName,
            caption: request.caption,
            formattedCaption: request.formattedCaption,
            authenticatedURL: LocalUploadPreviewStore.url(for: transactionID),
            requiresAuthentication: false,
            mimeType: request.mimeType,
            byteSize: UInt64(request.data.count)
        )
        var item = TimelineItem(
            id: rowID,
            eventID: rowID,
            serverEventID: isPending ? nil : eventID,
            senderID: senderID,
            timestamp: timestamp,
            kind: .mediaPlaceholder(resource),
            replyToEventID: request.replyToEventID,
            threadRootEventID: request.threadRootEventID,
            isEdited: false,
            reactions: [:],
            deliveryStatus: isPending ? .sending : .sent
        )
        item.transactionID = transactionID
        return item
    }
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
        LocalUploadPreviewStore.shared.data(for: resource.authenticatedURL)
    }

    func loadMediaData(for resource: MediaResource) async -> Data? {
        LocalUploadPreviewStore.shared.data(for: resource.authenticatedURL)
    }
}

struct MockMediaUploadService: MediaUploading {
    var shouldFail = false

    func upload(_ request: MediaUploadRequest) async -> MediaUploadState {
        if shouldFail {
            return .failed("Media could not be uploaded.")
        }
        let safeName = URL(fileURLWithPath: request.displayName).lastPathComponent
        let uploadID = "$upload-\(UUID().uuidString)"
        if request.mimeType.hasPrefix("image/") {
            LocalUploadPreviewStore.shared.store(request.data, key: uploadID)
        }
        let resource = MediaResource(
            id: uploadID,
            filename: safeName.isEmpty ? "Attachment" : safeName,
            caption: request.caption,
            formattedCaption: request.formattedCaption,
            authenticatedURL: LocalUploadPreviewStore.url(for: uploadID),
            requiresAuthentication: false,
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
