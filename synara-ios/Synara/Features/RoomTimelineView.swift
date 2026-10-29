import PhotosUI
import SwiftUI
import SynaraCore
import UniformTypeIdentifiers
#if canImport(UIKit)
    import UIKit
#endif

enum RoomTimelineFocusPolicy {
    static func initialMode(
        focusedEventID: String?,
        hasUnreadMessages: Bool,
        fullyReadEventID: String?,
        liveItems: [TimelineItem]
    ) -> RoomTimelineMode {
        if let focusedEventID, focusedEventID.isEmpty == false {
            return .focused(eventID: focusedEventID)
        }
        guard liveItems.isEmpty == false else {
            return .live
        }

        let receiptIndex = liveItems.lastIndex { item in
            item.hasCurrentUserReadReceipt && item.serverEventID != nil
        }
        let fullyReadIndex = fullyReadEventID.flatMap { eventID in
            liveItems.lastIndex { item in
                item.serverEventID == eventID
            }
        }

        let selectedIndex: Int?
        switch (receiptIndex, fullyReadIndex) {
        case let (receipt?, fullyRead?):
            selectedIndex = max(receipt, fullyRead)
        case let (receipt?, nil):
            selectedIndex = receipt
        case let (nil, fullyRead?):
            selectedIndex = fullyRead
        case (nil, nil):
            // An m.fully_read event outside this bounded live graph is not a
            // safe navigation target: a newer receipt may exist. Stay live.
            selectedIndex = nil
        }

        guard let selectedIndex,
              let liveTailIndex = liveItems.lastIndex(where: { $0.serverEventID != nil }),
              selectedIndex < liveTailIndex,
              let markerEventID = liveItems[selectedIndex].serverEventID
        else {
            return .live
        }
        return .unread(markerEventID: markerEventID)
    }

    /// Notification routes name an event, but the event-focused SDK timeline is
    /// a small `/context` window with no live growth whose back-pagination can
    /// report the start early (thread focus, no back gap). When the target is
    /// already inside the live window, present the live provider instead so
    /// history and new messages load exactly as they do from the room list.
    /// Returns nil when the event is outside the live window and only the
    /// event-focused provider can reach it.
    static func liveModeForEventRoute(
        focusedEventID: String,
        liveItems: [TimelineItem]
    ) -> RoomTimelineMode? {
        guard liveItems.contains(where: { $0.serverEventID == focusedEventID }) else {
            return nil
        }
        let liveTailEventID = liveItems.last(where: { $0.serverEventID != nil })?.serverEventID
        if liveTailEventID == focusedEventID {
            return .live
        }
        return .focused(eventID: focusedEventID)
    }

    /// How a routed event opens once Core has said where it lives. A thread
    /// reply is never in the room timeline (it hides in-thread events), and an
    /// event-focused open of one is a thread-only window with no back gap, so
    /// it must open the thread with the live room behind it.
    static func eventRouteDecision(
        focusedEventID: String,
        route: TimelineEventRoute,
        liveItems: [TimelineItem]
    ) -> RoomTimelineEventRouteDecision {
        switch route {
        case let .thread(rootEventID) where rootEventID != focusedEventID:
            return .thread(
                rootEventID: rootEventID,
                roomMode: liveModeForEventRoute(focusedEventID: rootEventID, liveItems: liveItems) ?? .live
            )
        case .missing:
            return .unavailable(notice: TimelineEventRouteNotice.missing)
        case .room, .unreachable, .thread:
            // An unreachable `/event` can still be served by the focused
            // provider's `/context`; Core falls back to the live room if not.
            return .room(liveMode: liveModeForEventRoute(focusedEventID: focusedEventID, liveItems: liveItems))
        }
    }
}

enum RoomTimelineEventRouteDecision: Equatable {
    /// Show the event in the room. `liveMode` presents the live provider; nil
    /// means only the event-focused provider can reach it.
    case room(liveMode: RoomTimelineMode?)
    /// Present the live room in `roomMode` and open the thread at the reply.
    case thread(rootEventID: String, roomMode: RoomTimelineMode)
    /// Present the live room and say why the message is not shown.
    case unavailable(notice: String)
}

enum RoomTimelineTitlePolicy {
    /// Notification routes may omit a title and must hydrate from the room list.
    /// An explicit route title (room-list tap, UI test) must not be replaced by a
    /// different cached room-list name.
    static func displayTitle(
        resolved: String?,
        routeTitle: String?,
        roomListName: String?
    ) -> String {
        if let routeTitle, routeTitle.isEmpty == false {
            return resolved ?? routeTitle
        }
        return resolved ?? roomListName ?? String(localized: "Room")
    }

    static func shouldAdoptRoomListName(routeTitle: String?) -> Bool {
        routeTitle == nil || routeTitle?.isEmpty == true
    }
}

enum RoomTimelinePaginationPolicy {
    /// A user scroll near the oldest row pages back. Short content that
    /// cannot scroll pages back on its own while Core still has history.
    static func shouldLoadOlderHistory(
        rowIndex: Int,
        topThreshold: Int,
        hasUserInteractedWithTimeline: Bool,
        hasPositionedInitialTimeline: Bool,
        isJumpingToLatest: Bool,
        isPaginating: Bool,
        hasReachedOldestMessages: Bool,
        isFillingShortContent: Bool = false,
        backwardPagination: TimelineBackwardPagination = .unknown
    ) -> Bool {
        guard rowIndex < topThreshold else {
            return false
        }
        guard hasPositionedInitialTimeline,
              isJumpingToLatest == false,
              isPaginating == false,
              hasReachedOldestMessages == false
        else {
            return false
        }
        if isFillingShortContent {
            return backwardPagination != .exhausted
        }
        return hasUserInteractedWithTimeline
    }

    /// "Reached the start" comes only from a page Core answered with the
    /// start of the room, and clears when Core reports more history again
    /// (it shrinks the room after a gappy sync).
    static func reachedOldestAfterUpdate(
        hasReachedOldestMessages: Bool,
        backwardPagination: TimelineBackwardPagination
    ) -> Bool {
        hasReachedOldestMessages && backwardPagination != .available
    }
}

enum RoomTimelineForwardPaginationPolicy {
    /// An event-focused window pages toward the present when the user reaches
    /// its newest row; the live provider is already there.
    static func shouldLoadNewer(
        providerIsLive: Bool,
        isPinned: Bool,
        hasPendingPlacement: Bool,
        isJumpingToLatest: Bool,
        isLoadingNewer: Bool
    ) -> Bool {
        providerIsLive == false
            && isPinned
            && hasPendingPlacement == false
            && isJumpingToLatest == false
            && isLoadingNewer == false
    }

    /// A focused window that cannot page further holds the live tail. When
    /// the user scrolled down to its bottom, the live provider takes over.
    /// Landing there from a route does not count: the live window may not
    /// hold the routed message.
    static func shouldReturnToLive(outcome: TimelineLoadOutcome, isPinned: Bool, hasUserScrolled: Bool) -> Bool {
        outcome == .empty && isPinned && hasUserScrolled
    }
}

enum RoomTimelinePositionState: Equatable {
    case preparing
    case placingInitial
    case followingLive
    case readingHistory
    case focusedEvent
}

enum RoomTimelineScrollPolicy {
    static func shouldFollowLiveAppend(
        position: RoomTimelinePositionState,
        isBottomVisible: Bool,
        focusedEventID: String?
    ) -> Bool {
        guard focusedEventID == nil else {
            return false
        }
        return isBottomVisible || position == .placingInitial || position == .followingLive
    }

    static func positionDuringUserDrag(
        current: RoomTimelinePositionState,
        translationHeight: CGFloat,
        focusedEventID: String?
    ) -> RoomTimelinePositionState {
        guard focusedEventID == nil, translationHeight > 0 else {
            return current
        }
        return .readingHistory
    }

    static func positionAfterUserDrag(
        isBottomVisible: Bool,
        focusedEventID: String?
    ) -> RoomTimelinePositionState {
        guard focusedEventID == nil else {
            return .focusedEvent
        }
        return isBottomVisible ? .followingLive : .readingHistory
    }
}

enum RoomTimelineSnapshotPolicy {
    static func shouldPreserveCurrentSnapshot(currentItemCount: Int, incomingItemCount: Int) -> Bool {
        currentItemCount > 0 && incomingItemCount == 0
    }
}

enum RoomTimelineFailurePresentationPolicy {
    static func retryMessage(
        for failure: TimelineLoadFailure,
        preservedItemCount: Int
    ) -> String? {
        preservedItemCount > 0 ? failure.userMessage : nil
    }
}

/// Where the "Unread messages" divider goes: above the first message after
/// the read marker that someone else sent. The user's own messages are read
/// by definition, so the divider never sits above one of them.
enum RoomTimelineUnreadDividerPolicy {
    static func dividerEventID(
        markerEventID: String?,
        items: [TimelineItem],
        currentUserID: String
    ) -> String? {
        guard let markerEventID, markerEventID.isEmpty == false,
              let markerIndex = items.firstIndex(where: {
                  $0.eventID == markerEventID || $0.id == markerEventID
              })
        else {
            return nil
        }
        return items[(markerIndex + 1)...].first { item in
            item.senderID != currentUserID
                && item.isLocalPending == false
                && item.localEcho == nil
        }?.eventID
    }
}

/// A room screen that comes back on screen (back from a pushed thread, a tab
/// switch, Settings and back) keeps its loaded window, position and stream;
/// only a different room or routed event, or a load that never finished,
/// starts over.
enum RoomTimelineReappearPolicy {
    static func canResume(
        loadedTaskID: String?,
        taskID: String,
        isStreamActive: Bool,
        hasPresentedTimeline: Bool
    ) -> Bool {
        loadedTaskID == taskID && isStreamActive && hasPresentedTimeline
    }
}

/// A timeline that failed to load tries again by itself once the likely
/// cause is gone: the connection is live again, or the room has appeared in
/// the room list (a cold notification open before the room synced).
enum RoomTimelineAutoRetryPolicy {
    static func shouldRetry(
        afterConnection previous: MatrixSyncStatus?,
        current: MatrixSyncStatus,
        hasFailedLoad: Bool
    ) -> Bool {
        guard hasFailedLoad, let previous else {
            return false
        }
        return OutgoingSendPolicy.becameSendReady(previous: previous, current: current)
    }

    static func shouldRetry(
        afterRoomListed wasListed: Bool,
        isListed: Bool,
        hasFailedLoad: Bool
    ) -> Bool {
        hasFailedLoad && isListed && wasListed == false
    }
}

enum RoomTimelineReadAcknowledgementPolicy {
    static func shouldSchedule(
        isApplicationActive: Bool,
        allowsReadReceipts: Bool,
        isLive: Bool,
        isConfirmedPinned: Bool,
        isJumpingToLatest: Bool,
        isUserInteracting: Bool,
        eventID: String,
        lastMarkedEventID: String?
    ) -> Bool {
        isApplicationActive
            && allowsReadReceipts
            && isLive
            && isConfirmedPinned
            && isJumpingToLatest == false
            && isUserInteracting == false
            && eventID != lastMarkedEventID
    }
}

/// A debounce replacement must replace the displayed anchor and receipt
/// target together. The task that drains this queue retains that exact pair
/// across the asynchronous write, even when another observation arrives.
struct RoomTimelineReadObservation: Equatable {
    let visibleEventID: String
    let receiptEventID: String
}

struct RoomTimelineReadMarkerQueue {
    private(set) var pending: RoomTimelineReadObservation?
    private(set) var firstQueuedAt: Date?

    mutating func enqueue(_ observation: RoomTimelineReadObservation, now: Date) {
        pending = observation
        if firstQueuedAt == nil { firstQueuedAt = now }
    }

    mutating func dequeue() -> RoomTimelineReadObservation? {
        defer { clear() }
        return pending
    }

    mutating func clear() {
        pending = nil
        firstQueuedAt = nil
    }
}

enum RoomTimelineReadMarkerQueuePolicy {
    static func delayNanoseconds(
        firstQueuedAt: Date,
        now: Date,
        debounceNanoseconds: UInt64,
        maximumLatencyNanoseconds: UInt64
    ) -> UInt64 {
        let elapsed = max(0, now.timeIntervalSince(firstQueuedAt))
        let elapsedNanoseconds = UInt64(elapsed * 1_000_000_000)
        let remainingMaximum = maximumLatencyNanoseconds > elapsedNanoseconds
            ? maximumLatencyNanoseconds - elapsedNanoseconds
            : 0
        return min(debounceNanoseconds, remainingMaximum)
    }
}

enum RoomTimelineReadMarkerTaskPolicy {
    static func ownsInstalledTask(installedGeneration: UInt64, currentGeneration: UInt64) -> Bool {
        installedGeneration == currentGeneration
    }
}

enum RoomTimelineTimestampRevealPolicy {
    static let displayDurationNanoseconds: UInt64 = 2_500_000_000

    static func horizontalOffset(
        isGroupedWithPrevious: Bool,
        isRevealed: Bool,
        width: CGFloat
    ) -> CGFloat {
        isGroupedWithPrevious && isRevealed ? -width : 0
    }

    static func taskMayDismiss(
        taskGeneration: UInt64,
        currentGeneration: UInt64,
        taskEventID: String,
        revealedEventID: String?,
        isCancelled: Bool
    ) -> Bool {
        isCancelled == false
            && taskGeneration == currentGeneration
            && revealedEventID == taskEventID
    }
}

enum RoomTimelineOwnAvatarPolicy {
    static func mayInstall(
        profileUserID: String?,
        expectedUserID: String,
        expectedTimelineTaskID: String,
        currentTimelineTaskID: String,
        isCancelled: Bool
    ) -> Bool {
        isCancelled == false
            && profileUserID == expectedUserID
            && expectedTimelineTaskID == currentTimelineTaskID
    }
}

enum RoomTimelineJumpLatestPolicy {
    static func shouldShow(isLive: Bool, isConfirmedPinned: Bool, hasItems: Bool, requested: Bool) -> Bool {
        // Geometry and provider identity are authoritative even if an earlier
        // command cleared the presentation request before layout settled.
        hasItems && (isLive == false || isConfirmedPinned == false)
    }

    /// Pin the current live rows only when this session is already on the live
    /// tail. Unread restore reuses the live provider while sitting on last-read;
    /// a brief inverted-table pin must not count as live-follow.
    static func shouldPinCurrentLiveWindow(
        providerIsLive: Bool,
        isLiveMode: Bool,
        isFollowingLive: Bool,
        isConfirmedPinned: Bool
    ) -> Bool {
        providerIsLive && (isConfirmedPinned || (isFollowingLive && isLiveMode))
    }

    /// A confirmed pin on the live provider follows the live tail. An unread
    /// or focused placement pins briefly before it scrolls to its row, so
    /// those positions adopt the pin only once their placement completed and
    /// no drag is in progress: the placed rows fit on screen at the live edge.
    static func shouldAdoptLiveFollowOnPin(
        position: RoomTimelinePositionState,
        hasPendingPlacement: Bool,
        isUserDragging: Bool
    ) -> Bool {
        switch position {
        case .readingHistory, .focusedEvent:
            return hasPendingPlacement == false && isUserDragging == false
        case .preparing, .placingInitial, .followingLive:
            return true
        }
    }
}

enum RoomTimelineJumpLastReadPolicy {
    /// Decide, once at open, whether a last-read frontier becomes an action.
    /// The room must have had unread activity at that moment and the marker must
    /// not already be in the initially loaded rows. Evaluating unread state
    /// continuously would be wrong: the timeline auto-advances `m.fully_read`
    /// about a second after the tail is visible, which would hide the action
    /// before the user has seen the messages between last-read and the tail.
    static func pendingMarker(
        fullyReadEventID: String?,
        initialEventIDs: [String],
        hasUnreadMessages: Bool
    ) -> String? {
        guard hasUnreadMessages, let marker = fullyReadEventID, marker.isEmpty == false else {
            return nil
        }
        return initialEventIDs.contains(marker) ? nil : marker
    }

    /// The pending marker is the single owner of visibility, mirroring the
    /// desktop presenter: it is cleared on successful placement, on an explicit
    /// Jump to Latest, or when the room changes. Rows loading later off-screen
    /// do not count as "already there".
    static func shouldShow(pendingMarkerEventID: String?, isMarkerVisible: Bool = false) -> Bool {
        guard let marker = pendingMarkerEventID, marker.isEmpty == false else {
            return false
        }
        // The marker row or the unread divider on screen is the answer already.
        return isMarkerVisible == false
    }

    enum Route: Equatable {
        /// The marker is in the live window: scroll there, stay live.
        case scrollLiveWindow
        /// Open an event-focused window beside the live provider.
        case openFocusedWindow
    }

    static func route(
        markerEventID: String,
        providerIsLive: Bool,
        loadedServerEventIDs: [String]
    ) -> Route {
        providerIsLive && loadedServerEventIDs.contains(markerEventID) ? .scrollLiveWindow : .openFocusedWindow
    }
}

enum RoomTimelineLatestCommandCompletionPolicy {
    static func shouldShowRecovery(success: Bool) -> Bool {
        success == false
    }
}

enum RoomTypingPresentation {
    static func displayName(for userID: String) -> String {
        let localPart = userID.split(separator: ":", maxSplits: 1).first.map(String.init) ?? userID
        let withoutSigil = localPart.hasPrefix("@") ? String(localPart.dropFirst()) : localPart
        return withoutSigil.isEmpty ? userID : withoutSigil
    }

    static func text(for userIDs: [String]) -> String? {
        let names = Array(Set(userIDs.map(displayName))).sorted()
        switch names.count {
        case 0:
            return nil
        case 1:
            return String(localized: "\(names[0]) is typing...")
        case 2:
            return String(localized: "\(names[0]) and \(names[1]) are typing...")
        default:
            return String(localized: "\(names[0]), \(names[1]), and \(names.count - 2) more are typing...")
        }
    }
}

struct RoomTimelineView: View {
    private enum TimelineScrollTarget {
        case bottom
        case event(String, anchor: UnitPoint)
    }

    private static let olderPaginationTopThreshold = 3
    private static let olderPaginationDebounceInterval: TimeInterval = 0.5
    private static let markFullyReadDelayNanoseconds: UInt64 = 1_000_000_000
    private static let markFullyReadMaximumLatencyNanoseconds: UInt64 = 2_000_000_000
    private static let timelineBottomLayoutDelayNanoseconds: UInt64 = 16_000_000
    private static let timelineBottomAnchorID = "timeline-bottom-anchor"

    let roomID: String
    let roomTitle: String?
    let focusedEventID: String?
    @Environment(\.appEnvironment) private var environment
    @Environment(\.synaraThemeBaseHex) private var themeBaseHex
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.scenePhase) private var scenePhase
    @State private var isReadSceneActive = false
    @State private var resolvedRoomTitle: String?
    @State private var state: TimelineViewState = .idle
    @State private var draft: String = ""
    @State private var sentDraft: ComposerSentDraft?
    @State private var sentDraftSerial: UInt64 = 0
    @StateObject private var quickReactions = TimelineQuickReactionStore()
    @State private var replyTarget: ComposerRelationTarget?
    @State private var editSession: ComposerEditSession?
    @State private var sendError: String?
    @State private var timelineAvailability = RoomTimelineAvailabilityState()
    @State private var hasAnchoredEvent = false
    @State private var uploadState: MediaUploadState = .idle
    @State private var viewerResource: MediaResource?
    @State private var selectedPhotos: [PhotosPickerItem] = []
    @State private var attachmentDrafts: [ComposerAttachmentDraft] = []
    @State private var attachmentSendTransaction: ComposerAttachmentSendTransaction?
    @State private var isSendingMessage = false
    @State private var agentActionMessage: String?
    @State private var roomNotesActionMessage: String?
    @State private var timelineActionMessage: String?
    @State private var reportTarget: TimelineItem?
    @State private var redactConfirmation = TimelineRedactConfirmation<TimelineItem>()
    @State private var reportReason = ""
    @State private var forwardTarget: TimelineItem?
    @State private var reactionsTarget: TimelineReactionsSheetTarget?
    @State private var threadDisplay: ThreadDisplayMode = ThreadDisplayMode.defaultMode
    @State private var expandedThreadRoots: Set<String> = []
    @State private var threadSheet: ThreadSheetTarget?
    @State private var eventRouteNotice: String?
    @State private var handledEventRouteID: String?
    @State private var inFlightTimelineActionKeys: Set<String> = []
    @State private var cryptoStatus: RoomCryptoStatus = .unknown
    @State private var cryptoActionMessage: String?
    @State private var isCryptoBannerDismissed = false
    @State private var isRoomDetailsPresented = false
    @State private var shouldReturnToListAfterDetailsDismiss = false
    @State private var isTimelineSearchPresented = false
    @State private var timelineSearchQuery = ""
    @State private var lastRenderedTimelineCount = 0
    @State private var showJumpToLatest = false
    @State private var hasPositionedInitialTimeline = false
    /// The row the "Unread messages" divider sits above. Fixed when the room
    /// opens and kept until the user leaves it, even after the read receipt
    /// is sent; never one of the user's own messages. Presentation only: it
    /// must never drive post-load scroll restore (v1.2.28 open-at-live-end
    /// policy).
    @State private var unreadDividerEventID: String?
    @State private var pendingLastReadEventID: String?
    @State private var isLastReadMarkerVisible = false
    /// The marker of a Jump to Last Read in progress. Its placement failing
    /// says so and returns to the latest messages instead of offering it again.
    @State private var lastReadJumpEventID: String?
    @State private var isLoadingNewerTimeline = false
    @State private var lastNewerPaginationAt = Date.distantPast
    @State private var sendLatestRequest = 0
    @State private var sendFollowLatestTask: Task<Void, Never>?
    @State private var hasReachedOldestMessages = false
    @State private var lastOlderPaginationAt = Date.distantPast
    @State private var olderPaginationCompletions: UInt64 = 0
    @State private var paginationScrollAnchorID: String?
    @State private var isJumpingToLatest = false
    @State private var isComposerFocused = false
    @State private var isTimelineBottomVisible = false
    @State private var timelineBottomAnchorGeneration: UInt64 = 0
    @State private var lastMarkedFullyReadEventID: String?
    @State private var markFullyReadTask: Task<Void, Never>?
    @State private var markFullyReadTaskGeneration: UInt64 = 0
    @State private var readMarkerQueue = RoomTimelineReadMarkerQueue()
    @State private var timelineUpdatesTask: Task<Void, Never>?
    /// The tab whose navigation holds this screen. While it still holds the
    /// room, a disappear means covered (a pushed thread, another tab), not
    /// left, and the stream keeps running.
    @State private var hostTab: AppTab?
    /// The room (and routed event) whose timeline finished loading. A
    /// re-appear of the same route resumes it instead of reloading.
    @State private var loadedTimelineTaskID: String?
    @State private var isTimelineStreamActive = false
    @State private var observedSyncStatus: MatrixSyncStatus?
    @State private var isRoomListed = false
    @State private var cryptoUpdatesTask: Task<Void, Never>?
    @State private var timelineSession: RoomTimelineSession?
    @State private var activeTimelineMode: RoomTimelineMode = .live
    @State private var timelineProviderIsLive = false
    @State private var typingUpdatesTask: Task<Void, Never>?
    @State private var typingUserIDs: [String] = []
    @State private var timelineScrollTask: Task<Void, Never>?
    @State private var sendAnimationItemIDs: Set<String> = []
    @State private var hasUserInteractedWithTimeline = false
    @State private var isUserDraggingTimeline = false
    @State private var timelinePosition: RoomTimelinePositionState = .preparing
    @State private var timelineTraceID = String(UUID().uuidString.prefix(8))
    @State private var timelineTraceStartedAt = Date()
    @State private var stableViewportCommand: StableTimelineViewportCommand?
    @State private var stableViewportCommandID: UInt64 = 0
    @State private var revealedTimestampEventID: String?
    @State private var timestampRevealTask: Task<Void, Never>?
    @State private var timestampRevealGeneration: UInt64 = 0
    @State private var ownAvatarURL: URL?
    private let timelineLogger = AppLogger()
    @Environment(\.openURL) private var openURL
    @Environment(\.dismiss) private var dismiss
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @Environment(\.accessibilityReduceMotion) private var accessibilityReduceMotion
    @Environment(\.synaraCanvasLayout) private var canvasLayout

    init(roomID: String, roomTitle: String?, focusedEventID: String? = nil) {
        self.roomID = roomID
        self.roomTitle = roomTitle
        self.focusedEventID = focusedEventID
    }

    private var displayRoomTitle: String {
        RoomTimelineTitlePolicy.displayTitle(
            resolved: resolvedRoomTitle,
            routeTitle: roomTitle,
            roomListName: environment.roomList.roomDisplayName(roomID: roomID)
        )
    }

    var body: some View {
        VStack(spacing: 0) {
            TimelineHeader(
                title: displayRoomTitle,
                subtitle: timelineSubtitle,
                glyph: SynaraRoomGlyph.glyph(for: roomSummary),
                avatarURL: roomSummary?.avatarURL,
                cryptoLabel: cryptoStatus.observing(loadedTimelineItems).roomHeaderLabel,
                cryptoSystemImage: cryptoStatus.observing(loadedTimelineItems).roomHeaderSystemImage,
                showsBackButton: canvasLayout.showsConversationBackButton,
                onSearch: { isTimelineSearchPresented = true },
                onDetails: { isRoomDetailsPresented = true },
                onBack: {
                    dismissKeyboard()
                    dismiss()
                }
            )
            timelineContent
            if let eventRouteNotice {
                TimelineEventRouteNoticeBanner(message: eventRouteNotice) {
                    self.eventRouteNotice = nil
                }
            }
            if let failure = timelineAvailability.failure {
                TimelineAvailabilityBanner(failure: failure) {
                    Task { await loadTimeline() }
                }
            }
            if let typingText = RoomTypingPresentation.text(for: typingUserIDs) {
                RoomTypingIndicator(text: typingText)
            }
            RoomIdentityWarningBanner(roomID: roomID, crypto: environment.crypto)
            Divider()
            ComposerView(
                roomID: roomID,
                text: $draft,
                placeholder: composerPlaceholder,
                showsPromptMetrics: isAgentRoom,
                replyTarget: replyTarget,
                editTarget: editSession?.editTarget,
                uploadState: uploadState,
                sendError: sendError,
                onCancelRelation: clearComposerRelation,
                onSend: sendMessage,
                onMockMediaUpload: draftMockMedia,
                onFileURL: draftPickedFile,
                onCameraImage: draftCameraImage,
                onUploadFailed: { message in
                    uploadState = .failed(message)
                },
                selectedPhotos: $selectedPhotos,
                attachmentDrafts: $attachmentDrafts,
                isSending: isSendingMessage,
                onPasteImages: draftPastedImages,
                isFocusedExternally: $isComposerFocused,
                sentDraft: sentDraft,
                onSendGif: sendGif
            )
            .padding(.bottom, canvasLayout == .stacked && !isComposerFocused ? SynaraSpacing.large : 0)
            .background { SynaraChrome.composer.ignoresSafeArea(edges: .bottom) }
            .synaraDockedDepth(
                .floating,
                boundaryColor: isAgentRoom ? SynaraColor.agent : SynaraColor.separator
            )
        }
        .background {
            (isAgentRoom ? SynaraChrome.agentReview : SynaraChrome.chat).ignoresSafeArea()
        }
        .ignoresSafeArea(.container, edges: canvasLayout == .stacked ? .bottom : [])
        .navigationTitle(displayRoomTitle)
        .navigationBarBackButtonHidden(true)
        .toolbar(.hidden, for: .navigationBar)
        .toolbar(canvasLayout.hidesTabBarInConversation ? .hidden : .automatic, for: .tabBar)
        // Only the agent conversation reads dark; the window, tab bar and
        // the rest of the app keep following the system appearance.
        .environment(\.colorScheme, conversationColorScheme)
        .sheet(item: $viewerResource) { resource in
            MediaViewer(resource: resource)
        }
        .sheet(isPresented: $isRoomDetailsPresented) {
            RoomDetailsView(
                roomID: roomID,
                fallbackTitle: displayRoomTitle,
                onLeaveRoom: {
                    shouldReturnToListAfterDetailsDismiss = true
                    isRoomDetailsPresented = false
                },
                onOpenMessage: { eventID in
                    isRoomDetailsPresented = false
                    Task { @MainActor in
                        await Task.yield()
                        environment.router.route(to: .room(id: roomID, eventID: eventID, title: displayRoomTitle))
                    }
                }
            )
        }
        .sheet(isPresented: $isTimelineSearchPresented) {
            TimelineSearchSheet(
                query: $timelineSearchQuery,
                items: loadedTimelineItems,
                onDismiss: {
                    timelineSearchQuery = ""
                    isTimelineSearchPresented = false
                }
            )
        }
        .sheet(item: $reactionsTarget) { target in
            let items = loadedTimelineItems
            TimelineReactionsSheet(
                // The live row, so reactions arriving while it is open show.
                item: items.first { $0.eventID == target.item.eventID } ?? target.item,
                initialKey: target.initialKey,
                currentUserID: currentUserID,
                roomID: roomID,
                directory: TimelineReactorDirectory(items: items),
                onDismiss: { reactionsTarget = nil }
            )
            .environment(\.appEnvironment, environment)
        }
        .sheet(item: $forwardTarget) { item in
            TimelineForwardSheet(
                sourceRoomID: roomID,
                sourceEncryption: cryptoStatus.encryption,
                sourceTransport: item.forwardTransport,
                roomList: environment.roomList,
                onForward: { targetRoomID, asQuote, confirmedEncryptionDowngrade in
                    await performForwardAction(
                        targetRoomID: targetRoomID,
                        asQuote: asQuote,
                        confirmedEncryptionDowngrade: confirmedEncryptionDowngrade,
                        to: item,
                    )
                }
            )
        }
        .timelineRedactConfirmation($redactConfirmation) { item in
            applyAction(.redact, to: item)
        }
        .alert("Report Message", isPresented: Binding(
            get: { reportTarget != nil },
            set: {
                if !$0 {
                    reportTarget = nil
                    reportReason = ""
                }
            }
        )) {
            TextField("Reason (optional)", text: Binding(
                get: { reportReason },
                set: { reportReason = String($0.prefix(512)) }
            ))
            Button("Cancel", role: .cancel) {
                reportTarget = nil
                reportReason = ""
            }
            Button("Report", role: .destructive) {
                guard let item = reportTarget else { return }
                let trimmedReason = reportReason.trimmingCharacters(in: .whitespacesAndNewlines)
                reportTarget = nil
                reportReason = ""
                applyAction(
                    .report(reason: trimmedReason.isEmpty ? nil : trimmedReason),
                    to: item,
                    successMessage: String(localized: "Message reported.")
                )
            }
        } message: {
            Text("The report is sent to your homeserver administrators.")
        }
        .alert("Message Action", isPresented: Binding(
            get: { timelineActionMessage != nil },
            set: { if !$0 { timelineActionMessage = nil } }
        )) {
            Button("OK") { timelineActionMessage = nil }
        } message: {
            Text(timelineActionMessage ?? "")
        }
        .alert("Agent Action", isPresented: Binding(
            get: { agentActionMessage != nil },
            set: { if !$0 { agentActionMessage = nil } }
        )) {
            Button("OK") {
                agentActionMessage = nil
            }
        } message: {
            Text(agentActionMessage ?? "")
        }
        .alert("Encryption", isPresented: Binding(
            get: { cryptoActionMessage != nil },
            set: { if !$0 { cryptoActionMessage = nil } }
        )) {
            Button("OK") {
                cryptoActionMessage = nil
            }
        } message: {
            Text(cryptoActionMessage ?? "")
        }
        .alert("Personal Notes", isPresented: Binding(
            get: { roomNotesActionMessage != nil },
            set: { if !$0 { roomNotesActionMessage = nil } }
        )) {
            Button("OK") { roomNotesActionMessage = nil }
        } message: {
            Text(roomNotesActionMessage ?? "")
        }
        .task {
            let matrix = environment.matrix
            await quickReactions.refresh { await matrix.recentEmoji() }
        }
        .task(id: "room-title-\(currentUserID)-\(roomID)") {
            // Notification routes carry stable room/event IDs, not room metadata.
            // Hydrate presentation from the room-list owner only when the route
            // omitted a title. An explicit title must keep the current viewport
            // identity; changing it must not replace the focused timeline.
            guard RoomTimelineTitlePolicy.shouldAdoptRoomListName(routeTitle: roomTitle) else {
                return
            }
            resolvedRoomTitle = environment.roomList.roomDisplayName(roomID: roomID)
            for await update in environment.roomList.roomUpdates() {
                guard !Task.isCancelled else { return }
                if case let .loaded(rooms) = update,
                   let name = rooms.first(where: { $0.id == roomID })?.name,
                   name != resolvedRoomTitle {
                    resolvedRoomTitle = name
                }
            }
        }
        .task(id: timelineTaskID) {
            if RoomTimelineReappearPolicy.canResume(
                loadedTaskID: loadedTimelineTaskID,
                taskID: timelineTaskID,
                isStreamActive: isTimelineStreamActive,
                hasPresentedTimeline: hasPresentedTimeline
            ) {
                resumeTimelineAfterReappear()
                return
            }
            resetTimelineState()
            let expectedTimelineTaskID = timelineTaskID
            let expectedUserID = currentUserID
            // Cold notification routes can construct this screen before the
            // root shell has restored/attached Core. Share the existing startup
            // gate instead of opening a timeline against an unprepared owner.
            if case let .signedIn(session) = environment.session.currentState {
                guard await environment.sessionReadiness.waitUntilPrepared(for: session) else { return }
            }
            guard !Task.isCancelled,
                  expectedTimelineTaskID == timelineTaskID,
                  expectedUserID == currentUserID else { return }
            let roomOpenSignpostID = PerformanceTrace.begin("RoomOpen")
            defer {
                PerformanceTrace.end("RoomOpen", id: roomOpenSignpostID)
            }
            async let initialCryptoStatus = loadCryptoStatus()
            async let ownAvatarLoad: Void = loadOwnAvatarURL(
                expectedTimelineTaskID: expectedTimelineTaskID,
                expectedUserID: expectedUserID
            )
            startTypingUpdates()
            startVerificationAutoRetry()
            await loadTimeline()
            applyOutgoingQueueToTimeline()
            _ = await initialCryptoStatus
            await ownAvatarLoad
            let openedStatus = await loadCryptoStatus()
            guard !Task.isCancelled, expectedTimelineTaskID == timelineTaskID else { return }
            // A verified session without undecryptable rows has nothing to
            // retry, so the room does not keep the session subscription.
            if !RoomCryptoObservationPolicy.needsSessionUpdates(openedStatus.observing(loadedTimelineItems)) {
                cryptoUpdatesTask?.cancel()
                cryptoUpdatesTask = nil
            }
        }
        .onAppear {
            isReadSceneActive = scenePhase == .active
            if hostTab == nil {
                hostTab = environment.router.selectedTab
            }
            threadDisplay = ThreadDisplayMode.resolve(
                environment.settings.string(for: ThreadDisplayMode.settingsKey)
            )
        }
        .sheet(item: $threadSheet) { target in
            ThreadTimelineView(
                roomID: roomID,
                rootEventID: target.rootEventID,
                roomTitle: displayRoomTitle,
                rootTitle: target.rootTitle,
                focusedEventID: target.focusedEventID
            )
            .environment(\.appEnvironment, environment)
            .presentationDetents([.medium, .large])
            .presentationDragIndicator(.visible)
        }
        .onDisappear {
            isReadSceneActive = false
            dismissKeyboard()
            // A pushed thread or another tab covers the room without leaving
            // it: keep the stream, window and position for when it comes
            // back. Leaving the room ends the stream so Core can close it.
            if environment.router.holdsRoom(id: roomID, eventID: focusedEventID, in: hostTab) == false {
                stopTimelineUpdates(reason: "room-left")
            }
            stopTypingUpdates()
            cryptoUpdatesTask?.cancel()
            cryptoUpdatesTask = nil
            cancelTimelineScroll()
            // A disappearing view no longer proves that its previously painted
            // tail is visible. Cancel the tracked automatic acknowledgement;
            // never launch an untracked read write from teardown.
            cancelMarkFullyRead()
            cancelTimestampReveal()
        }
        .onChange(of: scenePhase) { phase in
            // iOS 16 onChange closures can still capture the old environment
            // value. Store the delivered phase for callbacks and pending work.
            isReadSceneActive = phase == .active
            if phase == .active {
                // The viewport may already have painted a new tail while the
                // scene was inactive. Becoming active does not change its
                // geometry, so it need not emit another pinned callback.
                if let eventID = loadedTimelineItems.reversed().compactMap(\.serverEventID).first {
                    scheduleMarkFullyRead(eventID: eventID)
                }
            } else {
                cancelMarkFullyRead()
            }
        }
        .onReceive(environment.outgoingSends.queue.$items) { _ in
            applyOutgoingQueueToTimeline()
        }
        .onReceive(environment.connectionStatus.$status) { status in
            let previous = observedSyncStatus
            observedSyncStatus = status
            if RoomTimelineAutoRetryPolicy.shouldRetry(
                afterConnection: previous,
                current: status,
                hasFailedLoad: hasFailedTimelineLoad
            ) {
                retryFailedTimelineLoad(reason: "connection-live")
            }
        }
        .task(id: "room-listed-\(currentUserID)-\(roomID)") {
            isRoomListed = false
            for await update in environment.roomList.roomUpdates() {
                guard !Task.isCancelled else { return }
                guard case let .loaded(rooms) = update else { continue }
                let isListed = rooms.contains { $0.id == roomID }
                let wasListed = isRoomListed
                isRoomListed = isListed
                if RoomTimelineAutoRetryPolicy.shouldRetry(
                    afterRoomListed: wasListed,
                    isListed: isListed,
                    hasFailedLoad: hasFailedTimelineLoad
                ) {
                    retryFailedTimelineLoad(reason: "room-listed")
                }
            }
        }
        .onChange(of: draft) { value in
            environment.drafts.setDraft(value, roomID: roomID)
        }
        .onChange(of: timelineBottomAnchorGeneration) { _ in
            cancelTimestampReveal()
        }
        .onChange(of: isRoomDetailsPresented) { isPresented in
            guard isPresented == false, shouldReturnToListAfterDetailsDismiss else {
                return
            }
            shouldReturnToListAfterDetailsDismiss = false
            environment.router.popSelectedTabToRoot()
        }
        .onChange(of: selectedPhotos) { items in
            draftPickedPhotos(items)
        }
        .onChange(of: isComposerFocused) { focused in
            if focused {
                cancelTimelineScroll()
            }
        }
    }

    @ViewBuilder
    private var timelineContent: some View {
        switch state {
        case .idle, .loading:
            ScrollView {
                SynaraTimelineSkeletonList(rowCount: 8)
                    .padding(.horizontal, SynaraSpacing.medium)
                    .padding(.top, SynaraSpacing.medium)
                    .padding(.bottom, SynaraSpacing.small)
            }
            .background(isAgentRoom ? SynaraChrome.agentReview : SynaraChrome.chat)
            .accessibilityIdentifier("TimelineLoading")
        case .empty:
            SynaraEmptyState(
                title: String(localized: "No Messages"),
                systemImage: "text.bubble",
                message: String(localized: "Messages will appear here.")
            )
        case let .failed(message):
            SynaraErrorState(title: String(localized: "Could Not Load Timeline"), message: message) {
                Task {
                    await loadTimeline()
                }
            }
        case let .loaded(items, isPaginating):
            if StableScrollAnchoringFeatureFlag.isEnabled {
                stableTimelineContent(items: items, isPaginating: isPaginating)
            } else {
                ScrollViewReader { proxy in
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                            if isPaginating {
                                ProgressView()
                                    .controlSize(.small)
                                    .frame(maxWidth: .infinity)
                                    .padding(.vertical, SynaraSpacing.xSmall)
                                    .accessibilityIdentifier("TimelinePaginationIndicator")
                            }

                            if shouldShowCryptoBanner(items: items) {
                                CryptoRecoveryBanner(
                                    status: cryptoStatus.observing(items),
                                    onRetry: retryDecryption,
                                    onReviewSecurity: { environment.router.route(to: .settings) },
                                    onDismiss: { isCryptoBannerDismissed = true }
                                )
                            }

                            let threadReplyCounts = TimelineReplyCounter.replyCounts(for: items)
                            let replyPreviewsByEventID = TimelineReplyPreview.previewsByEventID(
                                in: items,
                                currentUserID: currentUserID
                            )
                            let reactorDirectory = TimelineReactorDirectory(items: items)

                            let dayHeaders = TimelineDayHeaderPolicy.headers(for: items)
                            let userID = currentUserID
                            let isInlineThreadMode = threadDisplay == .inline
                            ForEach(Array(items.enumerated()), id: \.element.eventID) { index, item in
                                if let day = dayHeaders[item.eventID] {
                                    TimelineDayHeader(day: day)
                                        .padding(.vertical, SynaraSpacing.small)
                                }
                                if shouldShowUnreadDivider(before: item, at: index, in: items) {
                                    UnreadMessagesDivider()
                                        .padding(.vertical, SynaraSpacing.small)
                                }
                                TimelineRow(
                                    item: item,
                                    currentUserID: userID,
                                    timeText: item.timestamp.timelineTime,
                                    isGroupedWithPrevious: isGroupedWithPrevious(index: index, items: items),
                                    isTimestampRevealed: false,
                                    animateSend: sendAnimationItemIDs.contains(item.id),
                                    replyPreview: TimelineRelationPresentation.replyPreview(
                                        for: item,
                                        locallyResolvedByEventID: replyPreviewsByEventID
                                    ),
                                    replyCount: TimelineRelationPresentation.replyCount(
                                        for: item,
                                        locallyCountedByRootID: threadReplyCounts
                                    ),
                                    availability: environment.eventActions.availability(
                                        for: item,
                                        currentUserID: userID,
                                        roomID: roomID
                                    ),
                                    onReply: { beginReply(item) },
                                    onReplyInThread: { replyInThread(item) },
                                    onOpenThread: { openThread(root: item) },
                                    isInlineThreadMode: isInlineThreadMode,
                                    isThreadExpanded: isThreadExpanded(item),
                                    inlineThread: inlineThread(
                                        for: item,
                                        replyCount: TimelineRelationPresentation.replyCount(
                                            for: item,
                                            locallyCountedByRootID: threadReplyCounts
                                        )
                                    ),
                                    onEdit: { beginEdit(item) },
                                    onRedact: { redactConfirmation.request(item) },
                                    onToggleReaction: { key in toggleReaction(key, on: item) },
                                    quickReactions: quickReactions,
                                    onReport: { beginReport(item) },
                                    onForward: { forwardTarget = item },
                                    onPollVote: { answerIDs in
                                        await performAction(
                                            .pollVote(answerIDs: answerIDs),
                                            to: item,
                                            successMessage: String(localized: "Vote submitted.")
                                        )
                                    },
                                    onDeclineCall: {
                                        await performAction(
                                            .declineCall,
                                            to: item,
                                            successMessage: String(localized: "Call declined.")
                                        )
                                    },
                                    onPinToNotes: { pinToNotes(item) },
                                    onOpenMedia: { resource in viewerResource = resource },
                                    onAgentAction: { action in
                                        executeAgentAction(action, sourceEventID: item.eventID)
                                    },
                                    onAgentApprovalReaction: { actionIdentifier in
                                        submitAgentApprovalDecision(actionIdentifier: actionIdentifier, sourceEventID: item.eventID)
                                    },
                                    onRetryFailedSend: {
                                        retryFailedMessage(item)
                                    },
                                    onDiscardFailedSend: {
                                        discardFailedMessage(item)
                                    },
                                    reactorNames: reactorDirectory.names(for: item),
                                    onShowReactions: { key in showReactions(for: item, key: key) }
                                )
                                .id(item.eventID)
                                .onAppear {
                                    if isAgentRoom == false, index < Self.olderPaginationTopThreshold {
                                        loadOlderTimelineIfNeeded(anchorItem: item, index: index, items: items)
                                    }
                                }
                            }

                            Color.clear
                                .frame(height: 1)
                                .id("\(Self.timelineBottomAnchorID)-\(timelineBottomAnchorGeneration)")
                                .accessibilityHidden(true)
                                .onAppear {
                                    isTimelineBottomVisible = true
                                    if timelineProviderIsLive {
                                        resumeLivePresentationAtBottom()
                                        timelinePosition = .followingLive
                                        if isJumpingToLatest == false,
                                           let latestEventID = items.reversed().compactMap(\.serverEventID).first
                                        {
                                            scheduleMarkFullyRead(eventID: latestEventID)
                                        }
                                        showJumpToLatest = false
                                    } else {
                                        loadNewerTimelineIfNeeded()
                                    }
                                    logTimelineEvent("bottom-visible", fields: ["items": "\(items.count)"])
                                }
                                .onDisappear {
                                    isTimelineBottomVisible = false
                                    cancelMarkFullyRead()
                                    if isUserDraggingTimeline, timelineProviderIsLive {
                                        timelinePosition = .readingHistory
                                    }
                                    if items.count > 1 && timelinePosition == .readingHistory {
                                        showJumpToLatest = true
                                    }
                                    logTimelineEvent(
                                        "bottom-hidden",
                                        fields: [
                                            "items": "\(items.count)",
                                            "userInteracted": "\(hasUserInteractedWithTimeline)",
                                        ]
                                    )
                                }
                        }
                        .padding(.horizontal, timelineHorizontalPadding)
                        .padding(.top, isAgentRoom ? SynaraSpacing.medium : SynaraSpacing.small)
                        .padding(.bottom, SynaraSpacing.small)
                    }
                    .scrollDismissesKeyboard(.interactively)
                    .background(isAgentRoom ? SynaraChrome.agentReview : SynaraChrome.chat)
                    .accessibilityIdentifier("TimelineList")
                    .simultaneousGesture(
                        DragGesture(minimumDistance: 8)
                            .onChanged { value in
                                hasUserInteractedWithTimeline = true
                                if isUserDraggingTimeline == false {
                                    cancelMarkFullyRead()
                                }
                                isUserDraggingTimeline = true
                                timelinePosition = RoomTimelineScrollPolicy.positionDuringUserDrag(
                                    current: timelinePosition,
                                    translationHeight: value.translation.height,
                                    focusedEventID: timelineProviderFocusedEventID
                                )
                                if timelinePosition == .readingHistory,
                                   isTimelineBottomVisible == false
                                {
                                    showJumpToLatest = true
                                }
                                if items.count > 8 {
                                    cancelTimelineScroll()
                                }
                            }
                            .onEnded { _ in
                                finishTimelineUserDrag()
                            }
                    )
                    .overlay(alignment: .bottomTrailing) {
                        timelineJumpControls(
                            items: items,
                            showLatest: RoomTimelineJumpLatestPolicy.shouldShow(
                                isLive: timelineProviderIsLive,
                                isConfirmedPinned: isTimelineBottomVisible,
                                hasItems: items.last != nil,
                                requested: showJumpToLatest
                            ),
                            onJumpToLatest: { jumpToLatest(proxy: proxy, currentItems: items) }
                        )
                    }
                    .overlay(alignment: .topLeading) { timelineNavigationRecovery }
                    .onChange(of: sendLatestRequest) { _ in
                        jumpToLatest(proxy: proxy, currentItems: loadedTimelineItems)
                    }
                    .onAppear {
                        lastRenderedTimelineCount = items.count
                        if scrollToInitialPosition(items: items, proxy: proxy) == false {
                            _ = scrollToAnchoredEvent(items: items, proxy: proxy)
                        }
                    }
                    .onChange(of: state) { currentState in
                        let traceID = PerformanceTrace.begin("TimelineStateOnChange")
                        defer { PerformanceTrace.end("TimelineStateOnChange", id: traceID) }
                        guard case let .loaded(updatedItems, isPaginating) = currentState else {
                            return
                        }
                        if let anchorID = paginationScrollAnchorID, isPaginating == false {
                            paginationScrollAnchorID = nil
                            timelinePosition = .readingHistory
                            performTimelineScroll(
                                proxy: proxy,
                                target: .event(anchorID, anchor: .top),
                                animated: false,
                                reason: "pagination-anchor"
                            )
                            lastRenderedTimelineCount = updatedItems.count
                            return
                        }
                        if scrollToInitialPosition(items: updatedItems, proxy: proxy) {
                            return
                        }
                        // Explicit focused-event deep links only; never restore old read markers.
                        if scrollToAnchoredEvent(items: updatedItems, proxy: proxy) {
                            return
                        }
                        _ = scrollToLatestMessageIfNeeded(items: updatedItems, proxy: proxy)
                    }
                }
            }
        }
    }

    private func stableTimelineContent(items: [TimelineItem], isPaginating: Bool) -> some View {
        let rows = stableViewportRows(items: items, isPaginating: isPaginating)
        let routeID = stableViewportRouteID
        let generation = timelineBottomAnchorGeneration

        return StableTimelineViewport(
            routeID: routeID,
            sessionGeneration: generation,
            rows: rows,
            command: stableViewportCommand,
            isLive: timelineProviderIsLive,
            isPaginating: isPaginating,
            backgroundColor: SynaraThemeRamp.uiColor(
                isAgentRoom ? SynaraChrome.agentReviewToken : SynaraChrome.chatToken,
                baseHex: themeBaseHex,
                dark: conversationColorScheme == .dark
            ),
            rowContent: { row in
                AnyView(stableViewportRowContent(row))
            },
            onBottomPinnedChanged: { callbackRouteID, callbackGeneration, isPinned, newestEventID in
                guard callbackRouteID == stableViewportRouteID,
                      callbackGeneration == timelineBottomAnchorGeneration
                else {
                    return
                }
                handleStableBottomPinnedChanged(isPinned: isPinned, newestEventID: newestEventID)
            },
            onUserInteractionChanged: { callbackRouteID, callbackGeneration, isInteracting in
                guard callbackRouteID == stableViewportRouteID,
                      callbackGeneration == timelineBottomAnchorGeneration
                else {
                    return
                }
                handleStableUserInteractionChanged(isInteracting: isInteracting)
            },
            onPaginationThresholdReached: { callbackRouteID, callbackGeneration, anchorEventID, fillsShortContent in
                guard callbackRouteID == stableViewportRouteID,
                      callbackGeneration == timelineBottomAnchorGeneration,
                      let oldestEventID = loadedTimelineItems.first?.eventID
                else {
                    return false
                }
                return loadOlderTimeline(
                    before: oldestEventID,
                    scrollAnchorID: anchorEventID,
                    isFillingShortContent: fillsShortContent
                )
            },
            onTimestampRevealRequested: { callbackRouteID, callbackGeneration, eventID in
                guard callbackRouteID == stableViewportRouteID,
                      callbackGeneration == timelineBottomAnchorGeneration
                else {
                    return
                }
                revealTimestamp(for: eventID)
            },
            onCommandCompleted: { callbackRouteID, callbackGeneration, command, success, targetEventID in
                guard callbackRouteID == stableViewportRouteID,
                      callbackGeneration == timelineBottomAnchorGeneration
                else {
                    return
                }
                handleStableCommandCompleted(command, success: success, targetEventID: targetEventID)
            },
            lastReadMarkerEventID: pendingLastReadEventID,
            paginationCompletions: olderPaginationCompletions,
            onLastReadMarkerVisibilityChanged: { callbackRouteID, callbackGeneration, isVisible in
                guard callbackRouteID == stableViewportRouteID,
                      callbackGeneration == timelineBottomAnchorGeneration
                else {
                    return
                }
                isLastReadMarkerVisible = isVisible
            }
        )
        .id(routeID)
        .background(isAgentRoom ? SynaraChrome.agentReview : SynaraChrome.chat)
        .overlay(alignment: .topLeading) { timelineNavigationRecovery }
        .overlay(alignment: .bottomTrailing) {
            timelineJumpControls(
                items: items,
                showLatest: RoomTimelineJumpLatestPolicy.shouldShow(
                    isLive: timelineProviderIsLive,
                    isConfirmedPinned: isTimelineBottomVisible,
                    hasItems: items.isEmpty == false,
                    requested: showJumpToLatest
                ),
                onJumpToLatest: { jumpToLatestStable(currentItems: items) }
            )
        }
    }

    /// Bottom-trailing navigation stack shared by both timeline render paths:
    /// "Jump to Last Read" (when the unread frontier is off screen) sits above
    /// the "Jump to latest" control so both read as one cluster.
    @ViewBuilder
    private func timelineJumpControls(
        items: [TimelineItem],
        showLatest: Bool,
        onJumpToLatest: @escaping () -> Void
    ) -> some View {
        let showLastRead = RoomTimelineJumpLastReadPolicy.shouldShow(
            pendingMarkerEventID: pendingLastReadEventID,
            isMarkerVisible: StableScrollAnchoringFeatureFlag.isEnabled && isLastReadMarkerVisible
        )
        if showLastRead || showLatest {
            VStack(alignment: .trailing, spacing: SynaraSpacing.small) {
                if showLastRead, let marker = pendingLastReadEventID {
                    JumpToLastReadButton { jumpToLastRead(eventID: marker) }
                        .transition(.scale.combined(with: .opacity))
                }
                if showLatest {
                    JumpToLatestButton(isLoading: isJumpingToLatest, action: onJumpToLatest)
                        .transition(.scale.combined(with: .opacity))
                }
            }
            .padding(.trailing, SynaraSpacing.large)
            .padding(.bottom, SynaraSpacing.medium)
        }
    }

    /// Rows are hosted in UIKit cells (`UIHostingConfiguration`), outside this
    /// view's SwiftUI tree. Carry the app environment and theme in
    /// explicitly, as sheets do, so avatars, media and inline threads never
    /// fall back to `AppEnvironment.mock()`.
    private func stableViewportRowContent(_ row: StableTimelineViewportRow) -> some View {
        stableViewportRowBody(row)
            .environment(\.appEnvironment, environment)
            .environment(\.synaraThemeBaseHex, themeBaseHex)
    }

    @ViewBuilder
    private func stableViewportRowBody(_ row: StableTimelineViewportRow) -> some View {
        switch row.content {
        case .pagination:
            ProgressView()
                .controlSize(.small)
                .frame(maxWidth: .infinity)
                .padding(.vertical, SynaraSpacing.xSmall)
                .accessibilityIdentifier("TimelinePaginationIndicator")
        case let .cryptoBanner(status):
            CryptoRecoveryBanner(
                status: status,
                onRetry: retryDecryption,
                onReviewSecurity: { environment.router.route(to: .settings) },
                onDismiss: { isCryptoBannerDismissed = true }
            )
            .padding(.horizontal, timelineHorizontalPadding)
            .padding(.vertical, SynaraSpacing.xSmall)
        case let .dayHeader(day):
            TimelineDayHeader(day: day)
                .padding(.horizontal, timelineHorizontalPadding)
                .padding(.vertical, SynaraSpacing.small)
        case .unreadDivider:
            UnreadMessagesDivider()
                .padding(.horizontal, timelineHorizontalPadding)
                .padding(.vertical, SynaraSpacing.small)
        case let .event(eventRow):
            TimelineRow(
                item: eventRow.item,
                currentUserID: currentUserID,
                timeText: eventRow.timeText,
                isGroupedWithPrevious: eventRow.isGroupedWithPrevious,
                isTimestampRevealed: eventRow.isTimestampRevealed,
                animateSend: eventRow.animateSend,
                replyPreview: eventRow.replyPreview,
                replyCount: eventRow.replyCount,
                availability: eventRow.availability,
                onReply: { beginReply(eventRow.item) },
                onReplyInThread: { replyInThread(eventRow.item) },
                onOpenThread: { openThread(root: eventRow.item) },
                isInlineThreadMode: eventRow.isInlineThreadMode,
                isThreadExpanded: eventRow.isThreadExpanded,
                inlineThread: inlineThread(for: eventRow.item, replyCount: eventRow.replyCount),
                onEdit: { beginEdit(eventRow.item) },
                onRedact: { redactConfirmation.request(eventRow.item) },
                onToggleReaction: { key in toggleReaction(key, on: eventRow.item) },
                quickReactions: quickReactions,
                onReport: { beginReport(eventRow.item) },
                onForward: { forwardTarget = eventRow.item },
                onPollVote: { answerIDs in
                    await performAction(
                        .pollVote(answerIDs: answerIDs),
                        to: eventRow.item,
                        successMessage: String(localized: "Vote submitted.")
                    )
                },
                onDeclineCall: {
                    await performAction(
                        .declineCall,
                        to: eventRow.item,
                        successMessage: String(localized: "Call declined.")
                    )
                },
                onPinToNotes: { pinToNotes(eventRow.item) },
                onOpenMedia: { resource in viewerResource = resource },
                onAgentAction: { action in
                    executeAgentAction(action, sourceEventID: eventRow.item.eventID)
                },
                onAgentApprovalReaction: { actionIdentifier in
                    submitAgentApprovalDecision(actionIdentifier: actionIdentifier, sourceEventID: eventRow.item.eventID)
                },
                onRetryFailedSend: { retryFailedMessage(eventRow.item) },
                onDiscardFailedSend: { discardFailedMessage(eventRow.item) },
                reactorNames: eventRow.reactorNames,
                onShowReactions: { key in showReactions(for: eventRow.item, key: key) }
            )
            .padding(.horizontal, timelineHorizontalPadding)
            .padding(.bottom, SynaraSpacing.xSmall)
        }
    }

    private func stableViewportRows(items: [TimelineItem], isPaginating: Bool) -> [StableTimelineViewportRow] {
        // Read once per pass, not once per row.
        let userID = currentUserID
        let isInlineThreadMode = threadDisplay == .inline
        let replyCounts = TimelineReplyCounter.replyCounts(for: items)
        let previews = TimelineReplyPreview.previewsByEventID(in: items, currentUserID: userID)
        let reactorDirectory = TimelineReactorDirectory(items: items)
        var rows: [StableTimelineViewportRow] = []
        rows.reserveCapacity(items.count + 3)

        if isPaginating {
            rows.append(.init(id: .pagination, content: .pagination))
        }
        if shouldShowCryptoBanner(items: items) {
            rows.append(.init(id: .cryptoBanner, content: .cryptoBanner(cryptoStatus.observing(items))))
        }

        let dayHeaders = TimelineDayHeaderPolicy.headers(for: items)
        for (index, item) in items.enumerated() {
            let stableEventID = item.eventID.isEmpty ? item.id : item.eventID
            if let day = dayHeaders[item.eventID] {
                rows.append(
                    .init(id: .dayHeader(TimelineDayHeaderPolicy.key(for: day)), content: .dayHeader(day))
                )
            }
            if shouldShowUnreadDivider(before: item, at: index, in: items) {
                rows.append(.init(id: .unreadDivider(stableEventID), content: .unreadDivider))
            }
            rows.append(
                .init(
                    id: .event(stableEventID),
                    content: .event(
                        .init(
                            item: item,
                            isGroupedWithPrevious: isGroupedWithPrevious(index: index, items: items),
                            isTimestampRevealed: revealedTimestampEventID == stableEventID,
                            animateSend: sendAnimationItemIDs.contains(item.id),
                            replyPreview: TimelineRelationPresentation.replyPreview(
                                for: item,
                                locallyResolvedByEventID: previews
                            ),
                            replyCount: TimelineRelationPresentation.replyCount(
                                for: item,
                                locallyCountedByRootID: replyCounts
                            ),
                            availability: environment.eventActions.availability(
                                for: item,
                                currentUserID: userID,
                                roomID: roomID
                            ),
                            isInlineThreadMode: isInlineThreadMode,
                            isThreadExpanded: isThreadExpanded(item),
                            timeText: item.timestamp.timelineTime,
                            reactorNames: reactorDirectory.names(for: item)
                        )
                    )
                )
            )
        }
        return rows
    }

    private func revealTimestamp(for eventID: String) {
        timestampRevealTask?.cancel()
        timestampRevealGeneration &+= 1
        let generation = timestampRevealGeneration
        withAnimation(accessibilityReduceMotion ? nil : .easeOut(duration: 0.16)) {
            revealedTimestampEventID = eventID
        }
        timestampRevealTask = Task { @MainActor in
            do {
                try await Task.sleep(nanoseconds: RoomTimelineTimestampRevealPolicy.displayDurationNanoseconds)
            } catch {
                return
            }
            guard RoomTimelineTimestampRevealPolicy.taskMayDismiss(
                taskGeneration: generation,
                currentGeneration: timestampRevealGeneration,
                taskEventID: eventID,
                revealedEventID: revealedTimestampEventID,
                isCancelled: Task.isCancelled
            ) else {
                return
            }
            withAnimation(accessibilityReduceMotion ? nil : .easeInOut(duration: 0.16)) {
                revealedTimestampEventID = nil
            }
            timestampRevealTask = nil
        }
    }

    private func cancelTimestampReveal() {
        timestampRevealTask?.cancel()
        timestampRevealGeneration &+= 1
        timestampRevealTask = nil
        revealedTimestampEventID = nil
    }

    @discardableResult
    private func scrollToInitialPosition(items: [TimelineItem], proxy: ScrollViewProxy) -> Bool {
        guard hasPositionedInitialTimeline == false,
              items.last != nil
        else {
            return false
        }

        switch activeTimelineMode {
        case .live:
            hasPositionedInitialTimeline = true
            timelinePosition = .placingInitial
            placeInitialTimelineAtBottom(proxy: proxy)
            return true
        case let .unread(markerEventID):
            guard let markerIndex = items.firstIndex(where: {
                $0.eventID == markerEventID || $0.id == markerEventID
            }) else {
                return false
            }
            let targetIndex = min(markerIndex + 1, items.count - 1)
            hasPositionedInitialTimeline = true
            timelinePosition = .readingHistory
            showJumpToLatest = true
            performTimelineScroll(
                proxy: proxy,
                target: .event(items[targetIndex].eventID, anchor: .top),
                animated: false,
                reason: "initial-first-unread",
                ignoreComposerFocus: true
            )
            return true
        case .focused:
            return false
        }
    }

    private func placeInitialTimelineAtBottom(proxy: ScrollViewProxy) {
        performTimelineScroll(
            proxy: proxy,
            target: .bottom,
            animated: false,
            reason: "initial-live-end",
            ignoreComposerFocus: true
        )
    }

    @discardableResult
    private func scrollToAnchoredEvent(items: [TimelineItem], proxy: ScrollViewProxy) -> Bool {
        guard hasAnchoredEvent == false,
              let focusedEventID
        else {
            return false
        }

        guard let target = items.first(where: { item in
            item.eventID == focusedEventID || item.id == focusedEventID
        }) else {
            return false
        }

        hasAnchoredEvent = true
        hasPositionedInitialTimeline = true
        timelinePosition = .focusedEvent
        performTimelineScroll(
            proxy: proxy,
            target: .event(target.eventID, anchor: .center),
            animated: true,
            reason: "focused-event"
        )
        if let latest = items.last {
            showJumpToLatest = target.eventID != latest.eventID && target.id != latest.id
        }
        return true
    }

    @discardableResult
    private func scrollToLatestMessageIfNeeded(items: [TimelineItem], proxy: ScrollViewProxy) -> Bool {
        defer {
            lastRenderedTimelineCount = items.count
        }

        guard timelineProviderIsLive,
              lastRenderedTimelineCount > 0,
              items.count > lastRenderedTimelineCount,
              RoomTimelineScrollPolicy.shouldFollowLiveAppend(
                  position: timelinePosition,
                  isBottomVisible: isTimelineBottomVisible,
                  focusedEventID: timelineProviderFocusedEventID
              )
        else {
            return false
        }

        scrollToTimelineBottom(
            proxy: proxy,
            animated: true,
            ignoreComposerFocus: true,
            reason: "live-append"
        )
        return true
    }

    private func scrollToTimelineBottom(
        proxy: ScrollViewProxy,
        animated: Bool,
        ignoreComposerFocus: Bool = false,
        reason: String
    ) {
        performTimelineScroll(
            proxy: proxy,
            target: .bottom,
            animated: animated,
            reason: reason,
            ignoreComposerFocus: ignoreComposerFocus
        )
    }

    private func performTimelineScroll(
        proxy: ScrollViewProxy,
        target: TimelineScrollTarget,
        animated: Bool,
        reason: String,
        ignoreComposerFocus: Bool = false
    ) {
        cancelTimelineScroll(reason: "superseded")
        let targetKind: String
        switch target {
        case .bottom:
            targetKind = "bottom"
        case .event:
            targetKind = "event"
        }
        logTimelineEvent(
            "scroll-requested",
            fields: ["reason": reason, "target": targetKind, "animated": "\(animated)"]
        )
        timelineScrollTask = Task { @MainActor in
            await Task.yield()
            try? await Task.sleep(nanoseconds: Self.timelineBottomLayoutDelayNanoseconds)
            guard Task.isCancelled == false, ignoreComposerFocus || isComposerFocused == false else {
                return
            }

            let scroll = {
                switch target {
                case .bottom:
                    proxy.scrollTo(
                        "\(Self.timelineBottomAnchorID)-\(timelineBottomAnchorGeneration)",
                        anchor: .bottom
                    )
                case let .event(eventID, anchor):
                    proxy.scrollTo(eventID, anchor: anchor)
                }
            }
            if animated {
                withAnimation(.easeInOut(duration: 0.2)) {
                    scroll()
                }
            } else {
                scroll()
            }
            logTimelineEvent("scroll-executed", fields: ["reason": reason, "target": targetKind])
            timelineScrollTask = nil
        }
    }

    private func cancelTimelineScroll(reason: String = "cancelled") {
        if timelineScrollTask != nil {
            logTimelineEvent("scroll-cancelled", fields: ["reason": reason])
        }
        timelineScrollTask?.cancel()
        timelineScrollTask = nil
    }

    private func finishTimelineUserDrag() {
        Task { @MainActor in
            await Task.yield()
            try? await Task.sleep(nanoseconds: Self.timelineBottomLayoutDelayNanoseconds)
            isUserDraggingTimeline = false
            timelinePosition = RoomTimelineScrollPolicy.positionAfterUserDrag(
                isBottomVisible: isTimelineBottomVisible,
                focusedEventID: timelineProviderFocusedEventID
            )
            if timelinePosition == .readingHistory {
                showJumpToLatest = true
            } else if timelinePosition == .followingLive {
                showJumpToLatest = false
                if let latestEventID = loadedTimelineItems.reversed().compactMap(\.serverEventID).first {
                    scheduleMarkFullyRead(eventID: latestEventID)
                }
            }
        }
    }

    private func logTimelineEvent(_ name: String, fields: [String: String] = [:]) {
        let elapsedMilliseconds = max(0, Int(Date().timeIntervalSince(timelineTraceStartedAt) * 1000))
        let details = fields.keys.sorted().compactMap { key in
            fields[key].map { "\(key)=\($0)" }
        }.joined(separator: " ")
        let suffix = details.isEmpty ? "" : " \(details)"
        timelineLogger.info(
            "trace=\(timelineTraceID) elapsedMs=\(elapsedMilliseconds) event=\(name)\(suffix)",
            category: .timeline
        )
    }

    private func pinToNotes(_ item: TimelineItem) {
        Task {
            let result = await environment.roomNotes.pinMessage(roomID: roomID, item: item)
            await MainActor.run {
                switch result {
                case .success:
                    roomNotesActionMessage = String(localized: "Message pinned to your private notes.")
                case .failure(let error):
                    roomNotesActionMessage = error.errorDescription ?? String(localized: "Could not pin this message.")
                }
            }
        }
    }

    private var currentUserID: String {
        if case let .signedIn(session) = environment.session.currentState {
            return session.userID
        }
        return "@local:matrix.org"
    }

    private var timelineSubtitle: String {
        guard case let .loaded(items, _) = state else {
            return String(localized: "Matrix room")
        }

        let participantCount = Set(items.map(\.senderID)).count
        guard participantCount > 0 else {
            return String(localized: "Matrix room")
        }
        if SynaraLaunchEnvironment.isUITest,
           roomID == "!project:matrix.org"
        {
            return "21 members"
        }
        return String(localized: "\(participantCount) members")
    }

    /// Agent rooms read dark; every other room follows the system.
    private var conversationColorScheme: ColorScheme {
        isAgentRoom ? .dark : colorScheme
    }

    private var isAgentRoom: Bool {
        environment.roomList.isAgentRoom(roomID: roomID)
    }

    private var roomSummary: RoomSummary? {
        environment.roomList.roomSummary(roomID: roomID)
    }

    private var timelineHorizontalPadding: CGFloat {
        horizontalSizeClass == .compact ? SynaraSpacing.small : SynaraSpacing.medium
    }

    private var composerPlaceholder: String {
        if let editTarget = editSession?.editTarget {
            return editTarget.isLocalPending
                ? String(localized: "Edit unsent message...")
                : String(localized: "Edit message...")
        }
        return isAgentRoom
            ? String(localized: "Reply to the agent workflow...")
            : String(localized: "Send a message...")
    }

    private func isGroupedWithPrevious(index: Int, items: [TimelineItem]) -> Bool {
        guard items.indices.contains(index) else { return false }
        let previous = index > 0 ? items[index - 1] : nil
        return TimelineMessageGroupingPolicy.shouldGroup(previous: previous, current: items[index])
    }

    private var timelineTaskID: String {
        roomID + (focusedEventID ?? "")
    }

    private var stableViewportRouteID: String {
        "\(roomID)|\(timelineTraceID)"
    }

    private func resetTimelineState() {
        cryptoUpdatesTask?.cancel()
        cryptoUpdatesTask = nil
        stopTimelineUpdates(reason: "room-reset")
        stopTypingUpdates()
        cancelTimelineScroll()
        sendFollowLatestTask?.cancel()
        sendFollowLatestTask = nil
        cancelTimestampReveal()
        ownAvatarURL = nil
        loadedTimelineTaskID = nil
        state = .idle
        draft = environment.drafts.draft(roomID: roomID)
        replyTarget = nil
        editSession = nil
        sendError = nil
        timelineAvailability = RoomTimelineAvailabilityState()
        hasAnchoredEvent = false
        uploadState = .idle
        viewerResource = nil
        selectedPhotos = []
        attachmentDrafts = []
        attachmentSendTransaction = nil
        isSendingMessage = false
        agentActionMessage = nil
        cryptoStatus = .unknown
        cryptoActionMessage = nil
        isCryptoBannerDismissed = false
        isRoomDetailsPresented = false
        shouldReturnToListAfterDetailsDismiss = false
        lastRenderedTimelineCount = 0
        showJumpToLatest = false
        hasPositionedInitialTimeline = false
        unreadDividerEventID = nil
        isLastReadMarkerVisible = false
        lastReadJumpEventID = nil
        isLoadingNewerTimeline = false
        lastNewerPaginationAt = .distantPast
        hasReachedOldestMessages = false
        lastOlderPaginationAt = .distantPast
        paginationScrollAnchorID = nil
        isJumpingToLatest = false
        isComposerFocused = false
        isTimelineBottomVisible = false
        timelineBottomAnchorGeneration = 0
        lastMarkedFullyReadEventID = nil
        hasUserInteractedWithTimeline = false
        isUserDraggingTimeline = false
        typingUserIDs = []
        activeTimelineMode = focusedEventID.map { .focused(eventID: $0) } ?? .live
        timelineProviderIsLive = false
        timelinePosition = focusedEventID == nil ? .preparing : .focusedEvent
        if let timelineSession {
            Task {
                await timelineSession.invalidate()
            }
        }
        timelineSession = nil
        timelineTraceID = String(UUID().uuidString.prefix(8))
        timelineTraceStartedAt = Date()
        stableViewportCommand = nil
        cancelMarkFullyRead()
        logTimelineEvent(
            "room-open",
            fields: ["mode": focusedEventID == nil ? "live-end" : "focused-event"]
        )
    }

    private func shouldShowUnreadDivider(before item: TimelineItem, at index: Int, in items: [TimelineItem]) -> Bool {
        if SynaraLaunchEnvironment.isUITest,
           roomID == "!project:matrix.org",
           index > 0,
           item.eventID.contains("$security:")
        {
            return true
        }

        guard let unreadDividerEventID else {
            return false
        }
        return item.eventID == unreadDividerEventID
    }

    /// Fixes the divider for this visit at the first message after `marker`
    /// that someone else sent. Own messages are read by definition, so the
    /// divider never sits above the user's own message.
    private func placeUnreadDivider(markerEventID: String?) {
        unreadDividerEventID = RoomTimelineUnreadDividerPolicy.dividerEventID(
            markerEventID: markerEventID,
            items: loadedTimelineItems,
            currentUserID: currentUserID
        )
    }

    private func applyTimelineOutcome(_ outcome: TimelineLoadOutcome, isPaginating: Bool = false) {
        let traceID = PerformanceTrace.begin("TimelineOutcomeApply")
        defer { PerformanceTrace.end("TimelineOutcomeApply", id: traceID) }
        let shouldRemainPaginating = isPaginating || currentTimelineIsPaginating
        switch outcome {
        case let .loaded(items):
            timelineAvailability.recordSuccess()
            guard items.isEmpty == false else {
                if case .loaded = state {
                    state = .loaded(loadedTimelineItems, isPaginating: shouldRemainPaginating)
                    logTimelineEvent("snapshot-empty-ignored", fields: ["source": "loaded"])
                } else {
                    state = .empty
                    logTimelineEvent("snapshot-empty", fields: ["source": "loaded"])
                }
                return
            }
            let currentEventIDs = Set(loadedTimelineItems.map(\.eventID))
            let newlyObservedCount = items.reduce(into: 0) { count, item in
                if currentEventIDs.contains(item.eventID) == false {
                    count += 1
                }
            }
            let newestEventAgeMilliseconds = items.last.map { item in
                max(0, Int(Date().timeIntervalSince(item.timestamp) * 1000))
            } ?? 0
            let merged = mergeTimelineItems(items, isPaginating: shouldRemainPaginating)
            state = .loaded(merged, isPaginating: shouldRemainPaginating)
            logTimelineEvent(
                "snapshot-applied",
                fields: [
                    "incoming": "\(items.count)",
                    "newEvents": "\(newlyObservedCount)",
                    "newestAgeMs": "\(newestEventAgeMilliseconds)",
                    "rendered": "\(merged.count)",
                    "paginating": "\(shouldRemainPaginating)",
                ]
            )
        case .empty:
            timelineAvailability.recordSuccess()
            if case let .loaded(currentItems, _) = state,
               RoomTimelineSnapshotPolicy.shouldPreserveCurrentSnapshot(
                   currentItemCount: currentItems.count,
                   incomingItemCount: 0
               )
            {
                state = .loaded(currentItems, isPaginating: shouldRemainPaginating)
                logTimelineEvent("snapshot-empty-ignored", fields: ["source": "empty"])
            } else if let pendingItems = localPendingItems, pendingItems.isEmpty == false {
                state = .loaded(pendingItems, isPaginating: shouldRemainPaginating)
            } else {
                state = .empty
                logTimelineEvent("snapshot-empty", fields: ["source": "empty"])
            }
        case let .failed(failure):
            if case let .loaded(currentItems, _) = state, currentItems.isEmpty == false {
                state = .loaded(currentItems, isPaginating: shouldRemainPaginating)
                timelineAvailability.recordFailure(failure, preservingRows: true)
                logTimelineEvent(
                    "snapshot-failed-preserved",
                    fields: [
                        "code": failure.diagnosticCode,
                        "rendered": "\(currentItems.count)",
                    ]
                )
            } else {
                timelineAvailability.recordFailure(failure, preservingRows: false)
                state = .failed(failure.userMessage)
                logTimelineEvent("snapshot-failed", fields: ["code": failure.diagnosticCode])
            }
        }
    }

    private var localPendingItems: [TimelineItem]? {
        let fromState: [TimelineItem]
        if case let .loaded(items, _) = state {
            fromState = TimelinePendingReconciler.pendingItems(from: items)
        } else {
            fromState = []
        }
        let combined = TimelinePendingReconciler.combining(
            localItems: fromState,
            storedPending: environment.outgoingSends.queue.timelineItems(in: roomID, threadRootEventID: nil)
        )
        return combined.isEmpty ? nil : combined
    }

    private var currentTimelineIsPaginating: Bool {
        guard case let .loaded(_, isPaginating) = state else {
            return false
        }
        return isPaginating
    }

    private func mergeTimelineItems(_ streamItems: [TimelineItem], isPaginating _: Bool) -> [TimelineItem] {
        let localItems: [TimelineItem]
        if case let .loaded(items, _) = state {
            localItems = items
        } else {
            localItems = []
        }

        let storedPending = environment.outgoingSends.queue.timelineItems(in: roomID, threadRootEventID: nil)
        environment.outgoingSends.dropConfirmed(matching: streamItems)
        return TimelinePendingReconciler.merge(
            streamItems: streamItems,
            localItems: TimelinePendingReconciler.combining(
                localItems: localItems,
                storedPending: storedPending
            ),
            currentUserID: currentUserID
        )
    }

    /// The thread chip and "Open Thread": inline mode expands or collapses the
    /// replies beneath the root; the other modes present the thread.
    private func openThread(root item: TimelineItem) {
        if threadDisplay == .inline {
            if expandedThreadRoots.contains(item.eventID) {
                expandedThreadRoots.remove(item.eventID)
            } else {
                expandedThreadRoots.insert(item.eventID)
            }
            return
        }
        presentThread(rootEventID: item.eventID, rootTitle: item.threadTitle)
    }

    /// Opens the full thread with its composer. Inline mode uses a sheet, the
    /// way desktop inline threads reply in the side panel.
    private func presentThread(rootEventID: String, rootTitle: String?, focusedEventID: String? = nil) {
        switch threadDisplay {
        case .inline, .sheet:
            threadSheet = ThreadSheetTarget(
                rootEventID: rootEventID,
                rootTitle: rootTitle,
                focusedEventID: focusedEventID
            )
        case .full:
            environment.router.route(
                to: .thread(
                    roomID: roomID,
                    rootEventID: rootEventID,
                    roomTitle: displayRoomTitle,
                    rootTitle: rootTitle,
                    focusedEventID: focusedEventID
                )
            )
        }
    }

    private func replyInThread(_ item: TimelineItem) {
        let rootEventID = item.threadRootEventID ?? item.eventID
        ThreadComposerFocusRequests.shared.request(roomID: roomID, rootEventID: rootEventID)
        presentThread(
            rootEventID: rootEventID,
            rootTitle: rootEventID == item.eventID ? item.threadTitle : nil
        )
    }

    private func isThreadExpanded(_ item: TimelineItem) -> Bool {
        threadDisplay == .inline && expandedThreadRoots.contains(item.eventID)
    }

    private func inlineThread(for item: TimelineItem, replyCount: Int) -> AnyView? {
        guard isThreadExpanded(item), replyCount > 0 else {
            return nil
        }
        return AnyView(
            InlineThreadReplies(
                roomID: roomID,
                rootEventID: item.eventID,
                replyCount: replyCount,
                onReplyInThread: { replyInThread(item) },
                onOpenThread: { presentThread(rootEventID: item.eventID, rootTitle: item.threadTitle) }
            )
        )
    }

    private func prepareTimelineUpdates() async {
        let preservesCurrentSnapshot: Bool
        if case .loaded = state {
            preservesCurrentSnapshot = true
            logTimelineEvent("initial-load-refresh-preserved")
        } else {
            preservesCurrentSnapshot = false
            state = .loading
        }
        if preservesCurrentSnapshot == false {
            showJumpToLatest = false
            hasPositionedInitialTimeline = false
            unreadDividerEventID = nil
            timelinePosition = focusedEventID == nil ? .preparing : .focusedEvent
        }
        logTimelineEvent("initial-load-started")
    }

    private func loadTimeline() async {
        await prepareTimelineUpdates()
        stopTimelineUpdates(reason: "session-open")

        let session = RoomTimelineSession(roomID: roomID, service: environment.timeline)
        let previousSession = timelineSession
        await MainActor.run {
            timelineSession = session
        }
        if let previousSession {
            await previousSession.invalidate()
        }

        let feed: RoomTimelineSessionFeed
        var routedThreadRootEventID: String?
        var routeNotice: String?
        if let focusedEventID, focusedEventID.isEmpty == false {
            guard let liveFeed = await session.open(mode: .live) else {
                return
            }
            // Ask Core where the event lives before choosing a provider.
            let route = await environment.timeline.resolveEventRoute(roomID: roomID, eventID: focusedEventID)
            switch RoomTimelineFocusPolicy.eventRouteDecision(
                focusedEventID: focusedEventID,
                route: route,
                liveItems: timelineItems(from: liveFeed.initialOutcome)
            ) {
            case let .room(liveMode?):
                logTimelineEvent("event-route-provider", fields: ["provider": "live"])
                feed = liveFeed.presenting(mode: liveMode)
            case .room(.none):
                guard let focusedFeed = await session.open(mode: .focused(eventID: focusedEventID)) else {
                    return
                }
                feed = focusedFeed
            case let .thread(rootEventID, roomMode):
                logTimelineEvent("event-route-provider", fields: ["provider": "live", "route": "thread"])
                feed = liveFeed.presenting(mode: roomMode)
                routedThreadRootEventID = rootEventID
            case let .unavailable(notice):
                logTimelineEvent("event-route-provider", fields: ["provider": "live", "route": "unavailable"])
                feed = liveFeed
                routeNotice = notice
            }
        } else {
            guard let liveFeed = await session.open(mode: .live) else {
                return
            }
            let hasUnreadMessages = environment.roomList.hasUnreadMessages(roomID: roomID)
            let fullyReadEventID = await environment.readMarkers.fullyReadEventID(roomID: roomID)
            let initialItems = timelineItems(from: liveFeed.initialOutcome)
            pendingLastReadEventID = RoomTimelineJumpLastReadPolicy.pendingMarker(
                fullyReadEventID: fullyReadEventID,
                initialEventIDs: initialItems.compactMap(\.serverEventID),
                hasUnreadMessages: hasUnreadMessages
            )
            let initialMode = RoomTimelineFocusPolicy.initialMode(
                focusedEventID: nil,
                hasUnreadMessages: hasUnreadMessages,
                fullyReadEventID: fullyReadEventID,
                liveItems: timelineItems(from: liveFeed.initialOutcome)
            )
            // The unread marker is already inside this bounded live snapshot.
            // Reuse its provider and stream so the viewport does not remount.
            feed = liveFeed.presenting(mode: initialMode)
        }

        guard Task.isCancelled == false,
              await MainActor.run(body: { timelineSession === session })
        else {
            await session.invalidate()
            return
        }

        let routedThread = routedThreadRootEventID
        let notice = routeNotice
        await MainActor.run {
            applySessionFeed(feed)
            loadedTimelineTaskID = timelineTaskID
            // A retry reloads the room; the routed thread or notice is shown once.
            guard let focusedEventID, handledEventRouteID != focusedEventID else {
                return
            }
            handledEventRouteID = focusedEventID
            if let notice {
                eventRouteNotice = notice
            }
            if let routedThread {
                presentThread(rootEventID: routedThread, rootTitle: nil, focusedEventID: focusedEventID)
            }
        }
    }

    private func timelineItems(from outcome: TimelineLoadOutcome) -> [TimelineItem] {
        guard case let .loaded(items) = outcome else {
            return []
        }
        return items
    }

    private func applySessionFeed(_ feed: RoomTimelineSessionFeed) {
        stopTimelineUpdates(reason: "stream-replaced")
        cancelMarkFullyRead()
        isTimelineBottomVisible = false
        timelineBottomAnchorGeneration = feed.generation
        activeTimelineMode = feed.mode
        timelineProviderIsLive = feed.providerIsLive
        applyTimelineOutcome(feed.initialOutcome)
        // A provider opened at the unread marker places the divider; any
        // other provider (jump to latest, a later page) keeps it in place.
        if case let .unread(markerEventID) = feed.mode {
            placeUnreadDivider(markerEventID: markerEventID)
        }
        if StableScrollAnchoringFeatureFlag.isEnabled {
            enqueueStableInitialCommand(for: feed.mode, generation: feed.generation)
        }
        logTimelineEvent(
            "stream-started",
            fields: ["mode": feed.mode.isLive ? "live" : "focused", "generation": "\(feed.generation)"]
        )
        isTimelineStreamActive = true
        let streamGeneration = feed.generation
        timelineUpdatesTask = Task {
            for await outcome in feed.updates {
                guard Task.isCancelled == false else {
                    return
                }
                await MainActor.run {
                    switch outcome {
                    case let .loaded(items):
                        applyTimelineOutcome(.loaded(items))
                        hasReachedOldestMessages = RoomTimelinePaginationPolicy.reachedOldestAfterUpdate(
                            hasReachedOldestMessages: hasReachedOldestMessages,
                            backwardPagination: currentBackwardPagination
                        )
                    case .empty:
                        timelineAvailability.recordSuccess()
                        if case .loading = state {
                            state = .empty
                            logTimelineEvent("stream-empty-initial")
                        } else {
                            logTimelineEvent("stream-empty-ignored")
                        }
                    case let .failed(failure):
                        if case let .loaded(items, _) = state,
                           RoomTimelineFailurePresentationPolicy.retryMessage(
                               for: failure,
                               preservedItemCount: items.count
                           ) != nil
                        {
                            timelineAvailability.recordFailure(failure, preservingRows: true)
                            logTimelineEvent(
                                "stream-failed-preserved",
                                fields: ["code": failure.diagnosticCode]
                            )
                            return
                        }
                        timelineAvailability.recordFailure(failure, preservingRows: false)
                        state = .failed(failure.userMessage)
                        logTimelineEvent("stream-failed", fields: ["code": failure.diagnosticCode])
                    }
                }
            }
            if Task.isCancelled == false {
                await MainActor.run {
                    logTimelineEvent("stream-ended")
                    if timelineBottomAnchorGeneration == streamGeneration {
                        isTimelineStreamActive = false
                    }
                }
            }
        }
    }

    private func stopTimelineUpdates(reason: String) {
        if timelineUpdatesTask != nil {
            logTimelineEvent("stream-stopped", fields: ["reason": reason])
        }
        timelineUpdatesTask?.cancel()
        timelineUpdatesTask = nil
        isTimelineStreamActive = false
    }

    private var hasPresentedTimeline: Bool {
        switch state {
        case .loaded, .empty:
            return true
        case .idle, .loading, .failed:
            return false
        }
    }

    private var hasFailedTimelineLoad: Bool {
        if case .failed = state {
            return true
        }
        return false
    }

    private func retryFailedTimelineLoad(reason: String) {
        guard hasFailedTimelineLoad else {
            return
        }
        logTimelineEvent("timeline-auto-retry", fields: ["reason": reason])
        state = .loading
        Task {
            await loadTimeline()
        }
    }

    /// The room is back on screen with its window and stream intact: restart
    /// only what stopped when it disappeared.
    private func resumeTimelineAfterReappear() {
        logTimelineEvent("room-reappeared", fields: ["items": "\(loadedTimelineItems.count)"])
        startTypingUpdates()
        startVerificationAutoRetry()
        applyOutgoingQueueToTimeline()
        if timelineProviderIsLive,
           isTimelineBottomVisible,
           let newestEventID = loadedTimelineItems.reversed().compactMap(\.serverEventID).first
        {
            scheduleMarkFullyRead(eventID: newestEventID)
        }
    }

    private func startTypingUpdates() {
        stopTypingUpdates()
        typingUpdatesTask = Task {
            for await userIDs in environment.timeline.typingUsers(roomID: roomID) {
                guard Task.isCancelled == false else {
                    return
                }
                let visibleUserIDs = Array(Set(userIDs.filter { $0 != currentUserID })).sorted()
                await MainActor.run {
                    typingUserIDs = visibleUserIDs
                }
            }
        }
    }

    private func stopTypingUpdates() {
        typingUpdatesTask?.cancel()
        typingUpdatesTask = nil
        typingUserIDs = []
    }

    /// Room status over the shared session status: `session` from a store
    /// update, or the store's (recent or single-flight) read.
    private func loadCryptoStatus(session: SessionCryptoStatus? = nil) async -> RoomCryptoStatus {
        let traceID = PerformanceTrace.begin("LoadCryptoStatus")
        defer { PerformanceTrace.end("LoadCryptoStatus", id: traceID) }
        let expectedTimelineTaskID = timelineTaskID
        let sessionStatus: SessionCryptoStatus
        if let session {
            sessionStatus = session
        } else {
            sessionStatus = await environment.sessionCrypto.read()
        }
        let status = await environment.crypto.roomStatus(roomID: roomID, session: sessionStatus)
        await MainActor.run {
            guard !Task.isCancelled, expectedTimelineTaskID == timelineTaskID else { return }
            cryptoStatus = status
        }
        return status
    }

    private func loadOwnAvatarURL(
        expectedTimelineTaskID: String,
        expectedUserID: String
    ) async {
        let profile = await environment.matrix.ownProfile()
        guard Task.isCancelled == false else {
            return
        }
        let resolvedAvatarURL = SharedCoreTimelineRows.senderAvatarURL(profile?.avatarURL)
        await MainActor.run {
            guard RoomTimelineOwnAvatarPolicy.mayInstall(
                profileUserID: profile?.userID,
                expectedUserID: expectedUserID,
                expectedTimelineTaskID: expectedTimelineTaskID,
                currentTimelineTaskID: timelineTaskID,
                isCancelled: Task.isCancelled
            ), currentUserID == expectedUserID
            else {
                return
            }
            ownAvatarURL = resolvedAvatarURL
            if let resolvedAvatarURL {
                environment.outgoingSends.hydrateSenderAvatarURL(
                    senderID: expectedUserID,
                    avatarURL: resolvedAvatarURL
                )
                applyOutgoingQueueToTimeline()
            }
        }
    }

    private func startVerificationAutoRetry() {
        cryptoUpdatesTask?.cancel()
        cryptoUpdatesTask = nil
        guard RoomCryptoObservationPolicy.needsSessionUpdates(cryptoStatus.observing(loadedTimelineItems)) else {
            return
        }
        let expectedTimelineTaskID = timelineTaskID
        cryptoUpdatesTask = Task {
            // The shared store's updates carry verification and recovery
            // changes. Subscribe before reading so completion cannot be lost
            // during a read; the store publishes only real changes.
            let updates = environment.sessionCrypto.updates()
            for await session in updates {
                guard !Task.isCancelled, expectedTimelineTaskID == timelineTaskID else { return }
                let previousStatus = cryptoStatus
                let status = await loadCryptoStatus(session: session)
                guard !Task.isCancelled, expectedTimelineTaskID == timelineTaskID else { return }
                if status != previousStatus { isCryptoBannerDismissed = false }
                let observed = status.observing(loadedTimelineItems)
                if observed.shouldRetryAfterVerification(previous: previousStatus) {
                    _ = await environment.crypto.retryDecryption(roomID: roomID)
                }
                // Retrying after verification is the purpose; once verified,
                // stop holding the session-wide subscription for this room.
                guard RoomCryptoObservationPolicy.needsSessionUpdates(observed) else { return }
            }
        }
    }

    private func retryDecryption() {
        Task {
            let result = await environment.crypto.retryDecryption(roomID: roomID)
            _ = await loadCryptoStatus()
            await MainActor.run {
                cryptoActionMessage = result.message
            }
        }
    }

    private func shouldShowCryptoBanner(items: [TimelineItem]) -> Bool {
        !isCryptoBannerDismissed && cryptoStatus.observing(items).needsCryptoActionBanner
    }

    private func sendMessage(body rawBody: String) {
        let trimmed = rawBody.trimmingCharacters(in: .whitespacesAndNewlines)
        let drafts = attachmentDrafts
        guard ComposerAttachmentDraftList.canBeginSend(
            isSending: isSendingMessage,
            text: trimmed,
            drafts: drafts
        ) else {
            if isSendingMessage == false {
                sendError = MessageSendError.emptyMessage.localizedDescription
            }
            return
        }

        let proposedIntent = ComposerEditFlow.sendIntent(
            body: rawBody,
            replyToEventID: replyTarget?.eventID,
            threadRootEventID: replyTarget?.threadRootEventID,
            session: editSession
        )
        let transaction = ComposerAttachmentSendTransaction.reusableOrNew(
            existing: attachmentSendTransaction,
            drafts: drafts,
            body: rawBody,
            proposedIntent: proposedIntent
        )
        let composerIntent = transaction.intent
        guard composerIntent.requiresStandaloneTextSend == false
            || trimmed.isEmpty == false
        else {
            sendError = MessageSendError.emptyMessage.localizedDescription
            return
        }

        sendError = nil
        isSendingMessage = true
        // Reply/edit intent belongs to the send gesture, not to mutable composer
        // state after an asynchronous upload has started. A partial retry also
        // retains this identity unless the user explicitly changes/cancels the
        // relation, which starts a new transaction.
        let plan = transaction.steps
        if drafts.isEmpty {
            attachmentSendTransaction = nil
            Task {
                await performSend(composerIntent, sentText: rawBody)
                await MainActor.run {
                    isSendingMessage = false
                }
            }
            return
        }
        attachmentSendTransaction = transaction

        Task {
            let signpostID = PerformanceTrace.begin("ComposerAttachmentDraftSend")
            defer {
                PerformanceTrace.end("ComposerAttachmentDraftSend", id: signpostID)
            }
            let uploaded = await ComposerAttachmentSend.uploadAll(
                drafts,
                steps: plan,
                roomID: roomID,
                replyToEventID: composerIntent.replyToEventID,
                threadRootEventID: composerIntent.threadRootEventID,
                uploader: environment.mediaUploader,
                onState: { state in
                    uploadState = state
                },
                onUploaded: { draft, item in
                    attachmentDrafts = ComposerAttachmentDraftList.remove(id: draft.id, from: attachmentDrafts)
                    if let activeTransaction = attachmentSendTransaction {
                        attachmentSendTransaction = activeTransaction.removingAttachment(id: draft.id)
                    }
                    appendAcknowledgedUpload(item)
                }
            )
            if uploaded {
                await MainActor.run {
                    attachmentSendTransaction = nil
                }
                if let trailingText = ComposerAttachmentSendPlan.trailingText(in: plan) {
                    await performSend(composerIntent.replacingBody(with: trailingText), sentText: rawBody)
                } else {
                    await MainActor.run {
                        uploadState = .idle
                        finishComposerSend(sentText: rawBody, persisting: true)
                        completeComposerRelation()
                        if StableScrollAnchoringFeatureFlag.isEnabled {
                            followLatestAfterSend()
                        } else {
                            sendLatestRequest &+= 1
                        }
                    }
                }
            }
            await MainActor.run {
                isSendingMessage = false
            }
        }
    }

    private func retryFailedMessage(_ item: TimelineItem) {
        guard item.isLocalPending else {
            // An SDK local echo: Core retries the same request (unwedge or
            // wake the room queue); it never sends a second event.
            sendError = nil
            Task {
                let retried = await environment.outgoingSends.retryLocalEcho(item, roomID: roomID)
                await MainActor.run {
                    if retried == false {
                        sendError = MessageSendError.failed.localizedDescription
                        SynaraHaptics.trigger(.warning)
                    }
                }
            }
            return
        }
        guard let queued = environment.outgoingSends.retry(
            item,
            roomID: roomID,
            senderID: currentUserID
        ) else {
            return
        }

        registerSendAnimation(for: queued.id, isRetry: true)
        sendError = nil
        applyOutgoingQueueToTimeline()
        Task {
            await transmitOutgoing(id: queued.id)
        }
    }

    /// Discard an unsent own message: a pending row Core never accepted, or
    /// an SDK local echo (Core aborts it in the send queue).
    private func discardFailedMessage(_ item: TimelineItem) {
        sendError = nil
        Task {
            let discarded = await environment.outgoingSends.discard(item, roomID: roomID)
            await MainActor.run {
                switch discarded {
                case .some(true):
                    removeDiscardedRow(item)
                case .some(false):
                    // Already sent: the row becomes the sent message.
                    break
                case .none:
                    sendError = String(localized: "The unsent message could not be discarded.")
                    SynaraHaptics.trigger(.warning)
                }
            }
        }
    }

    /// The SDK removes a discarded echo with its next timeline update; drop
    /// the row now so it does not linger as Failed.
    private func removeDiscardedRow(_ item: TimelineItem) {
        guard case let .loaded(items, isPaginating) = state else {
            return
        }
        state = .loaded(items.filter { $0.id != item.id }, isPaginating: isPaginating)
    }

    /// `sentText` is the composer text when the send began; only that is
    /// cleared once the send is accepted.
    @MainActor
    private func performSend(
        body rawBody: String,
        replyToEventID: String?,
        threadRootEventID: String?,
        editEventID: String?,
        retrying failedItem: TimelineItem? = nil,
        sentText: String
    ) async {
        let draftBody = rawBody.trimmingCharacters(in: .whitespacesAndNewlines)
        guard draftBody.isEmpty == false else {
            sendError = MessageSendError.emptyMessage.localizedDescription
            return
        }
        let formattedBody = ComposerMatrixFormatting.formattedBody(for: draftBody)
        let body = ComposerMatrixFormatting.plainBody(for: draftBody, formattedBody: formattedBody)

        let request = MessageSendRequest(
            roomID: roomID,
            body: body,
            formattedBody: formattedBody,
            replyToEventID: replyToEventID,
            editEventID: editEventID,
            threadRootEventID: threadRootEventID
        )
        let isEditing = request.editEventID != nil

        if isEditing {
            do {
                let signpostID = PerformanceTrace.begin("MessageSend")
                defer {
                    PerformanceTrace.end("MessageSend", id: signpostID)
                }
                let item = try await environment.messageSender.send(request)
                replace(item)
                finishComposerSend(sentText: sentText, persisting: true)
                completeComposerRelation()
                sendError = nil
            } catch {
                sendError = MessageSendError.failed.localizedDescription
                SynaraHaptics.trigger(.warning)
            }
            return
        }

        // The message is written to the durable outbox and the draft cleared
        // in one write, then handed to Core at once (whatever the sync state).
        let clientTransactionID = OutgoingQueuedMessage.newClientTransactionID()
        let queued = environment.outgoingSends.enqueue(
            localID: failedItem?.id ?? "$pending-\(clientTransactionID)",
            clientTransactionID: clientTransactionID,
            roomID: roomID,
            body: body,
            formattedBody: request.formattedBody,
            replyToEventID: replyToEventID,
            threadRootEventID: threadRootEventID,
            senderID: currentUserID,
            senderAvatarURL: ownAvatarURL,
            timestamp: failedItem?.timestamp ?? Date(),
            clearingDraft: true
        )
        registerSendAnimation(for: queued.id, isRetry: failedItem != nil)
        applyOutgoingQueueToTimeline()
        if StableScrollAnchoringFeatureFlag.isEnabled {
            followLatestAfterSend()
        } else {
            sendLatestRequest &+= 1
        }

        // The outbox write above already cleared the stored draft.
        finishComposerSend(sentText: sentText, persisting: false)
        completeComposerRelation()
        sendError = nil

        Task {
            await transmitOutgoing(id: queued.id)
        }
    }

    private func transmitOutgoing(id: String) async {
        let signpostID = PerformanceTrace.begin("MessageSend")
        defer {
            PerformanceTrace.end("MessageSend", id: signpostID)
        }
        await environment.outgoingSends.flush()
        await MainActor.run {
            applyOutgoingQueueToTimeline()
            guard let status = environment.outgoingSends.queue.item(id: id)?.deliveryStatus else {
                return
            }
            switch status {
            case .sent:
                sendError = nil
                SynaraHaptics.trigger(.lightImpact)
                if SynaraLaunchEnvironment.isUITest,
                   case let .loaded(items, _) = state,
                   let confirmed = items.first(where: { $0.id == id })
                {
                    reconcilePendingSend(localID: id, confirmed: confirmed.withDeliveryStatus(nil))
                }
            case .failed:
                SynaraHaptics.trigger(.warning)
            case .queued, .sending:
                sendError = nil
            }
        }
    }

    private func applyOutgoingQueueToTimeline() {
        let pendingItems = environment.outgoingSends.queue.timelineItems(in: roomID, threadRootEventID: nil)
        switch OutgoingQueueTimelineMerge.applying(
            pendingItems: pendingItems,
            to: outgoingQueuePresentation
        ) {
        case let .loaded(items, isPaginating):
            state = .loaded(items, isPaginating: isPaginating)
        case .idle, .loading, .empty, .failed:
            break
        }
    }

    private var outgoingQueuePresentation: OutgoingQueueTimelineMerge.Presentation {
        switch state {
        case .idle:
            return .idle
        case .loading:
            return .loading
        case .empty:
            return .empty
        case .failed(_):
            return .failed
        case let .loaded(items, isPaginating):
            return .loaded(items, isPaginating: isPaginating)
        }
    }

    private func reconcilePendingSend(localID: String, confirmed: TimelineItem) {
        guard case let .loaded(items, isPaginating) = state else {
            return
        }

        let withoutPending = items.filter { $0.id != localID }
        if withoutPending.contains(where: { $0.eventID == confirmed.eventID }) {
            state = .loaded(withoutPending, isPaginating: isPaginating)
            return
        }

        state = .loaded(withoutPending + [confirmed], isPaginating: isPaginating)
    }

    private func draftMockMedia(source: MediaUploadSource) {
        let draft: ComposerAttachmentDraft
        switch source {
        case .photoLibrary:
            draft = ComposerAttachmentDraft(
                displayName: "synara-upload.jpg",
                mimeType: "image/jpeg",
                data: Data("Synara test image".utf8),
                source: source
            )
        case .file:
            draft = ComposerAttachmentDraft(
                displayName: "synara-upload.pdf",
                mimeType: "application/pdf",
                data: Data("Synara test file".utf8),
                source: source
            )
        case .camera:
            draft = ComposerAttachmentDraft(
                displayName: "synara-camera.jpg",
                mimeType: "image/jpeg",
                data: Data("Synara test camera image".utf8),
                source: source
            )
        }
        applyIncomingDrafts([draft])
    }

    private func draftPickedFile(_ url: URL) {
        switch ComposerAttachmentDraftList.draft(fromFileURL: url) {
        case let .success(draft):
            applyIncomingDrafts([draft])
        case let .failure(rejection):
            uploadState = .failed(ComposerAttachmentDraftList.userMessage(for: rejection))
        }
    }

    #if canImport(UIKit)
        private func draftCameraImage(_ image: UIImage) {
            guard let data = MediaAttachmentSupport.jpegData(from: image) else {
                uploadState = .failed(String(localized: "Attachment could not be loaded. Try again."))
                return
            }
            applyIncomingDrafts([
                ComposerAttachmentDraft(
                    displayName: "synara-camera.jpg",
                    mimeType: "image/jpeg",
                    data: data,
                    source: .camera
                )
            ])
        }

        private func draftPastedImages(_ images: [UIImage]) {
            var incoming: [ComposerAttachmentDraft] = []
            incoming.reserveCapacity(images.count)
            for (index, image) in images.enumerated() {
                guard let data = MediaAttachmentSupport.jpegData(from: image) else {
                    continue
                }
                let displayName = images.count == 1 ? "synara-paste.jpg" : "synara-paste-\(index + 1).jpg"
                incoming.append(
                    ComposerAttachmentDraft(
                        displayName: displayName,
                        mimeType: "image/jpeg",
                        data: data,
                        source: .photoLibrary
                    )
                )
            }
            if incoming.isEmpty {
                uploadState = .failed(String(localized: "Attachment could not be loaded. Try again."))
                return
            }
            applyIncomingDrafts(incoming)
        }
    #endif

    private func draftPickedPhotos(_ items: [PhotosPickerItem]) {
        guard items.isEmpty == false else {
            return
        }
        Task {
            var incoming: [ComposerAttachmentDraft] = []
            var loadFailed = false
            incoming.reserveCapacity(items.count)
            for item in items {
                do {
                    guard let data = try await item.loadTransferable(type: Data.self), data.isEmpty == false else {
                        loadFailed = true
                        continue
                    }
                    let contentType = item.supportedContentTypes.first
                    incoming.append(
                        ComposerAttachmentDraft(
                            displayName: "synara-photo.\(contentType?.preferredFilenameExtension ?? "jpg")",
                            mimeType: contentType?.preferredMIMEType ?? "image/jpeg",
                            data: data,
                            source: .photoLibrary
                        )
                    )
                } catch {
                    loadFailed = true
                }
            }
            await MainActor.run {
                selectedPhotos = []
                applyIncomingDrafts(incoming)
                if incoming.isEmpty, loadFailed {
                    uploadState = .failed(String(localized: "Attachment could not be loaded. Try again."))
                }
            }
        }
    }

    private func applyIncomingDrafts(_ incoming: [ComposerAttachmentDraft]) {
        guard isSendingMessage == false else {
            return
        }
        let outcome = ComposerAttachmentDraftList.appending(incoming, to: attachmentDrafts)
        attachmentDrafts = outcome.drafts
        if let rejection = outcome.rejection {
            uploadState = .failed(ComposerAttachmentDraftList.userMessage(for: rejection))
        } else if incoming.isEmpty == false {
            uploadState = .idle
        }
    }

    private func loadOlderTimelineIfNeeded(anchorItem: TimelineItem, index: Int, items: [TimelineItem]) {
        let isPaginating: Bool
        if case let .loaded(_, currentIsPaginating) = state {
            isPaginating = currentIsPaginating
        } else {
            isPaginating = false
        }

        guard RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
            rowIndex: index,
            topThreshold: Self.olderPaginationTopThreshold,
            hasUserInteractedWithTimeline: hasUserInteractedWithTimeline,
            hasPositionedInitialTimeline: hasPositionedInitialTimeline,
            isJumpingToLatest: isJumpingToLatest,
            isPaginating: isPaginating,
            hasReachedOldestMessages: hasReachedOldestMessages
        ),
            let oldestEventID = items.first?.eventID
        else {
            return
        }
        loadOlderTimeline(before: oldestEventID, scrollAnchorID: anchorItem.eventID)
    }

    @discardableResult
    private func loadOlderTimeline(
        before eventID: String?,
        scrollAnchorID: String? = nil,
        isFillingShortContent: Bool = false
    ) -> Bool {
        guard let eventID,
              hasReachedOldestMessages == false,
              case .loaded(let items, false) = state
        else {
            return false
        }

        guard RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
            rowIndex: 0,
            topThreshold: Self.olderPaginationTopThreshold,
            hasUserInteractedWithTimeline: hasUserInteractedWithTimeline,
            hasPositionedInitialTimeline: hasPositionedInitialTimeline,
            isJumpingToLatest: isJumpingToLatest,
            isPaginating: false,
            hasReachedOldestMessages: hasReachedOldestMessages,
            isFillingShortContent: isFillingShortContent,
            backwardPagination: currentBackwardPagination
        ) else {
            return false
        }

        let now = Date()
        guard now.timeIntervalSince(lastOlderPaginationAt) >= Self.olderPaginationDebounceInterval else {
            return false
        }
        lastOlderPaginationAt = now

        if let scrollAnchorID {
            paginationScrollAnchorID = scrollAnchorID
        }

        // Filling short content is not the user leaving the live tail.
        if isFillingShortContent == false {
            timelinePosition = .readingHistory
        }
        logTimelineEvent(
            "pagination-started",
            fields: ["rendered": "\(items.count)", "fill": "\(isFillingShortContent)"]
        )
        state = .loaded(items, isPaginating: true)
        Task {
            let signpostID = PerformanceTrace.begin("TimelineLoadOlder")
            defer {
                PerformanceTrace.end("TimelineLoadOlder", id: signpostID)
            }
            guard let timelineSession,
                  let outcome = await timelineSession.loadOlder(before: eventID)
            else {
                await MainActor.run {
                    state = .loaded(loadedTimelineItems.isEmpty ? items : loadedTimelineItems, isPaginating: false)
                    paginationScrollAnchorID = nil
                    olderPaginationCompletions &+= 1
                }
                return
            }
            await MainActor.run {
                olderPaginationCompletions &+= 1
                let currentItems = loadedTimelineItems.isEmpty ? items : loadedTimelineItems
                switch outcome {
                case let .loaded(windowItems):
                    // The live provider stays live: the page is the same SDK
                    // timeline growing older, and its update stream keeps
                    // applying new messages, edits and receipts while the
                    // viewport restores the visible anchor.
                    timelineAvailability.recordSuccess()
                    let currentServerIDs = Set(currentItems.filter { $0.isLocalPending == false }.map(\.eventID))
                    let addedCount = windowItems.reduce(into: 0) { count, item in
                        if currentServerIDs.contains(item.eventID) == false {
                            count += 1
                        }
                    }
                    let merged = mergeTimelineItems(windowItems, isPaginating: false)
                    state = .loaded(merged, isPaginating: false)
                    logTimelineEvent(
                        "pagination-completed",
                        fields: ["added": "\(addedCount)", "rendered": "\(merged.count)"]
                    )
                case .empty:
                    timelineAvailability.recordSuccess()
                    hasReachedOldestMessages = true
                    paginationScrollAnchorID = nil
                    state = .loaded(currentItems, isPaginating: false)
                    logTimelineEvent("pagination-reached-start")
                case let .failed(failure):
                    paginationScrollAnchorID = nil
                    state = .loaded(currentItems, isPaginating: false)
                    timelineAvailability.recordFailure(failure, preservingRows: true)
                    logTimelineEvent("pagination-failed", fields: ["code": failure.diagnosticCode])
                }
            }
        }
        return true
    }

    private func scheduleMarkFullyRead(eventID visibleEventID: String) {
        guard let tail = loadedTimelineItems.last(where: { $0.serverEventID != nil }),
              tail.serverEventID == visibleEventID else { return }
        let eventID = tail.readReceiptEventID ?? visibleEventID
        guard RoomTimelineReadAcknowledgementPolicy.shouldSchedule(
            isApplicationActive: isReadSceneActive
                && UIApplication.shared.applicationState == .active,
            allowsReadReceipts: SynaraSharedConstants.boolSetting(
                SynaraSharedConstants.hideActivityKey
            ) == false,
            isLive: timelineProviderIsLive,
            isConfirmedPinned: isTimelineBottomVisible,
            isJumpingToLatest: isJumpingToLatest,
            isUserInteracting: isUserDraggingTimeline,
            eventID: eventID,
            lastMarkedEventID: lastMarkedFullyReadEventID
        ) else {
            return
        }

        guard MatrixServerEventIDPolicy.canAcknowledge(eventID) else {
            return
        }

        readMarkerQueue.enqueue(
            RoomTimelineReadObservation(visibleEventID: visibleEventID, receiptEventID: eventID),
            now: Date()
        )
        guard markFullyReadTask == nil else {
            return
        }

        let firstQueuedAt = readMarkerQueue.firstQueuedAt ?? Date()
        let delay = RoomTimelineReadMarkerQueuePolicy.delayNanoseconds(
            firstQueuedAt: firstQueuedAt,
            now: Date(),
            debounceNanoseconds: Self.markFullyReadDelayNanoseconds,
            maximumLatencyNanoseconds: Self.markFullyReadMaximumLatencyNanoseconds
        )
        markFullyReadTaskGeneration &+= 1
        let installedGeneration = markFullyReadTaskGeneration

        markFullyReadTask = Task {
            try? await Task.sleep(nanoseconds: delay)
            guard Task.isCancelled == false,
                  isReadSceneActive,
                  UIApplication.shared.applicationState == .active,
                  SynaraSharedConstants.boolSetting(
                      SynaraSharedConstants.hideActivityKey
                  ) == false,
                  timelineProviderIsLive,
                  isTimelineBottomVisible,
                  isJumpingToLatest == false,
                  isUserDraggingTimeline == false,
                  readMarkerQueue.pending != nil
            else {
                await MainActor.run {
                    clearMarkFullyReadTask(ifGenerationMatches: installedGeneration)
                }
                return
            }

            guard let observation = readMarkerQueue.dequeue() else {
                await MainActor.run {
                    clearMarkFullyReadTask(ifGenerationMatches: installedGeneration)
                }
                return
            }
            let didAcknowledge = await environment.readMarkers.markFullyRead(
                roomID: roomID,
                eventID: observation.receiptEventID
            )
            guard Task.isCancelled == false else {
                return
            }

            await MainActor.run {
                guard RoomTimelineReadMarkerTaskPolicy.ownsInstalledTask(
                    installedGeneration: installedGeneration,
                    currentGeneration: markFullyReadTaskGeneration
                ) else {
                    return
                }
                markFullyReadTask = nil
                if didAcknowledge {
                    // The divider stays where it was until the room is left.
                    lastMarkedFullyReadEventID = observation.receiptEventID
                    showJumpToLatest = false
                }
                if let nextObservation = readMarkerQueue.pending {
                    scheduleMarkFullyRead(eventID: nextObservation.visibleEventID)
                }
            }
        }
    }

    private func cancelMarkFullyRead() {
        markFullyReadTaskGeneration &+= 1
        markFullyReadTask?.cancel()
        markFullyReadTask = nil
        readMarkerQueue.clear()
    }

    private func clearMarkFullyReadTask(ifGenerationMatches installedGeneration: UInt64) {
        guard RoomTimelineReadMarkerTaskPolicy.ownsInstalledTask(
            installedGeneration: installedGeneration,
            currentGeneration: markFullyReadTaskGeneration
        ) else {
            return
        }
        markFullyReadTask = nil
    }

    private func jumpToLatest(proxy: ScrollViewProxy, currentItems: [TimelineItem]) {
        guard isJumpingToLatest == false,
              let timelineSession
        else {
            return
        }

        dismissKeyboard()
        isComposerFocused = false
        cancelTimelineScroll()
        cancelMarkFullyRead()
        paginationScrollAnchorID = nil
        hasReachedOldestMessages = false
        hasUserInteractedWithTimeline = false
        isJumpingToLatest = true
        pendingLastReadEventID = nil
        lastReadJumpEventID = nil
        showJumpToLatest = true

        if timelineProviderIsLive {
            isJumpingToLatest = false
            scrollToTimelineBottom(proxy: proxy, animated: true, ignoreComposerFocus: true, reason: "jump-latest-live")
            return
        }

        Task {
            let signpostID = PerformanceTrace.begin("TimelineJumpToLatest")
            defer {
                PerformanceTrace.end("TimelineJumpToLatest", id: signpostID)
            }
            let transition = await timelineSession.transitionToLive()
            await MainActor.run {
                guard self.timelineSession === timelineSession else { return }
                switch transition {
                case let .succeeded(feed):
                    hasPositionedInitialTimeline = true
                    timelinePosition = .placingInitial
                    applySessionFeed(feed)
                    lastRenderedTimelineCount = loadedTimelineItems.count
                    showJumpToLatest = true
                    isJumpingToLatest = false
                    Task { @MainActor in
                        await Task.yield()
                        scrollToTimelineBottom(
                            proxy: proxy,
                            animated: true,
                            ignoreComposerFocus: true,
                            reason: "jump-latest-live-provider"
                        )
                    }
                case .empty:
                    isJumpingToLatest = false
                    showJumpToLatest = true
                    timelineAvailability.recordFailure(
                        TimelineLoadFailure(
                            kind: .temporarilyUnavailable,
                            diagnosticCode: "timeline-jump-latest-empty"
                        ),
                        preservingRows: true
                    )
                    logTimelineEvent("jump-latest-empty-preserved", fields: ["rendered": "\(currentItems.count)"])
                case let .failed(failure):
                    isJumpingToLatest = false
                    showJumpToLatest = true
                    timelineAvailability.recordFailure(failure, preservingRows: true)
                    logTimelineEvent("jump-latest-failed-preserved", fields: ["rendered": "\(currentItems.count)"])
                case .superseded:
                    isJumpingToLatest = false
                    showJumpToLatest = timelineProviderIsLive == false
                    logTimelineEvent("jump-latest-superseded")
                }
            }
        }
    }

    private func jumpToLatestStable(
        currentItems: [TimelineItem],
        dismissComposer: Bool = true,
        animated: Bool = true
    ) {
        guard isJumpingToLatest == false,
              let timelineSession
        else {
            return
        }

        if dismissComposer {
            dismissKeyboard()
            isComposerFocused = false
        }
        cancelTimelineScroll()
        cancelMarkFullyRead()
        paginationScrollAnchorID = nil
        hasReachedOldestMessages = false
        hasUserInteractedWithTimeline = false
        pendingLastReadEventID = nil
        lastReadJumpEventID = nil

        // Already following the live tail: pin the current rows. Remounting
        // would drop local echoes and just-uploaded attachments that the live
        // fixture has not yet projected. Unread/history on a reused live
        // provider still remounts so send returns to newest.
        if RoomTimelineJumpLatestPolicy.shouldPinCurrentLiveWindow(
            providerIsLive: timelineProviderIsLive,
            isLiveMode: activeTimelineMode.isLive,
            isFollowingLive: timelinePosition == .followingLive,
            isConfirmedPinned: isTimelineBottomVisible
        ) {
            showJumpToLatest = false
            enqueueStableViewportCommand(
                .latest(animated: animated),
                generation: timelineBottomAnchorGeneration
            )
            return
        }

        isJumpingToLatest = true
        showJumpToLatest = true

        if timelineProviderIsLive {
            timelinePosition = .placingInitial
            enqueueStableViewportCommand(.latest(animated: animated), generation: timelineBottomAnchorGeneration)
            return
        }

        Task {
            let signpostID = PerformanceTrace.begin("TimelineJumpToLatest")
            defer { PerformanceTrace.end("TimelineJumpToLatest", id: signpostID) }
            let transition = await timelineSession.transitionToLive()
            await MainActor.run {
                guard self.timelineSession === timelineSession else { return }
                switch transition {
                case let .succeeded(feed):
                    hasPositionedInitialTimeline = false
                    timelinePosition = .placingInitial
                    applySessionFeed(feed)
                    enqueueStableViewportCommand(.latest(animated: animated), generation: feed.generation)
                    lastRenderedTimelineCount = loadedTimelineItems.count
                    showJumpToLatest = true
                case .empty:
                    isJumpingToLatest = false
                    showJumpToLatest = true
                    timelineAvailability.recordFailure(
                        TimelineLoadFailure(
                            kind: .temporarilyUnavailable,
                            diagnosticCode: "timeline-jump-latest-empty"
                        ),
                        preservingRows: true
                    )
                    logTimelineEvent("jump-latest-empty-preserved", fields: ["rendered": "\(currentItems.count)"])
                case let .failed(failure):
                    isJumpingToLatest = false
                    showJumpToLatest = true
                    timelineAvailability.recordFailure(failure, preservingRows: true)
                    logTimelineEvent("jump-latest-failed-preserved", fields: ["rendered": "\(currentItems.count)"])
                case .superseded:
                    isJumpingToLatest = false
                    showJumpToLatest = timelineProviderIsLive == false
                    logTimelineEvent("jump-latest-superseded")
                }
            }
        }
    }

    private func followLatestAfterSend() {
        sendFollowLatestTask?.cancel()
        let routeID = stableViewportRouteID
        hasUserInteractedWithTimeline = false
        jumpToLatestStable(currentItems: loadedTimelineItems, dismissComposer: false, animated: false)

        // The keyboard and the local-echo snapshot can settle after the first
        // viewport command. Keep the explicit send-to-latest intent briefly,
        // while letting a user's timeline drag take ownership immediately.
        sendFollowLatestTask = Task { @MainActor in
            for _ in 0 ..< 12 {
                try? await Task.sleep(nanoseconds: 500_000_000)
                guard Task.isCancelled == false,
                      stableViewportRouteID == routeID,
                      hasUserInteractedWithTimeline == false
                else {
                    return
                }
                if isTimelineBottomVisible == false, isJumpingToLatest == false {
                    jumpToLatestStable(currentItems: loadedTimelineItems, dismissComposer: false, animated: false)
                }
            }
        }
    }

    @ViewBuilder
    private var timelineNavigationRecovery: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            if loadedTimelineItems.count <= 1, hasReachedOldestMessages == false,
               let oldest = loadedTimelineItems.first?.eventID {
                Button("Load older messages") {
                    hasUserInteractedWithTimeline = true
                    hasPositionedInitialTimeline = true
                    _ = loadOlderTimeline(before: oldest, scrollAnchorID: oldest)
                }
                .accessibilityIdentifier("LoadOlderMessages")
            }
        }
        .buttonStyle(.bordered)
        .padding(SynaraSpacing.medium)
    }

    private func jumpToLastRead(eventID: String) {
        guard let timelineSession, lastReadJumpEventID == nil else { return }
        let routeID = stableViewportRouteID
        cancelMarkFullyRead()
        lastReadJumpEventID = eventID
        // The legacy scroll view places a read marker only when a provider opens.
        let isStableViewport = StableScrollAnchoringFeatureFlag.isEnabled
        switch RoomTimelineJumpLastReadPolicy.route(
            markerEventID: eventID,
            providerIsLive: timelineProviderIsLive && isStableViewport,
            loadedServerEventIDs: loadedTimelineItems.compactMap(\.serverEventID)
        ) {
        case .scrollLiveWindow:
            logTimelineEvent("jump-last-read", fields: ["provider": "live"])
            pendingLastReadEventID = nil
            placeUnreadDivider(markerEventID: eventID)
            activeTimelineMode = .unread(markerEventID: eventID)
            timelinePosition = .readingHistory
            enqueueStableViewportCommand(.readMarker(eventID: eventID), generation: timelineBottomAnchorGeneration)
        case .openFocusedWindow:
            logTimelineEvent("jump-last-read", fields: ["provider": "focused"])
            Task {
                // Its own stream key: the live stream keeps running until the
                // focused window is known to hold the marker.
                let result = await timelineSession.switchProvider(
                    to: .unread(markerEventID: eventID),
                    holding: eventID
                )
                await MainActor.run {
                    guard self.timelineSession === timelineSession, stableViewportRouteID == routeID else { return }
                    switch result {
                    case let .switched(feed):
                        pendingLastReadEventID = nil
                        hasPositionedInitialTimeline = false
                        applySessionFeed(feed)
                        if isStableViewport == false {
                            lastReadJumpEventID = nil
                        }
                    case .missing, .failed:
                        lastReadJumpEventID = nil
                        failJumpToLastRead()
                    case .superseded:
                        lastReadJumpEventID = nil
                    }
                }
            }
        }
    }

    /// The last-read message could not be shown: say so and stay on, or
    /// return to, the latest messages. Never a silent no-op.
    private func failJumpToLastRead() {
        logTimelineEvent("jump-last-read-unavailable")
        pendingLastReadEventID = nil
        eventRouteNotice = TimelineEventRouteNotice.lastReadUnavailable
        guard StableScrollAnchoringFeatureFlag.isEnabled else {
            sendLatestRequest &+= 1
            return
        }
        Task { @MainActor in
            await Task.yield()
            jumpToLatestStable(currentItems: loadedTimelineItems, dismissComposer: false)
        }
    }

    /// Pages an event-focused window toward the present when the user
    /// reaches its newest row, and hands over to the live provider once the
    /// window reaches the live tail.
    private func loadNewerTimelineIfNeeded() {
        guard RoomTimelineForwardPaginationPolicy.shouldLoadNewer(
            providerIsLive: timelineProviderIsLive,
            isPinned: isTimelineBottomVisible,
            hasPendingPlacement: stableViewportCommand != nil,
            isJumpingToLatest: isJumpingToLatest,
            isLoadingNewer: isLoadingNewerTimeline
        ),
            hasPositionedInitialTimeline,
            case .loaded = state,
            let timelineSession
        else {
            return
        }
        let now = Date()
        guard now.timeIntervalSince(lastNewerPaginationAt) >= Self.olderPaginationDebounceInterval else {
            return
        }
        lastNewerPaginationAt = now
        isLoadingNewerTimeline = true
        logTimelineEvent("forward-pagination-started", fields: ["rendered": "\(loadedTimelineItems.count)"])
        Task {
            let outcome = await timelineSession.loadNewer()
            await MainActor.run {
                guard self.timelineSession === timelineSession else { return }
                isLoadingNewerTimeline = false
                guard let outcome else { return }
                switch outcome {
                case let .loaded(windowItems):
                    timelineAvailability.recordSuccess()
                    state = .loaded(
                        mergeTimelineItems(windowItems, isPaginating: false),
                        isPaginating: currentTimelineIsPaginating
                    )
                    logTimelineEvent("forward-pagination-completed", fields: ["rendered": "\(loadedTimelineItems.count)"])
                case .empty:
                    logTimelineEvent("forward-pagination-reached-live")
                    if RoomTimelineForwardPaginationPolicy.shouldReturnToLive(
                        outcome: outcome,
                        isPinned: isTimelineBottomVisible,
                        hasUserScrolled: hasUserInteractedWithTimeline
                    ) {
                        if StableScrollAnchoringFeatureFlag.isEnabled {
                            jumpToLatestStable(currentItems: loadedTimelineItems, dismissComposer: false, animated: false)
                        } else {
                            sendLatestRequest &+= 1
                        }
                    }
                case let .failed(failure):
                    timelineAvailability.recordFailure(failure, preservingRows: true)
                    logTimelineEvent("forward-pagination-failed", fields: ["code": failure.diagnosticCode])
                }
            }
        }
    }

    /// Core's backward pagination state for the provider on screen.
    private var currentBackwardPagination: TimelineBackwardPagination {
        environment.timeline.backwardPagination(
            roomID: roomID,
            focusedEventID: timelineProviderFocusedEventID
        )
    }

    private func enqueueStableInitialCommand(for mode: RoomTimelineMode, generation: UInt64) {
        switch mode {
        case .live:
            timelinePosition = .placingInitial
            enqueueStableViewportCommand(.latest(animated: false), generation: generation)
        case let .unread(markerEventID):
            timelinePosition = .readingHistory
            showJumpToLatest = true
            enqueueStableViewportCommand(.readMarker(eventID: markerEventID), generation: generation)
        case let .focused(eventID):
            timelinePosition = .focusedEvent
            showJumpToLatest = true
            enqueueStableViewportCommand(.focused(eventID: eventID, animated: true), generation: generation)
        }
    }

    private func enqueueStableViewportCommand(
        _ kind: StableTimelineViewportCommand.Kind,
        generation: UInt64
    ) {
        stableViewportCommandID &+= 1
        stableViewportCommand = StableTimelineViewportCommand(
            id: stableViewportCommandID,
            routeID: stableViewportRouteID,
            sessionGeneration: generation,
            kind: kind
        )
    }

    private func handleStableBottomPinnedChanged(isPinned: Bool, newestEventID: String?) {
        isTimelineBottomVisible = isPinned
        if isPinned, timelineProviderIsLive == false {
            loadNewerTimelineIfNeeded()
        }
        if isPinned, timelineProviderIsLive {
            guard RoomTimelineJumpLatestPolicy.shouldAdoptLiveFollowOnPin(
                position: timelinePosition,
                hasPendingPlacement: stableViewportCommand != nil,
                isUserDragging: isUserDraggingTimeline
            ) else {
                return
            }
            resumeLivePresentationAtBottom()
            timelinePosition = .followingLive
            showJumpToLatest = false
            if let newestEventID {
                scheduleMarkFullyRead(eventID: newestEventID)
            }
        } else {
            cancelMarkFullyRead()
            if timelineProviderIsLive == false || timelinePosition == .readingHistory {
                showJumpToLatest = loadedTimelineItems.isEmpty == false
            }
        }
    }

    private func handleStableUserInteractionChanged(isInteracting: Bool) {
        isUserDraggingTimeline = isInteracting
        if isInteracting {
            sendFollowLatestTask?.cancel()
            sendFollowLatestTask = nil
            hasUserInteractedWithTimeline = true
            cancelTimelineScroll()
            cancelMarkFullyRead()
            if timelineProviderIsLive {
                timelinePosition = .readingHistory
            }
            return
        }

        timelinePosition = RoomTimelineScrollPolicy.positionAfterUserDrag(
            isBottomVisible: isTimelineBottomVisible,
            focusedEventID: timelineProviderFocusedEventID
        )
        if isTimelineBottomVisible {
            resumeLivePresentationAtBottom()
        }
        showJumpToLatest = timelineProviderIsLive == false || isTimelineBottomVisible == false
        if isTimelineBottomVisible,
           timelineProviderIsLive,
           let newestEventID = loadedTimelineItems.reversed().compactMap(\.serverEventID).first
        {
            scheduleMarkFullyRead(eventID: newestEventID)
        }
    }

    private var timelineProviderFocusedEventID: String? {
        RoomTimelineProviderPresentationPolicy.focusedEventID(
            providerIsLive: timelineProviderIsLive,
            currentMode: activeTimelineMode
        )
    }

    private func resumeLivePresentationAtBottom() {
        if timelineProviderIsLive, let timelineSession {
            // Back on the live tail: later updates may trim the rows above.
            Task { await timelineSession.followLiveTail() }
        }
        let nextMode = RoomTimelineProviderPresentationPolicy.modeWhenPinned(
            providerIsLive: timelineProviderIsLive,
            currentMode: activeTimelineMode
        )
        guard nextMode != activeTimelineMode else {
            return
        }
        // Following the live tail keeps the unread divider where it is.
        activeTimelineMode = nextMode
    }

    private func handleStableCommandCompleted(
        _ command: StableTimelineViewportCommand,
        success: Bool,
        targetEventID: String?
    ) {
        guard stableViewportCommand?.id == command.id else {
            return
        }
        // The viewport only reports failure after its bounded retry budget is
        // exhausted, so either outcome is terminal for this command ID.
        stableViewportCommand = nil
        var shouldRecoverMissingFocus = false

        switch command.kind {
        case .latest:
            isJumpingToLatest = false
            hasPositionedInitialTimeline = true
            if success {
                timelinePosition = .followingLive
                showJumpToLatest = false
            } else {
                timelinePosition = .readingHistory
                showJumpToLatest = RoomTimelineLatestCommandCompletionPolicy.shouldShowRecovery(
                    success: success
                )
            }
        case let .readMarker(eventID):
            hasPositionedInitialTimeline = true
            timelinePosition = .readingHistory
            showJumpToLatest = true
            if lastReadJumpEventID == eventID {
                lastReadJumpEventID = nil
                // A drag interrupting the placement is not a failure; a
                // window without the marker is.
                if success == false,
                   loadedTimelineItems.contains(where: { $0.serverEventID == eventID }) == false
                {
                    failJumpToLastRead()
                }
            } else if success == false {
                pendingLastReadEventID = eventID
            }
        case .focused:
            hasAnchoredEvent = success
            hasPositionedInitialTimeline = true
            timelinePosition = .focusedEvent
            showJumpToLatest = targetEventID != loadedTimelineItems.last?.eventID
            if success == false {
                // The rows loaded but the routed message is not among them (a
                // hidden event, or Core fell back to the live room). Say so
                // rather than reporting the whole timeline unavailable.
                eventRouteNotice = TimelineEventRouteNotice.notShown
                logTimelineEvent("timeline-focused-event-unavailable")
                shouldRecoverMissingFocus = true
            }
        }
        logTimelineEvent(
            "stable-command-completed",
            fields: ["success": "\(success)", "target": targetEventID ?? "none"]
        )
        if shouldRecoverMissingFocus {
            Task { @MainActor in
                await Task.yield()
                jumpToLatestStable(currentItems: loadedTimelineItems)
            }
        }
    }

    private func dismissKeyboard() {
        #if canImport(UIKit)
            ComposerTextInputRegistry.dismissKeyboard()
        #endif
    }

    private func beginReply(_ item: TimelineItem) {
        guard isSendingMessage == false else {
            return
        }
        attachmentSendTransaction = nil
        if editSession != nil {
            cancelEdit(restoreDraft: true)
        }
        replyTarget = ComposerRelationTarget(
            item: item,
            kind: .reply,
            currentUserID: currentUserID
        )
    }

    private func beginEdit(_ item: TimelineItem) {
        guard isSendingMessage == false else {
            return
        }
        attachmentSendTransaction = nil
        let currentDraft = editSession?.previousDraft ?? draft
        let session = ComposerEditFlow.begin(
            item: item,
            currentUserID: currentUserID,
            currentDraft: currentDraft
        )
        replyTarget = nil
        editSession = session
        if TimelinePendingReconciler.messageBody(for: item) != nil {
            draft = session.draft
            environment.drafts.setDraft(session.draft, roomID: roomID)
        }
        isComposerFocused = true
    }

    private func beginReport(_ item: TimelineItem) {
        reportReason = ""
        reportTarget = item
    }

    private func sendComposerText(_ rawBody: String) {
        sendMessage(body: rawBody)
    }

    /// A picked GIF goes out as a plain message whose body is the provider's
    /// media URL, as desktop sends it: Klipy and GIPHY forbid re-hosting, so
    /// it is never uploaded. It takes the reply relation and leaves the typed
    /// draft alone.
    private func sendGif(_ gif: GifResult) {
        guard isSendingMessage == false, editSession == nil else {
            return
        }
        let clientTransactionID = OutgoingQueuedMessage.newClientTransactionID()
        let queued = environment.outgoingSends.enqueue(
            localID: "$pending-\(clientTransactionID)",
            clientTransactionID: clientTransactionID,
            roomID: roomID,
            body: gif.url,
            formattedBody: nil,
            replyToEventID: replyTarget?.eventID,
            threadRootEventID: replyTarget?.threadRootEventID,
            senderID: currentUserID,
            senderAvatarURL: ownAvatarURL,
            timestamp: Date()
        )
        registerSendAnimation(for: queued.id)
        applyOutgoingQueueToTimeline()
        if StableScrollAnchoringFeatureFlag.isEnabled {
            followLatestAfterSend()
        } else {
            sendLatestRequest &+= 1
        }
        replyTarget = nil
        sendError = nil
        Task {
            await transmitOutgoing(id: queued.id)
        }
    }

    @MainActor
    private func performSend(_ intent: ComposerSendIntent, sentText: String) async {
        await performSend(
            body: intent.body,
            replyToEventID: intent.replyToEventID,
            threadRootEventID: intent.threadRootEventID,
            editEventID: intent.editEventID,
            retrying: intent.retrying,
            sentText: sentText
        )
    }

    /// A send that began with `sentText` was accepted. The composer drops
    /// that text from its live contents and keeps anything typed since (the
    /// keyboard stays up, so the user may have gone on typing during an
    /// upload or edit save). `draft` is left to the composer: it lags the
    /// editor by a debounce, and clearing it here would lose the newest
    /// keystrokes.
    private func finishComposerSend(sentText: String, persisting: Bool) {
        if persisting {
            let remaining = ComposerSentDraft.remainingText(current: draft, sent: sentText)
            if remaining.isEmpty {
                environment.drafts.clearDraft(roomID: roomID)
            } else {
                environment.drafts.setDraft(remaining, roomID: roomID)
            }
        }
        sentDraftSerial &+= 1
        sentDraft = ComposerSentDraft(id: sentDraftSerial, sentText: sentText)
    }

    /// Opens the Reactions sheet on `key`, or on the first reaction.
    private func showReactions(for item: TimelineItem, key: String?) {
        reactionsTarget = TimelineReactionsSheetTarget(item: item, initialKey: key)
    }

    /// A pill or quick reaction: toggles the user's reaction, and records
    /// an added one in the recent-emoji list desktop shares.
    private func toggleReaction(_ key: String, on item: TimelineItem) {
        let isAdding = item.reactionOwnership.contains(key) == false
        let matrix = environment.matrix
        Task {
            let succeeded = await performAction(.react(key), to: item)
            if succeeded, isAdding {
                await quickReactions.recordUse(key) { await matrix.recordRecentEmoji($0) }
            }
        }
    }

    private func applyAction(
        _ action: EventActionType,
        to item: TimelineItem,
        successMessage: String? = nil
    ) {
        Task {
            _ = await performAction(action, to: item, successMessage: successMessage)
        }
    }

    @MainActor
    private func performAction(
        _ action: EventActionType,
        to item: TimelineItem,
        successMessage: String? = nil
    ) async -> Bool {
        let inFlightKey = "\(item.eventID):\(action.inFlightKey)"
        guard inFlightTimelineActionKeys.insert(inFlightKey).inserted else {
            return false
        }
        // Once dispatched, the Core/SDK write owns completion. UI dismissal
        // cannot cancel or duplicate that side effect.
        defer { inFlightTimelineActionKeys.remove(inFlightKey) }
        do {
            let updated = try await environment.eventActions.apply(
                action,
                to: item,
                currentUserID: currentUserID,
                roomID: roomID
            )
            replace(updated)
            if let successMessage {
                timelineActionMessage = successMessage
            }
            return true
        } catch {
            timelineActionMessage = error.localizedDescription
            return false
        }
    }

    @MainActor
    private func performForwardAction(
        targetRoomID: String,
        asQuote: Bool,
        confirmedEncryptionDowngrade: Bool,
        to item: TimelineItem
    ) async -> EventActionError? {
        let action = EventActionType.forward(
            targetRoomID: targetRoomID,
            asQuote: asQuote,
            confirmedEncryptionDowngrade: confirmedEncryptionDowngrade
        )
        let inFlightKey = "\(item.eventID):\(action.inFlightKey)"
        guard inFlightTimelineActionKeys.insert(inFlightKey).inserted else {
            return .alreadyInProgress
        }
        defer { inFlightTimelineActionKeys.remove(inFlightKey) }
        do {
            let updated = try await environment.eventActions.apply(
                action,
                to: item,
                currentUserID: currentUserID,
                roomID: roomID
            )
            replace(updated)
            timelineActionMessage = String(localized: "Message forwarded.")
            return nil
        } catch let error as EventActionError {
            return error
        } catch {
            return .failed
        }
    }

    private func clearComposerRelation() {
        attachmentSendTransaction = nil
        if editSession != nil {
            cancelEdit(restoreDraft: true)
        }
        replyTarget = nil
    }

    private func completeComposerRelation() {
        attachmentSendTransaction = nil
        replyTarget = nil
        editSession = nil
    }

    private func cancelEdit(restoreDraft: Bool) {
        if restoreDraft, let session = editSession {
            draft = ComposerEditFlow.cancel(session)
            environment.drafts.setDraft(draft, roomID: roomID)
        }
        editSession = nil
    }

    private func appendAcknowledgedUpload(_ item: TimelineItem) {
        // HTTP send completion precedes the timeline echo. Reuse the same local
        // echo lifecycle as text, preserving the exact acknowledged event ID.
        // A queued upload arrives already pending under its transaction id and
        // keeps that state until the SDK echo for the same transaction shows.
        let merged = TimelinePendingReconciler.merge(
            streamItems: loadedTimelineItems,
            localItems: [item.isLocalPending ? item : item.withDeliveryStatus(.sent)],
            currentUserID: currentUserID
        )
        state = .loaded(merged, isPaginating: currentTimelineIsPaginating)
    }

    private func registerSendAnimation(for itemID: String, isRetry: Bool = false) {
        guard isRetry == false else {
            return
        }
        sendAnimationItemIDs.insert(itemID)
        Task { @MainActor in
            try? await Task.sleep(nanoseconds: 700_000_000)
            sendAnimationItemIDs.remove(itemID)
        }
    }

    private func replace(_ item: TimelineItem) {
        guard case let .loaded(items, isPaginating) = state else {
            return
        }
        state = .loaded(items.map { $0.id == item.id ? item : $0 }, isPaginating: isPaginating)
    }

    private func executeAgentAction(_ action: SynaraAgentCardAction, sourceEventID: String?) {
        switch SynaraAgentCardActionResolver.plan(for: action) {
        case let .success(plan):
            switch plan {
            case let .openURL(url):
                openURL(url)
            case let .copyText(text):
                #if canImport(UIKit)
                    UIPasteboard.general.string = text
                #endif
            case let .submitApproval(decision):
                submitAgentApproval(action, decision: decision, sourceEventID: sourceEventID)
            }
        case let .failure(error):
            switch error {
            case let .unsupportedKind(unsupported):
                agentActionMessage = String(localized: "Unsupported action: \(unsupported)")
            case .missingPayload:
                agentActionMessage = String(localized: "Action payload is missing")
            case .unsafeURL:
                agentActionMessage = String(localized: "Action link is not allowed")
            case .encodingFailure:
                agentActionMessage = String(localized: "Could not copy action payload")
            }
        }
    }

    private func submitAgentApproval(
        _ action: SynaraAgentCardAction,
        decision: SynaraAgentApprovalDecision,
        sourceEventID: String?
    ) {
        Task {
            do {
                try await environment.agentApprovals.submit(
                    SynaraAgentApprovalRequest(
                        roomID: roomID,
                        sourceEventID: sourceEventID,
                        action: action,
                        decision: decision
                    )
                )
                await MainActor.run {
                    if decision == .approve {
                        SynaraHaptics.trigger(.success)
                    } else {
                        SynaraHaptics.trigger(.lightImpact)
                    }
                    agentActionMessage = decision == .approve
                        ? String(localized: "Agent action approved")
                        : String(localized: "Agent action rejected")
                }
            } catch let error as SynaraAgentApprovalError {
                await MainActor.run {
                    agentActionMessage = error.errorDescription
                        ?? String(localized: "Agent action could not be submitted")
                }
            } catch {
                await MainActor.run {
                    agentActionMessage = String(localized: "Agent action could not be submitted")
                }
            }
        }
    }

    private func submitAgentApprovalDecision(actionIdentifier: String, sourceEventID: String) {
        Task {
            do {
                let outcome = try await environment.agentApprovalDecisions.submitDecision(
                    SynaraAgentApprovalPromptDecisionRequest(
                        roomID: roomID,
                        sourceEventID: sourceEventID,
                        actionIdentifier: actionIdentifier
                    )
                )
                await MainActor.run {
                    if actionIdentifier == SynaraAgentApprovalPromptReaction.deny.actionIdentifier {
                        SynaraHaptics.trigger(.lightImpact)
                    } else {
                        SynaraHaptics.trigger(.success)
                    }
                    agentActionMessage = outcome == .alreadyDecided
                        ? String(localized: "This approval was already decided on this account.")
                        : String(localized: "Approval decision sent.")
                }
            } catch let error as SynaraAgentApprovalError {
                await MainActor.run {
                    agentActionMessage = error.errorDescription
                        ?? String(localized: "Approval decision could not be submitted.")
                }
            } catch {
                await MainActor.run {
                    agentActionMessage = String(localized: "Approval decision could not be submitted.")
                }
            }
        }
    }

    private var loadedTimelineItems: [TimelineItem] {
        guard case let .loaded(items, _) = state else {
            return []
        }
        return items
    }
}

private enum TimelineViewState: Equatable {
    case idle
    case loading
    case empty
    case failed(String)
    case loaded([TimelineItem], isPaginating: Bool)
}

private struct TimelineEventRouteNoticeBanner: View {
    let message: String
    let onDismiss: () -> Void

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: "info.circle")
                .foregroundStyle(SynaraColor.secondaryText)
            Text(message)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
            Spacer(minLength: SynaraSpacing.small)
            Button("Dismiss", action: onDismiss)
                .buttonStyle(.bordered)
                .controlSize(.small)
                .accessibilityIdentifier("TimelineEventRouteNoticeDismiss")
        }
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.vertical, SynaraSpacing.xSmall)
        .background(SynaraColor.surface)
        .accessibilityIdentifier("TimelineEventRouteNotice")
    }
}

private struct TimelineAvailabilityBanner: View {
    let failure: TimelineLoadFailure
    let onRetry: () -> Void

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: "wifi.exclamationmark")
                .foregroundStyle(SynaraColor.warning)
            Text(failure.userMessage)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
            Spacer(minLength: SynaraSpacing.small)
            Button("Retry", action: onRetry)
                .buttonStyle(.bordered)
                .controlSize(.small)
                .accessibilityIdentifier("TimelineAvailabilityRetryButton")
        }
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.vertical, SynaraSpacing.xSmall)
        .background(SynaraColor.surface)
        .accessibilityIdentifier("TimelineAvailabilityBanner")
    }
}

/// Warns above the composer when a room member's cryptographic identity changed.
private struct RoomIdentityWarningBanner: View {
    private static let refreshNanoseconds: UInt64 = 30_000_000_000

    let roomID: String
    let crypto: CryptoStatusServicing
    @State private var warnings: [RoomIdentityWarning] = []
    @State private var isResolving = false
    @State private var failed = false

    var body: some View {
        Group {
            if let warning = RoomIdentityWarning.banner(for: warnings) {
                content(for: warning)
            }
        }
        .task(id: roomID) {
            warnings = []
            failed = false
            while Task.isCancelled == false {
                let next = await crypto.roomIdentityWarnings(roomID: roomID)
                if Task.isCancelled { break }
                warnings = next
                try? await Task.sleep(nanoseconds: Self.refreshNanoseconds)
            }
        }
    }

    private func content(for warning: RoomIdentityWarning) -> some View {
        let critical = warning.kind == .verificationViolation
        return HStack(spacing: SynaraSpacing.small) {
            Image(systemName: critical ? "exclamationmark.shield" : "shield")
                .foregroundStyle(critical ? SynaraColor.critical : SynaraColor.warning)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 2) {
                Text(warning.message)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                if failed {
                    Text("Couldn't update this identity change.")
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.criticalText)
                }
            }
            Spacer(minLength: SynaraSpacing.small)
            Button(warning.actionTitle) {
                resolve(warning)
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
            .disabled(isResolving)
            .accessibilityIdentifier("RoomIdentityWarningAction")
        }
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.vertical, SynaraSpacing.xSmall)
        .background(SynaraColor.surface)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("RoomIdentityWarningBanner")
    }

    private func resolve(_ warning: RoomIdentityWarning) {
        guard isResolving == false else { return }
        isResolving = true
        failed = false
        Task {
            let next = await crypto.resolveRoomIdentityWarning(roomID: roomID, warning: warning)
            if let next {
                warnings = next
            } else {
                failed = true
            }
            isResolving = false
        }
    }
}

private struct RoomTypingIndicator: View {
    let text: String

    var body: some View {
        HStack(spacing: SynaraSpacing.xSmall) {
            ProgressView()
                .controlSize(.mini)
            Text(text)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
                .lineLimit(1)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.vertical, SynaraSpacing.xSmall)
        .background(SynaraColor.surface)
        .accessibilityIdentifier("RoomTypingIndicator")
    }
}

private struct JumpToLatestButton: View {
    let isLoading: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            ZStack {
                Circle()
                    .fill(.ultraThinMaterial)
                Circle()
                    .stroke(SynaraColor.separator.opacity(0.8), lineWidth: 1)
                if isLoading {
                    ProgressView()
                        .controlSize(.small)
                        .tint(SynaraColor.accent)
                } else {
                    Image(systemName: "arrow.down")
                        .font(.system(size: 17, weight: .bold))
                }
            }
            .frame(width: 44, height: 44)
            .foregroundStyle(SynaraColor.accent)
            .shadow(color: .black.opacity(0.12), radius: 10, x: 0, y: 4)
        }
        .buttonStyle(.plain)
        .disabled(isLoading)
        .accessibilityLabel("Jump to latest")
        .accessibilityValue(isLoading ? "Loading latest messages" : "")
        .accessibilityIdentifier("JumpToLatestButton")
    }
}

struct JumpToLastReadButton: View {
    let action: () -> Void
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    var body: some View {
        Button(action: action) {
            if SynaraDynamicTypeLayout.jumpToLastReadShowsTitle(dynamicTypeSize) {
                HStack(spacing: SynaraSpacing.xSmall) {
                    Image(systemName: "arrow.up.to.line")
                        .font(.system(size: 13, weight: .bold))
                    Text("Jump to Last Read")
                        .font(SynaraTypography.fineMetaBold)
                        .lineLimit(1)
                }
                .padding(.horizontal, SynaraSpacing.medium)
                .frame(minHeight: SynaraHitTarget.minimum)
                .background(
                    Capsule()
                        .fill(.ultraThinMaterial)
                )
                .overlay(
                    Capsule()
                        .stroke(SynaraColor.separator.opacity(0.8), lineWidth: 1)
                )
                .foregroundStyle(SynaraColor.accentText)
                .shadow(color: .black.opacity(0.12), radius: 10, x: 0, y: 4)
            } else {
                // At accessibility sizes the titled pill spans most of the
                // width over the messages; match Jump to latest instead.
                ZStack {
                    Circle()
                        .fill(.ultraThinMaterial)
                    Circle()
                        .stroke(SynaraColor.separator.opacity(0.8), lineWidth: 1)
                    Image(systemName: "arrow.up.to.line")
                        .font(.system(size: 17, weight: .bold))
                }
                .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                .foregroundStyle(SynaraColor.accent)
                .shadow(color: .black.opacity(0.12), radius: 10, x: 0, y: 4)
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Jump to last read message")
        .accessibilityIdentifier("JumpToLastRead")
    }
}

private struct TimelineHeader: View {
    let title: String
    let subtitle: String
    var glyph: SynaraRoomGlyph = .channel
    var avatarURL: URL?
    let cryptoLabel: String?
    let cryptoSystemImage: String
    var showsBackButton = true
    let onSearch: () -> Void
    let onDetails: () -> Void
    let onBack: () -> Void
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            if showsBackButton {
                Button(action: onBack) {
                    Image(systemName: "chevron.left")
                        .font(.system(size: 19, weight: .semibold))
                        .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .padding(.leading, -5)
                .accessibilityLabel("Back")
            }

            VStack(alignment: .leading, spacing: 2) {
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.xSmall) {
                    // At accessibility sizes the glyph would take the
                    // title's width.
                    if dynamicTypeSize.isAccessibilitySize == false {
                        switch glyph {
                        case .channel:
                            Text("#")
                                .foregroundStyle(SynaraColor.secondaryText)
                                .accessibilityHidden(true)
                        case let .person(initial):
                            SynaraPersonGlyph(initial: initial, avatarURL: avatarURL, size: 22)
                                .alignmentGuide(.firstTextBaseline) { dimensions in
                                    dimensions[.bottom] - 4
                                }
                        }
                    }
                    Text(title)
                        .accessibilityIdentifier("TimelineRoomTitle")
                        .font(SynaraTypography.sectionTitle.weight(.semibold))
                        .foregroundStyle(SynaraColor.headingText)
                        .lineLimit(SynaraDynamicTypeLayout.titleLineLimit(dynamicTypeSize))
                        .fixedSize(horizontal: false, vertical: true)
                        .accessibilityAddTraits(.isHeader)
                }
                HStack(spacing: SynaraSpacing.small) {
                    Text(subtitle)
                    if let cryptoLabel {
                        Label(cryptoLabel, systemImage: cryptoSystemImage)
                            .foregroundStyle(
                                cryptoLabel == String(localized: "Encrypted")
                                    ? SynaraColor.secondaryText
                                    : SynaraColor.warningText
                            )
                            .accessibilityIdentifier("RoomEncryptionStatus")
                    }
                }
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
                .lineLimit(dynamicTypeSize.isAccessibilitySize ? 2 : 1)
            }
            .layoutPriority(1)

            Spacer(minLength: 0)

            HStack(spacing: 0) {
                Button(action: onSearch) {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 17, weight: .medium))
                        .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Search messages")
                .accessibilityIdentifier("TimelineSearchButton")
                Button(action: onDetails) {
                    Image(systemName: "ellipsis")
                        .font(.system(size: 17, weight: .medium))
                        .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Room details")
                .accessibilityIdentifier("RoomDetailsButton")
            }
            .padding(.trailing, -SynaraSpacing.small)
            .foregroundStyle(SynaraColor.primaryText)
        }
        .padding(.horizontal, SynaraSpacing.large)
        .padding(.vertical, SynaraSpacing.small)
        .background(SynaraColor.surface)
        .overlay(alignment: .bottom) {
            Divider()
        }
    }
}

private struct TimelineSearchSheet: View {
    @Binding var query: String
    let items: [TimelineItem]
    let onDismiss: () -> Void
    @FocusState private var isSearchFocused: Bool

    private var filteredItems: [TimelineItem] {
        TimelineSearchFilter.applySearchQuery(query, to: items)
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: SynaraSpacing.medium) {
                HStack(spacing: SynaraSpacing.small) {
                    Image(systemName: "magnifyingglass")
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityHidden(true)
                    TextField("Search loaded messages", text: $query)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .focused($isSearchFocused)
                        .accessibilityIdentifier("TimelineSearchField")
                    if query.isEmpty == false {
                        Button {
                            query = ""
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundStyle(SynaraColor.tertiaryText)
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Clear message search")
                    }
                }
                .padding(SynaraSpacing.medium)
                .synaraCard(fill: SynaraColor.secondarySurface)
                .padding(.horizontal, SynaraSpacing.large)
                .padding(.top, SynaraSpacing.small)

                if query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    SynaraEmptyState(
                        title: String(localized: "Search Messages"),
                        systemImage: "magnifyingglass",
                        message: String(localized: "Filter messages currently loaded in this room.")
                    )
                    .frame(maxHeight: .infinity)
                } else if filteredItems.isEmpty {
                    SynaraEmptyState(
                        title: String(localized: "No Matching Messages"),
                        systemImage: "text.magnifyingglass",
                        message: String(localized: "Try another keyword from the loaded timeline.")
                    )
                    .frame(maxHeight: .infinity)
                } else {
                    List(filteredItems) { item in
                        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                            HStack {
                                Text(item.senderDisplayName)
                                    .font(SynaraTypography.emphasis)
                                    .foregroundStyle(SynaraColor.primaryText)
                                Spacer()
                                Text(item.timestamp.timelineTime)
                                    .font(SynaraTypography.messageMeta)
                                    .foregroundStyle(SynaraColor.secondaryText)
                            }
                            Text(item.threadTitle)
                                .font(SynaraTypography.body)
                                .foregroundStyle(SynaraColor.secondaryText)
                                .lineLimit(3)
                        }
                        .padding(.vertical, SynaraSpacing.xSmall)
                        .accessibilityIdentifier("TimelineSearchResult-\(item.eventID)")
                    }
                    .listStyle(.plain)
                    .accessibilityIdentifier("TimelineSearchResults")
                }
            }
            .background(SynaraColor.surface)
            .navigationTitle("Search Messages")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done", action: onDismiss)
                        .accessibilityIdentifier("TimelineSearchDoneButton")
                }
            }
            .onAppear {
                isSearchFocused = true
            }
        }
        .presentationDetents([.medium, .large])
    }
}

struct ThreadTimelineView: View {
    let roomID: String
    let rootEventID: String
    let roomTitle: String?
    let rootTitle: String?
    /// A routed reply (notification or link) to scroll to once it loads.
    var focusedEventID: String? = nil
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @Environment(\.synaraCanvasLayout) private var canvasLayout
    @State private var state: TimelineViewState = .idle
    @State private var draft = ""
    @State private var sentDraft: ComposerSentDraft?
    @State private var sentDraftSerial: UInt64 = 0
    @StateObject private var quickReactions = TimelineQuickReactionStore()
    @State private var sendError: String?
    @State private var uploadState: MediaUploadState = .idle
    @State private var selectedPhotos: [PhotosPickerItem] = []
    @State private var attachmentDrafts: [ComposerAttachmentDraft] = []
    @State private var attachmentSendSteps: [ComposerAttachmentSendStep]?
    @State private var isSendingMessage = false
    @State private var isComposerFocused = false
    @State private var threadUpdatesTask: Task<Void, Never>?
    @State private var snapshotItems: [TimelineItem] = []
    @State private var pendingReplies: [TimelineItem] = []
    @State private var isLoadingEarlier = false
    @State private var reachedThreadStart = false
    @State private var hasSettledFocusedReply = false
    @State private var focusedReplyEarlierPages = 0
    @State private var notesActionMessage: String?
    @State private var timelineActionMessage: String?
    @State private var reportTarget: TimelineItem?
    @State private var redactConfirmation = TimelineRedactConfirmation<TimelineItem>()
    @State private var reportReason = ""
    @State private var forwardTarget: TimelineItem?
    @State private var reactionsTarget: TimelineReactionsSheetTarget?
    @State private var inFlightTimelineActionKeys: Set<String> = []
    @State private var cryptoStatus: RoomCryptoStatus = .unknown
    @State private var replyTarget: ComposerRelationTarget?
    @State private var editSession: ComposerEditSession?
    @State private var viewerResource: MediaResource?
    /// Bumped to scroll to the newest reply (after a send).
    @State private var scrollToNewestRequest: UInt64 = 0
    @State private var hasPlacedInitialThread = false
    private static let bottomAnchorID = "thread-bottom-anchor"

    var body: some View {
        VStack(spacing: 0) {
            ThreadHeader(
                subtitle: rootTitle ?? roomTitle ?? String(localized: "Room message"),
                onBack: { dismiss() }
            )

            threadContent

            Divider()

            ComposerView(
                roomID: roomID,
                text: $draft,
                placeholder: editSession == nil
                    ? String(localized: "Reply in thread...")
                    : String(localized: "Edit message..."),
                replyTarget: replyTarget,
                editTarget: editSession?.editTarget,
                uploadState: uploadState,
                sendError: sendError,
                onCancelRelation: clearComposerRelation,
                onSend: sendThreadReply,
                onMockMediaUpload: draftMockThreadAttachment,
                onFileURL: draftThreadFile,
                onCameraImage: draftThreadCameraImage,
                onUploadFailed: { message in
                    uploadState = .failed(message)
                },
                selectedPhotos: $selectedPhotos,
                attachmentDrafts: $attachmentDrafts,
                isSending: isSendingMessage,
                onPasteImages: draftThreadPastedImages,
                isFocusedExternally: $isComposerFocused,
                sentDraft: sentDraft,
                onSendGif: sendThreadGif
            )
            .padding(.bottom, canvasLayout == .stacked ? SynaraSpacing.large : 0)
        }
        .background(SynaraColor.surface)
        .ignoresSafeArea(.container, edges: canvasLayout == .stacked ? .bottom : [])
        .navigationBarBackButtonHidden(true)
        .toolbar(.hidden, for: .navigationBar)
        .toolbar(canvasLayout.hidesTabBarInConversation ? .hidden : .automatic, for: .tabBar)
        .task {
            let matrix = environment.matrix
            await quickReactions.refresh { await matrix.recentEmoji() }
        }
        .task(id: roomID + rootEventID) {
            async let status = environment.crypto.roomStatus(roomID: roomID)
            if ThreadComposerFocusRequests.shared.consume(roomID: roomID, rootEventID: rootEventID) {
                isComposerFocused = true
            }
            await loadThread()
            cryptoStatus = await status
            startThreadUpdates()
        }
        .onDisappear {
            threadUpdatesTask?.cancel()
            threadUpdatesTask = nil
            environment.timeline.closeThreadTimeline(roomID: roomID, rootEventID: rootEventID)
        }
        .onChange(of: selectedPhotos) { items in
            draftThreadPickedPhotos(items)
        }
        .onReceive(environment.outgoingSends.queue.$items) { _ in
            refreshPendingReplies()
        }
        .sheet(item: $viewerResource) { resource in
            MediaViewer(resource: resource)
        }
        .alert("Personal Notes", isPresented: notesActionMessageBinding) {
            Button("OK", role: .cancel) { notesActionMessage = nil }
        } message: {
            Text(notesActionMessage ?? String(localized: "Try again."))
        }
        .sheet(item: $reactionsTarget) { target in
            let items: [TimelineItem] = {
                guard case let .loaded(items, _) = state else { return [] }
                return items
            }()
            TimelineReactionsSheet(
                item: items.first { $0.eventID == target.item.eventID } ?? target.item,
                initialKey: target.initialKey,
                currentUserID: currentUserID,
                roomID: roomID,
                directory: TimelineReactorDirectory(items: items),
                onDismiss: { reactionsTarget = nil }
            )
            .environment(\.appEnvironment, environment)
        }
        .sheet(item: $forwardTarget) { item in
            TimelineForwardSheet(
                sourceRoomID: roomID,
                sourceEncryption: cryptoStatus.encryption,
                sourceTransport: item.forwardTransport,
                roomList: environment.roomList,
                onForward: { targetRoomID, asQuote, confirmedEncryptionDowngrade in
                    await performForwardAction(
                        targetRoomID: targetRoomID,
                        asQuote: asQuote,
                        confirmedEncryptionDowngrade: confirmedEncryptionDowngrade,
                        to: item,
                    )
                }
            )
        }
        .timelineRedactConfirmation($redactConfirmation) { item in
            applyAction(.redact, to: item)
        }
        .alert("Report Message", isPresented: Binding(
            get: { reportTarget != nil },
            set: {
                if !$0 {
                    reportTarget = nil
                    reportReason = ""
                }
            }
        )) {
            TextField("Reason (optional)", text: Binding(
                get: { reportReason },
                set: { reportReason = String($0.prefix(512)) }
            ))
            Button("Cancel", role: .cancel) {
                reportTarget = nil
                reportReason = ""
            }
            Button("Report", role: .destructive) {
                guard let item = reportTarget else { return }
                let trimmedReason = reportReason.trimmingCharacters(in: .whitespacesAndNewlines)
                reportTarget = nil
                reportReason = ""
                applyAction(
                    .report(reason: trimmedReason.isEmpty ? nil : trimmedReason),
                    to: item,
                    successMessage: String(localized: "Message reported.")
                )
            }
        } message: {
            Text("The report is sent to your homeserver administrators.")
        }
        .alert("Message Action", isPresented: Binding(
            get: { timelineActionMessage != nil },
            set: { if !$0 { timelineActionMessage = nil } }
        )) {
            Button("OK") { timelineActionMessage = nil }
        } message: {
            Text(timelineActionMessage ?? "")
        }
    }

    @ViewBuilder
    private var threadContent: some View {
        switch state {
        case .idle, .loading:
            ScrollView {
                SynaraTimelineSkeletonList(rowCount: 4)
                    .padding(.horizontal, SynaraSpacing.xLarge)
                    .padding(.vertical, SynaraSpacing.large)
            }
            .accessibilityIdentifier("ThreadLoading")
        case .empty:
            SynaraEmptyState(
                title: String(localized: "No Thread Replies"),
                systemImage: "bubble.left.and.bubble.right",
                message: String(localized: "Replies will appear here.")
            )
        case let .failed(message):
            SynaraErrorState(title: String(localized: "Could Not Load Thread"), message: message) {
                Task {
                    await loadThread()
                }
            }
        case let .loaded(items, _):
            let visibleItems = threadItems(from: items)
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                        if canLoadEarlier(visibleItems) {
                            Button(action: loadEarlierReplies) {
                                if isLoadingEarlier {
                                    ProgressView().controlSize(.small)
                                } else {
                                    Label("Load earlier replies", systemImage: "arrow.up")
                                }
                            }
                            .font(SynaraTypography.chipLabel)
                            .foregroundStyle(SynaraColor.accent)
                            .buttonStyle(.plain)
                            .disabled(isLoadingEarlier)
                            .frame(maxWidth: .infinity)
                            .accessibilityIdentifier("ThreadLoadEarlier")
                        }
                        let dayHeaders = TimelineDayHeaderPolicy.headers(for: visibleItems)
                        let replyPreviews = TimelineReplyPreview.previewsByEventID(
                            in: visibleItems,
                            currentUserID: currentUserID
                        )
                        let reactorDirectory = TimelineReactorDirectory(items: visibleItems)
                        ForEach(Array(visibleItems.enumerated()), id: \.element.id) { index, item in
                            if let day = dayHeaders[item.eventID] {
                                TimelineDayHeader(day: day)
                                    .padding(.vertical, SynaraSpacing.xSmall)
                            }
                            threadRow(
                                item,
                                isGroupedWithPrevious: TimelineMessageGroupingPolicy.shouldGroup(
                                    previous: index > 0 ? visibleItems[index - 1] : nil,
                                    current: item
                                ),
                                replyPreviews: replyPreviews,
                                reactorNames: reactorDirectory.names(for: item)
                            )
                            .id(item.id)
                        }
                        Color.clear
                            .frame(height: 1)
                            .id(Self.bottomAnchorID)
                            .accessibilityHidden(true)
                    }
                    .padding(.horizontal, SynaraSpacing.medium)
                    .padding(.top, SynaraSpacing.large)
                    .padding(.bottom, SynaraSpacing.large)
                }
                .scrollDismissesKeyboard(.interactively)
                .accessibilityIdentifier("ThreadTimelineList")
                .onAppear {
                    anchorFocusedReply(in: visibleItems, proxy: proxy)
                    placeInitialThread(items: visibleItems, proxy: proxy)
                }
                .onChange(of: visibleItems.map(\.id)) { _ in
                    anchorFocusedReply(in: visibleItems, proxy: proxy)
                    placeInitialThread(items: visibleItems, proxy: proxy)
                }
                .onChange(of: scrollToNewestRequest) { _ in
                    DispatchQueue.main.async {
                        withAnimation(.easeOut(duration: 0.2)) {
                            proxy.scrollTo(Self.bottomAnchorID, anchor: .bottom)
                        }
                    }
                }
                .onChange(of: isLoadingEarlier) { loading in
                    if loading == false {
                        anchorFocusedReply(in: visibleItems, proxy: proxy)
                    }
                }
            }
        }
    }

    /// The same row as the room timeline, with the same actions; a reply
    /// in a thread stays in this thread.
    private func threadRow(
        _ item: TimelineItem,
        isGroupedWithPrevious: Bool,
        replyPreviews: [String: TimelineReplyPreview],
        reactorNames: [String: String]
    ) -> some View {
        TimelineRow(
            item: item,
            currentUserID: currentUserID,
            timeText: item.timestamp.timelineTime,
            isGroupedWithPrevious: isGroupedWithPrevious,
            isTimestampRevealed: false,
            animateSend: false,
            replyPreview: TimelineRelationPresentation.replyPreview(
                for: item,
                locallyResolvedByEventID: replyPreviews
            ),
            replyCount: 0,
            availability: environment.eventActions.availability(
                for: item,
                currentUserID: currentUserID,
                roomID: roomID
            ),
            onReply: { beginReply(item) },
            onReplyInThread: { beginReply(item) },
            onOpenThread: {},
            isInlineThreadMode: false,
            isThreadExpanded: false,
            inlineThread: nil,
            onEdit: { beginEdit(item) },
            onRedact: { redactConfirmation.request(item) },
            onToggleReaction: { key in toggleReaction(key, on: item) },
            quickReactions: quickReactions,
            onReport: { beginReport(item) },
            onForward: { forwardTarget = item },
            onPollVote: { answerIDs in
                await performAction(
                    .pollVote(answerIDs: answerIDs),
                    to: item,
                    successMessage: String(localized: "Vote submitted.")
                )
            },
            onDeclineCall: {
                await performAction(
                    .declineCall,
                    to: item,
                    successMessage: String(localized: "Call declined.")
                )
            },
            onPinToNotes: { pinToNotes(item) },
            onOpenMedia: { resource in viewerResource = resource },
            onAgentAction: { _ in },
            onAgentApprovalReaction: { _ in },
            onRetryFailedSend: { retryFailedReply(item) },
            onDiscardFailedSend: { discardFailedReply(item) },
            reactorNames: reactorNames,
            onShowReactions: { key in reactionsTarget = TimelineReactionsSheetTarget(item: item, initialKey: key) }
        )
    }

    /// A thread opened without a routed reply starts at its newest reply,
    /// like the room timeline.
    private func placeInitialThread(items: [TimelineItem], proxy: ScrollViewProxy) {
        guard focusedEventID == nil, hasPlacedInitialThread == false, items.isEmpty == false else {
            return
        }
        hasPlacedInitialThread = true
        DispatchQueue.main.async {
            proxy.scrollTo(Self.bottomAnchorID, anchor: .bottom)
        }
    }

    /// Scroll a routed thread to its reply, loading earlier pages until the
    /// reply is in the window or the policy gives up.
    private func anchorFocusedReply(in items: [TimelineItem], proxy: ScrollViewProxy) {
        guard let focusedEventID, hasSettledFocusedReply == false, isLoadingEarlier == false else {
            return
        }
        switch ThreadFocusAnchorPolicy.nextStep(
            focusedEventID: focusedEventID,
            items: items,
            canLoadEarlier: canLoadEarlier(items),
            earlierPagesLoaded: focusedReplyEarlierPages
        ) {
        case let .scroll(itemID):
            hasSettledFocusedReply = true
            DispatchQueue.main.async {
                proxy.scrollTo(itemID, anchor: .center)
            }
        case .loadEarlier:
            focusedReplyEarlierPages += 1
            loadEarlierReplies()
        case .settle:
            hasSettledFocusedReply = true
        }
    }

    private func loadThread() async {
        state = .loading
        let signpostID = PerformanceTrace.begin("ThreadTimelineLoad")
        defer {
            PerformanceTrace.end("ThreadTimelineLoad", id: signpostID)
        }
        let outcome = await environment.timeline.loadThreadTimeline(roomID: roomID, rootEventID: rootEventID)
        await MainActor.run {
            applyThreadOutcome(outcome)
        }
    }

    private func startThreadUpdates() {
        threadUpdatesTask?.cancel()
        threadUpdatesTask = Task {
            for await outcome in environment.timeline.threadTimelineUpdates(
                roomID: roomID,
                rootEventID: rootEventID
            ) {
                guard Task.isCancelled == false else {
                    return
                }
                await MainActor.run {
                    switch outcome {
                    case let .loaded(items):
                        applyThreadOutcome(.loaded(items))
                    case .empty:
                        if case .loading = state {
                            state = .empty
                        }
                    case let .failed(failure):
                        if case .loaded = state {
                            return
                        }
                        state = .failed(failure.userMessage)
                    }
                }
            }
        }
    }

    private func applyThreadOutcome(_ outcome: TimelineLoadOutcome) {
        switch outcome {
        case let .loaded(items):
            snapshotItems = items
            pendingReplies = ThreadTimelineMerge.unresolved(pending: pendingReplies, snapshot: items)
            // The room timeline never shows thread replies, so this thread
            // confirms its own queued sends.
            environment.outgoingSends.dropConfirmed(matching: items)
            if items.contains(where: { $0.eventID == rootEventID }) {
                reachedThreadStart = true
            }
            showMergedItems()
        case .empty:
            showMergedItems()
        case let .failed(failure):
            state = .failed(failure.userMessage)
        }
    }

    /// Uploads this screen sent, plus this thread's queued text sends: each
    /// shows once, until the thread stream has its local or remote echo.
    private var pendingThreadItems: [TimelineItem] {
        pendingReplies + environment.outgoingSends.queue.timelineItems(
            in: roomID,
            threadRootEventID: rootEventID
        )
    }

    private func showMergedItems() {
        let merged = ThreadTimelineMerge.merged(snapshot: snapshotItems, pending: pendingThreadItems)
        state = merged.isEmpty ? .empty : .loaded(merged, isPaginating: false)
    }

    private func refreshPendingReplies() {
        switch state {
        case .loaded, .empty:
            showMergedItems()
        case .idle, .loading, .failed:
            break
        }
    }

    /// The thread stream reaches its start when the root is loaded.
    private func canLoadEarlier(_ items: [TimelineItem]) -> Bool {
        reachedThreadStart == false && items.contains(where: { $0.eventID == rootEventID }) == false
    }

    private func loadEarlierReplies() {
        guard isLoadingEarlier == false else { return }
        isLoadingEarlier = true
        Task {
            let outcome = await environment.timeline.loadOlderThreadTimeline(
                roomID: roomID,
                rootEventID: rootEventID
            )
            await MainActor.run {
                switch outcome {
                case .loaded:
                    applyThreadOutcome(outcome)
                case .empty:
                    reachedThreadStart = true
                    if focusedEventID != nil, hasSettledFocusedReply == false {
                        hasSettledFocusedReply = true
                    }
                case let .failed(failure):
                    timelineActionMessage = failure.userMessage
                }
                isLoadingEarlier = false
            }
        }
    }

    private func threadItems(from items: [TimelineItem]) -> [TimelineItem] {
        items.filter { TimelineThreadMembership.contains($0, rootEventID: rootEventID) }
    }

    private var notesActionMessageBinding: Binding<Bool> {
        Binding(
            get: { notesActionMessage != nil },
            set: { if $0 == false { notesActionMessage = nil } }
        )
    }

    private var currentUserID: String {
        if case let .signedIn(session) = environment.session.currentState {
            return session.userID
        }
        return "@local:matrix.org"
    }

    private func beginReport(_ item: TimelineItem) {
        reportReason = ""
        reportTarget = item
    }

    /// A pill or quick reaction: toggles the user's reaction, and records
    /// an added one in the recent-emoji list desktop shares.
    private func toggleReaction(_ key: String, on item: TimelineItem) {
        let isAdding = item.reactionOwnership.contains(key) == false
        let matrix = environment.matrix
        Task {
            let succeeded = await performAction(.react(key), to: item)
            if succeeded, isAdding {
                await quickReactions.recordUse(key) { await matrix.recordRecentEmoji($0) }
            }
        }
    }

    private func applyAction(
        _ action: EventActionType,
        to item: TimelineItem,
        successMessage: String? = nil
    ) {
        Task {
            _ = await performAction(action, to: item, successMessage: successMessage)
        }
    }

    @MainActor
    private func performAction(
        _ action: EventActionType,
        to item: TimelineItem,
        successMessage: String? = nil
    ) async -> Bool {
        let inFlightKey = "\(item.eventID):\(action.inFlightKey)"
        guard inFlightTimelineActionKeys.insert(inFlightKey).inserted else {
            return false
        }
        defer { inFlightTimelineActionKeys.remove(inFlightKey) }
        do {
            _ = try await environment.eventActions.apply(
                action,
                to: item,
                currentUserID: currentUserID,
                roomID: roomID
            )
            if let successMessage {
                timelineActionMessage = successMessage
            }
            return true
        } catch {
            timelineActionMessage = error.localizedDescription
            return false
        }
    }

    @MainActor
    private func performForwardAction(
        targetRoomID: String,
        asQuote: Bool,
        confirmedEncryptionDowngrade: Bool,
        to item: TimelineItem
    ) async -> EventActionError? {
        let action = EventActionType.forward(
            targetRoomID: targetRoomID,
            asQuote: asQuote,
            confirmedEncryptionDowngrade: confirmedEncryptionDowngrade
        )
        let inFlightKey = "\(item.eventID):\(action.inFlightKey)"
        guard inFlightTimelineActionKeys.insert(inFlightKey).inserted else {
            return .alreadyInProgress
        }
        defer { inFlightTimelineActionKeys.remove(inFlightKey) }
        do {
            _ = try await environment.eventActions.apply(
                action,
                to: item,
                currentUserID: currentUserID,
                roomID: roomID
            )
            timelineActionMessage = String(localized: "Message forwarded.")
            return nil
        } catch let error as EventActionError {
            return error
        } catch {
            return .failed
        }
    }

    private func pinToNotes(_ item: TimelineItem) {
        Task {
            let result = await environment.roomNotes.pinMessage(roomID: roomID, item: item)
            await MainActor.run {
                switch result {
                case .success:
                    notesActionMessage = String(localized: "Message pinned to your private room notes.")
                case .failure(let error):
                    notesActionMessage = error.errorDescription ?? String(localized: "The message could not be pinned.")
                }
            }
        }
    }

    private func sendThreadReply(body rawBody: String) {
        let body = rawBody.trimmingCharacters(in: .whitespacesAndNewlines)
        let drafts = attachmentDrafts
        guard ComposerAttachmentDraftList.canBeginSend(
            isSending: isSendingMessage,
            text: body,
            drafts: drafts
        ) else {
            if isSendingMessage == false {
                sendError = MessageSendError.emptyMessage.localizedDescription
            }
            return
        }

        sendError = nil
        isSendingMessage = true
        // The relation belongs to the send gesture, not to composer state
        // after an upload has started.
        let intent = ComposerEditFlow.sendIntent(
            body: rawBody,
            replyToEventID: replyTarget?.eventID,
            threadRootEventID: rootEventID,
            session: editSession
        )
        let plan = ComposerAttachmentSendPlan.reusableOrNew(
            existing: attachmentSendSteps,
            drafts: drafts,
            body: rawBody
        )
        attachmentSendSteps = plan
        Task {
            var uploaded = true
            if drafts.isEmpty == false {
                let uploadSignpostID = PerformanceTrace.begin("ThreadComposerAttachmentDraftSend")
                uploaded = await ComposerAttachmentSend.uploadAll(
                    drafts,
                    steps: plan,
                    roomID: roomID,
                    replyToEventID: intent.replyToEventID,
                    threadRootEventID: rootEventID,
                    uploader: environment.mediaUploader,
                    onState: { state in
                        uploadState = state
                    },
                    onUploaded: { draft, item in
                        attachmentDrafts = ComposerAttachmentDraftList.remove(id: draft.id, from: attachmentDrafts)
                        attachmentSendSteps = ComposerAttachmentSendPlan.removingAttachment(
                            id: draft.id,
                            from: attachmentSendSteps ?? plan
                        )
                        append(item)
                    }
                )
                PerformanceTrace.end("ThreadComposerAttachmentDraftSend", id: uploadSignpostID)
            }

            guard uploaded else {
                await MainActor.run {
                    isSendingMessage = false
                }
                return
            }

            guard let trailingText = ComposerAttachmentSendPlan.trailingText(in: plan) else {
                await MainActor.run {
                    attachmentSendSteps = nil
                    uploadState = .idle
                    finishComposerSend(sentText: rawBody)
                    isSendingMessage = false
                }
                return
            }

            await MainActor.run {
                attachmentSendSteps = nil
                isSendingMessage = false
            }
            await sendThreadText(intent.replacingBody(with: trailingText), sentText: rawBody)
        }
    }

    /// Text goes through the same durable outbox as the room composer, so a
    /// thread reply has the same Sending / Failed state, Retry and Discard,
    /// and shows once: the queued row gives way to the SDK echo with the
    /// same transaction id.
    @MainActor
    private func sendThreadText(_ intent: ComposerSendIntent, sentText: String) async {
        let draftBody = intent.body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard draftBody.isEmpty == false else {
            sendError = MessageSendError.emptyMessage.localizedDescription
            return
        }
        let formattedBody = ComposerMatrixFormatting.formattedBody(for: draftBody)
        let body = ComposerMatrixFormatting.plainBody(for: draftBody, formattedBody: formattedBody)

        if let editEventID = intent.editEventID {
            do {
                let signpostID = PerformanceTrace.begin("ThreadMessageSend")
                defer {
                    PerformanceTrace.end("ThreadMessageSend", id: signpostID)
                }
                _ = try await environment.messageSender.send(
                    MessageSendRequest(
                        roomID: roomID,
                        body: body,
                        formattedBody: formattedBody,
                        replyToEventID: nil,
                        editEventID: editEventID,
                        threadRootEventID: rootEventID
                    )
                )
                finishComposerSend(sentText: sentText)
                completeComposerRelation()
                sendError = nil
            } catch {
                sendError = MessageSendError.failed.localizedDescription
                SynaraHaptics.trigger(.warning)
            }
            return
        }

        let clientTransactionID = OutgoingQueuedMessage.newClientTransactionID()
        let queued = environment.outgoingSends.enqueue(
            localID: intent.retrying?.id ?? "$pending-\(clientTransactionID)",
            clientTransactionID: clientTransactionID,
            roomID: roomID,
            body: body,
            formattedBody: formattedBody,
            replyToEventID: intent.replyToEventID,
            threadRootEventID: rootEventID,
            senderID: currentUserID,
            timestamp: intent.retrying?.timestamp ?? Date()
        )
        finishComposerSend(sentText: sentText)
        completeComposerRelation()
        sendError = nil
        refreshPendingReplies()
        scrollToNewestRequest &+= 1
        await transmitThreadReply(id: queued.id)
    }

    /// As in the room composer: the GIF's provider URL as a plain thread
    /// reply, never re-uploaded, keeping the typed draft.
    private func sendThreadGif(_ gif: GifResult) {
        guard isSendingMessage == false, editSession == nil else {
            return
        }
        let clientTransactionID = OutgoingQueuedMessage.newClientTransactionID()
        let queued = environment.outgoingSends.enqueue(
            localID: "$pending-\(clientTransactionID)",
            clientTransactionID: clientTransactionID,
            roomID: roomID,
            body: gif.url,
            formattedBody: nil,
            replyToEventID: replyTarget?.eventID,
            threadRootEventID: rootEventID,
            senderID: currentUserID,
            timestamp: Date()
        )
        replyTarget = nil
        sendError = nil
        refreshPendingReplies()
        scrollToNewestRequest &+= 1
        Task {
            await transmitThreadReply(id: queued.id)
        }
    }

    private func transmitThreadReply(id: String) async {
        let signpostID = PerformanceTrace.begin("ThreadMessageSend")
        defer {
            PerformanceTrace.end("ThreadMessageSend", id: signpostID)
        }
        await environment.outgoingSends.flush()
        await MainActor.run {
            refreshPendingReplies()
            switch environment.outgoingSends.queue.item(id: id)?.deliveryStatus {
            case .failed:
                SynaraHaptics.trigger(.warning)
            case .sent:
                SynaraHaptics.trigger(.lightImpact)
            case .queued, .sending, .none:
                break
            }
        }
    }

    private func retryFailedReply(_ item: TimelineItem) {
        guard item.isLocalPending else {
            // An SDK local echo: Core retries the same request.
            sendError = nil
            Task {
                let retried = await environment.outgoingSends.retryLocalEcho(item, roomID: roomID)
                await MainActor.run {
                    if retried == false {
                        sendError = MessageSendError.failed.localizedDescription
                        SynaraHaptics.trigger(.warning)
                    }
                }
            }
            return
        }
        guard let queued = environment.outgoingSends.retry(
            item,
            roomID: roomID,
            senderID: currentUserID
        ) else {
            return
        }
        sendError = nil
        refreshPendingReplies()
        Task {
            await transmitThreadReply(id: queued.id)
        }
    }

    private func discardFailedReply(_ item: TimelineItem) {
        sendError = nil
        Task {
            let discarded = await environment.outgoingSends.discard(item, roomID: roomID)
            await MainActor.run {
                switch discarded {
                case .some(true):
                    // The SDK drops a discarded echo with its next update.
                    snapshotItems.removeAll { $0.id == item.id }
                    pendingReplies.removeAll { $0.id == item.id }
                    refreshPendingReplies()
                case .some(false):
                    break
                case .none:
                    sendError = String(localized: "The unsent message could not be discarded.")
                    SynaraHaptics.trigger(.warning)
                }
            }
        }
    }

    private func beginReply(_ item: TimelineItem) {
        guard isSendingMessage == false else {
            return
        }
        attachmentSendSteps = nil
        if editSession != nil {
            cancelEdit()
        }
        replyTarget = ComposerRelationTarget(item: item, kind: .reply, currentUserID: currentUserID)
        isComposerFocused = true
    }

    private func beginEdit(_ item: TimelineItem) {
        guard isSendingMessage == false else {
            return
        }
        attachmentSendSteps = nil
        let session = ComposerEditFlow.begin(
            item: item,
            currentUserID: currentUserID,
            currentDraft: editSession?.previousDraft ?? draft
        )
        replyTarget = nil
        editSession = session
        if TimelinePendingReconciler.messageBody(for: item) != nil {
            draft = session.draft
        }
        isComposerFocused = true
    }

    private func clearComposerRelation() {
        attachmentSendSteps = nil
        if editSession != nil {
            cancelEdit()
        }
        replyTarget = nil
    }

    private func completeComposerRelation() {
        attachmentSendSteps = nil
        replyTarget = nil
        editSession = nil
    }

    /// As in the room composer: drop the sent text, keep what was typed since.
    private func finishComposerSend(sentText: String) {
        sentDraftSerial &+= 1
        sentDraft = ComposerSentDraft(id: sentDraftSerial, sentText: sentText)
    }

    private func cancelEdit() {
        if let session = editSession {
            draft = ComposerEditFlow.cancel(session)
        }
        editSession = nil
    }

    private func draftMockThreadAttachment(source: MediaUploadSource) {
        let draft: ComposerAttachmentDraft
        switch source {
        case .photoLibrary:
            draft = ComposerAttachmentDraft(
                displayName: "thread-attachment.jpg",
                mimeType: "image/jpeg",
                data: Data("Synara thread attachment".utf8),
                source: source
            )
        case .file:
            draft = ComposerAttachmentDraft(
                displayName: "thread-attachment.pdf",
                mimeType: "application/pdf",
                data: Data("Synara thread file".utf8),
                source: source
            )
        case .camera:
            draft = ComposerAttachmentDraft(
                displayName: "thread-camera.jpg",
                mimeType: "image/jpeg",
                data: Data("Synara thread camera image".utf8),
                source: source
            )
        }
        applyThreadIncomingDrafts([draft])
    }

    private func draftThreadPickedPhotos(_ items: [PhotosPickerItem]) {
        guard items.isEmpty == false else {
            return
        }
        Task {
            var incoming: [ComposerAttachmentDraft] = []
            var loadFailed = false
            incoming.reserveCapacity(items.count)
            for item in items {
                do {
                    guard let data = try await item.loadTransferable(type: Data.self), data.isEmpty == false else {
                        loadFailed = true
                        continue
                    }
                    let contentType = item.supportedContentTypes.first
                    incoming.append(
                        ComposerAttachmentDraft(
                            displayName: "thread-photo.\(contentType?.preferredFilenameExtension ?? "jpg")",
                            mimeType: contentType?.preferredMIMEType ?? "image/jpeg",
                            data: data,
                            source: .photoLibrary
                        )
                    )
                } catch {
                    loadFailed = true
                }
            }
            await MainActor.run {
                selectedPhotos = []
                applyThreadIncomingDrafts(incoming)
                if incoming.isEmpty, loadFailed {
                    uploadState = .failed(String(localized: "Attachment could not be loaded. Try again."))
                }
            }
        }
    }

    private func applyThreadIncomingDrafts(_ incoming: [ComposerAttachmentDraft]) {
        guard isSendingMessage == false else {
            return
        }
        let outcome = ComposerAttachmentDraftList.appending(incoming, to: attachmentDrafts)
        attachmentDrafts = outcome.drafts
        if let rejection = outcome.rejection {
            uploadState = .failed(ComposerAttachmentDraftList.userMessage(for: rejection))
        } else if incoming.isEmpty == false {
            uploadState = .idle
        }
    }

    private func draftThreadFile(_ url: URL) {
        switch ComposerAttachmentDraftList.draft(fromFileURL: url) {
        case let .success(draft):
            applyThreadIncomingDrafts([draft])
        case let .failure(rejection):
            uploadState = .failed(ComposerAttachmentDraftList.userMessage(for: rejection))
        }
    }

    #if canImport(UIKit)
        private func draftThreadCameraImage(_ image: UIImage) {
            guard let data = MediaAttachmentSupport.jpegData(from: image) else {
                uploadState = .failed(String(localized: "Attachment could not be loaded. Try again."))
                return
            }
            applyThreadIncomingDrafts([
                ComposerAttachmentDraft(
                    displayName: "thread-camera.jpg",
                    mimeType: "image/jpeg",
                    data: data,
                    source: .camera
                )
            ])
        }

        private func draftThreadPastedImages(_ images: [UIImage]) {
            var incoming: [ComposerAttachmentDraft] = []
            incoming.reserveCapacity(images.count)
            for (index, image) in images.enumerated() {
                guard let data = MediaAttachmentSupport.jpegData(from: image) else {
                    continue
                }
                let displayName = images.count == 1 ? "thread-paste.jpg" : "thread-paste-\(index + 1).jpg"
                incoming.append(
                    ComposerAttachmentDraft(
                        displayName: displayName,
                        mimeType: "image/jpeg",
                        data: data,
                        source: .photoLibrary
                    )
                )
            }
            if incoming.isEmpty {
                uploadState = .failed(String(localized: "Attachment could not be loaded. Try again."))
                return
            }
            applyThreadIncomingDrafts(incoming)
        }
    #endif

    private func append(_ item: TimelineItem) {
        pendingReplies.append(item)
        showMergedItems()
    }
}

private struct ThreadHeader: View {
    let subtitle: String
    let onBack: () -> Void

    var body: some View {
        ZStack {
            HStack {
                Button(action: onBack) {
                    Image(systemName: "chevron.left")
                        .font(.system(size: 19, weight: .semibold))
                        .frame(width: 38, height: 38)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Back")

                Spacer()

                HStack(spacing: SynaraSpacing.medium) {
                    Image(systemName: "bell")
                    Image(systemName: "ellipsis")
                }
                .font(.system(size: 17, weight: .semibold))
                .foregroundStyle(SynaraColor.primaryText)
            }

            VStack(spacing: 2) {
                Text("Thread")
                    .font(SynaraTypography.sectionTitle.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                    .accessibilityIdentifier("ThreadTimelineTitle")
                Text(subtitle)
                    .font(SynaraTypography.messageMeta)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(1)
            }
            .padding(.horizontal, 80)
        }
        .padding(.horizontal, SynaraSpacing.large)
        .padding(.vertical, SynaraSpacing.medium)
        .background(SynaraColor.surface)
    }
}

private struct MessageTextSelectionSheet: View {
    let payload: TimelineMessageCopy.Payload
    @Environment(\.dismiss) private var dismiss
    @State private var revealsSpoilers = false

    var body: some View {
        let projection = selectionProjection
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: SynaraSpacing.medium) {
                    if projection.containsSpoilers {
                        Button(
                            revealsSpoilers ? "Hide Spoilers" : "Reveal Spoilers",
                            systemImage: revealsSpoilers ? "eye.slash" : "eye"
                        ) {
                            revealsSpoilers.toggle()
                        }
                        .buttonStyle(.bordered)
                        .accessibilityHint(
                            revealsSpoilers
                                ? "Conceals spoiler text again"
                                : "Makes spoiler text available for selection"
                        )
                    }

                    #if canImport(UIKit)
                        SelectableMessageTextView(
                            attributedText: NSAttributedString(
                                attributedRichText(projection.richText, includeLinks: false)
                            ),
                            accessibilityLabel: projection.richText.plainText
                        )
                        .frame(maxWidth: .infinity, alignment: .leading)
                    #else
                        Text(attributedRichText(projection.richText, includeLinks: false))
                            .font(SynaraTypography.messageBody)
                            .foregroundStyle(SynaraColor.primaryText)
                            .lineSpacing(2.5)
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .fixedSize(horizontal: false, vertical: true)
                            .accessibilityLabel(projection.richText.plainText)
                            .accessibilityHint("Select and copy any part of this message")
                    #endif

                    ForEach(Array(projection.codeBlocks.enumerated()), id: \.offset) { _, block in
                        MatrixCodeBlockView(block: block)
                    }
                }
                .padding(SynaraSpacing.large)
            }
            .background(SynaraColor.surface)
            .navigationTitle("Select Text")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done", action: dismiss.callAsFunction)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Copy All", systemImage: "doc.on.doc") {
                        TimelineMessageCopy.copyToPasteboard(payload)
                    }
                    .disabled(projection.containsSpoilers && revealsSpoilers == false)
                    .accessibilityHint(
                        projection.containsSpoilers && revealsSpoilers == false
                            ? "Reveal spoilers before copying the complete message"
                            : "Copies the complete message with its formatting"
                    )
                }
            }
        }
        .presentationDragIndicator(.visible)
        .accessibilityIdentifier("MessageTextSelectionSheet")
    }

    private var selectionProjection: MatrixHTMLRenderer.SelectionProjection {
        guard let html = payload.html else {
            return .init(
                richText: .init(runs: [.init(text: payload.plainText, style: [], link: nil)]),
                containsSpoilers: false,
                codeBlocks: []
            )
        }
        return MatrixHTMLRenderer.selectionProjection(
            body: payload.plainText,
            html: html,
            revealingSpoilers: revealsSpoilers
        )
    }
}

struct MatrixRichTextPresentationContext {
    let readingSurface: KeyPath<SynaraThemeTokens, String>
    let appliesStandardOwnMessageTint: Bool

    static let canvas = MatrixRichTextPresentationContext(
        readingSurface: \SynaraThemeTokens.surface,
        appliesStandardOwnMessageTint: false
    )
    static let otherMessage = MatrixRichTextPresentationContext(
        readingSurface: \SynaraThemeTokens.secondarySurface,
        appliesStandardOwnMessageTint: false
    )
    static let ownMessage = MatrixRichTextPresentationContext(
        readingSurface: \SynaraThemeTokens.surface,
        appliesStandardOwnMessageTint: true
    )

    static func semantic(_ keyPath: KeyPath<SynaraThemeTokens, String>) -> Self {
        MatrixRichTextPresentationContext(
            readingSurface: keyPath,
            appliesStandardOwnMessageTint: false
        )
    }
}

private struct MatrixFormattedMessageView: View {
    let fallbackBody: String
    let font: Font
    let presentationContext: MatrixRichTextPresentationContext
    private let segments: [MatrixHTMLRenderer.Segment]

    init(
        fallbackBody: String,
        html: String,
        font: Font,
        presentationContext: MatrixRichTextPresentationContext = .canvas
    ) {
        self.fallbackBody = fallbackBody
        self.font = font
        self.presentationContext = presentationContext
        segments = MatrixHTMLRenderer.segments(body: fallbackBody, html: html)
    }

    var body: some View {
        if segments.count == 1, case let .richText(text) = segments[0] {
            MatrixRichTextView(
                text: text,
                fallbackBody: fallbackBody,
                font: font,
                presentationContext: presentationContext
            )
        } else {
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                ForEach(identifiedMatrixSegments(segments)) { item in
                    MatrixSemanticSegmentView(
                        segment: item.segment,
                        fallbackBody: fallbackBody,
                        font: font,
                        presentationContext: presentationContext
                    )
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

private struct MatrixSemanticSegmentView: View {
    let segment: MatrixHTMLRenderer.Segment
    let fallbackBody: String
    let font: Font
    let presentationContext: MatrixRichTextPresentationContext

    @ViewBuilder var body: some View {
        switch segment {
        case let .richText(text):
            MatrixRichTextView(
                text: text,
                fallbackBody: fallbackBody,
                font: font,
                presentationContext: presentationContext
            )
        case let .inline(group):
            MatrixInlineGroupView(group: group, font: font, presentationContext: presentationContext)
        case let .heading(block):
            MatrixHeadingBlockView(block: block, presentationContext: presentationContext)
        case let .code(block):
            MatrixCodeBlockView(block: block)
        case let .quote(text):
            MatrixQuoteBlockView(text: text, font: font, presentationContext: presentationContext)
        case let .spoiler(block):
            MatrixSpoilerBlockView(block: block, font: font)
        case let .details(block):
            MatrixDetailsBlockView(block: block, font: font, presentationContext: presentationContext)
        case let .table(block):
            MatrixTableBlockView(block: block, presentationContext: presentationContext)
        }
    }
}

private struct MatrixHeadingBlockView: View {
    let block: MatrixHTMLRenderer.HeadingBlock
    let presentationContext: MatrixRichTextPresentationContext

    var body: some View {
        Text(
            attributedRichText(
                block.content,
                includeHeadingFonts: false,
                presentationContext: presentationContext
            )
        )
            .font(headingFont)
            .foregroundStyle(SynaraColor.primaryText)
            .lineSpacing(2.5)
            .lineLimit(nil)
            .textSelection(.enabled)
            .fixedSize(horizontal: false, vertical: true)
            .frame(maxWidth: .infinity, alignment: .leading)
            .accessibilityAddTraits(.isHeader)
    }

    private var headingFont: Font {
        switch block.level {
        case 1: return SynaraTypography.messageBody.weight(.bold)
        case 2: return SynaraTypography.messageBody.weight(.bold)
        case 3: return SynaraTypography.messageBody.weight(.semibold)
        case 4: return SynaraTypography.messageBody.weight(.semibold)
        case 5: return SynaraTypography.messageMeta.weight(.semibold)
        default: return SynaraTypography.messageMeta.weight(.medium)
        }
    }
}

private struct IdentifiedMatrixSegment: Identifiable {
    let id: String
    let segment: MatrixHTMLRenderer.Segment
}

/// Content-derived identities preserve disclosure/spoiler state when an
/// unrelated timeline update reconstructs the message view. The occurrence
/// suffix keeps duplicate semantic blocks distinct without relying on hashes.
private func identifiedMatrixSegments(
    _ segments: [MatrixHTMLRenderer.Segment]
) -> [IdentifiedMatrixSegment] {
    var occurrences: [String: Int] = [:]
    return segments.map { segment in
        let signature = matrixSegmentSignature(segment)
        let occurrence = occurrences[signature, default: 0]
        occurrences[signature] = occurrence + 1
        return IdentifiedMatrixSegment(
            id: "\(signature)\u{0}\(occurrence)",
            segment: segment
        )
    }
}

private func matrixSegmentSignature(_ segment: MatrixHTMLRenderer.Segment) -> String {
    switch segment {
    case let .richText(text): return "text\u{0}\(text.plainText)"
    case let .inline(group):
        return "inline\u{0}" + group.pieces.map { piece in
            switch piece {
            case let .richText(text): return "text:\(text.plainText)"
            case let .spoiler(block): return "spoiler:\(block.reason ?? ""):\(block.content.plainText)"
            }
        }.joined(separator: "\u{1}")
    case let .heading(block): return "heading:\(block.level)\u{0}\(block.content.plainText)"
    case let .code(block): return "code:\(block.language ?? "")\u{0}\(block.code)"
    case let .quote(text): return "quote\u{0}\(text.plainText)"
    case let .spoiler(block): return "spoiler:\(block.reason ?? "")\u{0}\(block.content.plainText)"
    case let .details(block): return "details\u{0}\(block.summary)\u{1}\(block.body)"
    case let .table(block):
        return "table\u{0}" + block.rows.map { $0.cells.map(\.plainText).joined(separator: "\u{2}") }
            .joined(separator: "\u{1}")
    }
}

private struct MatrixRichTextView: View {
    let text: MatrixHTMLRenderer.RichText
    let fallbackBody: String
    let font: Font
    let presentationContext: MatrixRichTextPresentationContext

    var body: some View {
        let displayText = text.runs.isEmpty
            ? MatrixHTMLRenderer.RichText(runs: [.init(text: fallbackBody, style: [], link: nil)])
            : text
        Text(attributedRichText(displayText, presentationContext: presentationContext))
            .font(font)
            .foregroundStyle(SynaraColor.primaryText)
            .lineSpacing(2.5)
            .lineLimit(nil)
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
            .fixedSize(horizontal: false, vertical: true)
    }
}

private let matrixInlineSpoilerScheme = "synara-inline-spoiler"

func matrixInlineSpoilerURL(index: Int) -> URL {
    URL(string: "\(matrixInlineSpoilerScheme)://reveal/\(index)")!
}

func matrixInlineSpoilerIndex(_ url: URL) -> Int? {
    guard url.scheme == matrixInlineSpoilerScheme,
          url.host == "reveal",
          let component = url.pathComponents.last,
          let index = Int(component), index >= 0
    else { return nil }
    return index
}

func matrixInlineAttributedText(
    _ group: MatrixHTMLRenderer.InlineGroup,
    revealedSpoilers: Set<Int>,
    presentationContext: MatrixRichTextPresentationContext = .canvas
) -> AttributedString {
    var output = AttributedString()
    var spoilerIndex = 0
    for piece in group.pieces {
        switch piece {
        case let .richText(text):
            output.append(attributedRichText(text, presentationContext: presentationContext))
        case let .spoiler(block):
            if revealedSpoilers.contains(spoilerIndex) {
                output.append(
                    attributedRichText(
                        block.content,
                        // Inline reveals remain part of the surrounding Text;
                        // unlike block spoilers, they do not paint a separate
                        // well. Resolve authored colors against that actual
                        // parent surface rather than an unpainted spoiler fill.
                        presentationContext: presentationContext
                    )
                )
            } else {
                let label = block.reason.map { String(localized: "[Spoiler: \($0) · Reveal]") }
                    ?? String(localized: "[Spoiler · Reveal]")
                var placeholder = AttributedString(label)
                placeholder.link = matrixInlineSpoilerURL(index: spoilerIndex)
                placeholder.foregroundColor = SynaraColor.secondaryText
                placeholder.backgroundColor = SynaraColor.richTextSpoilerBackground
                output.append(placeholder)
            }
            spoilerIndex += 1
        }
    }
    return output
}

private struct MatrixInlineGroupView: View {
    let group: MatrixHTMLRenderer.InlineGroup
    let font: Font
    let presentationContext: MatrixRichTextPresentationContext
    @State private var revealedSpoilers: Set<Int> = []

    var body: some View {
        Text(
            matrixInlineAttributedText(
                group,
                revealedSpoilers: revealedSpoilers,
                presentationContext: presentationContext
            )
        )
            .font(font)
            .foregroundStyle(SynaraColor.primaryText)
            .lineSpacing(2.5)
            .lineLimit(nil)
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
            .fixedSize(horizontal: false, vertical: true)
            .environment(\.openURL, OpenURLAction { url in
                guard let index = matrixInlineSpoilerIndex(url) else { return .systemAction }
                revealedSpoilers.insert(index)
                return .handled
            })
    }
}

private struct MatrixSpoilerBlockView: View {
    let block: MatrixHTMLRenderer.SpoilerBlock
    let font: Font
    @State private var isRevealed = false

    var body: some View {
        if isRevealed {
            Text(
                attributedRichText(
                    block.content,
                    presentationContext: .semantic(\SynaraThemeTokens.richTextSpoilerBackground)
                )
            )
            .font(font)
            .foregroundStyle(SynaraColor.primaryText)
            .lineSpacing(2.5)
            .textSelection(.enabled)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.horizontal, SynaraSpacing.small)
            .padding(.vertical, SynaraSpacing.xSmall)
            .background(SynaraColor.richTextSpoilerBackground)
            .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.small, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: SynaraRadius.small, style: .continuous)
                    .stroke(SynaraColor.richTextSpoilerBorder, lineWidth: 1)
            }
        } else {
            Button {
                isRevealed = true
            } label: {
                Text(
                    block.reason.map { String(localized: "Spoiler: \($0) · Reveal") }
                        ?? String(localized: "Spoiler · Reveal")
                )
                    .font(font)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .padding(.horizontal, SynaraSpacing.small)
                    .padding(.vertical, SynaraSpacing.xSmall)
                    .background(SynaraColor.richTextSpoilerBackground, in: Capsule())
                    .overlay {
                        Capsule()
                            .stroke(SynaraColor.richTextSpoilerBorder, lineWidth: 1)
                    }
            }
            .buttonStyle(.plain)
            .accessibilityLabel(
                block.reason.map { String(localized: "Reveal spoiler: \($0)") } ?? String(localized: "Reveal spoiler")
            )
        }
    }
}

private struct MatrixTableBlockView: View {
    let block: MatrixHTMLRenderer.TableBlock
    let presentationContext: MatrixRichTextPresentationContext

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            if let captionInlineContent = block.captionInlineContent {
                MatrixInlineGroupView(
                    group: captionInlineContent,
                    font: SynaraTypography.messageBody.weight(.semibold),
                    presentationContext: presentationContext
                )
                    .foregroundStyle(SynaraColor.primaryText)
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityAddTraits(.isHeader)
            } else if let caption = block.caption {
                Text(attributedRichText(caption, presentationContext: presentationContext))
                    .font(SynaraTypography.messageBody.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityAddTraits(.isHeader)
            }

            ScrollView(.horizontal, showsIndicators: true) {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(block.rows.enumerated()), id: \.offset) { rowIndex, row in
                        HStack(alignment: .top, spacing: 0) {
                            ForEach(Array(row.cells.enumerated()), id: \.offset) { cellIndex, cell in
                                Group {
                                    if let inlineContent = cell.inlineContent {
                                        MatrixInlineGroupView(
                                            group: inlineContent,
                                            font: cell.isHeader
                                                ? SynaraTypography.messageBody.weight(.semibold)
                                                : SynaraTypography.messageBody,
                                            presentationContext: .semantic(
                                                tableRowToken(
                                                    rowIndex: rowIndex,
                                                    isHeader: row.isHeader
                                                )
                                            )
                                        )
                                    } else {
                                        Text(
                                            attributedRichText(
                                                cell.content,
                                                presentationContext: .semantic(
                                                    tableRowToken(
                                                        rowIndex: rowIndex,
                                                        isHeader: row.isHeader
                                                    )
                                                )
                                            )
                                        )
                                    }
                                }
                                .font(
                                    cell.isHeader
                                        ? SynaraTypography.messageBody.weight(.semibold)
                                        : SynaraTypography.messageBody
                                )
                                .foregroundStyle(SynaraColor.primaryText)
                                .lineLimit(nil)
                                .fixedSize(horizontal: false, vertical: true)
                                .frame(
                                    minWidth: cellIndex == 0 ? 128 : 176,
                                    idealWidth: cellIndex == 0 ? 156 : 220,
                                    maxWidth: cellIndex == 0 ? 220 : 320,
                                    alignment: .topLeading
                                )
                                .padding(.horizontal, SynaraSpacing.medium)
                                .padding(.vertical, 10)
                                .accessibilityLabel(
                                    tableCellAccessibilityLabel(
                                        cell.plainText,
                                        rowIndex: rowIndex,
                                        cellIndex: cellIndex,
                                        isHeader: cell.isHeader
                                    )
                                )
                                .accessibilityAddTraits(cell.isHeader ? .isHeader : [])

                                if cellIndex < row.cells.count - 1 {
                                    Divider()
                                }
                            }
                        }
                        .background(tableRowSurface(rowIndex: rowIndex, isHeader: row.isHeader))
                        .accessibilityElement(children: .contain)

                        if rowIndex < block.rows.count - 1 {
                            Divider()
                        }
                    }
                }
            }
            .background(SynaraColor.richTextTableOdd)
            .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous)
                    .stroke(SynaraColor.separator, lineWidth: 1)
            }
        }
        .textSelection(.enabled)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Message table")
    }

    private func tableRowSurface(rowIndex: Int, isHeader: Bool) -> Color {
        if isHeader {
            return SynaraColor.richTextTableHeader
        }
        return rowIndex.isMultiple(of: 2)
            ? SynaraColor.richTextTableOdd
            : SynaraColor.richTextTableEven
    }

    private func tableRowToken(
        rowIndex: Int,
        isHeader: Bool
    ) -> KeyPath<SynaraThemeTokens, String> {
        if isHeader {
            return \SynaraThemeTokens.richTextTableHeader
        }
        return rowIndex.isMultiple(of: 2)
            ? \SynaraThemeTokens.richTextTableOdd
            : \SynaraThemeTokens.richTextTableEven
    }

    private func tableCellAccessibilityLabel(
        _ cell: String,
        rowIndex: Int,
        cellIndex: Int,
        isHeader: Bool
    ) -> String {
        if isHeader {
            return String(localized: "Column \(cellIndex + 1), \(cell)")
        }
        let headers = block.rows.first(where: \.isHeader)?.cells ?? []
        let header = headers.indices.contains(cellIndex) ? headers[cellIndex].plainText : nil
        if let header, header.isEmpty == false {
            return String(localized: "Row \(rowIndex + 1), \(header): \(cell)")
        }
        return String(localized: "Row \(rowIndex + 1), column \(cellIndex + 1): \(cell)")
    }
}

private struct MatrixQuoteBlockView: View {
    let text: MatrixHTMLRenderer.RichText
    let font: Font
    let presentationContext: MatrixRichTextPresentationContext

    var body: some View {
        Text(
            attributedRichText(
                text,
                includeHeadingFonts: false,
                presentationContext: presentationContext
            )
        )
            .font(font)
            .foregroundStyle(SynaraColor.secondaryText)
            .lineLimit(nil)
            .textSelection(.enabled)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.leading, SynaraSpacing.medium)
            .overlay(alignment: .leading) {
                RoundedRectangle(cornerRadius: 2, style: .continuous)
                    .fill(SynaraColor.secondaryText.opacity(0.75))
                    .frame(width: 3)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}

func attributedRichText(
    _ richText: MatrixHTMLRenderer.RichText,
    includeHeadingFonts: Bool = true,
    includeLinks: Bool = true,
    presentationContext: MatrixRichTextPresentationContext = .canvas
) -> AttributedString {
    var output = AttributedString()
    for run in richText.runs {
        var value = AttributedString(run.text)
        var intent: InlinePresentationIntent = []
        if run.style.contains(.bold) {
            intent.insert(.stronglyEmphasized)
        }
        if run.style.contains(.italic) {
            intent.insert(.emphasized)
        }
        if run.style.contains(.strikethrough) {
            intent.insert(.strikethrough)
        }
        if run.style.contains(.code) {
            intent.insert(.code)
        }
        if intent.isEmpty == false {
            value.inlinePresentationIntent = intent
        }
        if run.style.contains(.underline) {
            value.underlineStyle = .single
        }
        if run.style.contains(.code), run.style.contains(.underline) == false {
            // Attributed-string backgrounds preserve wrapping and selection.
            // This boundary matches the fill in standard contrast and becomes
            // measurable only when Increased Contrast asks for a second cue.
            value.underlineStyle = .single
            value.uiKit.underlineColor = UIColor(SynaraColor.richTextInlineCodeBoundary)
        }
        if run.style.contains(.superscript) {
            value.baselineOffset = 4
        } else if run.style.contains(.subscriptText) {
            value.baselineOffset = -3
        }
        if includeHeadingFonts, run.style.contains(.heading1) {
            value.font = .largeTitle.bold()
        } else if includeHeadingFonts, run.style.contains(.heading2) {
            value.font = .title.bold()
        } else if includeHeadingFonts, run.style.contains(.heading3) {
            value.font = .title2.bold()
        } else if includeHeadingFonts, run.style.contains(.heading4) {
            value.font = .title3.bold()
        } else if includeHeadingFonts, run.style.contains(.heading5) {
            value.font = .headline
        } else if includeHeadingFonts, run.style.contains(.heading6) {
            value.font = .subheadline.weight(.semibold)
        }
        let paintsInlineCode = run.style.contains(.code)
        if paintsInlineCode || run.foregroundColorHex != nil || run.backgroundColorHex != nil {
            let colors = SynaraRichTextColorPolicy.adaptiveColors(
                authoredForeground: run.foregroundColorHex,
                authoredBackground: run.backgroundColorHex,
                fallbackForeground: paintsInlineCode
                    ? \SynaraThemeTokens.richTextInlineCodeForeground
                    : \SynaraThemeTokens.primaryText,
                fallbackBackground: paintsInlineCode
                    ? \SynaraThemeTokens.richTextInlineCodeBackground
                    : presentationContext.readingSurface,
                appliesStandardOwnMessageTint: paintsInlineCode
                    ? false
                    : presentationContext.appliesStandardOwnMessageTint
            )
            value.foregroundColor = colors.foreground
            if paintsInlineCode || run.backgroundColorHex != nil {
                value.backgroundColor = colors.background
            }
        }
        if includeLinks {
            value.link = run.link
        }
        output.append(value)
    }
    return output
}

private struct MatrixDetailsBlockView: View {
    let block: MatrixHTMLRenderer.DetailsBlock
    let font: Font
    let presentationContext: MatrixRichTextPresentationContext
    @State private var isExpanded = false

    var body: some View {
        DisclosureGroup(isExpanded: $isExpanded) {
            VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                ForEach(identifiedMatrixSegments(block.content)) { item in
                    MatrixSemanticSegmentView(
                        segment: item.segment,
                        fallbackBody: "",
                        font: font,
                        presentationContext: presentationContext
                    )
                }
            }
            .padding(.top, SynaraSpacing.xSmall)
        } label: {
            Text(attributedRichText(block.summaryContent, presentationContext: presentationContext))
                .font(SynaraTypography.messageBody.weight(.semibold))
                .foregroundStyle(SynaraColor.primaryText)
                .lineLimit(nil)
        }
        .tint(SynaraColor.secondaryText)
    }
}

private struct MatrixCodeBlockView: View {
    let block: MatrixHTMLRenderer.CodeBlock

    private var code: String {
        block.code
    }

    private var lineNumbers: String {
        let count = MatrixHTMLRenderer.codeLineCount(code)
        return (1 ... count).map(String.init).joined(separator: "\n")
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(block.language ?? String(localized: "Code"))
                    .font(SynaraTypography.chipLabel)
                    .foregroundStyle(SynaraColor.primaryText)

                Spacer()

                Button {
                    #if canImport(UIKit)
                        UIPasteboard.general.string = code
                    #endif
                } label: {
                    Label("Copy", systemImage: "doc.on.doc")
                }
                .font(SynaraTypography.chipLabel)
                .buttonStyle(.plain)
                .foregroundStyle(SynaraColor.accent)
                .accessibilityLabel("Copy code")
                .accessibilityIdentifier("CodeBlockCopyButton")
            }
            .padding(.horizontal, SynaraSpacing.medium)
            .padding(.vertical, SynaraSpacing.small)
            .background(SynaraColor.richTextCodeBlockBackground)

            Divider()

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(alignment: .top, spacing: SynaraSpacing.small) {
                    Text(lineNumbers)
                        .font(SynaraTypography.monoBody)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .multilineTextAlignment(.trailing)
                        .monospacedDigit()
                        .fixedSize(horizontal: true, vertical: true)
                        .accessibilityHidden(true)

                    Text(code)
                        .font(SynaraTypography.monoBody)
                        .foregroundStyle(SynaraColor.primaryText)
                        .textSelection(.enabled)
                        .fixedSize(horizontal: true, vertical: true)
                }
                .padding(SynaraSpacing.medium)
            }
            .background(SynaraColor.richTextCodeBlockBackground)
        }
        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous)
                .stroke(SynaraColor.richTextCodeBlockBorder, lineWidth: 1)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct TimelineAvatar: View {
    let senderID: String
    let avatarURL: URL?
    let size: CGFloat
    @Environment(\.appEnvironment) private var environment
    @State private var avatarImage: UIImage?

    var body: some View {
        avatarContent
            .frame(width: size, height: size)
            .clipShape(Circle())
            .synaraDepth(.avatar, shape: Circle(), boundaryColor: SynaraColor.elevatedSurface)
            .task(id: avatarTaskID) {
                await loadAvatar()
            }
            .accessibilityHidden(true)
    }

    @ViewBuilder
    private var avatarContent: some View {
        if let avatarImage {
            Image(uiImage: avatarImage)
                .resizable()
                .scaledToFill()
                .frame(width: size, height: size)
                .clipShape(Circle())
        } else {
            Circle()
                .fill(avatarFill)
                .overlay {
                    Text(initials)
                        .font(.system(size: size * 0.34, weight: .semibold))
                        .foregroundStyle(Color.white)
                }
        }
    }

    @MainActor
    private func loadAvatar() async {
        avatarImage = nil

        guard let avatarURL,
              avatarURL.scheme == "mxc"
        else {
            return
        }

        let resource = MediaResource(
            id: avatarURL.absoluteString,
            filename: "\(senderID)-avatar",
            authenticatedURL: avatarURL,
            requiresAuthentication: true
        )
        if let data = await environment.mediaLoader.loadThumbnailData(
            for: resource,
            width: UInt64(max(1, Int(size * 3))),
            height: UInt64(max(1, Int(size * 3)))
        ),
            let image = UIImage(data: data)
        {
            avatarImage = image
        }
    }

    private var avatarTaskID: String {
        "\(senderID)|\(avatarURL?.absoluteString ?? "profile")"
    }

    private var initials: String {
        String(displayName.prefix(1)).uppercased()
    }

    private var displayName: String {
        guard senderID.hasPrefix("@") else {
            return senderID
        }
        return senderID
            .dropFirst()
            .split(separator: ":")
            .first
            .map(String.init) ?? senderID
    }

    private var avatarFill: Color {
        SynaraAvatarPalette.color(for: senderID)
    }
}

private struct RoomDetailsView: View {
    let roomID: String
    let fallbackTitle: String
    let onLeaveRoom: () -> Void
    let onOpenMessage: (String) -> Void
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @State private var details: RoomDetails?
    @State private var profileName = ""
    @State private var profileTopic = ""
    @State private var canonicalAlias = ""
    @State private var alternativeAliases = ""
    @State private var selectedAvatarPhoto: PhotosPickerItem?
    @State private var inviteUserID = ""
    @State private var notificationMode: SynaraRoomNotificationMode = .default
    @State private var isApplyingLoadedNotificationMode = false
    @State private var message: String?
    @State private var isLoading = false
    @State private var isLeaveConfirmationPresented = false

    var body: some View {
        NavigationStack {
            Form {
                Section("Personal") {
                    NavigationLink {
                        RoomNotesView(
                            roomID: roomID,
                            roomTitle: notesRoomTitle,
                            onOpenMessage: onOpenMessage
                        )
                    } label: {
                        Label("Personal Notes", systemImage: "note.text")
                    }
                    .accessibilityIdentifier("RoomPersonalNotesLink")
                }

                Section("Room") {
                    TextField("Name", text: $profileName)
                        .disabled(details?.canEditName != true || isLoading)
                        .accessibilityIdentifier("RoomProfileNameField")
                    TextField("Topic", text: $profileTopic, axis: .vertical)
                        .lineLimit(1 ... 3)
                        .disabled(details?.canEditTopic != true || isLoading)
                        .accessibilityIdentifier("RoomProfileTopicField")
                    if let message {
                        Text(message)
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .accessibilityIdentifier("RoomDetailsMessage")
                    }
                    SettingsInfo(title: String(localized: "Room ID"), value: roomID)
                    SettingsInfo(
                        title: String(localized: "Encryption"),
                        value: details?.encryptionLabel ?? String(localized: "Unknown")
                    )
                    SettingsInfo(title: String(localized: "Members"), value: "\(details?.memberCount ?? 0)")
                    SettingsInfo(
                        title: String(localized: "Avatar"),
                        value: details?.avatarURL ?? String(localized: "None")
                    )
                }

                if let powerLevels = details?.powerLevels {
                    Section("Permissions") {
                        SettingsInfo(title: String(localized: "Your level"), value: "\(powerLevels.ownUserLevel)")
                        PermissionInfo(
                            title: String(localized: "Send messages"),
                            threshold: powerLevels.eventsDefault,
                            allowed: true
                        )
                        PermissionInfo(
                            title: String(localized: "Invite users"),
                            threshold: powerLevels.invite,
                            allowed: powerLevels.canInvite
                        )
                        PermissionInfo(
                            title: String(localized: "Change name"),
                            threshold: powerLevels.roomName,
                            allowed: powerLevels.canEditName
                        )
                        PermissionInfo(
                            title: String(localized: "Change topic"),
                            threshold: powerLevels.roomTopic,
                            allowed: powerLevels.canEditTopic
                        )
                        PermissionInfo(
                            title: String(localized: "Change avatar"),
                            threshold: powerLevels.roomAvatar,
                            allowed: powerLevels.canEditAvatar
                        )
                        PermissionInfo(
                            title: String(localized: "Moderate"),
                            threshold: powerLevels.kick,
                            allowed: powerLevels.canKick || powerLevels.canBan || powerLevels.canRedactOther
                        )
                    }
                    .accessibilityIdentifier("RoomPermissionsSection")
                }

                Section("Notifications") {
                    Picker("Mode", selection: $notificationMode) {
                        ForEach(SynaraRoomNotificationMode.allCases) { mode in
                            Text(mode.roomDetailsTitle).tag(mode)
                        }
                    }
                    .accessibilityIdentifier("RoomNotificationModePicker")
                    .onChange(of: notificationMode) { mode in
                        guard isApplyingLoadedNotificationMode == false else {
                            return
                        }
                        updateNotificationMode(mode)
                    }
                }

                Section("Members") {
                    TextField("@user:server", text: $inviteUserID)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .accessibilityIdentifier("RoomInviteUserField")
                    Button("Invite User", action: inviteUser)
                        .disabled(isLoading || inviteUserID.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || details?.canInvite == false)
                        .accessibilityIdentifier("RoomInviteUserButton")
                    ForEach(details?.members ?? []) { member in
                        RoomMemberPresenceRow(
                            member: member,
                            roomID: roomID,
                            ownUserID: ownUserID,
                            powerLevels: details?.powerLevels,
                            onChanged: {
                                await loadDetails()
                            }
                        )
                    }
                }

                Section("Aliases And Avatar") {
                    TextField("#room:server", text: $canonicalAlias)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .disabled(details?.canEditAliases != true || isLoading)
                        .accessibilityIdentifier("RoomCanonicalAliasField")
                    TextField("#alias:server, #other:server", text: $alternativeAliases, axis: .vertical)
                        .lineLimit(1 ... 3)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .disabled(details?.canEditAliases != true || isLoading)
                        .accessibilityIdentifier("RoomAlternativeAliasesField")
                    PhotosPicker(selection: $selectedAvatarPhoto, matching: .images) {
                        Label("Upload Avatar", systemImage: "photo")
                    }
                    .disabled(details?.canEditAvatar != true || isLoading)
                    .accessibilityIdentifier("RoomAvatarUploadButton")
                    Button("Remove Avatar", role: .destructive, action: removeAvatar)
                        .disabled(details?.canEditAvatar != true || isLoading || details?.avatarURL == nil)
                        .accessibilityIdentifier("RoomAvatarRemoveButton")
                }

                Section("Danger Zone") {
                    Button("Leave Room", role: .destructive) {
                        isLeaveConfirmationPresented = true
                    }
                    .accessibilityIdentifier("LeaveRoomButton")
                }
            }
            .navigationTitle("Room Details")
            .accessibilityIdentifier("RoomDetailsScreen")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save", action: saveProfile)
                        .disabled(canSaveProfile == false)
                        .accessibilityIdentifier("RoomProfileSaveButton")
                }
            }
            .confirmationDialog("Leave this room?", isPresented: $isLeaveConfirmationPresented) {
                Button("Leave Room", role: .destructive) {
                    leaveRoom()
                }
                .accessibilityIdentifier("ConfirmLeaveRoomButton")
                Button("Cancel", role: .cancel) {}
            } message: {
                Text("The room will be removed from your joined room list.")
            }
            .task {
                await loadDetails()
            }
            .onChange(of: selectedAvatarPhoto) { item in
                if let item {
                    uploadAvatar(item)
                }
            }
        }
    }

    private func loadDetails() async {
        let loadedDetails = await environment.roomManagement.roomDetails(roomID: roomID)
        await MainActor.run {
            details = loadedDetails
            isApplyingLoadedNotificationMode = true
            notificationMode = loadedDetails?.notificationMode ?? .default
            isApplyingLoadedNotificationMode = false
            profileName = loadedDetails?.name ?? fallbackTitle
            profileTopic = loadedDetails?.topic ?? ""
            let aliases = loadedDetails?.aliases ?? []
            canonicalAlias = aliases.first ?? ""
            alternativeAliases = aliases.dropFirst().joined(separator: ", ")
        }
    }

    private var notesRoomTitle: String {
        let loadedName = details?.name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let loadedName, loadedName.isEmpty == false, loadedName != roomID else {
            return fallbackTitle
        }
        return loadedName
    }

    private var ownUserID: String? {
        if case .signedIn(let session) = environment.session.currentState {
            return session.userID
        }
        return nil
    }

    private var profileNameChange: String? {
        guard details?.canEditName == true else {
            return nil
        }
        let trimmedName = profileName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmedName != details?.name else {
            return nil
        }
        return trimmedName
    }

    private var profileTopicChange: String? {
        guard details?.canEditTopic == true else {
            return nil
        }
        let trimmedTopic = profileTopic.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmedTopic != (details?.topic ?? "") else {
            return nil
        }
        return trimmedTopic
    }

    private var canSaveProfile: Bool {
        guard isLoading == false else {
            return false
        }
        if let profileNameChange, profileNameChange.isEmpty {
            return false
        }
        return profileNameChange != nil || profileTopicChange != nil || aliasChange != nil
    }

    private var aliasChange: (canonical: String?, alternatives: [String])? {
        guard details?.canEditAliases == true else {
            return nil
        }

        let canonical = canonicalAlias.trimmingCharacters(in: .whitespacesAndNewlines)
        let alternatives = alternativeAliases
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { $0.isEmpty == false }
        let current = details?.aliases ?? []
        let updated = (canonical.isEmpty ? [] : [canonical]) + alternatives
        guard updated != current else {
            return nil
        }
        return (canonical.isEmpty ? nil : canonical, alternatives)
    }

    private func saveProfile() {
        isLoading = true
        Task {
            do {
                try await environment.roomManagement.updateRoomProfile(
                    RoomProfileUpdateRequest(
                        roomID: roomID,
                        name: profileNameChange,
                        topic: profileTopicChange,
                        canonicalAlias: aliasChange?.canonical,
                        alternativeAliases: aliasChange?.alternatives
                    )
                )
                await loadDetails()
                await MainActor.run {
                    message = String(localized: "Profile updated.")
                    isLoading = false
                }
            } catch let error as RoomManagementError {
                await MainActor.run {
                    message = error.localizedDescription
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    message = RoomManagementError.failed.localizedDescription
                    isLoading = false
                }
            }
        }
    }

    private func updateNotificationMode(_ mode: SynaraRoomNotificationMode) {
        Task {
            do {
                try await environment.roomManagement.setNotificationMode(mode, roomID: roomID)
                await MainActor.run {
                    message = String(localized: "Notification mode updated.")
                }
            } catch {
                await MainActor.run {
                    message = RoomManagementError.failed.localizedDescription
                }
            }
        }
    }

    private func inviteUser() {
        let userID = inviteUserID
        isLoading = true
        Task {
            do {
                try await environment.roomManagement.inviteUser(roomID: roomID, userID: userID)
                await MainActor.run {
                    inviteUserID = ""
                    message = String(localized: "Invitation sent.")
                    isLoading = false
                }
            } catch let error as RoomManagementError {
                await MainActor.run {
                    message = error.localizedDescription
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    message = RoomManagementError.failed.localizedDescription
                    isLoading = false
                }
            }
        }
    }

    private func uploadAvatar(_ item: PhotosPickerItem) {
        isLoading = true
        Task {
            do {
                guard let data = try await item.loadTransferable(type: Data.self), data.isEmpty == false else {
                    throw RoomManagementError.failed
                }
                try await environment.roomManagement.updateRoomProfile(
                    RoomProfileUpdateRequest(
                        roomID: roomID,
                        name: nil,
                        topic: nil,
                        avatar: .upload(data: data, mimeType: "image/jpeg")
                    )
                )
                await loadDetails()
                await MainActor.run {
                    selectedAvatarPhoto = nil
                    message = String(localized: "Avatar updated.")
                    isLoading = false
                }
            } catch let error as RoomManagementError {
                await MainActor.run {
                    selectedAvatarPhoto = nil
                    message = error.localizedDescription
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    selectedAvatarPhoto = nil
                    message = RoomManagementError.failed.localizedDescription
                    isLoading = false
                }
            }
        }
    }

    private func removeAvatar() {
        isLoading = true
        Task {
            do {
                try await environment.roomManagement.updateRoomProfile(
                    RoomProfileUpdateRequest(
                        roomID: roomID,
                        name: nil,
                        topic: nil,
                        avatar: .remove
                    )
                )
                await loadDetails()
                await MainActor.run {
                    message = String(localized: "Avatar removed.")
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    message = RoomManagementError.failed.localizedDescription
                    isLoading = false
                }
            }
        }
    }

    private func leaveRoom() {
        isLoading = true
        Task {
            do {
                try await environment.roomManagement.leaveRoom(roomID: roomID)
                await MainActor.run {
                    onLeaveRoom()
                }
            } catch {
                await MainActor.run {
                    message = RoomManagementError.failed.localizedDescription
                    isLoading = false
                }
            }
        }
    }
}

private extension SynaraRoomNotificationMode {
    /// Picker copy; `rawValue` stays the stable identifier.
    var roomDetailsTitle: String {
        switch self {
        case .default:
            return String(localized: "Default")
        case .allMessages:
            return String(localized: "All")
        case .mentionsOnly:
            return String(localized: "Mentions")
        case .mute:
            return String(localized: "Mute")
        }
    }
}

private struct RoomMemberPresenceRow: View {
    let member: RoomMemberSummary
    let roomID: String
    let ownUserID: String?
    let powerLevels: RoomPowerLevelSummary?
    let onChanged: () async -> Void
    @Environment(\.appEnvironment) private var environment
    @State private var presence: SharedCorePresence?
    @State private var userStatus: SharedCoreUserStatusSnapshot?
    @State private var isActionsPresented = false

    var body: some View {
        Button {
            isActionsPresented = true
        } label: {
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(member.title)
                    .font(SynaraTypography.body)
                    .foregroundStyle(SynaraColor.primaryText)
                Text(presence?.displayName ?? member.membership.displayName)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                if let status = userStatus?.userStatus {
                    let label = [status.emoji, status.text]
                        .filter { $0.isEmpty == false }
                        .joined(separator: " ")
                    if label.isEmpty == false {
                        Text(label)
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                }
                if userStatus?.inCall != nil {
                    Text("In a call")
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.accent)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .accessibilityIdentifier("RoomMemberRow-\(member.userID)")
        .sheet(isPresented: $isActionsPresented) {
            RoomMemberActionsView(
                roomID: roomID,
                member: member,
                ownUserID: ownUserID,
                powerLevels: powerLevels,
                onChanged: onChanged
            )
        }
        .task(id: member.userID) {
            presence = await environment.matrix.presence(userID: member.userID)
            userStatus = await environment.matrix.userStatus(userID: member.userID)
        }
    }
}

private struct PermissionInfo: View {
    let title: String
    let threshold: Int64
    let allowed: Bool

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(title)
                    .font(SynaraTypography.body)
                    .foregroundStyle(SynaraColor.primaryText)
                Text("Requires \(threshold)")
                    .font(SynaraTypography.messageMeta)
                    .foregroundStyle(SynaraColor.secondaryText)
            }

            Spacer()

            SynaraStatusChip(
                title: allowed ? String(localized: "Allowed") : String(localized: "Restricted"),
                tint: allowed ? SynaraColor.success : SynaraColor.warning,
                systemImage: allowed ? "checkmark.circle" : "lock"
            )
        }
        .accessibilityElement(children: .combine)
    }
}

private struct SettingsInfo: View {
    let title: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            Text(title)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
            Text(value)
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.primaryText)
                .textSelection(.enabled)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(title), \(value)")
    }
}

private struct CryptoRecoveryBanner: View {
    private var tint: Color {
        status.unableToDecryptCount > 0 || status.recovery == .incomplete
            ? SynaraColor.warning : SynaraColor.accent
    }

    let status: RoomCryptoStatus
    let onRetry: () -> Void
    let onReviewSecurity: () -> Void
    let onDismiss: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            HStack(alignment: .top, spacing: SynaraSpacing.small) {
                Label(status.cryptoBannerTitle, systemImage: status.unableToDecryptCount > 0 || status.recovery == .incomplete ? "lock.trianglebadge.exclamationmark" : "lock.shield")
                    .font(SynaraTypography.emphasis)
                    .foregroundStyle(SynaraColor.primaryText)
                    .frame(maxWidth: .infinity, alignment: .leading)

                Button(action: onDismiss) {
                    Image(systemName: "xmark")
                        .font(SynaraTypography.chipLabel)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .frame(width: 24, height: 24)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Dismiss encryption banner")
                .accessibilityIdentifier("EncryptedRecoveryDismissButton")
            }

            Text(status.cryptoBannerDetail)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
                .fixedSize(horizontal: false, vertical: true)

            HStack(spacing: SynaraSpacing.small) {
                if status.unableToDecryptCount > 0 {
                    Button("Retry Decryption", action: onRetry)
                        .buttonStyle(.borderedProminent)
                        .accessibilityIdentifier("EncryptedRecoveryRetryButton")
                }
                Button("Review Security", action: onReviewSecurity)
                    .buttonStyle(.bordered)
                    .accessibilityIdentifier("EncryptedRecoverySettingsButton")
            }
        }
        .padding(SynaraSpacing.medium)
        .synaraCard(fill: tint.opacity(0.10), stroke: tint)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("EncryptedRecoveryBanner")
    }
}

private struct TimelineRow: View {
    let item: TimelineItem
    let currentUserID: String
    /// The message's clock time, formatted once by the row builder.
    let timeText: String
    let isGroupedWithPrevious: Bool
    let isTimestampRevealed: Bool
    let animateSend: Bool
    let replyPreview: TimelineReplyPreview?
    let replyCount: Int
    let availability: EventActionAvailability
    let onReply: () -> Void
    let onReplyInThread: () -> Void
    let onOpenThread: () -> Void
    let isInlineThreadMode: Bool
    let isThreadExpanded: Bool
    let inlineThread: AnyView?
    let onEdit: () -> Void
    let onRedact: () -> Void
    /// Adds or removes the user's reaction with this key.
    let onToggleReaction: (String) -> Void
    /// The long-press menu's quick reactions (most-used emoji).
    @ObservedObject var quickReactions: TimelineQuickReactionStore
    let onReport: () -> Void
    let onForward: () -> Void
    let onPollVote: ([String]) async -> Bool
    let onDeclineCall: () async -> Bool
    let onPinToNotes: () -> Void
    let onOpenMedia: (MediaResource) -> Void
    let onAgentAction: (SynaraAgentCardAction) -> Void
    let onAgentApprovalReaction: (String) -> Void
    let onRetryFailedSend: () -> Void
    var onDiscardFailedSend: () -> Void = {}
    /// Display names for this message's reaction senders, by user id.
    var reactorNames: [String: String] = [:]
    /// Opens the Reactions sheet on a reaction key (nil: the first one);
    /// nil where the sheet is unavailable.
    var onShowReactions: ((String?) -> Void)?
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @Environment(\.appEnvironment) private var environment
    @State private var isSelectingText = false
    /// A GIF link that failed to load shows as its link instead.
    @State private var failedInlineGifURL: String?
    @State private var isDeclinePending = false

    var body: some View {
        if case let .systemNotice(text) = item.kind {
            TimelineSystemNoticeRow(text: text, timestamp: item.timestamp, eventID: item.eventID)
        } else {
            messageRow
        }
    }

    @ViewBuilder
    private var messageRow: some View {
        let row = HStack(alignment: .top, spacing: rowHorizontalSpacing) {
            if isGroupedWithPrevious {
                if groupedLeadingGutter > 0 {
                    Color.clear
                        .frame(width: groupedLeadingGutter, height: 1)
                }
            } else {
                TimelineAvatar(senderID: item.senderID, avatarURL: item.senderAvatarURL, size: avatarSize)
            }

            VStack(alignment: .leading, spacing: 5) {
                if isGroupedWithPrevious == false {
                    HStack(alignment: .center, spacing: SynaraSpacing.small) {
                        HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                            Text(senderDisplayName)
                                .font(SynaraTypography.emphasis)
                                .foregroundStyle(SynaraColor.headingText)
                                .lineLimit(1)
                            Text(timeText)
                                .font(SynaraTypography.messageMeta)
                                .foregroundStyle(SynaraColor.secondaryText)
                            if item.isEdited {
                                Text("edited")
                                    .font(SynaraTypography.messageMeta)
                                    .foregroundStyle(SynaraColor.tertiaryText)
                            }
                        }

                        Spacer(minLength: SynaraSpacing.xSmall)

                        Menu {
                            messageActions
                        } label: {
                            Image(systemName: "ellipsis")
                                .font(.system(size: 12, weight: .semibold))
                                .foregroundStyle(SynaraColor.secondaryText)
                                .frame(width: 28, height: 28)
                                .background(SynaraColor.elevatedSurface)
                                .clipShape(Circle())
                                .synaraDepth(.layered, shape: Circle())
                                .frame(width: 44, height: 44)
                                .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Message actions")
                        // A one-element row offers these as custom actions.
                        .accessibilityHidden(combinesAccessibilityChildren)
                        .accessibilityIdentifier("TimelineItemActions-\(item.eventID)")
                    }
                }

                messageContent
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.top, isGroupedWithPrevious ? 0 : 7)
        .accessibilityElement(children: accessibilityChildBehavior)
        .accessibilityLabel(accessibilitySummary)
        .accessibilityHint(accessibilityHint)
        .accessibilityActions {
            ForEach(accessibilityActionList, id: \.title) { action in
                Button(action.title) { performAccessibilityAction(action) }
            }
        }
        .accessibilityIdentifier("TimelineItem-\(item.eventID)")
        .sheet(isPresented: $isSelectingText) {
            if let copyPayload = TimelineMessageCopy.payload(for: item) {
                MessageTextSelectionSheet(payload: copyPayload)
            }
        }

        let timestampRevealOffset = RoomTimelineTimestampRevealPolicy.horizontalOffset(
            isGroupedWithPrevious: isGroupedWithPrevious,
            isRevealed: isTimestampRevealed,
            width: timestampRevealWidth
        )
        let timestampRevealProgress = isGroupedWithPrevious && isTimestampRevealed ? 1.0 : 0.0

        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            ZStack(alignment: .trailing) {
                if isGroupedWithPrevious {
                    Text(timeText)
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .opacity(timestampRevealProgress)
                        .accessibilityHidden(true)
                }
                withFailedRetryAccessibilityAction(row)
                    .offset(x: timestampRevealOffset)
            }
            .clipped()
            .synaraSendSlideIn(isEnabled: animateSend, fromTrailing: isOutgoing)
            if item.deliveryStatus == .failed, availability.canEdit {
                failedMessageEditButton
            }
        }
    }

    private var timestampRevealWidth: CGFloat {
        64
    }

    @ViewBuilder
    private func withFailedRetryAccessibilityAction<Content: View>(_ content: Content) -> some View {
        let canRetry = TimelineRowAccessibility.retryActionTitle(
            deliveryStatus: item.presentedDeliveryStatus
        ) != nil && OutgoingSendPolicy.canRetry(item)
        let canDiscard = isOutgoing && OutgoingSendPolicy.canDiscard(item)
        if canRetry && canDiscard {
            content
                .accessibilityAction(named: Text("Retry"), onRetryFailedSend)
                .accessibilityAction(named: Text("Discard"), onDiscardFailedSend)
        } else if canRetry {
            content.accessibilityAction(named: Text("Retry"), onRetryFailedSend)
        } else if canDiscard {
            content.accessibilityAction(named: Text("Discard"), onDiscardFailedSend)
        } else {
            content
        }
    }

    /// Retry for a failed own message whose cause can succeed on retry.
    private var bubbleRetryAction: (() -> Void)? {
        item.presentedDeliveryStatus == .failed && OutgoingSendPolicy.canRetry(item)
            ? onRetryFailedSend
            : nil
    }

    /// Discard for a failed own message (pending, or SDK local echo).
    private var bubbleDiscardAction: (() -> Void)? {
        item.presentedDeliveryStatus == .failed && isOutgoing && OutgoingSendPolicy.canDiscard(item)
            ? onDiscardFailedSend
            : nil
    }

    @ViewBuilder
    private var messageActions: some View {
        if isOutgoing, OutgoingSendPolicy.canDiscard(item) {
            Button("Discard Unsent Message", systemImage: "trash", role: .destructive) {
                onDiscardFailedSend()
            }
            .accessibilityIdentifier("TimelineItemDiscard-\(item.eventID)")
        }
        if let copyPayload = TimelineMessageCopy.payload(for: item) {
            Button("Copy", systemImage: "doc.on.doc") {
                TimelineMessageCopy.copyToPasteboard(copyPayload)
            }
            .accessibilityIdentifier("TimelineItemCopy-\(item.eventID)")
            Button("Select Text", systemImage: "text.cursor") {
                isSelectingText = true
            }
            .accessibilityIdentifier("TimelineItemSelectText-\(item.eventID)")
        }
        if availability.canReply {
            Button("Reply", systemImage: "arrowshape.turn.up.left", action: onReply)
            Button("Reply in Thread", systemImage: "text.bubble", action: onReplyInThread)
                .accessibilityIdentifier("TimelineItemReplyInThread-\(item.eventID)")
        }
        if replyCount > 0 {
            Button("Open Thread", systemImage: "bubble.left.and.bubble.right", action: onOpenThread)
        }
        if availability.canEdit {
            Button("Edit", systemImage: "pencil", action: onEdit)
        }
        if TimelineReactionTogglePolicy.canToggle(availability: availability, ownership: item.reactionOwnership) {
            quickReactionMenu
        }
        if item.reactions.isEmpty == false, let onShowReactions {
            Button("View Reactions", systemImage: "person.2") {
                onShowReactions(nil)
            }
            .accessibilityIdentifier("TimelineItemViewReactions-\(item.eventID)")
        }
        if availability.canForward {
            Button("Forward", systemImage: "arrowshape.turn.up.right", action: onForward)
                .accessibilityIdentifier("TimelineItemForward-\(item.eventID)")
        }
        if availability.canDeclineCall {
            Button("Decline Call", systemImage: "phone.down.fill", role: .destructive) {
                guard isDeclinePending == false else { return }
                isDeclinePending = true
                Task { @MainActor in
                    _ = await onDeclineCall()
                    isDeclinePending = false
                }
            }
                .disabled(isDeclinePending)
                .accessibilityIdentifier("TimelineItemDeclineCall-\(item.eventID)")
        }
        if TimelinePinActionAvailability.forItem(item).canPinToPrivateNotes {
            Button("Pin to Notes", systemImage: "note.text.badge.plus", action: onPinToNotes)
                .accessibilityIdentifier("TimelineItemPinToNotes-\(item.eventID)")
        }
        if availability.canReport {
            Button("Report", systemImage: "exclamationmark.bubble", role: .destructive, action: onReport)
                .accessibilityIdentifier("TimelineItemReport-\(item.eventID)")
        }
        if availability.canRedact {
            Button("Redact", systemImage: "trash", role: .destructive, action: onRedact)
        }
    }

    private var threadChipHint: String {
        guard isInlineThreadMode else {
            return String(localized: "Opens the thread.")
        }
        return isThreadExpanded ? String(localized: "Collapses the replies.") : String(localized: "Shows the replies.")
    }

    private var failedMessageEditButton: some View {
        HStack(spacing: rowHorizontalSpacing) {
            Color.clear
                .frame(width: isGroupedWithPrevious ? groupedLeadingGutter : avatarSize, height: 1)
            Button(action: onEdit) {
                Label("Edit", systemImage: "pencil")
                    .font(SynaraTypography.chipLabel)
                    .foregroundStyle(SynaraColor.accentText)
                    .padding(.horizontal, SynaraSpacing.small)
                    .padding(.vertical, SynaraSpacing.xSmall)
                    .background(SynaraColor.accent.opacity(0.12))
                    .clipShape(Capsule())
                    .frame(minWidth: SynaraHitTarget.minimum, minHeight: SynaraHitTarget.minimum)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Edit unsent message")
            .accessibilityIdentifier("TimelineItemEdit-\(item.eventID)")
            Spacer(minLength: 0)
        }
    }

    @ViewBuilder
    private var messageContent: some View {
        let content = VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            if let replyToEventID = item.replyToEventID {
                replyQuoteLabel(for: replyToEventID)
            }

            bubbleWrappedBodyContent
                .contextMenu {
                    messageActions
                } preview: {
                    bubbleWrappedBodyContent
                }

            if let xPostLink {
                XPostEmbedView(url: xPostLink)
            }

            if item.reactions.isEmpty == false {
                let canToggleReactions = TimelineReactionTogglePolicy.canToggle(
                    availability: availability,
                    ownership: item.reactionOwnership
                )
                HStack(spacing: SynaraSpacing.xSmall) {
                    ForEach(Array(item.reactions.keys.sorted().enumerated()), id: \.element) { index, reaction in
                        ReactionPill(
                            title: reaction,
                            count: item.reactions[reaction] ?? 0,
                            isSelected: item.reactionOwnership.contains(reaction),
                            accessibilityText: reactionPillAccessibilityLabel(reaction),
                            animationIndex: index,
                            onToggle: canToggleReactions ? { onToggleReaction(reaction) } : nil,
                            onShowSenders: onShowReactions.map { show in { show(reaction) } }
                        )
                    }
                }
            }

            if replyCount > 0 {
                Button(action: onOpenThread) {
                    HStack(spacing: SynaraSpacing.xSmall) {
                        Image(systemName: "bubble.left.and.bubble.right.fill")
                            .accessibilityHidden(true)
                        Text(TimelineRowAccessibility.replyCountLabel(replyCount))
                        if isInlineThreadMode {
                            Image(systemName: isThreadExpanded ? "chevron.up" : "chevron.down")
                                .accessibilityHidden(true)
                        }
                    }
                    .font(SynaraTypography.chipLabel)
                    .foregroundStyle(SynaraColor.accentText)
                    .frame(minHeight: SynaraHitTarget.minimum)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("ThreadButton-\(item.eventID)")
                .accessibilityHint(threadChipHint)
                .accessibilityValue(isInlineThreadMode ? (isThreadExpanded ? "Expanded" : "Collapsed") : "")
            }

            if let inlineThread {
                inlineThread
            }
        }

        content
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder
    private var bubbleWrappedBodyContent: some View {
        if let poll = item.poll {
            SynaraMessageBubble(
                alignment: bubbleAlignment,
                variant: .standard,
                depth: SynaraSurfaceDepthRole.emphasizedMessage,
                isGrouped: isGroupedWithPrevious,
                showsBackground: true,
                deliveryStatus: nil
            ) {
                TimelinePollCard(
                    poll: poll,
                    canVote: availability.canVote,
                    onVote: onPollVote
                )
            }
        } else if let approvalPrompt {
            // Only a request still waiting on you carries the critical plane.
            let isPending = item.agentApprovalDecision == nil
            SynaraMessageBubble(
                alignment: bubbleAlignment,
                variant: isPending ? .critical : .standard,
                depth: isPending ? .critical : .layered,
                isGrouped: isGroupedWithPrevious,
                showsBackground: true,
                deliveryStatus: nil
            ) {
                AgentApprovalPromptTimelineCard(
                    prompt: approvalPrompt,
                    eventID: item.eventID,
                    decision: item.agentApprovalDecision,
                    onReaction: onAgentApprovalReaction
                )
            }
        } else {
            switch item.kind {
            case let .text(body):
                if let inlineGif {
                    SynaraMessageBubble(
                        alignment: bubbleAlignment,
                        variant: .standard,
                        depth: SynaraSurfaceDepthRole.standardMessage,
                        isGrouped: isGroupedWithPrevious,
                        showsBackground: false,
                        deliveryStatus: item.presentedDeliveryStatus,
                        statusEventID: item.eventID,
                        onRetryFailedSend: bubbleRetryAction,
                        onDiscardFailedSend: bubbleDiscardAction,
                        encryptionShield: item.encryptionShield
                    ) {
                        InlineGifView(link: inlineGif) {
                            failedInlineGifURL = inlineGif.url
                        }
                    }
                } else {
                    plainTextBubble(body)
                }
            case let .formattedText(body, html):
                SynaraMessageBubble(
                    alignment: bubbleAlignment,
                    variant: .standard,
                    depth: SynaraSurfaceDepthRole.standardMessage,
                    isGrouped: isGroupedWithPrevious,
                    showsBackground: SynaraSurfaceDepthRole.standardMessageShowsBackground,
                    deliveryStatus: item.presentedDeliveryStatus,
                    statusEventID: item.eventID,
                    onRetryFailedSend: bubbleRetryAction,
                    onDiscardFailedSend: bubbleDiscardAction,
                    encryptionShield: item.encryptionShield
                ) {
                    MatrixFormattedMessageView(
                        fallbackBody: body,
                        html: html,
                        font: SynaraTypography.messageBody,
                        presentationContext: bubbleAlignment == .own
                            ? .ownMessage
                            : .otherMessage
                    )
                }
            case .encryptedPlaceholder:
                SynaraMessageBubble(
                    alignment: bubbleAlignment,
                    variant: .encrypted,
                    depth: SynaraSurfaceDepthRole.emphasizedMessage,
                    isGrouped: isGroupedWithPrevious,
                    showsBackground: true,
                    deliveryStatus: nil
                ) {
                    Label(
                        "Encrypted content unavailable. Actions and media downloads are blocked until keys are available.",
                        systemImage: "lock"
                    )
                    .font(SynaraTypography.messageBody)
                    .foregroundStyle(SynaraColor.secondaryText)
                }
            case let .agentCard(card):
                SynaraMessageBubble(
                    alignment: bubbleAlignment,
                    variant: .agent,
                    depth: SynaraSurfaceDepthRole.emphasizedMessage,
                    isGrouped: isGroupedWithPrevious,
                    showsBackground: true,
                    deliveryStatus: nil
                ) {
                    AgentCardTimelineRow(card: card, onAction: onAgentAction)
                }
            default:
                bodyContent
            }
        }
    }

    private func plainTextBubble(_ body: String) -> some View {
        SynaraMessageBubble(
            text: body,
            alignment: bubbleAlignment,
            isGrouped: isGroupedWithPrevious,
            deliveryStatus: item.presentedDeliveryStatus,
            statusEventID: item.eventID,
            onRetryFailedSend: bubbleRetryAction,
            onDiscardFailedSend: bubbleDiscardAction,
            encryptionShield: item.encryptionShield
        )
    }

    /// A plain message that is nothing but a Klipy or GIPHY link plays as
    /// the GIF (Settings → Play GIF Links Inline), as on desktop.
    private var inlineGif: InlineGifLink? {
        guard case let .text(body) = item.kind,
              environment.settings.bool(for: RichLinkSettings.inlineGifLinksKey),
              let link = GifLinks.messageInlineGif(body),
              link.url != failedInlineGifURL
        else {
            return nil
        }
        return link
    }

    /// The message's first link, when it is an X post (Settings → Show X Posts).
    private var xPostLink: String? {
        let body: String
        switch item.kind {
        case let .text(text):
            body = text
        case let .formattedText(text, _):
            body = text
        default:
            return nil
        }
        // Cheap test first: most messages hold no X link at all.
        guard body.contains("x.com/") || body.contains("twitter.com/"),
              inlineGif == nil,
              environment.settings.bool(for: RichLinkSettings.xEmbedsKey)
        else {
            return nil
        }
        return XPostEmbed.firstStatusLink(in: body)
    }

    @ViewBuilder
    private var bodyContent: some View {
        switch item.kind {
        case .text, .formattedText, .encryptedPlaceholder, .agentCard:
            EmptyView()
        case let .mediaPlaceholder(resource):
            if resource.isEncrypted {
                MediaAttachmentCard(resource: resource)
                    .accessibilityIdentifier("EncryptedMediaPlaceholder-\(resource.filename)")
            } else {
                Button {
                    onOpenMedia(resource)
                } label: {
                    MediaAttachmentCard(resource: resource)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("MediaPlaceholder-\(resource.filename)")
            }
        case .redacted:
            SynaraMessageBubble(
                alignment: bubbleAlignment,
                variant: .standard,
                depth: SynaraSurfaceDepthRole.emphasizedMessage,
                isGrouped: isGroupedWithPrevious,
                showsBackground: true,
                deliveryStatus: nil
            ) {
                Text("Message deleted")
                    .font(SynaraTypography.messageBody)
                    .foregroundStyle(SynaraColor.secondaryText)
            }
        case let .unknown(type):
            SynaraMessageBubble(
                alignment: bubbleAlignment,
                variant: .standard,
                depth: SynaraSurfaceDepthRole.emphasizedMessage,
                isGrouped: isGroupedWithPrevious,
                showsBackground: true,
                deliveryStatus: nil
            ) {
                Text(TimelineUnavailableMessageCopy.text)
                    .font(SynaraTypography.messageBody)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .accessibilityIdentifier("TimelineUnavailableMessage-\(type)")
            }
        case let .systemNotice(text):
            // Rendered by `TimelineSystemNoticeRow`; kept for exhaustiveness.
            Text(text)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
        }
    }

    private var bubbleAlignment: SynaraMessageBubbleAlignment {
        isOutgoing ? .own : .other
    }

    private var accessibilitySummary: String {
        TimelineRowAccessibility.summary(
            sender: senderDisplayName,
            time: timeText,
            // A played GIF link reads as "GIF", not as its URL.
            kind: inlineGif == nil ? item.kind : .text(String(localized: "GIF")),
            isEdited: item.isEdited,
            reactions: item.reactions,
            ownReactions: item.reactionOwnership,
            replyCount: replyCount
        )
    }

    /// One element per message, first of a group or not, unless the row
    /// holds controls VoiceOver must reach on their own (retry, polls,
    /// approval and agent cards, the thread chip). The menu's actions are
    /// then custom actions on that element.
    private var accessibilityChildBehavior: AccessibilityChildBehavior {
        combinesAccessibilityChildren ? .combine : .contain
    }

    /// UI automation keeps descendants (as `RoomListRow` does) so it can
    /// reach the rendered body and the actions menu.
    private var combinesAccessibilityChildren: Bool {
        rowContainsAccessibleChildren == false
            && SynaraLaunchEnvironment.isUITest == false
    }

    private var rowContainsAccessibleChildren: Bool {
        TimelineRowAccessibility.containsChildren(
            deliveryStatus: item.presentedDeliveryStatus,
            kind: item.kind,
            replyCount: replyCount,
            hasApprovalPrompt: approvalPrompt != nil,
            hasInteractivePoll: item.poll != nil
        )
            // An X card's play and "View on X" buttons must stay reachable.
            || xPostLink != nil
    }

    /// The quick reactions as a row of emoji at the top of the menu (a
    /// "React" submenu before iOS 17, which has no palette menus). Each one
    /// toggles the user's reaction, as the pills do.
    @ViewBuilder
    private var quickReactionMenu: some View {
        if #available(iOS 17.0, *) {
            ControlGroup {
                quickReactionButtons
                    // Palette items keep the menu open by default; a
                    // reaction is a finished choice, like any other action.
                    .menuActionDismissBehavior(.enabled)
            }
            .controlGroupStyle(.palette)
        } else {
            Menu {
                quickReactionButtons
            } label: {
                Label("React", systemImage: "face.smiling")
            }
        }
    }

    private var quickReactionButtons: some View {
        ForEach(quickReactions.keys, id: \.self) { key in
            let action = TimelineRowAccessibility.Action.toggleReaction(
                key: key,
                isOwn: item.reactionOwnership.contains(key)
            )
            Button {
                onToggleReaction(key)
            } label: {
                // The palette builds its items from the label: a label set
                // only on the Button is dropped there, and VoiceOver would
                // read the bare emoji name instead of the action.
                Text(key)
                    .accessibilityLabel(action.title)
            }
            .accessibilityLabel(action.title)
            .accessibilityIdentifier("QuickReaction-\(key)")
        }
    }

    private var accessibilityActionList: [TimelineRowAccessibility.Action] {
        guard rowContainsAccessibleChildren == false else { return [] }
        return TimelineRowAccessibility.actions(
            availability: availability,
            replyCount: replyCount,
            canCopy: TimelineMessageCopy.payload(for: item) != nil,
            canPinToNotes: TimelinePinActionAvailability.forItem(item).canPinToPrivateNotes,
            reactions: item.reactions,
            ownReactions: item.reactionOwnership,
            quickReactions: quickReactions.keys,
            canViewReactions: onShowReactions != nil
        )
    }

    private func performAccessibilityAction(_ action: TimelineRowAccessibility.Action) {
        switch action {
        case .reply:
            onReply()
        case .replyInThread:
            onReplyInThread()
        case .openThread:
            onOpenThread()
        case .react:
            if let key = quickReactions.keys.first {
                onToggleReaction(key)
            }
        case let .toggleReaction(key, _):
            onToggleReaction(key)
        case .viewReactions:
            onShowReactions?(nil)
        case .edit:
            onEdit()
        case .copy:
            if let payload = TimelineMessageCopy.payload(for: item) {
                TimelineMessageCopy.copyToPasteboard(payload)
            }
        case .selectText:
            isSelectingText = true
        case .forward:
            onForward()
        case .pinToNotes:
            onPinToNotes()
        case .declineCall:
            guard isDeclinePending == false else { return }
            isDeclinePending = true
            Task { @MainActor in
                _ = await onDeclineCall()
                isDeclinePending = false
            }
        case .report:
            onReport()
        case .redact:
            onRedact()
        }
    }

    private var accessibilityHint: String {
        if approvalPrompt != nil {
            return item.agentApprovalDecision == nil
                ? String(localized: "Review available approval reactions")
                : String(localized: "You already decided this approval")
        }

        if item.presentedDeliveryStatus == .failed {
            guard OutgoingSendPolicy.canRetry(item) else {
                return String(localized: "This message was not sent. Use the Discard action to remove it")
            }
            return availability.canEdit && item.isLocalPending
                ? String(localized: "Tap Retry to send this message again. Edit is also available.")
                : String(localized: "Tap Retry to send this message again")
        }

        switch item.kind {
        case .agentCard:
            return String(localized: "Review available agent actions")
        default:
            // A one-element row lists its actions in the VoiceOver actions rotor.
            return rowContainsAccessibleChildren
                ? String(localized: "Use the Message actions button or long press for message actions")
                : ""
        }
    }

    private var isOutgoing: Bool {
        item.senderID == currentUserID
    }

    private var approvalPrompt: SynaraAgentApprovalPrompt? {
        guard item.isAgentApproval else {
            return nil
        }
        return SynaraAgentApprovalPromptDetector.detect(in: item)
    }

    private var isCompactWidth: Bool {
        horizontalSizeClass == .compact
    }

    private var avatarSize: CGFloat {
        isCompactWidth ? 28 : 30
    }

    private var groupedLeadingGutter: CGFloat {
        avatarSize
    }

    private var rowHorizontalSpacing: CGFloat {
        SynaraSpacing.small
    }

    @ViewBuilder
    private func replyQuoteLabel(for _: String) -> some View {
        if let preview = replyPreview {
            VStack(alignment: .leading, spacing: 2) {
                Text("Replying to \(preview.senderName)")
                    .font(SynaraTypography.supporting.weight(.semibold))
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(1)
                Text(preview.snippet)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(3)
            }
            .padding(.leading, SynaraSpacing.small)
            .overlay(alignment: .leading) {
                RoundedRectangle(cornerRadius: 2, style: .continuous)
                    .fill(SynaraColor.accent.opacity(0.55))
                    .frame(width: 3)
            }
        } else {
            Label("Replying to a message", systemImage: "arrowshape.turn.up.left")
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
                .lineLimit(1)
        }
    }

    private var senderDisplayName: String {
        guard item.senderID.hasPrefix("@") else {
            return item.senderID
        }

        return item.resolvedSenderDisplayName(currentUserID: currentUserID)
    }

    /// "👍, 3 reactions: Alice, Bob and you".
    private func reactionPillAccessibilityLabel(_ key: String) -> String {
        TimelineRowAccessibility.reactionPillLabel(
            key: key,
            count: item.reactions[key] ?? 0,
            includesYou: TimelineReactionSenders.includesCurrentUser(key: key, in: item, currentUserID: currentUserID),
            otherNames: TimelineReactionSenders.otherNames(
                key: key,
                in: item,
                currentUserID: currentUserID,
                names: reactorNames
            )
        )
    }
}

private struct MediaAttachmentCard: View {
    let resource: MediaResource
    @Environment(\.appEnvironment) private var environment
    @State private var thumbnailImage: UIImage?
    @State private var isLoadingThumbnail = false

    private let imageThumbnailHeight: CGFloat = 180

    var body: some View {
        Group {
            if resource.isImageMedia {
                imageAttachmentCard
            } else {
                fileAttachmentCard
            }
        }
        .task(id: thumbnailTaskID) {
            await loadThumbnailIfNeeded()
        }
    }

    @ViewBuilder
    private var imageAttachmentCard: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            imageThumbnailView
                .frame(maxWidth: .infinity)
                .frame(height: imageThumbnailHeight)
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))

            mediaMetadataRow(showDownloadIcon: false)
        }
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.vertical, SynaraSpacing.small)
        .synaraCard(fill: SynaraColor.surface)
    }

    @ViewBuilder
    private var fileAttachmentCard: some View {
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: resource.safeDescription.localizedCaseInsensitiveContains(".pdf") ? "doc.richtext" : "doc.text")
                .font(.system(size: 22, weight: .regular))
                .foregroundStyle(SynaraColor.secondaryText)
                .frame(width: 30, height: 30)
                .accessibilityHidden(true)

            mediaMetadataRow(showDownloadIcon: true)

            Spacer(minLength: 0)
        }
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.vertical, SynaraSpacing.small)
        .synaraCard(fill: SynaraColor.surface)
    }

    @ViewBuilder
    private func mediaMetadataRow(showDownloadIcon: Bool) -> some View {
        HStack(spacing: SynaraSpacing.small) {
            VStack(alignment: .leading, spacing: 3) {
                Text(resource.safeDescription)
                    .font(SynaraTypography.emphasis)
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(1)
                if let caption = resource.caption {
                    if let html = resource.formattedCaption, html.isEmpty == false {
                        MatrixFormattedMessageView(
                            fallbackBody: caption,
                            html: html,
                            font: SynaraTypography.messageBody
                        )
                    } else {
                        Text(caption)
                            .font(SynaraTypography.messageBody)
                            .foregroundStyle(SynaraColor.primaryText)
                    }
                }
                if let sizeText = MediaFormatting.formattedFileSize(resource.byteSize) {
                    Text(sizeText)
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                }
                if resource.isEncrypted {
                    Text("Encrypted media requires recovered keys")
                        .font(SynaraTypography.fineMeta)
                        .foregroundStyle(SynaraColor.warningText)
                        .lineLimit(2)
                }
            }

            Spacer(minLength: 0)

            if showDownloadIcon {
                Image(systemName: resource.isEncrypted ? "lock.fill" : "arrow.down.to.line")
                    .font(.system(size: 17, weight: .medium))
                    .foregroundStyle(SynaraColor.secondaryText)
            }
        }
    }

    @ViewBuilder
    private var imageThumbnailView: some View {
        if resource.isEncrypted || resource.authenticatedURL == nil {
            unavailableImagePlaceholder
        } else if let thumbnailImage {
            if thumbnailImage.isAnimated {
                AnimatedImageView(image: thumbnailImage, contentMode: .scaleAspectFill)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .clipped()
            } else {
                Image(uiImage: thumbnailImage)
                    .resizable()
                    .scaledToFill()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .clipped()
            }
        } else if isLoadingThumbnail {
            ZStack {
                unavailableImagePlaceholder
                ProgressView()
            }
        } else {
            unavailableImagePlaceholder
        }
    }

    private var unavailableImagePlaceholder: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .fill(SynaraColor.secondarySurface)
            Image(systemName: resource.isEncrypted ? "lock.fill" : "photo")
                .font(.system(size: 28, weight: .semibold))
                .foregroundStyle(SynaraColor.secondaryText)
        }
    }

    private var thumbnailTaskID: String {
        "\(resource.id)|\(resource.authenticatedURL?.absoluteString ?? "unavailable")|\(resource.isEncrypted)"
    }

    @MainActor
    private func loadThumbnailIfNeeded() async {
        thumbnailImage = nil
        isLoadingThumbnail = false

        guard resource.isImageMedia,
              resource.isEncrypted == false,
              resource.authenticatedURL != nil
        else {
            return
        }

        isLoadingThumbnail = true
        defer { isLoadingThumbnail = false }

        // A server thumbnail of a GIF is one still frame; play the original.
        if resource.isAnimatedImageMedia,
           (resource.byteSize ?? 0) <= MediaResource.maxAnimatedImageBytes,
           let data = await environment.mediaLoader.loadMediaData(for: resource),
           data.count <= MediaResource.maxAnimatedImageBytes,
           let image = await AnimatedImageDecoder.decode(data, limits: .inline)
        {
            thumbnailImage = image
            return
        }

        let pixelWidth = UInt64(max(1, Int(UIScreen.main.bounds.width * UIScreen.main.scale)))
        let pixelHeight = UInt64(max(1, Int(imageThumbnailHeight * UIScreen.main.scale)))
        if let data = await environment.mediaLoader.loadThumbnailData(
            for: resource,
            width: pixelWidth,
            height: pixelHeight
        ),
            let image = UIImage(data: data)
        {
            thumbnailImage = image
        }
    }
}

/// What a row shows for an event this app cannot display. User copy: never
/// the event's wire type.
enum TimelineUnavailableMessageCopy {
    static let text = String(localized: "This message can't be shown in Synara.")
}

/// The day the rows below belong to, in the unread divider's rule style.
private struct TimelineDayHeader: View {
    let day: Date
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    var body: some View {
        let label = TimelineDayHeaderPolicy.label(for: day)
        HStack(spacing: SynaraSpacing.medium) {
            Rectangle()
                .fill(SynaraColor.separator.opacity(0.55))
                .frame(height: 0.5)
            Text(label)
                .font(SynaraTypography.chipLabel)
                .foregroundStyle(SynaraColor.secondaryText)
                .multilineTextAlignment(.center)
                .lineLimit(SynaraDynamicTypeLayout.ruleLabelLineLimit(dynamicTypeSize))
                .fixedSize(horizontal: dynamicTypeSize.isAccessibilitySize == false, vertical: true)
                .layoutPriority(1)
            Rectangle()
                .fill(SynaraColor.separator.opacity(0.55))
                .frame(height: 0.5)
        }
        .frame(maxWidth: .infinity)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(label)
        .accessibilityAddTraits(.isHeader)
        .accessibilityIdentifier("TimelineDayHeader")
    }
}

/// A membership or room-state change ("Ada joined the room"): one quiet
/// centered line with no avatar or bubble, so it never reads as a message
/// from its sender.
private struct TimelineSystemNoticeRow: View {
    let text: String
    let timestamp: Date
    let eventID: String

    var body: some View {
        Text(text)
            .font(SynaraTypography.messageMeta)
            .foregroundStyle(SynaraColor.secondaryText)
            .multilineTextAlignment(.center)
            .lineLimit(3)
            .frame(maxWidth: .infinity)
            .padding(.horizontal, SynaraSpacing.xLarge)
            .padding(.vertical, SynaraSpacing.xSmall)
            .accessibilityElement(children: .ignore)
            .accessibilityLabel("\(text), \(timestamp.timelineTime)")
            .accessibilityIdentifier("TimelineSystemNotice-\(eventID)")
    }
}

struct UnreadMessagesDivider: View {
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Rectangle()
                .fill(SynaraColor.separator.opacity(0.55))
                .frame(height: 0.5)
            // The label wins the width over the rules ("Unr…" at AX5).
            Text("Unread messages")
                .font(SynaraTypography.chipLabel)
                .foregroundStyle(SynaraColor.accentText)
                .multilineTextAlignment(.center)
                .lineLimit(SynaraDynamicTypeLayout.ruleLabelLineLimit(dynamicTypeSize))
                .fixedSize(horizontal: dynamicTypeSize.isAccessibilitySize == false, vertical: true)
                .layoutPriority(1)
            Rectangle()
                .fill(SynaraColor.separator.opacity(0.55))
                .frame(height: 0.5)
        }
        .padding(.leading, dynamicTypeSize.isAccessibilitySize ? 0 : 46)
        // A heading, so the rotor can jump to where unread messages start.
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Unread messages")
        .accessibilityAddTraits(.isHeader)
        .accessibilityIdentifier("TimelineUnreadDivider")
    }
}

private struct TimelinePollCard: View {
    let poll: TimelinePollPresentation
    let canVote: Bool
    let onVote: ([String]) async -> Bool
    @State private var selectedAnswerIDs: Set<String>
    @State private var isSubmitting = false
    @State private var pendingAnswerIDs: Set<String>?
    @State private var pollDispatchSettled = false
    @State private var pendingProjectionObserved = false

    init(
        poll: TimelinePollPresentation,
        canVote: Bool,
        onVote: @escaping ([String]) async -> Bool
    ) {
        self.poll = poll
        self.canVote = canVote
        self.onVote = onVote
        _selectedAnswerIDs = State(initialValue: Set(poll.answers.filter(\.isOwn).map(\.id)))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                Image(systemName: "chart.bar.xaxis")
                    .foregroundStyle(SynaraColor.accent)
                Text(poll.question)
                    .font(SynaraTypography.emphasis)
                    .foregroundStyle(SynaraColor.headingText)
                    .fixedSize(horizontal: false, vertical: true)
            }

            ForEach(poll.answers) { answer in
                Button {
                    let next = TimelinePollSelectionPolicy.toggledSelection(
                        current: selectedAnswerIDs,
                        answerID: answer.id,
                        availableAnswerIDs: availableAnswerIDs,
                        maximumSelections: poll.maximumSelections
                    )
                    selectedAnswerIDs = next
                    if poll.maximumSelections == 1,
                       let submission = submission(for: next)
                    {
                        submit(submission)
                    }
                } label: {
                    HStack(spacing: SynaraSpacing.small) {
                        Image(systemName: selectedAnswerIDs.contains(answer.id) ? "checkmark.circle.fill" : "circle")
                            .foregroundStyle(
                                selectedAnswerIDs.contains(answer.id)
                                    ? SynaraColor.accent
                                    : SynaraColor.secondaryText
                            )
                        Text(answer.text)
                            .font(SynaraTypography.messageBody)
                            .foregroundStyle(SynaraColor.primaryText)
                            .frame(maxWidth: .infinity, alignment: .leading)
                        Text("\(answer.voteCount)")
                            .font(SynaraTypography.messageMeta)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .monospacedDigit()
                    }
                    .padding(.horizontal, SynaraSpacing.small)
                    .padding(.vertical, SynaraSpacing.xSmall)
                    .background(
                        selectedAnswerIDs.contains(answer.id)
                            ? SynaraColor.accent.opacity(0.12)
                            : SynaraColor.elevatedSurface
                    )
                    .clipShape(RoundedRectangle(cornerRadius: 9, style: .continuous))
                }
                .buttonStyle(.plain)
                .disabled(canVote == false || poll.isClosed || isSubmitting)
                .accessibilityElement(children: .combine)
                .accessibilityAddTraits(selectedAnswerIDs.contains(answer.id) ? .isSelected : [])
                .accessibilityLabel("\(answer.text), \(answer.voteCount) votes")
                .accessibilityIdentifier("TimelinePollAnswer-\(answer.id)")
            }

            if poll.maximumSelections > 1,
               let submission = submission(for: selectedAnswerIDs)
            {
                Button(submission.isEmpty ? "Clear vote" : "Submit vote") {
                    submit(submission)
                }
                .buttonStyle(.borderedProminent)
                .disabled(isSubmitting)
                .accessibilityIdentifier("TimelinePollSubmitVote")
            }

            Text(pollFooter)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Poll: \(poll.question)")
        .onChange(of: poll) { updatedPoll in
            let authoritative = Set(updatedPoll.answers.filter(\.isOwn).map(\.id))
            selectedAnswerIDs = authoritative
            if pendingAnswerIDs == authoritative {
                pendingProjectionObserved = true
                finishPollSubmissionIfComplete()
            }
        }
    }

    private var availableAnswerIDs: Set<String> {
        Set(poll.answers.map(\.id))
    }

    private var originalAnswerIDs: Set<String> {
        Set(poll.answers.filter(\.isOwn).map(\.id))
    }

    private func submission(for selection: Set<String>) -> [String]? {
        TimelinePollSelectionPolicy.submission(
            selection: selection,
            original: originalAnswerIDs,
            availableAnswerIDs: availableAnswerIDs,
            maximumSelections: poll.maximumSelections,
            canVote: canVote,
            isClosed: poll.isClosed
        )
    }

    private func submit(_ answerIDs: [String]) {
        guard isSubmitting == false else { return }
        let requested = Set(answerIDs)
        pendingAnswerIDs = requested
        pollDispatchSettled = false
        pendingProjectionObserved = false
        isSubmitting = true
        Task { @MainActor in
            let accepted = await onVote(answerIDs)
            if accepted == false {
                resetPollSubmission()
            } else {
                pollDispatchSettled = true
                finishPollSubmissionIfComplete()
            }
        }
    }

    private func finishPollSubmissionIfComplete() {
        guard pollDispatchSettled, pendingProjectionObserved else { return }
        resetPollSubmission()
    }

    private func resetPollSubmission() {
        pendingAnswerIDs = nil
        pollDispatchSettled = false
        pendingProjectionObserved = false
        isSubmitting = false
    }

    private var pollFooter: String {
        if poll.isClosed {
            return String(localized: "Poll closed")
        }
        if poll.maximumSelections == 1 {
            return String(localized: "Choose one answer")
        }
        return String(localized: "Choose up to \(poll.maximumSelections) answers")
    }
}

private struct TimelineForwardSheet: View {
    private enum ForwardAlert: Identifiable {
        case confirmDowngrade(RoomSummary)
        case blocked(String)

        var id: String {
            switch self {
            case .confirmDowngrade(let target):
                "confirm-downgrade-\(target.id)"
            case .blocked(let message):
                "blocked-\(message)"
            }
        }
    }

    let sourceRoomID: String
    let sourceEncryption: SynaraRoomEncryptionStatus
    let sourceTransport: TimelineForwardTransport
    let roomList: any RoomListServicing
    let onForward: (String, Bool, Bool) async -> EventActionError?

    @Environment(\.dismiss) private var dismiss
    @State private var state: RoomListState = .loading
    @State private var query = ""
    @State private var asQuote = false
    @State private var forwardingRoomID: String?
    @State private var forwardAlert: ForwardAlert?

    var body: some View {
        NavigationStack {
            Group {
                switch state {
                case .idle, .loading:
                    ProgressView("Loading rooms…")
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                case .empty:
                    VStack(spacing: SynaraSpacing.medium) {
                        Image(systemName: "bubble.left.and.bubble.right")
                            .font(.title)
                            .foregroundStyle(SynaraColor.secondaryText)
                        Text("No Rooms Available")
                            .font(SynaraTypography.emphasis)
                        Text("Join another room before forwarding this message.")
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .multilineTextAlignment(.center)
                    }
                    .padding(SynaraSpacing.large)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                case .failed(let message):
                    SynaraErrorState(title: String(localized: "Could Not Load Rooms"), message: message) {
                        Task { await loadRooms() }
                    }
                case .loaded:
                    List {
                        if sourceTransport == .text {
                            Section {
                                Toggle("Forward as quote", isOn: $asQuote)
                            } footer: {
                                Text("Quotes include the original sender in the forwarded message.")
                            }
                        }
                        Section("Choose a room") {
                            ForEach(filteredRooms) { room in
                                Button {
                                    requestForward(to: room)
                                } label: {
                                    HStack(spacing: SynaraSpacing.small) {
                                        Image(systemName: room.kind == .directMessage ? "person" : "number")
                                            .foregroundStyle(SynaraColor.secondaryText)
                                        Text(room.name)
                                            .foregroundStyle(SynaraColor.primaryText)
                                        Spacer(minLength: SynaraSpacing.small)
                                        if room.encryptionStatus == .encrypted {
                                            Image(systemName: "lock.fill")
                                                .foregroundStyle(SynaraColor.secondaryText)
                                                .accessibilityLabel("Encrypted room")
                                        }
                                        if forwardingRoomID == room.id {
                                            ProgressView()
                                                .controlSize(.small)
                                        }
                                    }
                                }
                                .disabled(
                                    forwardingRoomID != nil
                                        || sourceEncryption == .unknown
                                        || sourceEncryption == .unavailable
                                        || room.encryptionStatus == .unknown
                                        || room.encryptionStatus == .unavailable
                                )
                                .accessibilityIdentifier("TimelineForwardRoom-\(room.id)")
                            }
                        }
                        if sourceEncryption == .unknown || sourceEncryption == .unavailable {
                            Section {
                                Label("Room encryption status is still loading.", systemImage: "lock.trianglebadge.exclamationmark")
                                    .foregroundStyle(SynaraColor.secondaryText)
                            }
                        }
                    }
                    .listStyle(.insetGrouped)
                    .searchable(text: $query, prompt: "Search rooms")
                }
            }
            .navigationTitle("Forward Message")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .disabled(forwardingRoomID != nil)
                }
            }
        }
        .task { await loadRooms() }
        .alert(item: $forwardAlert) { alert in
            switch alert {
            case .confirmDowngrade(let target):
                Alert(
                    title: Text("Forward to an Unencrypted Room?"),
                    message: Text("The source room is encrypted, but this room is not. The forwarded copy will not be protected by room encryption."),
                    primaryButton: .destructive(Text("Forward Anyway")) {
                        forward(to: target, confirmedEncryptionDowngrade: true)
                    },
                    secondaryButton: .cancel()
                )
            case .blocked(let message):
                Alert(
                    title: Text("Cannot Forward"),
                    message: Text(message),
                    dismissButton: .cancel(Text("OK"))
                )
            }
        }
        .accessibilityIdentifier("TimelineForwardSheet")
    }

    private var filteredRooms: [RoomSummary] {
        guard case .loaded(let rooms) = state else { return [] }
        let eligible = rooms.filter {
            $0.id != sourceRoomID && $0.membership == .joined
        }
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return eligible.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
        }
        return eligible
            .filter { $0.name.localizedStandardContains(trimmed) }
            .sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    @MainActor
    private func loadRooms() async {
        state = .loading
        state = await roomList.loadRooms()
    }

    private func forward(
        to target: RoomSummary,
        confirmedEncryptionDowngrade: Bool = false
    ) {
        guard forwardingRoomID == nil else { return }
        forwardingRoomID = target.id
        Task { @MainActor in
            let error = await onForward(
                target.id,
                sourceTransport == .text && asQuote,
                confirmedEncryptionDowngrade
            )
            forwardingRoomID = nil
            switch error {
            case nil:
                dismiss()
            case .some(.forwardDowngradeNotConfirmed):
                forwardAlert = .confirmDowngrade(target)
            case .some(.forwardEncryptionUnavailable):
                forwardAlert = .blocked(
                    EventActionError.forwardEncryptionUnavailable.errorDescription
                        ?? String(localized: "Room encryption status is unavailable. Forwarding was not started.")
                )
            case .some(let error):
                forwardAlert = .blocked(error.errorDescription ?? String(localized: "Forwarding was not completed."))
            }
        }
    }

    private func requestForward(to target: RoomSummary) {
        guard forwardingRoomID == nil else { return }
        switch TimelineForwardSecurityPolicy.decision(
            sourceEncryption: sourceEncryption,
            targetEncryption: target.encryptionStatus
        ) {
        case .unavailable:
            forwardAlert = .blocked(
                String(localized: "Room encryption status is unavailable. Forwarding was not started.")
            )
            return
        case .confirmDowngrade:
            forwardAlert = .confirmDowngrade(target)
            return
        case .proceed:
            forward(to: target)
        }
    }

}

private struct ReactionPill: View {
    let title: String
    let count: Int
    var isSelected = false
    /// "👍, 3 reactions: Alice, Bob and you"; VoiceOver names the emoji.
    var accessibilityText: String
    var animationIndex = 0
    /// Adds or removes the user's reaction; nil when it cannot change.
    var onToggle: (() -> Void)?
    /// Opens the Reactions sheet on this emoji; nil when it is unavailable.
    var onShowSenders: (() -> Void)?

    private var reactionAnimationKey: String {
        "\(title)-\(count)"
    }

    var body: some View {
        if onToggle == nil, onShowSenders == nil {
            pill
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(accessibilityText)
                .accessibilityAddTraits(isSelected ? .isSelected : [])
        } else {
            // Tap toggles (or, when the reaction cannot change, shows who
            // reacted); a long press shows who reacted. Plain gestures, not a
            // Button, so finishing a long press never also toggles.
            pill
                .contentShape(Capsule())
                .onTapGesture(perform: activate)
                .onLongPressGesture(minimumDuration: 0.4) {
                    onShowSenders?()
                }
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(accessibilityText)
                .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
                .accessibilityHint(
                    onToggle == nil
                        ? TimelineRowAccessibility.showReactorsActionTitle
                        : TimelineRowAccessibility.reactionToggleHint(isOwn: isSelected)
                )
                .accessibilityAction { activate() }
                .modifier(ShowReactorsAccessibilityAction(onShowSenders: onShowSenders))
                .accessibilityIdentifier("ReactionPill-\(title)")
        }
    }

    private func activate() {
        if let onToggle {
            onToggle()
        } else {
            onShowSenders?()
        }
    }

    private var pill: some View {
        HStack(spacing: SynaraSpacing.xSmall) {
            Text(title)
                .font(SynaraTypography.messageMeta)
            Text("\(count)")
                .font(SynaraTypography.chipLabel)
                .monospacedDigit()
        }
        .padding(.horizontal, 7)
        .padding(.vertical, 3)
        .foregroundStyle(isSelected ? SynaraColor.accentText : SynaraColor.primaryText)
        .background(isSelected ? SynaraColor.accent.opacity(0.14) : SynaraColor.elevatedSurface)
        .clipShape(Capsule())
        .synaraDepth(.layered, shape: Capsule(), boundaryColor: isSelected ? SynaraColor.accent : SynaraColor.separator)
        .synaraReactionPop(animationIndex: animationIndex, animationKey: reactionAnimationKey)
    }
}

/// "Show who reacted" as a pill's custom action, where the sheet exists.
private struct ShowReactorsAccessibilityAction: ViewModifier {
    let onShowSenders: (() -> Void)?

    @ViewBuilder
    func body(content: Content) -> some View {
        if let onShowSenders {
            content.accessibilityAction(named: Text(TimelineRowAccessibility.showReactorsActionTitle)) {
                onShowSenders()
            }
        } else {
            content
        }
    }
}

/// Who reacted to a message: a tab per emoji with its count, and the people
/// behind it. Names fill in from the room's members once they load.
private struct TimelineReactionsSheet: View {
    let item: TimelineItem
    let initialKey: String?
    let currentUserID: String
    let roomID: String
    let directory: TimelineReactorDirectory
    let onDismiss: () -> Void
    @Environment(\.appEnvironment) private var environment
    @State private var selectedKey: String?
    @State private var members: [RoomMemberSummary] = []

    private var groups: [TimelineReactionGroup] {
        var directory = directory
        directory.merge(members: members)
        return TimelineReactionSenders.groups(for: item, currentUserID: currentUserID, directory: directory)
    }

    var body: some View {
        let groups = self.groups
        let selectedGroup = groups.first { $0.key == selectedKey } ?? groups.first
        NavigationStack {
            VStack(spacing: 0) {
                if let selectedGroup {
                    reactionTabs(groups, selectedKey: selectedGroup.key)
                    reactorList(selectedGroup)
                } else {
                    SynaraEmptyState(
                        title: String(localized: "No Reactions"),
                        systemImage: "face.smiling",
                        message: String(localized: "Nobody has reacted to this message.")
                    )
                    .frame(maxHeight: .infinity)
                }
            }
            .background(SynaraColor.surface)
            .navigationTitle("Reactions")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done", action: onDismiss)
                        .accessibilityIdentifier("TimelineReactionsDoneButton")
                }
            }
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
        .onAppear {
            if selectedKey == nil {
                selectedKey = TimelineReactionsSheetTarget(item: item, initialKey: initialKey).selectedKey(in: groups)
            }
        }
        .task(id: roomID) {
            members = await environment.roomManagement.roomDetails(roomID: roomID)?.members ?? []
        }
    }

    private func reactionTabs(_ groups: [TimelineReactionGroup], selectedKey: String) -> some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: SynaraSpacing.small) {
                ForEach(groups) { group in
                    let isSelected = group.key == selectedKey
                    let countText = String(localized: "\(group.count) reactions")
                    Button {
                        self.selectedKey = group.key
                    } label: {
                        HStack(spacing: SynaraSpacing.xSmall) {
                            Text(group.key)
                            Text("\(group.count)")
                                .monospacedDigit()
                        }
                        .font(SynaraTypography.chipLabel)
                        .foregroundStyle(isSelected ? SynaraColor.accentText : SynaraColor.primaryText)
                        .padding(.horizontal, SynaraSpacing.medium)
                        .frame(minHeight: SynaraHitTarget.minimum)
                        .background(isSelected ? SynaraColor.accent.opacity(0.14) : SynaraColor.secondarySurface)
                        .clipShape(Capsule())
                        .contentShape(Capsule())
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel(String(localized: "\(group.key), \(countText)"))
                    .accessibilityAddTraits(isSelected ? [.isSelected] : [])
                    .accessibilityIdentifier("TimelineReactionsTab-\(group.key)")
                }
            }
            .padding(.horizontal, SynaraSpacing.large)
            .padding(.vertical, SynaraSpacing.small)
        }
    }

    private func reactorList(_ group: TimelineReactionGroup) -> some View {
        List {
            ForEach(group.reactors) { reactor in
                HStack(spacing: SynaraSpacing.medium) {
                    TimelineAvatar(senderID: reactor.userID, avatarURL: reactor.avatarURL, size: 36)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(reactor.title)
                            .font(SynaraTypography.emphasis)
                            .foregroundStyle(SynaraColor.primaryText)
                            .lineLimit(1)
                        Text(reactor.userID)
                            .font(SynaraTypography.messageMeta)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    Spacer(minLength: 0)
                }
                .padding(.vertical, SynaraSpacing.xSmall)
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("TimelineReactor-\(reactor.userID)")
            }
            if group.unnamedCount > 0 {
                Text(String(localized: "\(group.unnamedCount) more people"))
                    .font(SynaraTypography.messageMeta)
                    .foregroundStyle(SynaraColor.secondaryText)
            }
        }
        .listStyle(.plain)
        .accessibilityIdentifier("TimelineReactorList")
    }
}

private struct AgentApprovalPromptTimelineCard: View {
    let prompt: SynaraAgentApprovalPrompt
    let eventID: String
    /// This account's own terminal reaction, projected by Core.
    let decision: SynaraAgentApprovalCardDecision?
    let onReaction: (String) -> Void
    @State private var confirmApproveAlways = false

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.medium) {
            HStack(alignment: .top, spacing: SynaraSpacing.small) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 17, weight: .semibold))
                    .foregroundStyle(SynaraColor.critical)
                    .accessibilityHidden(true)

                VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                    Text(prompt.title)
                        .font(SynaraTypography.emphasis)
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(2)
                        .accessibilityIdentifier("AgentApprovalPromptTitle-\(eventID)")
                    Text(prompt.body)
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }

            if let command = prompt.command {
                sourceBlock(command)
            } else if let sourceContext = prompt.sourceContext, sourceContext.isEmpty == false {
                sourceBlock(sourceContext)
            }

            if let decision {
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                    Image(systemName: decision.reaction.systemImage)
                        .font(SynaraTypography.emphasis)
                        .foregroundStyle(decision.reaction.tint)
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(decision.title)
                            .font(SynaraTypography.emphasis)
                            .foregroundStyle(SynaraColor.primaryText)
                        Text(decision.detail())
                            .font(SynaraTypography.messageMeta)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                    Spacer(minLength: 0)
                }
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("AgentApprovalPromptDecision-\(eventID)")
            } else {
                if confirmApproveAlways {
                    VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                        Text("Approve always permanently trusts this command pattern. Confirm only if you intend to allow it without future prompts.")
                            .font(SynaraTypography.messageMeta)
                            .foregroundStyle(SynaraColor.criticalText)
                            .fixedSize(horizontal: false, vertical: true)
                        HStack(spacing: SynaraSpacing.small) {
                            Button("Confirm approve always") {
                                confirmApproveAlways = false
                                onReaction(SynaraAgentApprovalPromptReaction.approveAlways.actionIdentifier)
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(SynaraColor.critical)
                            .accessibilityIdentifier("AgentApprovalPromptConfirmApproveAlways-\(eventID)")

                            Button("Cancel") {
                                confirmApproveAlways = false
                            }
                            .buttonStyle(.bordered)
                            .tint(SynaraColor.secondaryText)
                        }
                    }
                }

                ViewThatFits(in: .horizontal) {
                    HStack(spacing: SynaraSpacing.small) {
                        actionButtons(stacked: false)
                    }
                    VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                        actionButtons(stacked: true)
                    }
                }

                Text("To approve for the whole session, reply !approve session.")
                    .font(SynaraTypography.messageMeta)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    /// The command (or the full request when there is no command) shown
    /// once, as plain selectable text on a quiet inset.
    private func sourceBlock(_ text: String) -> some View {
        ScrollView(.vertical, showsIndicators: true) {
            Text(text)
                .font(.system(.caption, design: .monospaced))
                .foregroundStyle(SynaraColor.primaryText)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxHeight: 160)
        .fixedSize(horizontal: false, vertical: true)
        .padding(SynaraSpacing.small)
        .synaraAccessibleSurfaceFill(
            SynaraColor.mutedControl.opacity(0.45),
            opaqueFill: SynaraColor.surface
        )
        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
        .accessibilityIdentifier("AgentApprovalPromptSource-\(eventID)")
    }

    @ViewBuilder
    private func actionButtons(stacked: Bool) -> some View {
        ForEach(SynaraAgentApprovalPromptReaction.allCases) { action in
            if confirmApproveAlways, action == .approveAlways {
                EmptyView()
            } else {
                actionButton(action, stacked: stacked)
            }
        }
    }

    /// Side by side the labels keep one line so `ViewThatFits` can measure
    /// them; stacked (large Dynamic Type) they take the full width and wrap.
    @ViewBuilder
    private func actionButton(_ action: SynaraAgentApprovalPromptReaction, stacked: Bool) -> some View {
        let button = Button {
            if action == .approveAlways {
                confirmApproveAlways = true
            } else {
                confirmApproveAlways = false
                onReaction(action.actionIdentifier)
            }
        } label: {
            Label(action.title, systemImage: action.systemImage)
                .labelStyle(.titleAndIcon)
                .font(.subheadline.weight(.semibold))
                .lineLimit(stacked ? 2 : 1)
                .multilineTextAlignment(.leading)
                .fixedSize(horizontal: !stacked, vertical: false)
                .frame(
                    maxWidth: stacked ? .infinity : nil,
                    minHeight: SynaraHitTarget.minimum,
                    alignment: stacked ? .leading : .center
                )
                .contentShape(Rectangle())
                .foregroundStyle(action == .approveOnce ? Color.white : action.labelTint)
        }
        .controlSize(.large)
        .tint(action == .approveOnce ? SynaraColor.accentFill : action.buttonTint)
        .accessibilityLabel(action.accessibilityTitle)
        .accessibilityIdentifier("AgentApprovalPromptReaction-\(action.accessibilityIdentifierSuffix)-\(eventID)")

        // Approve once is the expected path; the other two stay quieter.
        if action == .approveOnce {
            button.buttonStyle(.borderedProminent)
        } else {
            button.buttonStyle(.bordered)
        }
    }
}

private struct AgentApprovalButtonStyle: ViewModifier {
    let action: SynaraAgentCardAction

    func body(content: Content) -> some View {
        switch action.kind {
        case .some("approve"):
            content
                .buttonStyle(.borderedProminent)
                .tint(SynaraColor.accentFill)
        case .some("reject"):
            content
                .foregroundStyle(SynaraColor.criticalText)
                .buttonStyle(.bordered)
                .tint(SynaraColor.critical)
        default:
            content
                .buttonStyle(.borderedProminent)
                .tint(action.tint)
        }
    }
}

private struct AgentCardLinkPreview: View {
    let urlString: String

    private var isPolicySafeLink: Bool {
        SynaraContractURLPolicy.isSafeHTTPS(urlString)
    }

    var body: some View {
        HStack(spacing: SynaraSpacing.xSmall) {
            Image(systemName: isPolicySafeLink ? "lock" : "link")
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityHidden(true)
            Text(urlString)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.accent)
                .lineLimit(1)
                .truncationMode(.middle)
            Spacer(minLength: 0)
        }
        .accessibilityElement(children: .combine)
        .accessibilityHint(isPolicySafeLink ? String(localized: "Opens HTTPS link") : "")
    }
}

private struct AgentCardTimelineRow: View {
    let card: SynaraAgentCard
    let onAction: (SynaraAgentCardAction) -> Void

    var body: some View {
        let visibleActions = card.actions.filter { SynaraAgentCardActionResolver.shouldRender($0) }

        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            VStack(alignment: .leading, spacing: 2) {
                Text(card.title)
                    .font(SynaraTypography.emphasis)
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(2)
                    .accessibilityIdentifier("AgentCardTitle")
                if let status = card.status {
                    Text(status)
                        .font(SynaraTypography.messageMeta.weight(.medium))
                        .foregroundStyle(SynaraColor.agentText)
                        .lineLimit(1)
                }
            }

            if let summary = card.summary {
                Text(summary)
                    .font(SynaraTypography.body)
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(3)
            }

            AgentApprovalDetails(card: card)

            if let linkAction = visibleActions.first(where: { $0.url != nil }),
               let previewURL = linkAction.url
            {
                AgentCardLinkPreview(urlString: previewURL)
            }

            if visibleActions.isEmpty == false {
                let approvalActions = visibleActions.filter(\.isApprovalDecision)
                let secondaryActions = visibleActions.filter { action in
                    action.isApprovalDecision == false && action.url != nil
                }

                ForEach(secondaryActions, id: \.id) { action in
                    Button {
                        onAction(action)
                    } label: {
                        HStack {
                            Text(action.title)
                                .font(SynaraTypography.body.weight(.medium))
                            Spacer()
                            Image(systemName: "arrow.up.right")
                                .font(SynaraTypography.messageMeta.weight(.semibold))
                                .accessibilityHidden(true)
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .tint(SynaraColor.accent)
                    .accessibilityHint("Performs \(action.title)")
                    .accessibilityIdentifier("AgentCardAction-\(action.id)")
                }

                if approvalActions.isEmpty == false {
                    HStack(spacing: SynaraSpacing.small) {
                        ForEach(approvalActions, id: \.id) { action in
                            Button {
                                onAction(action)
                            } label: {
                                Label(action.title, systemImage: action.systemImage)
                                    .labelStyle(.titleAndIcon)
                                    .font(SynaraTypography.body.weight(.semibold))
                                    .frame(maxWidth: .infinity)
                            }
                            .modifier(AgentApprovalButtonStyle(action: action))
                            .accessibilityHint("Performs \(action.title)")
                            .accessibilityIdentifier("AgentCardAction-\(action.id)")
                        }
                    }
                }
            }
        }
    }
}

/// Facts the card actually carries, as plain label/value rows. Rows
/// without data are left out rather than filled with placeholder text.
private struct AgentApprovalDetails: View {
    let card: SynaraAgentCard

    var body: some View {
        let rows = detailRows
        if rows.isEmpty == false {
            VStack(spacing: SynaraSpacing.xSmall) {
                ForEach(rows, id: \.title) { row in
                    AgentDetailRow(title: row.title, value: row.value)
                }
            }
            .padding(.vertical, SynaraSpacing.xSmall)
            .overlay(alignment: .top) {
                Rectangle()
                    .fill(SynaraColor.separator.opacity(0.6))
                    .frame(height: 0.5)
                    .accessibilityHidden(true)
            }
        }
    }

    private var detailRows: [(title: String, value: String)] {
        var rows: [(title: String, value: String)] = []
        if let target = card.artifacts.first?.title {
            rows.append((String(localized: "Target"), target))
        }
        let changes = card.artifacts.count + card.code.count + card.diffs.count
        if changes > 0 {
            rows.append((String(localized: "Changes"), String(localized: "\(changes) items changed")))
        }
        if card.logs.isEmpty == false {
            rows.append((String(localized: "Checks"), String(localized: "\(card.logs.count) passed")))
        }
        return rows
    }
}

private struct AgentDetailRow: View {
    let title: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.medium) {
            Text(title)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
                .frame(width: 76, alignment: .leading)
            Text(value)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.primaryText)
                .lineLimit(3)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .accessibilityElement(children: .combine)
    }
}

private extension SynaraAgentCardAction {
    var systemImage: String {
        switch kind {
        case .some("approve"):
            return "checkmark"
        case .some("reject"):
            return "xmark"
        case .some("open"), .some("open_url"):
            return "safari"
        default:
            return "doc.on.doc"
        }
    }

    /// Approve and reject are styled by `AgentApprovalButtonStyle`.
    var tint: Color {
        SynaraColor.agent
    }
}

private extension SynaraAgentApprovalPromptReaction {
    /// Monochrome glyphs for the buttons and decision line. The emoji in
    /// `reactionKey` are protocol values that are sent, never shown.
    var systemImage: String {
        switch self {
        case .approveOnce:
            return "checkmark"
        case .approveAlways:
            return "infinity"
        case .deny:
            return "xmark"
        }
    }

    var buttonTint: Color {
        self == .deny ? SynaraColor.critical : SynaraColor.accent
    }

    /// Label colour on the bordered (tinted) buttons; the AA text variants.
    var labelTint: Color {
        self == .deny ? SynaraColor.criticalText : SynaraColor.accentText
    }

    var accessibilityTitle: String {
        switch self {
        case .approveOnce:
            return String(localized: "Approve once")
        case .approveAlways:
            return String(localized: "Approve always")
        case .deny:
            return String(localized: "Deny")
        }
    }

    var tint: Color {
        switch self {
        case .approveOnce, .approveAlways:
            return SynaraColor.success
        case .deny:
            return SynaraColor.critical
        }
    }
}

private struct ComposerView: View {
    let roomID: String
    @Binding var text: String
    let placeholder: String
    var showsPromptMetrics = false
    let replyTarget: ComposerRelationTarget?
    let editTarget: ComposerRelationTarget?
    let uploadState: MediaUploadState
    let sendError: String?
    let onCancelRelation: () -> Void
    let onSend: (String) -> Void
    let onMockMediaUpload: (MediaUploadSource) -> Void
    let onFileURL: (URL) -> Void
    #if canImport(UIKit)
        let onCameraImage: (UIImage) -> Void
    #endif
    let onUploadFailed: (String) -> Void
    @Binding var selectedPhotos: [PhotosPickerItem]
    @Binding var attachmentDrafts: [ComposerAttachmentDraft]
    var isSending = false
    #if canImport(UIKit)
        var onPasteImages: ([UIImage]) -> Void = { _ in }
    #endif
    @Binding var isFocusedExternally: Bool
    /// The latest finished send; the composer drops what it sent and keeps
    /// what was typed since.
    var sentDraft: ComposerSentDraft?
    /// Sends a picked GIF as its provider link, with the composer's reply
    /// or thread relation; the typed draft is kept.
    var onSendGif: ((GifResult) -> Void)?
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @State private var typingNotifier: ComposerTypingNotifier?
    @State private var isAttachmentSheetPresented = false
    @State private var gifPickerRequest: GifPickerRequest?
    /// The attachment sheet hands over to the GIF picker once it is gone.
    @State private var opensGifPickerAfterAttachmentSheet = false
    @State private var gifCommandError: String?
    @State private var isFileImporterPresented = false
    #if canImport(UIKit)
        @State private var isCameraPresented = false
    #endif
    @State private var isFormattingBarVisible = false
    @State private var composerSelection = ComposerTextSelection.empty
    @State private var formattingRevision = 0
    @State private var liveText: String?
    @State private var draftPublishTask: Task<Void, Never>?
    @State private var composerFieldHeight: CGFloat = {
        #if canImport(UIKit)
            ComposerTextMetrics.singleLineHeight(font: UIFont.preferredFont(forTextStyle: .callout))
        #else
            34
        #endif
    }()

    #if canImport(UIKit)
        // The UIKit editor owns first-responder changes through its binding.
        // FocusState requires a SwiftUI .focused field, which this branch does not use.
        @State private var isComposerFocused = false
    #else
        @FocusState private var isComposerFocused: Bool
    #endif

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            if let replyTarget {
                ComposerRelationBanner(
                    target: replyTarget,
                    isCancelDisabled: isSending,
                    onCancel: onCancelRelation
                )
            }

            if let editTarget {
                ComposerRelationBanner(
                    target: editTarget,
                    isCancelDisabled: isSending,
                    onCancel: onCancelRelation
                )
            }

            if let sendError {
                Text(sendError)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.criticalText)
                    .accessibilityIdentifier("ComposerErrorText")
            }

            if let gifCommandError {
                Text(gifCommandError)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.criticalText)
                    .accessibilityIdentifier("ComposerGifCommandError")
            }

            if attachmentDrafts.isEmpty == false {
                composerAttachmentDrafts
            }

            if isFormattingBarVisible {
                ComposerFormattingBar { format in
                    applyFormatting(format)
                }
                .padding(.vertical, SynaraSpacing.xSmall)
                .transition(.move(edge: .bottom).combined(with: .opacity))
            }

            composerControls

            if showsPromptMetrics, shouldShowPromptMetrics {
                composerPromptMetrics
            }

            if case let .uploading(progress) = uploadState {
                ProgressView(value: progress)
                    .accessibilityIdentifier("MediaUploadProgress")
            } else if case let .failed(message) = uploadState {
                Text(message)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.criticalText)
                    .accessibilityIdentifier("MediaUploadErrorText")
            }
        }
        .padding(.horizontal, SynaraSpacing.small)
        .padding(.top, SynaraSpacing.xSmall)
        .padding(.bottom, SynaraSpacing.xSmall)
        .background {
            SynaraChrome.composer
                .ignoresSafeArea(.container, edges: .bottom)
        }
        .animation(.easeInOut(duration: 0.18), value: isFormattingBarVisible)
        .animation(.easeInOut(duration: 0.18), value: shouldShowPromptMetrics)
        .onChange(of: isComposerFocused) { focused in
            isFocusedExternally = focused
            updateOutgoingTyping()
        }
        .onChange(of: currentText) { _ in
            updateOutgoingTyping()
            gifCommandError = nil
        }
        .onChange(of: text) { value in
            reconcileExternalText(value)
        }
        .onChange(of: sentDraft) { sent in
            if let sent {
                finishSentDraft(sent)
            }
        }
        .onChange(of: roomID) { _ in
            typingNotifier?.stop()
            typingNotifier = makeTypingNotifier()
        }
        .onAppear {
            if liveText == nil {
                liveText = text
            }
            if typingNotifier == nil {
                typingNotifier = makeTypingNotifier()
            }
            isFocusedExternally = isComposerFocused
            updateOutgoingTyping()
        }
        .onDisappear {
            flushExternalText()
            isFocusedExternally = false
            typingNotifier?.stop()
        }
        .sheet(isPresented: $isAttachmentSheetPresented, onDismiss: {
            if opensGifPickerAfterAttachmentSheet {
                opensGifPickerAfterAttachmentSheet = false
                gifPickerRequest = GifPickerRequest()
            }
        }) {
            AttachmentOptionsSheet(
                onMockMediaUpload: { source in
                    isAttachmentSheetPresented = false
                    onMockMediaUpload(source)
                },
                onFile: {
                    isAttachmentSheetPresented = false
                    guard remainingAttachmentSlots > 0 else {
                        onUploadFailed(ComposerAttachmentDraftList.userMessage(for: .limitReached))
                        return
                    }
                    isFileImporterPresented = true
                },
                onCamera: {
                    isAttachmentSheetPresented = false
                    guard remainingAttachmentSlots > 0 else {
                        onUploadFailed(ComposerAttachmentDraftList.userMessage(for: .limitReached))
                        return
                    }
                    #if canImport(UIKit)
                        if CameraCaptureSupport.isAvailable {
                            isCameraPresented = true
                        } else {
                            onUploadFailed(String(localized: "Camera is not available on this device."))
                        }
                    #endif
                },
                selectedPhotos: $selectedPhotos,
                maxSelectionCount: remainingAttachmentSlots,
                onGif: canOfferGifPicker ? {
                    opensGifPickerAfterAttachmentSheet = true
                    isAttachmentSheetPresented = false
                } : nil
            )
            .presentationDetents([.height(canOfferGifPicker ? 320 : 260)])
            .presentationDragIndicator(.visible)
        }
        .sheet(item: $gifPickerRequest) { request in
            if let provider = gifSearch.provider {
                GifPickerSheet(provider: provider, initialQuery: request.initialQuery) { gif in
                    onSendGif?(gif)
                }
                .presentationDetents([.medium, .large])
                .presentationDragIndicator(.visible)
            }
        }
        .onChange(of: selectedPhotos) { items in
            if items.isEmpty == false {
                isAttachmentSheetPresented = false
            }
        }
        .fileImporter(
            isPresented: $isFileImporterPresented,
            allowedContentTypes: [.item],
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case let .success(urls):
                guard isSending == false, let url = urls.first else {
                    return
                }
                onFileURL(url)
            case .failure:
                onUploadFailed(String(localized: "Attachment could not be loaded. Try again."))
            }
        }
        #if canImport(UIKit)
        .fullScreenCover(isPresented: $isCameraPresented) {
            CameraImagePicker(
                onImage: { image in
                    isCameraPresented = false
                    onCameraImage(image)
                },
                onCancel: {
                    isCameraPresented = false
                }
            )
            .ignoresSafeArea()
        }
        #endif
    }

    private var sendButtonTint: Color {
        canSubmit && isSending == false ? SynaraColor.accent : SynaraColor.secondarySurface
    }

    @ViewBuilder
    private var composerControls: some View {
        if dynamicTypeSize.isAccessibilitySize {
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                composerInputSurface(showsFormattingToggle: false)

                HStack(spacing: SynaraSpacing.small) {
                    attachmentButton
                    formattingButton
                    Spacer(minLength: SynaraSpacing.small)
                    sendButton
                }
            }
        } else {
            HStack(alignment: .center, spacing: SynaraSpacing.xSmall) {
                attachmentButton
                composerInputSurface(showsFormattingToggle: true)
                sendButton
            }
        }
    }

    private var attachmentButton: some View {
        Button {
            isAttachmentSheetPresented = true
        } label: {
            Image(systemName: "plus")
                .font(.system(size: 16, weight: .semibold))
                .frame(width: 34, height: 34)
                .background(SynaraColor.secondarySurface)
                .foregroundStyle(SynaraColor.secondaryText)
                .clipShape(Circle())
                .synaraDepth(.layered, shape: Circle())
                .frame(width: 44, height: 44)
        }
        .buttonStyle(SynaraTactileButtonStyle())
        .contentShape(Rectangle())
        .disabled(isSending)
        .accessibilityLabel("Attach")
        .accessibilityIdentifier("AttachmentButton")
    }

    private var formattingButton: some View {
        Button {
            withAnimation(.easeInOut(duration: 0.18)) {
                isFormattingBarVisible.toggle()
            }
            isComposerFocused = true
        } label: {
            Image(systemName: isFormattingBarVisible ? "textformat.alt" : "textformat")
                .font(.system(size: 14, weight: .semibold))
                .frame(width: 28, height: 28)
                .foregroundStyle(isFormattingBarVisible ? SynaraColor.accent : SynaraColor.secondaryText)
                .background(isFormattingBarVisible ? SynaraColor.accent.opacity(0.12) : Color.clear)
                .clipShape(Circle())
                .frame(width: 44, height: 44)
        }
        .buttonStyle(SynaraTactileButtonStyle())
        .contentShape(Rectangle())
        .accessibilityLabel(isFormattingBarVisible ? "Hide formatting toolbar" : "Show formatting toolbar")
        .accessibilityAddTraits(isFormattingBarVisible ? .isSelected : [])
        .accessibilityIdentifier("ComposerFormattingToggle")
    }

    @ViewBuilder
    private var sendButton: some View {
        if canSubmit {
            Button(action: submitMessage) {
                Image(systemName: "paperplane.fill")
                    .font(.system(size: 16, weight: .semibold))
                    .frame(width: 34, height: 34)
                    .background(sendButtonTint)
                    .foregroundStyle(Color.white)
                    .clipShape(Circle())
                    .frame(width: 44, height: 44)
            }
            .buttonStyle(SynaraTactileButtonStyle())
            .contentShape(Rectangle())
            .disabled(isSending)
            .accessibilityLabel(editTarget == nil ? "Send" : "Save edit")
            .accessibilityHint(composerSendAccessibilityHint)
            .accessibilityIdentifier("ComposerSendButton")
        }
    }

    private func composerInputSurface(showsFormattingToggle: Bool) -> some View {
        HStack(alignment: .center, spacing: SynaraSpacing.xSmall) {
            composerField

            if showsFormattingToggle {
                formattingButton
            }
        }
        .padding(.leading, SynaraSpacing.small)
        .padding(.trailing, SynaraSpacing.xSmall)
        .padding(.vertical, 5)
        .background {
            RoundedRectangle(cornerRadius: SynaraRadius.composer, style: .continuous)
                .fill(SynaraColor.surface)
        }
        .synaraDepth(.layered, cornerRadius: SynaraRadius.composer)
    }

    private var canSubmit: Bool {
        if editTarget != nil {
            return currentText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
        }
        return ComposerAttachmentDraftList.canSend(text: currentText, drafts: attachmentDrafts)
    }

    private var resolvedPlaceholder: String {
        if let editTarget {
            return editTarget.isLocalPending
                ? String(localized: "Edit unsent message...")
                : String(localized: "Edit message...")
        }
        return placeholder
    }

    private var composerSendAccessibilityHint: String {
        if editTarget != nil {
            return isSending
                ? String(localized: "Saving the edited message")
                : String(localized: "Saves the edited message")
        }
        return isSending
            ? String(localized: "Sending the current message and attachments")
            : String(localized: "Sends the current message and attachments")
    }

    private var remainingAttachmentSlots: Int {
        max(0, ComposerAttachmentDraftList.maxCount - attachmentDrafts.count)
    }

    private var gifSearch: GifSearchAvailability {
        GifSearchAvailability.resolve(settings: environment.settings)
    }

    /// GIFs go out as new messages, so an edit never offers them.
    private var canOfferGifPicker: Bool {
        onSendGif != nil && editTarget == nil && gifSearch.provider != nil
    }

    /// `/gif` or `/gif <search>` as the whole message opens the picker with
    /// that search instead of sending. Returns whether it handled the text.
    private func handleGifCommand(_ messageBody: String) -> Bool {
        guard onSendGif != nil,
              editTarget == nil,
              let query = GifCommand.query(in: messageBody)
        else {
            return false
        }
        let availability = gifSearch
        guard availability.provider != nil else {
            gifCommandError = availability.commandUnavailableMessage
            return true
        }
        draftPublishTask?.cancel()
        liveText = ""
        composerSelection = .empty
        formattingRevision += 1
        text = ""
        gifCommandError = nil
        gifPickerRequest = GifPickerRequest(initialQuery: query)
        return true
    }

    private var composerAttachmentDrafts: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: SynaraSpacing.small) {
                ForEach(attachmentDrafts) { draft in
                    ComposerAttachmentDraftChip(draft: draft, isSending: isSending) {
                        guard isSending == false else {
                            return
                        }
                        attachmentDrafts = ComposerAttachmentDraftList.remove(
                            id: draft.id,
                            from: attachmentDrafts
                        )
                    }
                }
            }
            .padding(.horizontal, SynaraSpacing.xSmall)
            .padding(.vertical, SynaraSpacing.xSmall)
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("ComposerAttachmentDraftList")
        }
    }

    private var shouldShowPromptMetrics: Bool {
        currentText.isEmpty == false || isComposerFocused
    }

    private var composerLineCount: Int {
        max(1, currentText.components(separatedBy: .newlines).count)
    }

    private var composerPromptMetricsText: String {
        let characters = String(localized: "\(currentText.count) chars")
        let lines = String(localized: "\(composerLineCount) lines")
        return "\(characters) · \(lines)"
    }

    private var composerPromptMetrics: some View {
        HStack(spacing: SynaraSpacing.small) {
            Spacer()
            Text(composerPromptMetricsText)
                .font(SynaraTypography.composerMetric)
                .foregroundStyle(SynaraColor.tertiaryText)
                .monospacedDigit()
                .accessibilityIdentifier("ComposerPromptMetrics")
        }
        .transition(.opacity.combined(with: .move(edge: .bottom)))
    }

    @ViewBuilder
    private var composerField: some View {
        #if canImport(UIKit)
            ComposerTextView(
                text: composerTextBinding,
                selection: $composerSelection,
                height: $composerFieldHeight,
                placeholder: resolvedPlaceholder,
                formattingRevision: formattingRevision,
                isFocused: $isComposerFocused,
                onPasteImages: onPasteImages
            )
            .frame(height: composerFieldHeight)
        #else
            TextField(resolvedPlaceholder, text: composerTextBinding, axis: .vertical)
                .font(SynaraTypography.body)
                .focused($isComposerFocused)
                .lineLimit(1 ... 5)
                .frame(minHeight: composerFieldHeight)
                .accessibilityIdentifier("ComposerTextField")
                .onChange(of: currentText) { _ in
                    updateComposerFieldHeight()
                }
                .onChange(of: isComposerFocused) { _ in
                    updateComposerFieldHeight()
                }
        #endif
    }

    private func updateComposerFieldHeight() {
        #if canImport(UIKit)
            let singleLineHeight = ComposerTextMetrics.singleLineHeight(
                font: UIFont.preferredFont(forTextStyle: .callout)
            )
            if currentText.isEmpty, isComposerFocused == false {
                composerFieldHeight = singleLineHeight
                return
            }

            let lineCount = max(1, currentText.components(separatedBy: .newlines).count)
            let estimatedLineHeight = UIFont.preferredFont(forTextStyle: .callout).lineHeight
            let estimatedHeight = ceil(estimatedLineHeight * CGFloat(lineCount))
                + ComposerTextMetrics.textContainerInset.top
                + ComposerTextMetrics.textContainerInset.bottom
            composerFieldHeight = min(
                max(estimatedHeight, singleLineHeight),
                ComposerTextMetrics.maxHeight
            )
        #endif
    }

    private func applyFormatting(_ format: ComposerMarkdownFormat) {
        #if canImport(UIKit)
            let targetSelection = ComposerTextInputRegistry.selectionForFormatting(
                format,
                fallback: composerSelection
            )
        #else
            let targetSelection = composerSelection
        #endif
        let result = ComposerMarkdown.apply(format, to: currentText, selection: targetSelection)
        setLiveText(result.text)
        composerSelection = result.selection
        formattingRevision += 1
        isComposerFocused = true
    }

    private func submitMessage() {
        guard isSending == false else {
            return
        }
        // The keyboard stays up after a send, as in Messages; the cleared
        // draft reaches the focused editor through `reconcileExternalText`.
        #if canImport(UIKit)
            ComposerTextInputRegistry.commitPendingInput()
        #endif
        typingNotifier?.stop()
        let messageBody = currentText
        if handleGifCommand(messageBody) {
            return
        }
        text = messageBody
        onSend(messageBody)
    }

    private func updateOutgoingTyping() {
        let shouldType =
            isComposerFocused
            && currentText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            && SynaraSharedConstants.boolSetting(SynaraSharedConstants.hideActivityKey) == false
        typingNotifier?.update(isTyping: shouldType)
    }

    private func makeTypingNotifier() -> ComposerTypingNotifier {
        let matrix = environment.matrix
        let roomID = roomID
        return ComposerTypingNotifier(
            send: ComposerTypingNotifier.serialSender { typing in
                await matrix.setOutgoingTyping(roomID: roomID, typing: typing)
            }
        )
    }

    private var currentText: String {
        liveText ?? text
    }

    private var composerTextBinding: Binding<String> {
        Binding(
            get: { currentText },
            set: { setLiveText($0) }
        )
    }

    private func setLiveText(_ value: String) {
        liveText = value
        draftPublishTask?.cancel()
        draftPublishTask = Task { @MainActor in
            try? await Task.sleep(nanoseconds: 120_000_000)
            guard Task.isCancelled == false, text != value else { return }
            text = value
        }
    }

    private func reconcileExternalText(_ value: String) {
        guard value != currentText else { return }
        draftPublishTask?.cancel()
        liveText = value
        // A focused editor keeps its own text; a revision bump makes it take
        // this one (a send clearing the draft, an edit loading its body).
        composerSelection = ComposerTextSelection(location: (value as NSString).length, length: 0)
        formattingRevision += 1
    }

    /// Works from the live editor text, not the debounced draft, so
    /// keystrokes from the last moment before the send finished survive.
    private func finishSentDraft(_ sent: ComposerSentDraft) {
        let remaining = ComposerSentDraft.remainingText(current: currentText, sent: sent.sentText)
        draftPublishTask?.cancel()
        if remaining != currentText {
            liveText = remaining
            composerSelection = ComposerTextSelection(location: (remaining as NSString).length, length: 0)
            formattingRevision += 1
        }
        if text != remaining {
            text = remaining
        }
    }

    private func flushExternalText() {
        draftPublishTask?.cancel()
        let value = currentText
        if text != value {
            text = value
        }
    }
}

private struct ComposerFormattingBar: View {
    let onFormat: (ComposerMarkdownFormat) -> Void

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            // Keep every action at a 44-point hit target while the row scrolls
            // horizontally on compact devices.
            HStack(spacing: 2) {
                ForEach(ComposerMarkdownFormat.allCases) { format in
                    Button {
                        onFormat(format)
                    } label: {
                        Group {
                            if let headingLabel = format.headingLabel {
                                Text(headingLabel)
                                    .font(.system(size: 13, weight: .semibold))
                            } else {
                                Image(systemName: format.systemImage)
                                    .font(.system(size: 15, weight: .semibold))
                            }
                        }
                            .frame(width: 34, height: 34)
                            .background(SynaraColor.surface)
                            .foregroundStyle(SynaraColor.primaryText)
                            .clipShape(
                                RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)
                            )
                            .frame(width: 44, height: 44)
                    }
                    .buttonStyle(SynaraTactileButtonStyle())
                    .contentShape(Rectangle())
                    .accessibilityLabel(format.accessibilityLabel)
                    .accessibilityIdentifier("ComposerFormat-\(format.rawValue)")
                }
            }
            .padding(.horizontal, SynaraSpacing.xSmall)
            .padding(.vertical, SynaraSpacing.xSmall)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background {
            RoundedRectangle(cornerRadius: SynaraRadius.composer, style: .continuous)
                .fill(SynaraColor.secondarySurface)
        }
        .synaraDepth(.floating, cornerRadius: SynaraRadius.composer)
        .accessibilityIdentifier("ComposerFormattingBar")
    }
}

private struct AttachmentOptionsSheet: View {
    let onMockMediaUpload: (MediaUploadSource) -> Void
    let onFile: () -> Void
    let onCamera: () -> Void
    @Binding var selectedPhotos: [PhotosPickerItem]
    var maxSelectionCount: Int
    /// Opens GIF search; `nil` when this build or the user's settings do
    /// not offer it.
    var onGif: (() -> Void)?

    private var options: [AttachmentOption] {
        onGif == nil ? Self.mediaOptions : Self.mediaOptions + [Self.gifOption]
    }

    private static let gifOption = AttachmentOption(
        identifier: "GIF",
        title: String(localized: "GIF"),
        systemImage: "play.rectangle",
        tint: SynaraColor.accent,
        kind: .gif
    )

    private static let mediaOptions: [AttachmentOption] = [
        AttachmentOption(
            identifier: "Photo or Video",
            title: String(localized: "Photo or Video"),
            systemImage: "photo",
            tint: SynaraColor.success,
            kind: .photo
        ),
        AttachmentOption(
            identifier: "File",
            title: String(localized: "File"),
            systemImage: "doc",
            tint: SynaraColor.accent,
            kind: .file
        ),
        AttachmentOption(
            identifier: "Camera",
            title: String(localized: "Camera"),
            systemImage: "camera",
            tint: SynaraColor.warning,
            kind: .camera
        ),
    ]

    private var isUITestEnvironment: Bool {
        SynaraLaunchEnvironment.isUITest
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: SynaraSpacing.small) {
                ForEach(options) { option in
                    attachmentButton(for: option)
                }
            }
            .padding(.horizontal, SynaraSpacing.medium)
            .padding(.top, SynaraSpacing.small)
            .padding(.bottom, SynaraSpacing.medium)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
            .background(SynaraColor.surface)
            .navigationTitle("Attach")
            .navigationBarTitleDisplayMode(.inline)
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("AttachmentOptionsSheet")
    }

    @ViewBuilder
    private func attachmentButton(for option: AttachmentOption) -> some View {
        switch option.kind {
        case .photo:
            if isUITestEnvironment {
                Button {
                    onMockMediaUpload(.photoLibrary)
                } label: {
                    AttachmentOptionLabel(option: option)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("AttachmentOption-\(option.identifier)")
            } else if maxSelectionCount > 0 {
                PhotosPicker(
                    selection: $selectedPhotos,
                    maxSelectionCount: maxSelectionCount,
                    matching: .any(of: [.images, .videos])
                ) {
                    AttachmentOptionLabel(option: option)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("AttachmentOption-\(option.identifier)")
            } else {
                AttachmentOptionLabel(option: option)
                    .opacity(0.45)
                    .accessibilityIdentifier("AttachmentOption-\(option.identifier)")
            }
        case .file:
            Button {
                if maxSelectionCount == 0 {
                    return
                }
                if isUITestEnvironment {
                    onMockMediaUpload(.file)
                } else {
                    onFile()
                }
            } label: {
                AttachmentOptionLabel(option: option)
            }
            .buttonStyle(.plain)
            .disabled(maxSelectionCount == 0)
            .opacity(maxSelectionCount == 0 ? 0.45 : 1)
            .accessibilityIdentifier("AttachmentOption-\(option.identifier)")
        case .camera:
            Button {
                if maxSelectionCount == 0 {
                    return
                }
                if isUITestEnvironment {
                    onMockMediaUpload(.camera)
                } else {
                    onCamera()
                }
            } label: {
                AttachmentOptionLabel(option: option)
            }
            .buttonStyle(.plain)
            .disabled(maxSelectionCount == 0)
            .opacity(maxSelectionCount == 0 ? 0.45 : 1)
            .accessibilityIdentifier("AttachmentOption-\(option.identifier)")
        case .gif:
            // A GIF is a link message, not an attachment, so the attachment
            // limit does not apply.
            Button {
                onGif?()
            } label: {
                AttachmentOptionLabel(option: option)
            }
            .buttonStyle(.plain)
            .accessibilityLabel(Text("Send a GIF"))
            .accessibilityHint(Text("Searches KLIPY for a GIF to send"))
            .accessibilityIdentifier("AttachmentOption-\(option.identifier)")
        }
    }
}

private struct ComposerAttachmentDraftChip: View {
    let draft: ComposerAttachmentDraft
    var isSending = false
    let onRemove: () -> Void

    var body: some View {
        ZStack(alignment: .topTrailing) {
            preview
                .frame(width: 56, height: 56)
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)
                        .stroke(SynaraColor.separator.opacity(0.45), lineWidth: 0.5)
                        .allowsHitTesting(false)
                )
                .accessibilityElement()
                .accessibilityLabel(draft.displayName)
                .accessibilityIdentifier("ComposerAttachmentDraft-\(draft.displayName)")

            Button(action: onRemove) {
                Image(systemName: "xmark.circle.fill")
                    .font(.system(size: 16, weight: .semibold))
                    .symbolRenderingMode(.palette)
                    .foregroundStyle(Color.white, Color.black.opacity(0.72))
            }
            .buttonStyle(.plain)
            .disabled(isSending)
            .offset(x: 6, y: -6)
            .accessibilityLabel("Remove \(draft.displayName)")
            .accessibilityIdentifier("ComposerAttachmentDraftRemove-\(draft.displayName)")
        }
        .accessibilityElement(children: .contain)
    }

    @ViewBuilder
    private var preview: some View {
        #if canImport(UIKit)
            if draft.isImage, let image = UIImage(data: draft.data) {
                Image(uiImage: image)
                    .resizable()
                    .scaledToFill()
            } else {
                placeholder
            }
        #else
            placeholder
        #endif
    }

    private var placeholder: some View {
        ZStack {
            SynaraColor.secondarySurface
            Image(systemName: draft.previewSystemImage)
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(SynaraColor.secondaryText)
        }
    }
}

private struct AttachmentOption: Identifiable {
    enum Kind {
        case photo
        case file
        case camera
        case gif
    }

    /// Stable English name for the accessibility identifier; `title` is localized.
    let identifier: String
    let title: String
    let systemImage: String
    let tint: Color
    let kind: Kind

    var id: String { identifier }
}

private struct AttachmentOptionLabel: View {
    let option: AttachmentOption

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            icon
                .frame(width: 30, height: 30)
                .foregroundStyle(SynaraColor.secondaryText)

            Text(option.title)
                .font(SynaraTypography.supporting.weight(.semibold))
                .foregroundStyle(SynaraColor.primaryText)
                .lineLimit(1)
                .minimumScaleFactor(0.68)
                .allowsTightening(true)

            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, minHeight: 52, alignment: .leading)
        .padding(.horizontal, SynaraSpacing.medium)
        .background(SynaraColor.surface)
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .overlay(
            RoundedRectangle(cornerRadius: 14)
                .stroke(SynaraColor.separator.opacity(0.45), lineWidth: 0.5)
                .allowsHitTesting(false)
        )
    }

    /// SF Symbols has no GIF glyph; the GIF entry shows the word instead.
    @ViewBuilder
    private var icon: some View {
        if option.kind == .gif {
            Text(verbatim: "GIF")
                .font(.system(size: 10, weight: .heavy))
                .accessibilityHidden(true)
        } else {
            Image(systemName: option.systemImage)
                .font(.system(size: 18, weight: .semibold))
        }
    }
}

private struct ComposerToolIcon: View {
    let title: String?
    let systemImage: String?

    init(title: String) {
        self.title = title
        systemImage = nil
    }

    init(systemImage: String) {
        title = nil
        self.systemImage = systemImage
    }

    var body: some View {
        Group {
            if let title {
                Text(title)
                    .font(SynaraTypography.messageBody.weight(.medium))
            } else if let systemImage {
                Image(systemName: systemImage)
                    .font(SynaraTypography.messageBody.weight(.medium))
            }
        }
        .foregroundStyle(SynaraColor.secondaryText)
        .accessibilityHidden(true)
    }
}

private struct ComposerRelationBanner: View {
    let target: ComposerRelationTarget
    let isCancelDisabled: Bool
    let onCancel: () -> Void

    var body: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.small) {
            VStack(alignment: .leading, spacing: 2) {
                Label(target.bannerTitle, systemImage: target.kind == .edit ? "pencil" : "arrowshape.turn.up.left")
                    .font(SynaraTypography.supporting.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(1)
                    .accessibilityAddTraits(.isHeader)
                Text(target.snippet)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(2)
            }
            Spacer(minLength: SynaraSpacing.small)
            Button("Cancel", action: onCancel)
                .disabled(isCancelDisabled)
                .accessibilityLabel(target.kind == .edit ? "Cancel editing" : "Cancel reply")
        }
        .padding(SynaraSpacing.small)
        .synaraCard(fill: SynaraColor.secondarySurface)
        .accessibilityIdentifier(target.kind == .edit ? "ComposerEditBanner" : "ComposerReplyBanner")
    }
}

private struct MediaViewer: View {
    let resource: MediaResource
    @Environment(\.dismiss) private var dismiss
    @Environment(\.appEnvironment) private var environment
    @State private var image: UIImage?
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var scale: CGFloat = 1
    @State private var lastScale: CGFloat = 1

    var body: some View {
        NavigationStack {
            Group {
                if let errorMessage {
                    mediaErrorView(message: errorMessage)
                } else if let image {
                    ZoomableMediaImage(image: image, scale: $scale, lastScale: $lastScale)
                } else if isLoading {
                    VStack(spacing: SynaraSpacing.medium) {
                        ProgressView()
                        Text("Loading media...")
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else {
                    mediaErrorView(message: String(localized: "Media could not be loaded."))
                }
            }
            .padding(.horizontal, resource.isImageMedia ? 0 : SynaraSpacing.xLarge)
            .navigationTitle(resource.safeDescription)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
        .task(id: resource.id) {
            await loadMedia()
        }
        .accessibilityIdentifier("MediaViewer")
    }

    @ViewBuilder
    private func mediaErrorView(message: String) -> some View {
        VStack(spacing: SynaraSpacing.large) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 48, weight: .semibold))
                .foregroundStyle(SynaraColor.warning)
            Text(message)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
                .multilineTextAlignment(.center)
        }
        .padding(SynaraSpacing.xLarge)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    @MainActor
    private func loadMedia() async {
        image = nil
        errorMessage = nil
        scale = 1
        lastScale = 1

        guard resource.isEncrypted == false else {
            errorMessage = String(localized: "Encrypted media requires recovered keys before it can be opened.")
            return
        }

        guard resource.authenticatedURL != nil else {
            errorMessage = String(localized: "Media is unavailable.")
            return
        }

        isLoading = true
        defer { isLoading = false }

        guard let data = await environment.mediaLoader.loadMediaData(for: resource) else {
            errorMessage = String(localized: "Media could not be loaded.")
            return
        }
        let loadedImage: UIImage?
        if resource.isAnimatedImageMedia {
            loadedImage = await AnimatedImageDecoder.decode(data, limits: .fullScreen)
        } else {
            loadedImage = UIImage(data: data)
        }
        guard let loadedImage else {
            errorMessage = String(localized: "Media could not be loaded.")
            return
        }

        image = loadedImage
    }
}

private struct ZoomableMediaImage: View {
    let image: UIImage
    @Binding var scale: CGFloat
    @Binding var lastScale: CGFloat

    @ViewBuilder
    private var zoomedImage: some View {
        if image.isAnimated {
            AnimatedImageView(image: image, contentMode: .scaleAspectFit)
        } else {
            Image(uiImage: image)
                .resizable()
                .scaledToFit()
        }
    }

    var body: some View {
        GeometryReader { geometry in
            ScrollView([.horizontal, .vertical], showsIndicators: false) {
                zoomedImage
                    .frame(
                        width: geometry.size.width * max(scale, 1),
                        height: geometry.size.height * max(scale, 1)
                    )
                    .gesture(
                        MagnificationGesture()
                            .onChanged { value in
                                scale = max(1, lastScale * value)
                            }
                            .onEnded { value in
                                lastScale = max(1, lastScale * value)
                                scale = lastScale
                            }
                    )
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

/// The inline thread beneath a root message: the latest replies from the
/// thread's own stream, a way to reveal earlier ones, and entry points to
/// reply or open the full thread.
private struct InlineThreadReplies: View {
    let roomID: String
    let rootEventID: String
    let replyCount: Int
    let onReplyInThread: () -> Void
    let onOpenThread: () -> Void
    @Environment(\.appEnvironment) private var environment
    @State private var replies: [TimelineItem] = []
    @State private var limit = InlineThreadWindow.initialReplies
    @State private var isLoading = true
    @State private var isLoadingEarlier = false
    @State private var reachedStart = false
    @State private var failed = false
    @State private var updatesTask: Task<Void, Never>?

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            if isLoading && replies.isEmpty {
                ProgressView()
                    .controlSize(.small)
                    .accessibilityLabel("Loading replies")
            } else if failed && replies.isEmpty {
                Text("Replies could not be loaded.")
                    .font(SynaraTypography.messageMeta)
                    .foregroundStyle(SynaraColor.secondaryText)
            }

            if hasEarlier {
                Button(action: showEarlier) {
                    Group {
                        if isLoadingEarlier {
                            ProgressView().controlSize(.small)
                        } else {
                            Text("Show earlier replies")
                        }
                    }
                    .frame(minWidth: SynaraHitTarget.minimum, minHeight: SynaraHitTarget.minimum, alignment: .leading)
                    .contentShape(Rectangle())
                }
                .font(SynaraTypography.chipLabel)
                .foregroundStyle(SynaraColor.accentText)
                .buttonStyle(.plain)
                .disabled(isLoadingEarlier)
                .accessibilityIdentifier("InlineThreadEarlier-\(rootEventID)")
            }

            ForEach(InlineThreadWindow.visible(replies, limit: limit)) { reply in
                InlineThreadReplyRow(item: reply)
                    .contentShape(Rectangle())
                    .onTapGesture(perform: onOpenThread)
            }

            HStack(spacing: SynaraSpacing.medium) {
                Button(action: onReplyInThread) {
                    Label("Reply in thread", systemImage: "arrowshape.turn.up.left")
                        .frame(minHeight: SynaraHitTarget.minimum)
                        .contentShape(Rectangle())
                }
                .accessibilityIdentifier("InlineThreadReply-\(rootEventID)")
                Button(action: onOpenThread) {
                    Label("Open thread", systemImage: "arrow.up.left.and.arrow.down.right")
                        .frame(minHeight: SynaraHitTarget.minimum)
                        .contentShape(Rectangle())
                }
                .accessibilityIdentifier("InlineThreadOpen-\(rootEventID)")
            }
            .font(SynaraTypography.chipLabel)
            .foregroundStyle(SynaraColor.accentText)
            .buttonStyle(.plain)
        }
        .padding(.leading, SynaraSpacing.medium)
        .overlay(alignment: .leading) {
            Rectangle()
                .fill(SynaraColor.accent.opacity(0.35))
                .frame(width: 2)
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("InlineThread-\(rootEventID)")
        .task(id: roomID + rootEventID) {
            await load()
            startUpdates()
        }
        .onDisappear {
            updatesTask?.cancel()
            updatesTask = nil
            environment.timeline.closeThreadTimeline(roomID: roomID, rootEventID: rootEventID)
        }
    }

    private var hasEarlier: Bool {
        replies.count > limit || (reachedStart == false && replies.count < replyCount)
    }

    private func load() async {
        let outcome = await environment.timeline.loadThreadTimeline(roomID: roomID, rootEventID: rootEventID)
        apply(outcome)
        isLoading = false
    }

    private func startUpdates() {
        updatesTask?.cancel()
        updatesTask = Task {
            for await outcome in environment.timeline.threadTimelineUpdates(
                roomID: roomID,
                rootEventID: rootEventID
            ) {
                guard Task.isCancelled == false else { return }
                await MainActor.run { apply(outcome) }
            }
        }
    }

    @MainActor
    private func apply(_ outcome: TimelineLoadOutcome) {
        switch outcome {
        case let .loaded(items):
            replies = InlineThreadWindow.replies(in: items, rootEventID: rootEventID)
            reachedStart = reachedStart || items.contains { $0.eventID == rootEventID }
            failed = false
        case .empty:
            break
        case .failed:
            failed = true
        }
    }

    private func showEarlier() {
        if replies.count > limit {
            limit += InlineThreadWindow.revealStep
            return
        }
        isLoadingEarlier = true
        Task {
            let outcome = await environment.timeline.loadOlderThreadTimeline(
                roomID: roomID,
                rootEventID: rootEventID
            )
            await MainActor.run {
                if case .empty = outcome {
                    reachedStart = true
                }
                apply(outcome)
                limit += InlineThreadWindow.revealStep
                isLoadingEarlier = false
            }
        }
    }
}

private struct InlineThreadReplyRow: View {
    let item: TimelineItem

    var body: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.small) {
            TimelineAvatar(senderID: item.senderID, avatarURL: item.senderAvatarURL, size: 22)
            VStack(alignment: .leading, spacing: 2) {
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                    Text(item.senderDisplayName)
                        .font(SynaraTypography.emphasis)
                        .foregroundStyle(SynaraColor.headingText)
                        .lineLimit(1)
                    Text(item.timestamp.timelineTime)
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                }
                Text(item.threadTitle)
                    .font(SynaraTypography.body)
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(4)
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("InlineThreadItem-\(item.eventID)")
    }
}

private extension Date {
    var timelineTime: String {
        SynaraTimeFormatting.clockTime(self)
    }
}

private extension TimelineItem {
    var threadTitle: String {
        switch kind {
        case let .text(body):
            return body
        case let .formattedText(body, _):
            return body
        case let .mediaPlaceholder(resource):
            return resource.safeDescription
        case let .agentCard(card):
            return card.title
        case .redacted:
            return String(localized: "Deleted message")
        case .encryptedPlaceholder:
            return String(localized: "Encrypted message")
        case let .systemNotice(text):
            return text
        case .unknown:
            return TimelineUnavailableMessageCopy.text
        }
    }
}

struct RoomTimelineView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            RoomTimelineView(roomID: "!project:matrix.org", roomTitle: "Project")
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
