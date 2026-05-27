import Foundation

struct MediaResource: Identifiable, Equatable {
    let id: String
    let filename: String
    let authenticatedURL: URL?
    let requiresAuthentication: Bool

    var safeDescription: String {
        filename
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
        resource.authenticatedURL == nil ? .failed("Media is unavailable.") : .thumbnail(resource)
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
