import Foundation
import SwiftUI

/// How a thread opens from the room timeline. Mirrors the desktop Thread
/// Display setting (inline / side panel / full view); on iOS the side panel
/// becomes a sheet.
enum ThreadDisplayMode: String, CaseIterable, Identifiable, Sendable {
    case inline
    case sheet
    case full

    static let settingsKey = "synara.settings.threadDisplay"
    static let defaultMode: ThreadDisplayMode = .inline

    var id: String { rawValue }

    var title: String {
        switch self {
        case .inline: "Inline"
        case .sheet: "Sheet"
        case .full: "Full Screen"
        }
    }

    var detail: String {
        switch self {
        case .inline: "Replies expand beneath their first message in the room."
        case .sheet: "Threads open in a sheet over the room."
        case .full: "Threads open on their own screen."
        }
    }

    static func resolve(_ rawValue: String?) -> ThreadDisplayMode {
        rawValue.flatMap(ThreadDisplayMode.init(rawValue:)) ?? defaultMode
    }
}

/// A thread presented as a sheet over the room.
struct ThreadSheetTarget: Identifiable, Equatable {
    let rootEventID: String
    let rootTitle: String?
    /// A routed reply (notification or link) the thread scrolls to.
    var focusedEventID: String? = nil

    var id: String { rootEventID }
}

/// Scrolls a thread opened from a notification or link to the routed reply.
/// The thread stream starts with its newest replies, so an older reply needs
/// a few "Load earlier replies" pages first.
enum ThreadFocusAnchorPolicy {
    enum Step: Equatable {
        case scroll(itemID: String)
        case loadEarlier
        /// The reply is not reachable: leave the thread at its newest replies.
        case settle
    }

    static let maximumEarlierPages = 5

    static func nextStep(
        focusedEventID: String,
        items: [TimelineItem],
        canLoadEarlier: Bool,
        earlierPagesLoaded: Int
    ) -> Step {
        if let target = items.first(where: {
            $0.eventID == focusedEventID || $0.serverEventID == focusedEventID
        }) {
            return .scroll(itemID: target.id)
        }
        if canLoadEarlier, earlierPagesLoaded < maximumEarlierPages {
            return .loadEarlier
        }
        return .settle
    }
}

/// "Reply in thread" asks the thread screen to focus its composer when it
/// appears, whichever presentation opens it.
final class ThreadComposerFocusRequests: @unchecked Sendable {
    static let shared = ThreadComposerFocusRequests()

    private let lock = NSLock()
    private var pending: Set<String> = []

    func request(roomID: String, rootEventID: String) {
        lock.lock()
        pending.insert(Self.key(roomID: roomID, rootEventID: rootEventID))
        lock.unlock()
    }

    /// Returns true once per request.
    func consume(roomID: String, rootEventID: String) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return pending.remove(Self.key(roomID: roomID, rootEventID: rootEventID)) != nil
    }

    private static func key(roomID: String, rootEventID: String) -> String {
        roomID + "\u{1F}" + rootEventID
    }
}

enum ThreadTimelineMerge {
    /// Sent replies stay visible until the thread stream contains them, so a
    /// snapshot taken before the echo arrives cannot make them disappear.
    static func merged(snapshot: [TimelineItem], pending: [TimelineItem]) -> [TimelineItem] {
        let stillPending = unresolved(pending: pending, snapshot: snapshot)
        guard stillPending.isEmpty == false else {
            return snapshot
        }
        return (snapshot + stillPending).sorted { $0.timestamp < $1.timestamp }
    }

    /// Pending sends that the snapshot has now delivered can be forgotten.
    /// A queued attachment has no event id yet; the SDK local echo in the
    /// snapshot carries its transaction id instead.
    static func unresolved(pending: [TimelineItem], snapshot: [TimelineItem]) -> [TimelineItem] {
        let known = Set(snapshot.flatMap { [$0.eventID, $0.serverEventID].compactMap { $0 } })
        let knownTransactions = Set(snapshot.compactMap(\.transactionID))
        return pending.filter { item in
            known.contains(item.eventID) == false
                && (item.serverEventID.map { known.contains($0) } ?? false) == false
                && (item.transactionID.map { knownTransactions.contains($0) } ?? false) == false
        }
    }
}

enum InlineThreadWindow {
    static let initialReplies = 5
    static let revealStep = 10

    /// Replies in the thread stream, oldest first, without the root.
    static func replies(in items: [TimelineItem], rootEventID: String) -> [TimelineItem] {
        items
            .filter { $0.eventID != rootEventID && TimelineThreadMembership.contains($0, rootEventID: rootEventID) }
            .sorted { $0.timestamp < $1.timestamp }
    }

    /// The newest `limit` replies; earlier ones are revealed on request.
    static func visible(_ replies: [TimelineItem], limit: Int) -> [TimelineItem] {
        Array(replies.suffix(max(0, limit)))
    }
}
