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

    var transactionID: String {
        "synara-attachment-\(id.uuidString.lowercased())"
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
    static let maxBytesPerItem = 32 * 1024 * 1024

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

enum ComposerAttachmentSendStep: Equatable {
    case attachment(id: UUID, caption: String?)
    case text(body: String)
}

enum ComposerAttachmentSendPlan {
    static func make(
        drafts: [ComposerAttachmentDraft],
        body rawBody: String
    ) -> [ComposerAttachmentSendStep] {
        let body = rawBody.trimmingCharacters(in: .whitespacesAndNewlines)

        if drafts.count == 1, let draft = drafts.first {
            return [.attachment(id: draft.id, caption: body.isEmpty ? nil : body)]
        }

        var steps = drafts.map {
            ComposerAttachmentSendStep.attachment(id: $0.id, caption: nil)
        }
        if body.isEmpty == false {
            steps.append(.text(body: body))
        }
        return steps
    }

    static func trailingText(in steps: [ComposerAttachmentSendStep]) -> String? {
        steps.compactMap { step in
            guard case let .text(body) = step else { return nil }
            let trimmed = body.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? nil : trimmed
        }.last
    }

    static func reusableOrNew(
        existing: [ComposerAttachmentSendStep]?,
        drafts: [ComposerAttachmentDraft],
        body rawBody: String
    ) -> [ComposerAttachmentSendStep] {
        let body = rawBody.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let existing,
              attachmentIDs(in: existing) == drafts.map(\.id) else {
            return make(drafts: drafts, body: body)
        }
        if composerBody(in: existing) == body {
            return existing
        }
        if existing.contains(where: { step in
            if case .text = step { return true }
            return false
        }) {
            return existing.map { step in
                if case .text = step {
                    return .text(body: body)
                }
                return step
            }
        }
        if existing.count == 1,
           case let .attachment(id, _) = existing[0] {
            return [.attachment(id: id, caption: body.isEmpty ? nil : body)]
        }
        if !body.isEmpty {
            return existing + [.text(body: body)]
        }
        return existing
    }

    static func removingAttachment(
        id: UUID,
        from steps: [ComposerAttachmentSendStep]
    ) -> [ComposerAttachmentSendStep] {
        steps.filter { step in
            guard case let .attachment(stepID, _) = step else { return true }
            return stepID != id
        }
    }

    private static func attachmentIDs(in steps: [ComposerAttachmentSendStep]) -> [UUID] {
        steps.compactMap { step in
            guard case let .attachment(id, _) = step else { return nil }
            return id
        }
    }

    private static func composerBody(in steps: [ComposerAttachmentSendStep]) -> String {
        for step in steps.reversed() {
            switch step {
            case let .text(body):
                return body
            case let .attachment(_, caption) where caption != nil:
                return caption ?? ""
            case .attachment:
                continue
            }
        }
        return ""
    }
}

enum ComposerAttachmentSend {
    static func uploadAll(
        _ drafts: [ComposerAttachmentDraft],
        steps: [ComposerAttachmentSendStep],
        roomID: String,
        replyToEventID: String?,
        threadRootEventID: String?,
        uploader: MediaUploading,
        onState: @escaping @MainActor (MediaUploadState) -> Void,
        onUploaded: @escaping @MainActor (ComposerAttachmentDraft, TimelineItem) -> Void
    ) async -> Bool {
        for step in steps {
            guard case let .attachment(id, caption) = step,
                  let draft = drafts.first(where: { $0.id == id }) else {
                continue
            }
            await onState(.uploading(progress: 0.25))
            let result = await uploader.upload(
                MediaUploadRequest(
                    roomID: roomID,
                    source: draft.source,
                    displayName: draft.displayName,
                    data: draft.data,
                    mimeType: draft.mimeType,
                    caption: caption,
                    formattedCaption: caption.flatMap(ComposerMatrixFormatting.formattedBody(for:)),
                    replyToEventID: replyToEventID,
                    threadRootEventID: threadRootEventID,
                    transactionID: draft.transactionID
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
