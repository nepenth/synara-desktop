import Foundation

struct ComposerAttachmentDraft: Identifiable, Equatable {
    let id: UUID
    let displayName: String
    let mimeType: String
    let data: Data
    let source: MediaUploadSource

    init(
        id: UUID = UUID(),
        displayName: String,
        mimeType: String,
        data: Data,
        source: MediaUploadSource
    ) {
        self.id = id
        self.displayName = displayName
        self.mimeType = mimeType
        self.data = data
        self.source = source
    }

    var isImage: Bool {
        mimeType.hasPrefix("image/")
    }

    var isVideo: Bool {
        mimeType.hasPrefix("video/")
    }

    var previewSystemImage: String {
        if isImage {
            return "photo"
        }
        if isVideo {
            return "film"
        }
        return "doc"
    }
}

enum ComposerAttachmentDraftRejection: Error, Equatable {
    case empty
    case tooLarge
    case unsupportedType
    case limitReached
    case couldNotLoad
}

struct ComposerAttachmentDraftAddOutcome: Equatable {
    var drafts: [ComposerAttachmentDraft]
    var addedCount: Int
    var rejection: ComposerAttachmentDraftRejection?
}

enum ComposerAttachmentDraftList {
    static let maxCount = 10
    /// Matches synara-core upload/attachment enqueue (`p6.4-file-too-large` / `p7.4-file-too-large`).
    static let maxBytesPerItem = 100 * 1024 * 1024

    static func canSend(text: String, drafts: [ComposerAttachmentDraft]) -> Bool {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty == false || drafts.isEmpty == false
    }

    static func canBeginSend(
        isSending: Bool,
        text: String,
        drafts: [ComposerAttachmentDraft]
    ) -> Bool {
        isSending == false && canSend(text: text, drafts: drafts)
    }

    static func remove(
        id: UUID,
        from drafts: [ComposerAttachmentDraft]
    ) -> [ComposerAttachmentDraft] {
        drafts.filter { $0.id != id }
    }

    static func appending(
        _ incoming: [ComposerAttachmentDraft],
        to drafts: [ComposerAttachmentDraft]
    ) -> ComposerAttachmentDraftAddOutcome {
        var next = drafts
        var addedCount = 0
        for draft in incoming {
            if let rejection = validate(draft, against: next) {
                return ComposerAttachmentDraftAddOutcome(
                    drafts: next,
                    addedCount: addedCount,
                    rejection: rejection
                )
            }
            next.append(draft)
            addedCount += 1
        }
        return ComposerAttachmentDraftAddOutcome(
            drafts: next,
            addedCount: addedCount,
            rejection: nil
        )
    }

    static func validate(
        _ draft: ComposerAttachmentDraft,
        against drafts: [ComposerAttachmentDraft]
    ) -> ComposerAttachmentDraftRejection? {
        if drafts.count >= maxCount {
            return .limitReached
        }
        if draft.data.isEmpty {
            return .empty
        }
        if draft.data.count > maxBytesPerItem {
            return .tooLarge
        }
        if isAllowedMimeType(draft.mimeType) == false {
            return .unsupportedType
        }
        return nil
    }

    static func isAllowedMimeType(_ mimeType: String) -> Bool {
        mimeType.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
    }

    static func draft(fromFileURL url: URL) -> Result<ComposerAttachmentDraft, ComposerAttachmentDraftRejection> {
        let didAccess = url.startAccessingSecurityScopedResource()
        defer {
            if didAccess {
                url.stopAccessingSecurityScopedResource()
            }
        }

        do {
            if let fileSize = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize,
               fileSize > maxBytesPerItem {
                return .failure(.tooLarge)
            }

            let data = try Data(contentsOf: url)
            let draft = ComposerAttachmentDraft(
                displayName: MediaAttachmentSupport.displayName(for: url),
                mimeType: MediaAttachmentSupport.mimeType(for: url),
                data: data,
                source: .file
            )
            if let rejection = validate(draft, against: []) {
                return .failure(rejection)
            }
            return .success(draft)
        } catch {
            return .failure(.couldNotLoad)
        }
    }

    static func userMessage(for rejection: ComposerAttachmentDraftRejection) -> String {
        switch rejection {
        case .empty:
            return "Attachment is empty."
        case .tooLarge:
            return "Attachment is too large."
        case .unsupportedType:
            return "This file type cannot be attached."
        case .limitReached:
            return "You can attach up to \(maxCount) attachments."
        case .couldNotLoad:
            return "Attachment could not be loaded. Try again."
        }
    }
}

enum ComposerAttachmentSend {
    static func uploadAll(
        _ drafts: [ComposerAttachmentDraft],
        roomID: String,
        uploader: MediaUploading,
        onState: @escaping @MainActor (MediaUploadState) -> Void,
        onUploaded: @escaping @MainActor (ComposerAttachmentDraft, TimelineItem) -> Void
    ) async -> Bool {
        for draft in drafts {
            await onState(.uploading(progress: 0.25))
            let result = await uploader.upload(
                MediaUploadRequest(
                    roomID: roomID,
                    source: draft.source,
                    displayName: draft.displayName,
                    data: draft.data,
                    mimeType: draft.mimeType
                )
            )
            switch result {
            case let .uploaded(item):
                await onUploaded(draft, item)
                await onState(result)
            case let .failed(message):
                await onState(.failed(message))
                return false
            case .idle, .uploading:
                await onState(.failed("Media could not be uploaded."))
                return false
            }
        }
        return true
    }
}
