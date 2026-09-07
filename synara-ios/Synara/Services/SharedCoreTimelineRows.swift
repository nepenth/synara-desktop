import Foundation
import SynaraCore

/// P4-S16/S29 map of privacy-safe SharedCore timeline rows to product items.
///
/// Skips virtual separators/markers. Poll / membership / state / call /
/// other rows use the existing body text. Does not load media bytes or
/// invent mxc URLs. This is not iOS-on-engine and not P4 acceptance.
enum SharedCoreTimelineRows {
    static func items(
        from rows: [TimelineViewRowDto],
        visibleTailEventID: String? = nil,
        receiptTailEventID: String? = nil
    ) -> [TimelineItem] {
        var items = rows.compactMap(item(from:))
        if let index = items.lastIndex(where: { $0.serverEventID != nil }),
           items[index].serverEventID == visibleTailEventID {
            items[index].readReceiptEventID = receiptTailEventID
        }
        return items
    }

    /// Returns an authoritative product outcome only when the native owner has
    /// either projected a displayable row or proved backward pagination is
    /// exhausted. `nil` means the caller must continue through the owner's
    /// pagination route; it must never be presented as an empty room.
    static func authoritativeOutcome(
        from rows: [TimelineViewRowDto],
        paginationBackward: String,
        visibleTailEventID: String? = nil,
        receiptTailEventID: String? = nil
    ) -> TimelineLoadOutcome? {
        let items = items(from: rows, visibleTailEventID: visibleTailEventID,
                          receiptTailEventID: receiptTailEventID)
        if items.isEmpty == false {
            return .loaded(items)
        }
        return paginationBackward == "exhausted" ? .empty : nil
    }

    static func item(from row: TimelineViewRowDto) -> TimelineItem? {
        guard let kind = displayKind(
            rowKind: row.kind,
            body: row.body,
            formattedBody: row.formattedBody,
            agentCardJSON: row.agentCardJson,
            messageType: row.messageType,
            mediaHandleId: row.mediaHandleId,
            mediaMimeType: row.mediaMimeType,
            mediaFilename: row.mediaFilename,
            mediaCaption: row.mediaCaption
        ) else {
            return nil
        }
        let eventID = row.eventId.isEmpty ? row.itemId : row.eventId
        let timestamp = Date(timeIntervalSince1970: TimeInterval(row.originServerTs) / 1000)
        return TimelineItem(
            id: row.itemId,
            eventID: eventID,
            senderID: row.sender,
            senderProfileDisplayName: row.senderName,
            senderAvatarURL: senderAvatarURL(row.senderAvatarUrl),
            timestamp: timestamp,
            kind: kind,
            replyToEventID: row.replyToEventId,
            threadRootEventID: row.threadRootEventId,
            replyPreview: replyPreview(from: row.replyPreview),
            threadSummary: threadSummary(from: row.threadSummary),
            poll: poll(from: row.poll),
            actionCapabilities: actionCapabilities(from: row.capabilities),
            forwardTransport: forwardTransport(row.forwardTransport),
            isEdited: row.edited,
            isAgentApproval: row.isAgentApproval,
            reactions: reactions(from: row.reactions),
            reactionOwnership: reactionOwnership(from: row.reactions),
            isEncrypted: row.kind == "encrypted"
        )
    }

    /// Timeline avatars are metadata-only Matrix content URIs. Reject every
    /// other scheme at the Swift boundary before it can reach the media owner.
    static func senderAvatarURL(_ rawValue: String?) -> URL? {
        guard let rawValue,
              rawValue.hasPrefix("mxc://"),
              let url = URL(string: rawValue),
              url.scheme == "mxc",
              url.host?.isEmpty == false,
              url.pathComponents.count > 1
        else {
            return nil
        }
        return url
    }

    static func displayKind(
        rowKind: String,
        body: String,
        formattedBody: String?,
        agentCardJSON: String? = nil,
        messageType: String? = nil,
        mediaHandleId: String? = nil,
        mediaMimeType: String? = nil,
        mediaFilename: String? = nil,
        mediaCaption: String? = nil
    ) -> TimelineItem.Kind? {
        if let agentCard = SynaraAgentCardPayloadParser.parse(payloadJSON: agentCardJSON) {
            return .agentCard(agentCard)
        }
        if let media = mediaPlaceholder(
            rowKind: rowKind,
            messageType: messageType,
            mediaHandleId: mediaHandleId,
            mediaMimeType: mediaMimeType,
            mediaFilename: mediaFilename,
            mediaCaption: mediaCaption,
            formattedCaption: formattedBody
        ) {
            return .mediaPlaceholder(media)
        }
        switch rowKind {
        case "date_separator", "read_marker", "unread_marker", "timeline_start", "pagination":
            return nil
        case "redacted":
            return .redacted
        case "encrypted":
            return .encryptedPlaceholder
        case "message":
            if let html = formattedBody, html.isEmpty == false {
                return .formattedText(body: body, html: html)
            }
            return .text(body)
        case "poll", "membership", "state", "call", "other", "sticker":
            let trimmed = body.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty == false {
                return .text(trimmed)
            }
            return .unknown(type: rowKind)
        default:
            return .unknown(type: rowKind)
        }
    }

    static func reactions(from rows: [TimelineViewReactionDto]) -> [String: Int] {
        reactionCounts(rows.map { ($0.key, $0.count) })
    }

    static func reactionOwnership(from rows: [TimelineViewReactionDto]) -> TimelineReactionOwnership {
        guard rows.allSatisfy({ $0.own != nil }) else {
            return .unknown
        }
        return .known(Set(rows.compactMap { $0.own == true ? $0.key : nil }))
    }

    static func replyPreview(from dto: TimelineViewReplyPreviewDto?) -> TimelineReplyPreview? {
        guard let dto else { return nil }
        return TimelineReplyPreview(
            eventID: dto.eventId,
            senderID: dto.senderId,
            senderName: dto.senderName,
            snippet: TimelineReplyPreview.truncatedSnippet(dto.body)
        )
    }

    static func threadSummary(from dto: TimelineViewThreadSummaryDto?) -> TimelineThreadSummary? {
        guard let dto else { return nil }
        return TimelineThreadSummary(
            rootEventID: dto.rootEventId,
            replyCount: Int(dto.replyCount),
            latestEventID: dto.latestEventId
        )
    }

    static func poll(from dto: TimelineViewPollDto?) -> TimelinePollPresentation? {
        guard let dto else { return nil }
        return TimelinePollPresentation(
            question: dto.question,
            isClosed: dto.closed,
            maximumSelections: Int(dto.maxSelections),
            answers: dto.answers.map {
                TimelinePollAnswer(
                    id: $0.id,
                    text: $0.text,
                    voteCount: Int($0.voteCount),
                    isOwn: $0.own
                )
            }
        )
    }

    static func actionCapabilities(
        from dto: TimelineViewRowCapabilitiesDto?
    ) -> TimelineRowActionCapabilities? {
        guard let dto else { return nil }
        return TimelineRowActionCapabilities(
            canReact: dto.react,
            canReply: dto.reply,
            canEdit: dto.edit,
            canRedact: dto.redact,
            canReport: dto.report,
            canPin: dto.pin,
            canForward: dto.forward,
            canVote: dto.vote,
            canDeclineCall: dto.declineCall
        )
    }

    static func forwardTransport(_ coreValue: String?) -> TimelineForwardTransport {
        switch coreValue {
        case "text": .text
        case "media": .media
        default: .unavailable
        }
    }

    static func reactionCounts(_ rows: [(key: String, count: UInt32)]) -> [String: Int] {
        var counts: [String: Int] = [:]
        for row in rows where row.key.isEmpty == false {
            counts[row.key] = Int(row.count)
        }
        return counts
    }

    static func mediaPlaceholder(
        rowKind: String,
        messageType: String?,
        mediaHandleId: String?,
        mediaMimeType: String?,
        mediaFilename: String?,
        mediaCaption: String?,
        formattedCaption: String?
    ) -> MediaResource? {
        guard let handle = mediaHandleId?.trimmingCharacters(in: .whitespacesAndNewlines),
              handle.isEmpty == false,
              let url = URL(string: "synara-timeline-media://\(handle)") else {
            return nil
        }
        let filename = mediaFilename?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let caption = mediaCaption?.trimmingCharacters(in: .whitespacesAndNewlines)
        return MediaResource(
            id: handle,
            filename: filename.isEmpty ? (messageType ?? rowKind) : filename,
            caption: caption?.isEmpty == false ? caption : nil,
            formattedCaption: formattedCaption,
            authenticatedURL: url,
            requiresAuthentication: true,
            isEncrypted: false,
            mimeType: mediaMimeType
        )
    }
}
