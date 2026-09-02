import PhotosUI
import SwiftUI
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
        guard hasUnreadMessages, liveItems.isEmpty == false else {
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
}

enum RoomTimelinePaginationPolicy {
    static func shouldLoadOlderHistory(
        rowIndex: Int,
        topThreshold: Int,
        hasUserInteractedWithTimeline: Bool,
        hasPositionedInitialTimeline: Bool,
        isJumpingToLatest: Bool,
        isPaginating: Bool,
        hasReachedOldestMessages: Bool
    ) -> Bool {
        guard rowIndex < topThreshold else {
            return false
        }
        guard hasUserInteractedWithTimeline,
              hasPositionedInitialTimeline,
              isJumpingToLatest == false,
              isPaginating == false,
              hasReachedOldestMessages == false
        else {
            return false
        }
        return true
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
        hasItems && (isLive == false || (requested && isConfirmedPinned == false))
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
            return "\(names[0]) is typing..."
        case 2:
            return "\(names[0]) and \(names[1]) are typing..."
        default:
            return "\(names[0]), \(names[1]), and \(names.count - 2) more are typing..."
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
    @State private var state: TimelineViewState = .idle
    @State private var draft: String = ""
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
    /// Used only for unread-divider presentation after the user has reached a live event.
    /// Must never drive post-load scroll restore (v1.2.28 open-at-live-end policy).
    @State private var initialReadMarkerEventID: String?
    @State private var hasReachedOldestMessages = false
    @State private var lastOlderPaginationAt = Date.distantPast
    @State private var paginationScrollAnchorID: String?
    @State private var isJumpingToLatest = false
    @State private var isComposerFocused = false
    @State private var isTimelineBottomVisible = false
    @State private var timelineBottomAnchorGeneration: UInt64 = 0
    @State private var lastMarkedFullyReadEventID: String?
    @State private var markFullyReadTask: Task<Void, Never>?
    @State private var markFullyReadTaskGeneration: UInt64 = 0
    @State private var pendingMarkFullyReadEventID: String?
    @State private var firstPendingMarkFullyReadAt: Date?
    @State private var timelineUpdatesTask: Task<Void, Never>?
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

    init(roomID: String, roomTitle: String?, focusedEventID: String? = nil) {
        self.roomID = roomID
        self.roomTitle = roomTitle
        self.focusedEventID = focusedEventID
    }

    var body: some View {
        VStack(spacing: 0) {
            TimelineHeader(
                title: roomTitle ?? "Room",
                subtitle: timelineSubtitle,
                cryptoLabel: cryptoStatus.roomHeaderLabel,
                cryptoSystemImage: cryptoStatus.roomHeaderSystemImage,
                onSearch: { isTimelineSearchPresented = true },
                onDetails: { isRoomDetailsPresented = true },
                onBack: {
                    dismissKeyboard()
                    dismiss()
                }
            )
            timelineContent
            if let failure = timelineAvailability.failure {
                TimelineAvailabilityBanner(failure: failure) {
                    Task { await loadTimeline() }
                }
            }
            if let typingText = RoomTypingPresentation.text(for: typingUserIDs) {
                RoomTypingIndicator(text: typingText)
            }
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
                isFocusedExternally: $isComposerFocused
            )
            .background(SynaraChrome.composer)
            .synaraDockedDepth(
                .floating,
                boundaryColor: isAgentRoom ? SynaraColor.agent : SynaraColor.separator
            )
        }
        .background(isAgentRoom ? SynaraChrome.agentReview : SynaraChrome.chat)
        .navigationTitle(roomTitle ?? "Room")
        .navigationBarBackButtonHidden(true)
        .toolbar(.hidden, for: .navigationBar)
        .toolbar(.hidden, for: .tabBar)
        .preferredColorScheme(isAgentRoom ? .dark : nil)
        .sheet(item: $viewerResource) { resource in
            MediaViewer(resource: resource)
        }
        .sheet(isPresented: $isRoomDetailsPresented) {
            RoomDetailsView(
                roomID: roomID,
                fallbackTitle: roomTitle ?? "Room",
                onLeaveRoom: {
                    shouldReturnToListAfterDetailsDismiss = true
                    isRoomDetailsPresented = false
                },
                onOpenMessage: { eventID in
                    isRoomDetailsPresented = false
                    Task { @MainActor in
                        await Task.yield()
                        environment.router.route(to: .room(id: roomID, eventID: eventID, title: roomTitle))
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
        .task(id: timelineTaskID) {
            resetTimelineState()
            let expectedTimelineTaskID = timelineTaskID
            let expectedUserID = currentUserID
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
            flushOutgoingSendsIfReady(environment.connectionStatus.status)
            _ = await initialCryptoStatus
            await ownAvatarLoad
            _ = await loadCryptoStatus()
        }
        .onDisappear {
            dismissKeyboard()
            stopTimelineUpdates(reason: "view-disappeared")
            stopTypingUpdates()
            cancelTimelineScroll()
            // A disappearing view no longer proves that its previously painted
            // tail is visible. Cancel the tracked automatic acknowledgement;
            // never launch an untracked read write from teardown.
            cancelMarkFullyRead()
            cancelTimestampReveal()
        }
        .onChange(of: scenePhase) { phase in
            if phase != .active {
                cancelMarkFullyRead()
            }
        }
        .onReceive(environment.outgoingSends.queue.$items) { _ in
            applyOutgoingQueueToTimeline()
        }
        .onReceive(environment.connectionStatus.$status) { status in
            flushOutgoingSendsIfReady(status)
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
            SynaraEmptyState(title: "No Messages", systemImage: "text.bubble", message: "Messages will appear here.")
        case let .failed(message):
            SynaraErrorState(title: "Could Not Load Timeline", message: message) {
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
                                    status: cryptoStatus,
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

                            ForEach(Array(items.enumerated()), id: \.element.eventID) { index, item in
                                if shouldShowUnreadDivider(before: item, at: index, in: items) {
                                    UnreadMessagesDivider()
                                        .padding(.vertical, SynaraSpacing.small)
                                }
                                TimelineRow(
                                    item: item,
                                    currentUserID: currentUserID,
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
                                    availability: environment.eventActions.availability(for: item, currentUserID: currentUserID),
                                    onReply: { beginReply(item) },
                                    onOpenThread: { openThread(root: item) },
                                    onEdit: { beginEdit(item) },
                                    onRedact: { applyAction(.redact, to: item) },
                                    onReact: { applyAction(.react("👍"), to: item) },
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
                                    }
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
                        if RoomTimelineJumpLatestPolicy.shouldShow(
                            isLive: timelineProviderIsLive,
                            isConfirmedPinned: isTimelineBottomVisible,
                            hasItems: items.last != nil,
                            requested: showJumpToLatest
                        ) {
                            JumpToLatestButton(isLoading: isJumpingToLatest) {
                                jumpToLatest(proxy: proxy, currentItems: items)
                            }
                            .padding(.trailing, SynaraSpacing.large)
                            .padding(.bottom, SynaraSpacing.medium)
                            .transition(.scale.combined(with: .opacity))
                        }
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
                dark: colorScheme == .dark
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
            onPaginationThresholdReached: { callbackRouteID, callbackGeneration, anchorEventID in
                guard callbackRouteID == stableViewportRouteID,
                      callbackGeneration == timelineBottomAnchorGeneration,
                      let oldestEventID = loadedTimelineItems.first?.eventID
                else {
                    return false
                }
                return loadOlderTimeline(before: oldestEventID, scrollAnchorID: anchorEventID)
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
            }
        )
        .id(routeID)
        .background(isAgentRoom ? SynaraChrome.agentReview : SynaraChrome.chat)
        .overlay(alignment: .bottomTrailing) {
            if RoomTimelineJumpLatestPolicy.shouldShow(
                isLive: timelineProviderIsLive,
                isConfirmedPinned: isTimelineBottomVisible,
                hasItems: items.isEmpty == false,
                requested: showJumpToLatest
            ) {
                JumpToLatestButton(isLoading: isJumpingToLatest) {
                    jumpToLatestStable(currentItems: items)
                }
                .padding(.trailing, SynaraSpacing.large)
                .padding(.bottom, SynaraSpacing.medium)
                .transition(.scale.combined(with: .opacity))
            }
        }
    }

    @ViewBuilder
    private func stableViewportRowContent(_ row: StableTimelineViewportRow) -> some View {
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
        case .unreadDivider:
            UnreadMessagesDivider()
                .padding(.horizontal, timelineHorizontalPadding)
                .padding(.vertical, SynaraSpacing.small)
        case let .event(eventRow):
            TimelineRow(
                item: eventRow.item,
                currentUserID: currentUserID,
                isGroupedWithPrevious: eventRow.isGroupedWithPrevious,
                isTimestampRevealed: eventRow.isTimestampRevealed,
                animateSend: eventRow.animateSend,
                replyPreview: eventRow.replyPreview,
                replyCount: eventRow.replyCount,
                availability: eventRow.availability,
                onReply: { beginReply(eventRow.item) },
                onOpenThread: { openThread(root: eventRow.item) },
                onEdit: { beginEdit(eventRow.item) },
                onRedact: { applyAction(.redact, to: eventRow.item) },
                onReact: { applyAction(.react("👍"), to: eventRow.item) },
                onPinToNotes: { pinToNotes(eventRow.item) },
                onOpenMedia: { resource in viewerResource = resource },
                onAgentAction: { action in
                    executeAgentAction(action, sourceEventID: eventRow.item.eventID)
                },
                onAgentApprovalReaction: { actionIdentifier in
                    submitAgentApprovalDecision(actionIdentifier: actionIdentifier, sourceEventID: eventRow.item.eventID)
                },
                onRetryFailedSend: { retryFailedMessage(eventRow.item) }
            )
            .padding(.horizontal, timelineHorizontalPadding)
            .padding(.bottom, SynaraSpacing.xSmall)
        }
    }

    private func stableViewportRows(items: [TimelineItem], isPaginating: Bool) -> [StableTimelineViewportRow] {
        let replyCounts = TimelineReplyCounter.replyCounts(for: items)
        let previews = TimelineReplyPreview.previewsByEventID(in: items, currentUserID: currentUserID)
        var rows: [StableTimelineViewportRow] = []
        rows.reserveCapacity(items.count + 3)

        if isPaginating {
            rows.append(.init(id: .pagination, content: .pagination))
        }
        if shouldShowCryptoBanner(items: items) {
            rows.append(.init(id: .cryptoBanner, content: .cryptoBanner(cryptoStatus)))
        }

        for (index, item) in items.enumerated() {
            let stableEventID = item.eventID.isEmpty ? item.id : item.eventID
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
                                currentUserID: currentUserID
                            )
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
                    roomNotesActionMessage = "Message pinned to your private notes."
                case .failure(let error):
                    roomNotesActionMessage = error.errorDescription ?? "Could not pin this message."
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
            return "Matrix room"
        }

        let participantCount = Set(items.map(\.senderID)).count
        guard participantCount > 0 else {
            return "Matrix room"
        }
        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1",
           roomID == "!project:matrix.org"
        {
            return "21 members"
        }
        return "\(participantCount) members"
    }

    private var isAgentRoom: Bool {
        environment.roomList.isAgentRoom(roomID: roomID)
    }

    private var timelineHorizontalPadding: CGFloat {
        horizontalSizeClass == .compact ? SynaraSpacing.small : SynaraSpacing.medium
    }

    private var composerPlaceholder: String {
        if let editTarget = editSession?.editTarget {
            return editTarget.isLocalPending ? "Edit unsent message..." : "Edit message..."
        }
        return isAgentRoom ? "Reply to the agent workflow..." : "Send a message..."
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
        stopTimelineUpdates(reason: "room-reset")
        stopTypingUpdates()
        cancelTimelineScroll()
        cancelTimestampReveal()
        ownAvatarURL = nil
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
        initialReadMarkerEventID = nil
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
        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1",
           roomID == "!project:matrix.org",
           index > 0,
           item.eventID.contains("$security:")
        {
            return true
        }

        guard let markerID = initialReadMarkerEventID,
              markerID.isEmpty == false,
              item.eventID != markerID,
              item.id != markerID
        else {
            return false
        }

        guard index > 0 else {
            return false
        }

        let previous = items[index - 1]
        return previous.eventID == markerID || previous.id == markerID
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
            storedPending: environment.outgoingSends.queue.timelineItems(in: roomID)
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

        let storedPending = environment.outgoingSends.queue.timelineItems(in: roomID)
        environment.outgoingSends.dropConfirmed(matching: streamItems, currentUserID: currentUserID)
        return TimelinePendingReconciler.merge(
            streamItems: streamItems,
            localItems: TimelinePendingReconciler.combining(
                localItems: localItems,
                storedPending: storedPending
            ),
            currentUserID: currentUserID
        )
    }

    private func openThread(root item: TimelineItem) {
        environment.router.route(
            to: .thread(
                roomID: roomID,
                rootEventID: item.eventID,
                roomTitle: roomTitle,
                rootTitle: item.threadTitle
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
            initialReadMarkerEventID = nil
            timelinePosition = focusedEventID == nil ? .preparing : .focusedEvent
        }
        logTimelineEvent("initial-load-started")
        let signpostID = PerformanceTrace.begin("TimelineInitialLoad")
        defer {
            PerformanceTrace.end("TimelineInitialLoad", id: signpostID)
        }
        await MainActor.run {
            initialReadMarkerEventID = nil
        }
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
        if let focusedEventID, focusedEventID.isEmpty == false {
            guard let focusedFeed = await session.open(mode: .focused(eventID: focusedEventID)) else {
                return
            }
            feed = focusedFeed
        } else {
            guard let liveFeed = await session.open(mode: .live) else {
                return
            }
            let hasUnreadMessages = environment.roomList.hasUnreadMessages(roomID: roomID)
            let fullyReadEventID = hasUnreadMessages
                ? await environment.readMarkers.fullyReadEventID(roomID: roomID)
                : nil
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

        await MainActor.run {
            applySessionFeed(feed)
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
        initialReadMarkerEventID = {
            if case let .unread(markerEventID) = feed.mode {
                return markerEventID
            }
            return nil
        }()
        applyTimelineOutcome(feed.initialOutcome)
        if StableScrollAnchoringFeatureFlag.isEnabled {
            enqueueStableInitialCommand(for: feed.mode, generation: feed.generation)
        }
        logTimelineEvent(
            "stream-started",
            fields: ["mode": feed.mode.isLive ? "live" : "focused", "generation": "\(feed.generation)"]
        )
        timelineUpdatesTask = Task {
            for await outcome in feed.updates {
                guard Task.isCancelled == false else {
                    return
                }
                await MainActor.run {
                    switch outcome {
                    case let .loaded(items):
                        applyTimelineOutcome(.loaded(items))
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

    private func loadCryptoStatus() async -> RoomCryptoStatus {
        let traceID = PerformanceTrace.begin("LoadCryptoStatus")
        defer { PerformanceTrace.end("LoadCryptoStatus", id: traceID) }
        let status = await environment.crypto.roomStatus(roomID: roomID)
        await MainActor.run {
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
        Task {
            for await update in environment.crypto.verificationUpdates() {
                if case .finished = update, cryptoStatus.unableToDecryptCount > 0 {
                    // Post-verification success: auto-retry decryption to clear "Retry Decryption" / UTD banners
                    // in this room. This is the strict requirement for the flow after successful SAS.
                    _ = await environment.crypto.retryDecryption(roomID: roomID)
                    _ = await loadCryptoStatus()
                    logTimelineEvent("post-verification-retry", fields: ["utdBefore": "\(cryptoStatus.unableToDecryptCount)"])
                }
            }
        }
    }

    private func retryDecryption() {
        Task {
            let result = await environment.crypto.retryDecryption(roomID: roomID)
            _ = await loadCryptoStatus()
            await MainActor.run {
                stopTimelineUpdates(reason: "decryption-reload")
            }
            await loadTimeline()
            await MainActor.run {
                cryptoActionMessage = result.message
            }
        }
    }

    private func shouldShowCryptoBanner(items: [TimelineItem]) -> Bool {
        guard isCryptoBannerDismissed == false else {
            return false
        }

        if cryptoStatus.needsCryptoActionBanner {
            return true
        }

        guard cryptoStatus.isEncrypted else {
            return false
        }

        return items.contains { item in
            if case .encryptedPlaceholder = item.kind {
                return true
            }
            return false
        }
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
                await performSend(composerIntent)
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
                    append(item)
                }
            )
            if uploaded {
                await MainActor.run {
                    attachmentSendTransaction = nil
                }
                if let trailingText = ComposerAttachmentSendPlan.trailingText(in: plan) {
                    await performSend(composerIntent.replacingBody(with: trailingText))
                } else {
                    await MainActor.run {
                        uploadState = .idle
                        draft = ""
                        environment.drafts.clearDraft(roomID: roomID)
                        completeComposerRelation()
                    }
                }
            }
            await MainActor.run {
                isSendingMessage = false
            }
        }
    }

    private func retryFailedMessage(_ item: TimelineItem) {
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

    @MainActor
    private func performSend(
        body rawBody: String,
        replyToEventID: String?,
        threadRootEventID: String?,
        editEventID: String?,
        retrying failedItem: TimelineItem? = nil
    ) async {
        let body = rawBody.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            sendError = MessageSendError.emptyMessage.localizedDescription
            return
        }

        let request = MessageSendRequest(
            roomID: roomID,
            body: body,
            formattedBody: ComposerMatrixFormatting.formattedBody(for: body),
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
                draft = ""
                environment.drafts.clearDraft(roomID: roomID)
                completeComposerRelation()
                sendError = nil
            } catch {
                sendError = MessageSendError.failed.localizedDescription
                SynaraHaptics.trigger(.warning)
            }
            return
        }

        let queued = environment.outgoingSends.enqueue(
            localID: failedItem?.id ?? "$pending-\(UUID().uuidString)",
            roomID: roomID,
            body: body,
            formattedBody: request.formattedBody,
            replyToEventID: replyToEventID,
            threadRootEventID: threadRootEventID,
            senderID: currentUserID,
            senderAvatarURL: ownAvatarURL,
            timestamp: failedItem?.timestamp ?? Date()
        )
        registerSendAnimation(for: queued.id, isRetry: failedItem != nil)
        applyOutgoingQueueToTimeline()

        draft = ""
        environment.drafts.clearDraft(roomID: roomID)
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
        await environment.outgoingSends.transmitIfNeeded(id: id)
        await MainActor.run {
            applyOutgoingQueueToTimeline()
            guard let status = environment.outgoingSends.queue.item(id: id)?.deliveryStatus else {
                return
            }
            switch status {
            case .sent:
                sendError = nil
                SynaraHaptics.trigger(.lightImpact)
                if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1",
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

    private func flushOutgoingSendsIfReady(_ status: MatrixSyncStatus) {
        guard OutgoingSendPolicy.isSendReady(status) else {
            return
        }
        Task {
            await environment.outgoingSends.flushWhenSendReady()
            await MainActor.run {
                applyOutgoingQueueToTimeline()
            }
        }
    }

    private func applyOutgoingQueueToTimeline() {
        let pendingItems = environment.outgoingSends.queue.timelineItems(in: roomID)
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
                uploadState = .failed("Attachment could not be loaded. Try again.")
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
                uploadState = .failed("Attachment could not be loaded. Try again.")
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
                    uploadState = .failed("Attachment could not be loaded. Try again.")
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
    private func loadOlderTimeline(before eventID: String?, scrollAnchorID: String? = nil) -> Bool {
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
            hasReachedOldestMessages: hasReachedOldestMessages
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

        timelinePosition = .readingHistory
        logTimelineEvent("pagination-started", fields: ["rendered": "\(items.count)"])
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
                    state = .loaded(items, isPaginating: false)
                    paginationScrollAnchorID = nil
                }
                return
            }
            let updatedGeneration = await timelineSession.currentGeneration()
            await MainActor.run {
                let currentItems = loadedTimelineItems.isEmpty ? items : loadedTimelineItems
                switch outcome {
                case let .loaded(boundedItems):
                    timelineAvailability.recordSuccess()
                    let currentServerIDs = Set(currentItems.filter { $0.isLocalPending == false }.map(\.eventID))
                    let addedCount = boundedItems.reduce(into: 0) { count, item in
                        if currentServerIDs.contains(item.eventID) == false {
                            count += 1
                        }
                    }
                    stopTimelineUpdates(reason: "history-provider-activated")
                    timelineBottomAnchorGeneration = updatedGeneration
                    activeTimelineMode = .focused(eventID: eventID)
                    timelineProviderIsLive = false
                    let merged = mergeTimelineItems(boundedItems, isPaginating: false)
                    state = .loaded(merged, isPaginating: false)
                    showJumpToLatest = true
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

    private func scheduleMarkFullyRead(eventID: String) {
        guard RoomTimelineReadAcknowledgementPolicy.shouldSchedule(
            isApplicationActive: scenePhase == .active
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

        pendingMarkFullyReadEventID = eventID
        if firstPendingMarkFullyReadAt == nil {
            firstPendingMarkFullyReadAt = Date()
        }
        guard markFullyReadTask == nil else {
            return
        }

        let firstQueuedAt = firstPendingMarkFullyReadAt ?? Date()
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
                  scenePhase == .active,
                  UIApplication.shared.applicationState == .active,
                  SynaraSharedConstants.boolSetting(
                      SynaraSharedConstants.hideActivityKey
                  ) == false,
                  timelineProviderIsLive,
                  isTimelineBottomVisible,
                  isJumpingToLatest == false,
                  isUserDraggingTimeline == false,
                  pendingMarkFullyReadEventID != nil
            else {
                await MainActor.run {
                    clearMarkFullyReadTask(ifGenerationMatches: installedGeneration)
                }
                return
            }

            guard let observedEventID = pendingMarkFullyReadEventID else {
                await MainActor.run {
                    clearMarkFullyReadTask(ifGenerationMatches: installedGeneration)
                }
                return
            }
            pendingMarkFullyReadEventID = nil
            firstPendingMarkFullyReadAt = nil
            let didAcknowledge = await environment.readMarkers.markFullyRead(
                roomID: roomID,
                eventID: observedEventID
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
                    lastMarkedFullyReadEventID = observedEventID
                    initialReadMarkerEventID = observedEventID
                    showJumpToLatest = false
                }
                if pendingMarkFullyReadEventID != nil,
                   let nextEventID = pendingMarkFullyReadEventID
                {
                    scheduleMarkFullyRead(eventID: nextEventID)
                }
            }
        }
    }

    private func cancelMarkFullyRead() {
        markFullyReadTaskGeneration &+= 1
        markFullyReadTask?.cancel()
        markFullyReadTask = nil
        pendingMarkFullyReadEventID = nil
        firstPendingMarkFullyReadAt = nil
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
        showJumpToLatest = true

        Task {
            let signpostID = PerformanceTrace.begin("TimelineJumpToLatest")
            defer {
                PerformanceTrace.end("TimelineJumpToLatest", id: signpostID)
            }
            let transition = await timelineSession.transitionToLive()
            await MainActor.run {
                switch transition {
                case let .succeeded(feed):
                    initialReadMarkerEventID = nil
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

    private func jumpToLatestStable(currentItems: [TimelineItem]) {
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
        showJumpToLatest = true

        Task {
            let signpostID = PerformanceTrace.begin("TimelineJumpToLatest")
            defer { PerformanceTrace.end("TimelineJumpToLatest", id: signpostID) }
            let transition = await timelineSession.transitionToLive()
            await MainActor.run {
                switch transition {
                case let .succeeded(feed):
                    initialReadMarkerEventID = nil
                    hasPositionedInitialTimeline = false
                    timelinePosition = .placingInitial
                    applySessionFeed(feed)
                    enqueueStableViewportCommand(.latest(animated: true), generation: feed.generation)
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
        if isPinned, timelineProviderIsLive {
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
        let nextMode = RoomTimelineProviderPresentationPolicy.modeWhenPinned(
            providerIsLive: timelineProviderIsLive,
            currentMode: activeTimelineMode
        )
        guard nextMode != activeTimelineMode else {
            return
        }
        activeTimelineMode = nextMode
        initialReadMarkerEventID = nil
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
        case .readMarker:
            hasPositionedInitialTimeline = true
            timelinePosition = .readingHistory
            showJumpToLatest = true
        case .focused:
            hasAnchoredEvent = success
            hasPositionedInitialTimeline = true
            timelinePosition = .focusedEvent
            showJumpToLatest = targetEventID != loadedTimelineItems.last?.eventID
            if success == false {
                timelineAvailability.recordFailure(
                    TimelineLoadFailure(
                        kind: .viewUnavailable,
                        diagnosticCode: "timeline-focused-event-unavailable"
                    ),
                    preservingRows: true
                )
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

    private func sendComposerText(_ rawBody: String) {
        sendMessage(body: rawBody)
    }

    @MainActor
    private func performSend(_ intent: ComposerSendIntent) async {
        await performSend(
            body: intent.body,
            replyToEventID: intent.replyToEventID,
            threadRootEventID: intent.threadRootEventID,
            editEventID: intent.editEventID,
            retrying: intent.retrying
        )
    }

    private func applyAction(_ action: EventActionType, to item: TimelineItem) {
        Task {
            do {
                let updated = try await environment.eventActions.apply(action, to: item, currentUserID: currentUserID, roomID: roomID)
                await MainActor.run {
                    replace(updated)
                }
            } catch {
                await MainActor.run {
                    sendError = "Action could not be completed. Try again."
                }
            }
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

    private func append(_ item: TimelineItem) {
        switch state {
        case let .loaded(items, isPaginating):
            state = .loaded(items + [item], isPaginating: isPaginating)
        default:
            state = .loaded([item], isPaginating: false)
        }
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
                agentActionMessage = "Unsupported action: \(unsupported)"
            case .missingPayload:
                agentActionMessage = "Action payload is missing"
            case .unsafeURL:
                agentActionMessage = "Action link is not allowed"
            case .encodingFailure:
                agentActionMessage = "Could not copy action payload"
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
                    agentActionMessage = decision == .approve ? "Agent action approved" : "Agent action rejected"
                }
            } catch let error as SynaraAgentApprovalError {
                await MainActor.run {
                    agentActionMessage = error.errorDescription ?? "Agent action could not be submitted"
                }
            } catch {
                await MainActor.run {
                    agentActionMessage = "Agent action could not be submitted"
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
                        ? "This approval was already decided on this account."
                        : "Approval decision sent."
                }
            } catch let error as SynaraAgentApprovalError {
                await MainActor.run {
                    agentActionMessage = error.errorDescription ?? "Approval decision could not be submitted."
                }
            } catch {
                await MainActor.run {
                    agentActionMessage = "Approval decision could not be submitted."
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

private struct TimelineHeader: View {
    let title: String
    let subtitle: String
    let cryptoLabel: String?
    let cryptoSystemImage: String
    let onSearch: () -> Void
    let onDetails: () -> Void
    let onBack: () -> Void

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Button(action: onBack) {
                Image(systemName: "chevron.left")
                    .font(.system(size: 19, weight: .semibold))
                    .frame(width: 34, height: 34)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Back")

            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: SynaraSpacing.xSmall) {
                    Text("#")
                        .foregroundStyle(SynaraColor.secondaryText)
                    Text(title)
                        .font(SynaraTypography.sectionTitle.weight(.semibold))
                        .foregroundStyle(SynaraColor.headingText)
                        .lineLimit(1)
                }
                HStack(spacing: SynaraSpacing.small) {
                    Text(subtitle)
                    if let cryptoLabel {
                        Label(cryptoLabel, systemImage: cryptoSystemImage)
                            .foregroundStyle(
                                cryptoLabel == "Encrypted"
                                    ? SynaraColor.secondaryText
                                    : SynaraColor.warning
                            )
                            .accessibilityIdentifier("RoomEncryptionStatus")
                    }
                }
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
                .lineLimit(1)
            }

            Spacer()

            HStack(spacing: SynaraSpacing.small) {
                Button(action: onSearch) {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 17, weight: .medium))
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Search messages")
                .accessibilityIdentifier("TimelineSearchButton")
                Button(action: onDetails) {
                    Image(systemName: "ellipsis")
                        .font(.system(size: 17, weight: .medium))
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Room details")
                .accessibilityIdentifier("RoomDetailsButton")
            }
            .foregroundStyle(SynaraColor.primaryText)
        }
        .padding(.horizontal, SynaraSpacing.large)
        .padding(.vertical, SynaraSpacing.medium)
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
                        title: "Search Messages",
                        systemImage: "magnifyingglass",
                        message: "Filter messages currently loaded in this room."
                    )
                    .frame(maxHeight: .infinity)
                } else if filteredItems.isEmpty {
                    SynaraEmptyState(
                        title: "No Matching Messages",
                        systemImage: "text.magnifyingglass",
                        message: "Try another keyword from the loaded timeline."
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
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @State private var state: TimelineViewState = .idle
    @State private var draft = ""
    @State private var sendError: String?
    @State private var uploadState: MediaUploadState = .idle
    @State private var selectedPhotos: [PhotosPickerItem] = []
    @State private var attachmentDrafts: [ComposerAttachmentDraft] = []
    @State private var attachmentSendSteps: [ComposerAttachmentSendStep]?
    @State private var isSendingMessage = false
    @State private var isComposerFocused = false
    @State private var threadUpdatesTask: Task<Void, Never>?
    @State private var notesActionMessage: String?

    var body: some View {
        VStack(spacing: 0) {
            ThreadHeader(
                subtitle: rootTitle ?? roomTitle ?? "Room message",
                onBack: { dismiss() }
            )

            threadContent

            Divider()

            ComposerView(
                roomID: roomID,
                text: $draft,
                placeholder: "Reply in thread...",
                replyTarget: nil,
                editTarget: nil,
                uploadState: uploadState,
                sendError: sendError,
                onCancelRelation: {},
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
                isFocusedExternally: $isComposerFocused
            )
        }
        .background(SynaraColor.surface)
        .navigationBarBackButtonHidden(true)
        .toolbar(.hidden, for: .navigationBar)
        .toolbar(.hidden, for: .tabBar)
        .task(id: roomID + rootEventID) {
            await loadThread()
            startThreadUpdates()
        }
        .onDisappear {
            threadUpdatesTask?.cancel()
            threadUpdatesTask = nil
        }
        .onChange(of: selectedPhotos) { items in
            draftThreadPickedPhotos(items)
        }
        .alert("Personal Notes", isPresented: notesActionMessageBinding) {
            Button("OK", role: .cancel) { notesActionMessage = nil }
        } message: {
            Text(notesActionMessage ?? "Try again.")
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
            SynaraEmptyState(title: "No Thread Replies", systemImage: "bubble.left.and.bubble.right", message: "Replies will appear here.")
        case let .failed(message):
            SynaraErrorState(title: "Could Not Load Thread", message: message) {
                Task {
                    await loadThread()
                }
            }
        case let .loaded(items, _):
            let visibleItems = threadItems(from: items)
            ScrollView {
                LazyVStack(alignment: .leading, spacing: SynaraSpacing.medium) {
                    ForEach(visibleItems) { item in
                        ThreadMessageRow(
                            item: item,
                            onPinToNotes: { pinToNotes(item) }
                        )
                    }
                }
                .padding(.horizontal, SynaraSpacing.xLarge)
                .padding(.top, SynaraSpacing.large)
                .padding(.bottom, SynaraSpacing.large)
            }
            .accessibilityIdentifier("ThreadTimelineList")
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
            state = items.isEmpty ? .empty : .loaded(items, isPaginating: false)
        case .empty:
            state = .empty
        case let .failed(failure):
            state = .failed(failure.userMessage)
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

    private func pinToNotes(_ item: TimelineItem) {
        Task {
            let result = await environment.roomNotes.pinMessage(roomID: roomID, item: item)
            await MainActor.run {
                switch result {
                case .success:
                    notesActionMessage = "Message pinned to your private room notes."
                case .failure(let error):
                    notesActionMessage = error.errorDescription ?? "The message could not be pinned."
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
                    replyToEventID: nil,
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
                    draft = ""
                    isSendingMessage = false
                }
                return
            }

            do {
                let signpostID = PerformanceTrace.begin("ThreadMessageSend")
                defer {
                    PerformanceTrace.end("ThreadMessageSend", id: signpostID)
                }
                let item = try await environment.messageSender.send(
                    MessageSendRequest(
                        roomID: roomID,
                        body: trailingText,
                        formattedBody: ComposerMatrixFormatting.formattedBody(for: trailingText),
                        replyToEventID: nil,
                        editEventID: nil,
                        threadRootEventID: rootEventID
                    )
                )
                await MainActor.run {
                    attachmentSendSteps = nil
                    draft = ""
                    append(item)
                    isSendingMessage = false
                }
            } catch let error as MessageSendError {
                await MainActor.run {
                    sendError = error.localizedDescription
                    isSendingMessage = false
                }
            } catch {
                await MainActor.run {
                    sendError = MessageSendError.failed.localizedDescription
                    isSendingMessage = false
                }
            }
        }
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
                    uploadState = .failed("Attachment could not be loaded. Try again.")
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
                uploadState = .failed("Attachment could not be loaded. Try again.")
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
                uploadState = .failed("Attachment could not be loaded. Try again.")
                return
            }
            applyThreadIncomingDrafts(incoming)
        }
    #endif

    private func append(_ item: TimelineItem) {
        switch state {
        case let .loaded(items, isPaginating):
            state = .loaded(items + [item], isPaginating: isPaginating)
        default:
            state = .loaded([item], isPaginating: false)
        }
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

private struct ThreadMessageRow: View {
    let item: TimelineItem
    let onPinToNotes: () -> Void
    @State private var isSelectingText = false

    var body: some View {
        VStack(spacing: SynaraSpacing.medium) {
            HStack(alignment: .top, spacing: SynaraSpacing.medium) {
                TimelineAvatar(senderID: item.senderID, avatarURL: item.senderAvatarURL, size: 34)

                VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                    HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                        Text(item.senderDisplayName)
                            .font(SynaraTypography.emphasis)
                            .foregroundStyle(SynaraColor.primaryText)
                        Text(item.timestamp.timelineTime)
                            .font(SynaraTypography.messageMeta)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }

                    threadBody

                    if item.reactions.isEmpty == false {
                        HStack(spacing: SynaraSpacing.xSmall) {
                            ForEach(Array(item.reactions.keys.sorted().enumerated()), id: \.element) { index, reaction in
                                ReactionPill(
                                    title: reaction,
                                    count: item.reactions[reaction] ?? 0,
                                    isSelected: item.reactionOwnership.contains(reaction),
                                    animationIndex: index
                                )
                            }
                            ReactionPill(
                                title: "face.smiling",
                                count: nil,
                                isSystemImage: true,
                                animationIndex: item.reactions.count
                            )
                        }
                    }
                }

                Spacer(minLength: 0)
            }

            Divider()
                .padding(.leading, 46)
        }
        .contextMenu {
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
            if item.actionCapabilities?.canPin ?? (item.serverEventID != nil) {
                Button("Pin to Notes", systemImage: "note.text.badge.plus", action: onPinToNotes)
                    .accessibilityIdentifier("ThreadItemPinToNotes-\(item.eventID)")
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("ThreadItem-\(item.eventID)")
        .sheet(isPresented: $isSelectingText) {
            if let copyPayload = TimelineMessageCopy.payload(for: item) {
                MessageTextSelectionSheet(payload: copyPayload)
            }
        }
    }

    @ViewBuilder
    private var threadBody: some View {
        if let poll = item.poll {
            TimelinePollCard(poll: poll)
        } else {
            switch item.kind {
        case let .text(body):
            Text(body)
                .font(SynaraTypography.messageBody)
                .foregroundStyle(SynaraColor.primaryText)
                .lineSpacing(2.5)
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
        case let .formattedText(body, html):
            MatrixFormattedMessageView(fallbackBody: body, html: html, font: SynaraTypography.messageBody)
        case let .mediaPlaceholder(resource):
            MediaAttachmentCard(resource: resource)
        case .redacted:
            Text("Message deleted")
                .font(SynaraTypography.messageBody)
                .foregroundStyle(SynaraColor.secondaryText)
        case .encryptedPlaceholder:
            Label("Encrypted content unavailable.", systemImage: "lock")
                .font(SynaraTypography.messageBody)
                .foregroundStyle(SynaraColor.secondaryText)
        case let .agentCard(card):
            AgentCardTimelineRow(card: card, onAction: { _ in })
        case let .unknown(type):
            Text("Unsupported event: \(type)")
                .font(SynaraTypography.messageBody)
                .foregroundStyle(SynaraColor.secondaryText)
            }
        }
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

                    Text(attributedRichText(projection.richText, includeLinks: false))
                        .font(SynaraTypography.messageBody)
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineSpacing(2.5)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .fixedSize(horizontal: false, vertical: true)
                        .accessibilityLabel(projection.richText.plainText)
                        .accessibilityHint("Select and copy any part of this message")
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
                containsSpoilers: false
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
                let label = block.reason.map { "[Spoiler: \($0) · Reveal]" } ?? "[Spoiler · Reveal]"
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
                Text(block.reason.map { "Spoiler: \($0) · Reveal" } ?? "Spoiler · Reveal")
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
                block.reason.map { "Reveal spoiler: \($0)" } ?? "Reveal spoiler"
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
            return "Column \(cellIndex + 1), \(cell)"
        }
        let headers = block.rows.first(where: \.isHeader)?.cells ?? []
        let header = headers.indices.contains(cellIndex) ? headers[cellIndex].plainText : nil
        if let header, header.isEmpty == false {
            return "Row \(rowIndex + 1), \(header): \(cell)"
        }
        return "Row \(rowIndex + 1), column \(cellIndex + 1): \(cell)"
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
                Text(block.language ?? "Code")
                    .font(SynaraTypography.chipLabel)
                    .foregroundStyle(SynaraColor.primaryText)

                Spacer()

                Button("Copy") {
                    #if canImport(UIKit)
                        UIPasteboard.general.string = code
                    #endif
                }
                .font(SynaraTypography.chipLabel)
                .buttonStyle(.plain)
                .foregroundStyle(SynaraColor.primaryText)
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
        switch senderID.lowercased() {
        case "@mina:matrix.org":
            return "Mina"
        case "@alex:matrix.org":
            return "Alex"
        case "@ravi:matrix.org":
            return "Ravi"
        case "@local:matrix.org", "@you:matrix.org":
            return "You"
        default:
            guard senderID.hasPrefix("@") else {
                return senderID
            }
            return senderID
                .dropFirst()
                .split(separator: ":")
                .first
                .map(String.init) ?? senderID
        }
    }

    private var avatarFill: LinearGradient {
        let colors: [Color]
        switch senderID.lowercased() {
        case "@mina:matrix.org":
            colors = [Color(red: 0.88, green: 0.48, blue: 0.32), Color(red: 0.34, green: 0.18, blue: 0.12)]
        case "@alex:matrix.org":
            colors = [Color(red: 0.16, green: 0.53, blue: 0.65), Color(red: 0.07, green: 0.16, blue: 0.24)]
        case "@ravi:matrix.org":
            colors = [Color(red: 0.70, green: 0.42, blue: 0.23), Color(red: 0.16, green: 0.10, blue: 0.08)]
        default:
            colors = [SynaraColor.accent, SynaraColor.secondaryText]
        }
        return LinearGradient(colors: colors, startPoint: .topLeading, endPoint: .bottomTrailing)
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
                    SettingsInfo(title: "Room ID", value: roomID)
                    SettingsInfo(title: "Encryption", value: details?.isEncrypted == true ? "Encrypted" : "Not encrypted")
                    SettingsInfo(title: "Members", value: "\(details?.memberCount ?? 0)")
                    SettingsInfo(title: "Avatar", value: details?.avatarURL ?? "None")
                }

                if let powerLevels = details?.powerLevels {
                    Section("Permissions") {
                        SettingsInfo(title: "Your level", value: "\(powerLevels.ownUserLevel)")
                        PermissionInfo(title: "Send messages", threshold: powerLevels.eventsDefault, allowed: true)
                        PermissionInfo(title: "Invite users", threshold: powerLevels.invite, allowed: powerLevels.canInvite)
                        PermissionInfo(title: "Change name", threshold: powerLevels.roomName, allowed: powerLevels.canEditName)
                        PermissionInfo(title: "Change topic", threshold: powerLevels.roomTopic, allowed: powerLevels.canEditTopic)
                        PermissionInfo(title: "Change avatar", threshold: powerLevels.roomAvatar, allowed: powerLevels.canEditAvatar)
                        PermissionInfo(title: "Moderate", threshold: powerLevels.kick, allowed: powerLevels.canKick || powerLevels.canBan || powerLevels.canRedactOther)
                    }
                    .accessibilityIdentifier("RoomPermissionsSection")
                }

                Section("Notifications") {
                    Picker("Mode", selection: $notificationMode) {
                        ForEach(SynaraRoomNotificationMode.allCases) { mode in
                            Text(mode.rawValue).tag(mode)
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
                    ForEach(details?.members.prefix(12) ?? []) { member in
                        RoomMemberPresenceRow(member: member)
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
                    message = "Profile updated."
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
                    message = "Notification mode updated."
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
                    message = "Invitation sent."
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
                    message = "Avatar updated."
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
                    message = "Avatar removed."
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

private struct RoomMemberPresenceRow: View {
    let member: RoomMemberSummary
    @Environment(\.appEnvironment) private var environment
    @State private var presence: SharedCorePresence?

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            Text(member.userID)
                .font(SynaraTypography.body)
            Text(presence?.displayName ?? member.membership)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
        }
        .accessibilityIdentifier("RoomMemberPresenceRow")
        .task(id: member.userID) {
            presence = await environment.matrix.presence(userID: member.userID)
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
                title: allowed ? "Allowed" : "Restricted",
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
    let status: RoomCryptoStatus
    let onRetry: () -> Void
    let onReviewSecurity: () -> Void
    let onDismiss: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            HStack(alignment: .top, spacing: SynaraSpacing.small) {
                Label(title, systemImage: "lock.trianglebadge.exclamationmark")
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

            Text(detail)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
                .fixedSize(horizontal: false, vertical: true)

            HStack(spacing: SynaraSpacing.small) {
                Button("Retry Decryption", action: onRetry)
                    .buttonStyle(.borderedProminent)
                    .accessibilityIdentifier("EncryptedRecoveryRetryButton")
                Button("Review Security", action: onReviewSecurity)
                    .buttonStyle(.bordered)
                    .accessibilityIdentifier("EncryptedRecoverySettingsButton")
            }
        }
        .padding(SynaraSpacing.medium)
        .synaraCard(fill: SynaraColor.warning.opacity(0.10), stroke: SynaraColor.warning)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("EncryptedRecoveryBanner")
    }

    private var title: String {
        "Encrypted history needs attention"
    }

    private var detail: String {
        if status.verification == .unverified {
            return "This device is not verified. Verify another session from Settings before trusting encrypted history."
        }
        if status.recovery == .incomplete {
            return "This room is encrypted, but recovery is incomplete. Verify another session or recover keys before acting on undecrypted messages."
        }
        if status.unableToDecryptCount > 0 {
            return "Some encrypted events are missing keys. Retry decryption after sync, or review device verification and recovery in Settings."
        }
        return "Encrypted messages in this room need attention. Review device verification and recovery in Settings."
    }
}

private struct TimelineRow: View {
    let item: TimelineItem
    let currentUserID: String
    let isGroupedWithPrevious: Bool
    let isTimestampRevealed: Bool
    let animateSend: Bool
    let replyPreview: TimelineReplyPreview?
    let replyCount: Int
    let availability: EventActionAvailability
    let onReply: () -> Void
    let onOpenThread: () -> Void
    let onEdit: () -> Void
    let onRedact: () -> Void
    let onReact: () -> Void
    let onPinToNotes: () -> Void
    let onOpenMedia: (MediaResource) -> Void
    let onAgentAction: (SynaraAgentCardAction) -> Void
    let onAgentApprovalReaction: (String) -> Void
    let onRetryFailedSend: () -> Void
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @State private var isSelectingText = false

    var body: some View {
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
                            Text(item.timestamp.timelineTime)
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
                                .synaraDepth(.raised, shape: Circle())
                                .frame(width: 44, height: 44)
                                .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Message actions")
                        .accessibilityIdentifier("TimelineItemActions-\(item.eventID)")
                    }
                }

                messageContent
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.top, isGroupedWithPrevious ? 0 : 7)
        .contextMenu {
            messageActions
        }
        .accessibilityElement(children: accessibilityChildBehavior)
        .accessibilityLabel(accessibilitySummary)
        .accessibilityHint(accessibilityHint)
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
                    Text(item.timestamp.timelineTime)
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
        if TimelineRowAccessibility.retryActionTitle(deliveryStatus: item.deliveryStatus) != nil {
            content.accessibilityAction(named: Text("Retry"), onRetryFailedSend)
        } else {
            content
        }
    }

    @ViewBuilder
    private var messageActions: some View {
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
        }
        if replyCount > 0 {
            Button("Open Thread", systemImage: "bubble.left.and.bubble.right", action: onOpenThread)
        }
        if availability.canEdit {
            Button("Edit", systemImage: "pencil", action: onEdit)
        }
        if availability.canReact {
            Button("React", systemImage: "face.smiling", action: onReact)
        }
        if item.actionCapabilities?.canPin ?? (item.serverEventID != nil) {
            Button("Pin to Notes", systemImage: "note.text.badge.plus", action: onPinToNotes)
                .accessibilityIdentifier("TimelineItemPinToNotes-\(item.eventID)")
        }
        if availability.canRedact {
            Button("Redact", systemImage: "trash", role: .destructive, action: onRedact)
        }
    }

    private var failedMessageEditButton: some View {
        HStack(spacing: rowHorizontalSpacing) {
            Color.clear
                .frame(width: isGroupedWithPrevious ? groupedLeadingGutter : avatarSize, height: 1)
            Button(action: onEdit) {
                Label("Edit", systemImage: "pencil")
                    .font(SynaraTypography.chipLabel)
                    .foregroundStyle(SynaraColor.accent)
                    .padding(.horizontal, SynaraSpacing.small)
                    .padding(.vertical, SynaraSpacing.xSmall)
                    .background(SynaraColor.accent.opacity(0.12))
                    .clipShape(Capsule())
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

            if item.reactions.isEmpty == false {
                HStack(spacing: SynaraSpacing.xSmall) {
                    ForEach(Array(item.reactions.keys.sorted().enumerated()), id: \.element) { index, reaction in
                        ReactionPill(
                            title: reaction,
                            count: item.reactions[reaction] ?? 0,
                            isSelected: item.reactionOwnership.contains(reaction),
                            animationIndex: index
                        )
                    }
                    ReactionPill(
                        title: "face.smiling",
                        count: nil,
                        isSystemImage: true,
                        animationIndex: item.reactions.count
                    )
                }
            }

            if replyCount > 0 {
                Button(action: onOpenThread) {
                    HStack(spacing: SynaraSpacing.xSmall) {
                        Image(systemName: "bubble.left.and.bubble.right.fill")
                        Text(replyCount == 1 ? "1 reply" : "\(replyCount) replies")
                    }
                    .font(SynaraTypography.chipLabel)
                    .foregroundStyle(SynaraColor.accent)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("ThreadButton-\(item.eventID)")
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
                TimelinePollCard(poll: poll)
            }
        } else if let approvalPrompt {
            SynaraMessageBubble(
                alignment: bubbleAlignment,
                variant: .agent,
                depth: .critical,
                isGrouped: isGroupedWithPrevious,
                showsBackground: true,
                deliveryStatus: nil
            ) {
                AgentApprovalPromptTimelineCard(
                    prompt: approvalPrompt,
                    eventID: item.eventID,
                    onReaction: onAgentApprovalReaction
                )
            }
        } else {
            switch item.kind {
            case let .text(body):
                SynaraMessageBubble(
                    text: body,
                    alignment: bubbleAlignment,
                    isGrouped: isGroupedWithPrevious,
                    deliveryStatus: item.deliveryStatus,
                    statusEventID: item.eventID,
                    onRetryFailedSend: item.deliveryStatus == .failed ? onRetryFailedSend : nil
                )
            case let .formattedText(body, html):
                SynaraMessageBubble(
                    alignment: bubbleAlignment,
                    variant: .standard,
                    depth: SynaraSurfaceDepthRole.standardMessage,
                    isGrouped: isGroupedWithPrevious,
                    showsBackground: SynaraSurfaceDepthRole.standardMessageShowsBackground,
                    deliveryStatus: item.deliveryStatus,
                    statusEventID: item.eventID,
                    onRetryFailedSend: item.deliveryStatus == .failed ? onRetryFailedSend : nil
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
                Text("Unsupported event: \(type)")
                    .font(SynaraTypography.messageBody)
                    .foregroundStyle(SynaraColor.secondaryText)
            }
        }
    }

    private var bubbleAlignment: SynaraMessageBubbleAlignment {
        isOutgoing ? .own : .other
    }

    private var accessibilitySummary: String {
        let senderAndTime = "\(item.senderID) at \(item.timestamp.timelineTime)"
        switch item.kind {
        case let .text(body):
            return "\(senderAndTime): \(body)"
        case let .formattedText(body, _):
            return "\(senderAndTime): \(body)"
        case let .mediaPlaceholder(resource):
            if resource.isEncrypted {
                return "\(senderAndTime) sent encrypted media that cannot be opened until keys are available"
            }
            if let caption = resource.caption {
                return "\(senderAndTime) sent \(resource.safeDescription): \(caption)"
            }
            return "\(senderAndTime) sent \(resource.safeDescription)"
        case .redacted:
            return "\(senderAndTime): message deleted"
        case .encryptedPlaceholder:
            return "\(senderAndTime): encrypted message unavailable"
        case let .unknown(type):
            return "\(senderAndTime): unsupported event \(type)"
        case let .agentCard(card):
            let status = card.status.map { ", status \($0)" } ?? ""
            let primaryAction = card.actions.first(where: SynaraAgentCardActionResolver.shouldRender)
                .map { ", primary action \($0.title)" } ?? ""
            return "\(senderAndTime): agent card: \(card.title)\(status)\(primaryAction)"
        }
    }

    private var accessibilityChildBehavior: AccessibilityChildBehavior {
        if isGroupedWithPrevious == false {
            return .contain
        }

        return TimelineRowAccessibility.containsChildren(
            deliveryStatus: item.deliveryStatus,
            kind: item.kind,
            replyCount: replyCount,
            hasApprovalPrompt: approvalPrompt != nil
        ) ? .contain : .combine
    }

    private var accessibilityHint: String {
        if approvalPrompt != nil {
            return "Review available approval reactions"
        }

        if item.deliveryStatus == .failed {
            return availability.canEdit
                ? "Tap Retry to send this message again. Edit is also available."
                : "Tap Retry to send this message again"
        }

        switch item.kind {
        case .agentCard:
            return "Review available agent actions"
        default:
            return isGroupedWithPrevious
                ? "Swipe left to reveal the sent time. Long press for message actions"
                : "Use the Message actions button or long press for message actions"
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
            SynaraIconTile(
                title: resource.safeDescription,
                systemImage: resource.safeDescription.localizedCaseInsensitiveContains(".pdf") ? "doc.richtext.fill" : "doc.text.fill",
                tint: resource.safeDescription.localizedCaseInsensitiveContains(".pdf") ? SynaraColor.critical : SynaraColor.accent,
                size: 30
            )

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
                        .foregroundStyle(SynaraColor.warning)
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
            Image(uiImage: thumbnailImage)
                .resizable()
                .scaledToFill()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .clipped()
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

private struct UnreadMessagesDivider: View {
    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Rectangle()
                .fill(SynaraColor.separator.opacity(0.55))
                .frame(height: 0.5)
            Text("Unread messages")
                .font(SynaraTypography.chipLabel)
                .foregroundStyle(SynaraColor.accent)
                .lineLimit(1)
            Rectangle()
                .fill(SynaraColor.separator.opacity(0.55))
                .frame(height: 0.5)
        }
        .padding(.leading, 46)
    }
}

private struct TimelinePollCard: View {
    let poll: TimelinePollPresentation

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
                HStack(spacing: SynaraSpacing.small) {
                    Image(systemName: answer.isOwn ? "checkmark.circle.fill" : "circle")
                        .foregroundStyle(answer.isOwn ? SynaraColor.accent : SynaraColor.secondaryText)
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
                .background(answer.isOwn ? SynaraColor.accent.opacity(0.12) : SynaraColor.elevatedSurface)
                .clipShape(RoundedRectangle(cornerRadius: 9, style: .continuous))
                .accessibilityElement(children: .combine)
                .accessibilityAddTraits(answer.isOwn ? .isSelected : [])
                .accessibilityLabel("\(answer.text), \(answer.voteCount) votes")
            }

            Text(pollFooter)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Poll: \(poll.question)")
    }

    private var pollFooter: String {
        if poll.isClosed {
            return "Poll closed"
        }
        if poll.maximumSelections == 1 {
            return "Choose one answer"
        }
        return "Choose up to \(poll.maximumSelections) answers"
    }
}

private struct ReactionPill: View {
    let title: String
    let count: Int?
    var isSystemImage = false
    var isSelected = false
    var animationIndex = 0

    private var reactionAnimationKey: String {
        if let count {
            return "\(title)-\(count)-\(isSystemImage)"
        }
        return "\(title)-\(isSystemImage)"
    }

    var body: some View {
        HStack(spacing: SynaraSpacing.xSmall) {
            if isSystemImage {
                Image(systemName: title)
                    .font(SynaraTypography.messageMeta)
            } else {
                Text(title)
                    .font(SynaraTypography.messageMeta)
            }
            if let count {
                Text("\(count)")
                    .font(SynaraTypography.chipLabel)
                    .monospacedDigit()
            }
        }
        .padding(.horizontal, 7)
        .padding(.vertical, 3)
        .foregroundStyle(isSelected ? SynaraColor.accent : SynaraColor.primaryText)
        .background(isSelected ? SynaraColor.accent.opacity(0.14) : SynaraColor.elevatedSurface)
        .clipShape(Capsule())
        .synaraDepth(.raised, shape: Capsule())
        .accessibilityAddTraits(isSelected ? .isSelected : [])
        .synaraReactionPop(animationIndex: animationIndex, animationKey: reactionAnimationKey)
    }
}

private struct AgentApprovalPromptTimelineCard: View {
    let prompt: SynaraAgentApprovalPrompt
    let eventID: String
    let onReaction: (String) -> Void
    @State private var isCommandExpanded = true
    @State private var isSourceExpanded = false
    @State private var confirmApproveAlways = false

    private var actionColumns: [GridItem] {
        Array(
            repeating: GridItem(.flexible(minimum: 68), spacing: SynaraSpacing.small),
            count: SynaraAgentApprovalPromptReaction.allCases.count
        )
    }

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.medium) {
            HStack(alignment: .top, spacing: SynaraSpacing.small) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 19, weight: .semibold))
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
                DisclosureGroup(isExpanded: $isCommandExpanded) {
                    ScrollView(.vertical, showsIndicators: true) {
                        Text(command)
                            .font(.system(.caption, design: .monospaced))
                            .foregroundStyle(SynaraColor.primaryText)
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.top, SynaraSpacing.xSmall)
                    }
                    .frame(maxHeight: 160)
                } label: {
                    Text(prompt.commandPreview.map { "Command: \($0)" } ?? "Review command")
                        .font(SynaraTypography.messageMeta.weight(.medium))
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(2)
                }
                .padding(SynaraSpacing.small)
                .synaraAccessibleSurfaceFill(
                    SynaraColor.surface.opacity(0.72),
                    opaqueFill: SynaraColor.surface
                )
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
                .synaraDepth(
                    .raised,
                    cornerRadius: SynaraRadius.control,
                    boundaryColor: SynaraColor.critical
                )
            }

            if let sourceContext = prompt.sourceContext, sourceContext.isEmpty == false {
                DisclosureGroup(isExpanded: $isSourceExpanded) {
                    VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                        if let replyInstructions = prompt.replyInstructions {
                            Text(replyInstructions)
                                .font(.system(.caption2, design: .monospaced))
                                .foregroundStyle(SynaraColor.secondaryText)
                                .textSelection(.enabled)
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                        ScrollView(.vertical, showsIndicators: true) {
                            Text(sourceContext)
                                .font(.system(.caption2, design: .monospaced))
                                .foregroundStyle(SynaraColor.secondaryText)
                                .textSelection(.enabled)
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                        .frame(maxHeight: 140)
                    }
                    .padding(.top, SynaraSpacing.xSmall)
                } label: {
                    Text("Full approval prompt")
                        .font(SynaraTypography.messageMeta.weight(.medium))
                        .foregroundStyle(SynaraColor.primaryText)
                }
                .padding(SynaraSpacing.small)
                .synaraAccessibleSurfaceFill(
                    SynaraColor.surface.opacity(0.55),
                    opaqueFill: SynaraColor.surface
                )
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
                .synaraDepth(.raised, cornerRadius: SynaraRadius.control)
            }

            if confirmApproveAlways {
                VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                    Text("Approve always permanently trusts this command pattern. Confirm only if you intend to allow it without future prompts.")
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.critical)
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
                    }
                }
                .padding(SynaraSpacing.small)
                .synaraAccessibleSurfaceFill(
                    SynaraColor.critical.opacity(0.08),
                    opaqueFill: SynaraColor.secondarySurface
                )
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
                .synaraDepth(
                    .floating,
                    cornerRadius: SynaraRadius.control,
                    boundaryColor: SynaraColor.critical
                )
            }

            LazyVGrid(columns: actionColumns, spacing: SynaraSpacing.small) {
                ForEach(SynaraAgentApprovalPromptReaction.allCases) { action in
                    if confirmApproveAlways, action == .approveAlways {
                        EmptyView()
                    } else {
                        Button {
                            if action == .approveAlways {
                                confirmApproveAlways = true
                            } else {
                                confirmApproveAlways = false
                                onReaction(action.actionIdentifier)
                            }
                        } label: {
                            VStack(spacing: SynaraSpacing.xSmall) {
                                Text(action.reactionKey)
                                    .font(.system(size: 21, weight: .semibold))
                                Text(action.title)
                                    .font(SynaraTypography.chipLabel.weight(.semibold))
                                    .multilineTextAlignment(.center)
                                    .lineLimit(2)
                            }
                            .frame(maxWidth: .infinity)
                            .frame(minHeight: 58)
                            .foregroundStyle(action.tint)
                            .synaraAccessibleSurfaceFill(
                                action.tint.opacity(action == .deny ? 0.08 : 0.10),
                                opaqueFill: SynaraColor.secondarySurface
                            )
                            .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
                            .synaraDepth(
                                .raised,
                                cornerRadius: SynaraRadius.control,
                                boundaryColor: action.tint
                            )
                        }
                        .buttonStyle(SynaraTactileButtonStyle())
                        .accessibilityLabel("\(action.title) \(action.reactionKey)")
                        .accessibilityIdentifier("AgentApprovalPromptReaction-\(action.accessibilityIdentifierSuffix)-\(eventID)")
                    }
                }
            }

            Text("Session approval is available only by replying !approve session; Hermes does not define a session-approval reaction.")
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

private struct AgentApprovalButtonStyle: ViewModifier {
    let action: SynaraAgentCardAction

    func body(content: Content) -> some View {
        switch action.kind {
        case .some("approve"):
            content
                .buttonStyle(.plain)
                .padding(.vertical, SynaraSpacing.small)
                .padding(.horizontal, SynaraSpacing.medium)
                .foregroundStyle(SynaraColor.success)
                .background(SynaraColor.success.opacity(0.08))
                .overlay(
                    RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)
                        .stroke(SynaraColor.success, lineWidth: 1.5)
                )
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
        case .some("reject"):
            content
                .buttonStyle(.plain)
                .padding(.vertical, SynaraSpacing.small)
                .padding(.horizontal, SynaraSpacing.medium)
                .foregroundStyle(SynaraColor.critical)
                .background(SynaraColor.critical.opacity(0.06))
                .overlay(
                    RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)
                        .stroke(SynaraColor.critical, lineWidth: 1.5)
                )
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
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
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            Text("Preview")
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
            HStack {
                Text(urlString)
                    .font(SynaraTypography.messageMeta)
                    .foregroundStyle(SynaraColor.accent)
                    .lineLimit(1)
                Spacer()
                Image(systemName: "arrow.up.right.square")
                    .foregroundStyle(SynaraColor.secondaryText)
            }
            if isPolicySafeLink {
                HStack(spacing: SynaraSpacing.xSmall) {
                    Image(systemName: "link")
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                    Text("Opens HTTPS link")
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                }
            }
        }
        .padding(SynaraSpacing.small)
        .synaraCard(
            fill: SynaraColor.surface.opacity(0.7),
            opaqueFill: SynaraColor.surface,
            stroke: SynaraColor.agent
        )
    }
}

private struct AgentCardTimelineRow: View {
    let card: SynaraAgentCard
    let onAction: (SynaraAgentCardAction) -> Void

    var body: some View {
        let visibleActions = card.actions.filter { SynaraAgentCardActionResolver.shouldRender($0) }

        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            HStack(alignment: .center, spacing: SynaraSpacing.small) {
                SynaraAvatar(title: "Agent", systemImage: "sparkles", tint: SynaraColor.agent, size: 28)

                VStack(alignment: .leading, spacing: 2) {
                    Text(card.title)
                        .font(SynaraTypography.emphasis)
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(2)
                        .accessibilityIdentifier("AgentCardTitle")
                    Text("Agent workflow")
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                }

                Spacer()

                if let status = card.status {
                    SynaraStatusChip(title: status, tint: SynaraColor.agent, systemImage: "circle.dashed")
                        .lineLimit(1)
                }
            }

            if let summary = card.summary {
                Text(summary)
                    .font(SynaraTypography.body)
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(2)
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
                                .font(SynaraTypography.emphasis)
                            Spacer()
                            Image(systemName: "chevron.right")
                                .accessibilityHidden(true)
                        }
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .tint(action.tint)
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
                                    .font(SynaraTypography.emphasis)
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

private struct AgentApprovalDetails: View {
    let card: SynaraAgentCard

    var body: some View {
        VStack(spacing: SynaraSpacing.small) {
            AgentDetailRow(title: "Target", value: card.artifacts.first?.title ?? "Synara workflow")
            AgentDetailRow(title: "Changes", value: changeSummary)
            AgentDetailRow(title: "Checks", value: checkSummary, valueTint: SynaraColor.success)
            AgentDetailRow(title: "Summary", value: card.summary ?? card.title)
        }
        .padding(SynaraSpacing.small)
        .synaraCard(
            fill: SynaraColor.surface.opacity(0.65),
            opaqueFill: SynaraColor.surface,
            stroke: SynaraColor.agent
        )
    }

    private var changeSummary: String {
        let count = card.artifacts.count + card.code.count + card.diffs.count
        return count == 1 ? "1 item changed" : "\(max(count, 1)) items changed"
    }

    private var checkSummary: String {
        if card.logs.isEmpty {
            return "Ready"
        }
        return "\(card.logs.count) passed"
    }
}

private struct AgentDetailRow: View {
    let title: String
    let value: String
    var valueTint: Color = SynaraColor.primaryText

    var body: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.medium) {
            Text(title)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.secondaryText)
                .frame(width: 76, alignment: .leading)
            Text(value)
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(valueTint)
                .lineLimit(3)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

private extension SynaraAgentCardAction {
    var systemImage: String {
        switch kind {
        case .some("approve"):
            return "checkmark.circle.fill"
        case .some("reject"):
            return "xmark.circle"
        case .some("open"), .some("open_url"):
            return "safari"
        default:
            return "doc.on.doc"
        }
    }

    var tint: Color {
        switch kind {
        case .some("reject"):
            return SynaraColor.critical
        case .some("approve"):
            return SynaraColor.success
        default:
            return SynaraColor.agent
        }
    }
}

private extension SynaraAgentApprovalPromptReaction {
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
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @State private var lastOutgoingTyping = false
    @State private var isAttachmentSheetPresented = false
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

    @FocusState private var isComposerFocused: Bool

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
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("ComposerErrorText")
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
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("MediaUploadErrorText")
            }
        }
        .padding(.horizontal, SynaraSpacing.small)
        .padding(.top, SynaraSpacing.xSmall)
        .padding(.bottom, SynaraSpacing.xSmall)
        .background(SynaraChrome.composer)
        .animation(.easeInOut(duration: 0.18), value: isFormattingBarVisible)
        .animation(.easeInOut(duration: 0.18), value: shouldShowPromptMetrics)
        .onChange(of: isComposerFocused) { focused in
            isFocusedExternally = focused
            updateOutgoingTyping()
        }
        .onChange(of: currentText) { _ in
            updateOutgoingTyping()
        }
        .onChange(of: text) { value in
            reconcileExternalText(value)
        }
        .onAppear {
            if liveText == nil {
                liveText = text
            }
            isFocusedExternally = isComposerFocused
            updateOutgoingTyping()
        }
        .onDisappear {
            flushExternalText()
            isFocusedExternally = false
            setOutgoingTyping(false)
        }
        .sheet(isPresented: $isAttachmentSheetPresented) {
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
                            onUploadFailed("Camera is not available on this device.")
                        }
                    #endif
                },
                selectedPhotos: $selectedPhotos,
                maxSelectionCount: remainingAttachmentSlots
            )
            .presentationDetents([.height(260)])
            .presentationDragIndicator(.visible)
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
                onUploadFailed("Attachment could not be loaded. Try again.")
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
                .synaraDepth(.raised, shape: Circle())
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
                .synaraDepth(isFormattingBarVisible ? .raised : .content, shape: Circle())
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
                    .synaraDepth(.raised, shape: Circle(), boundaryColor: SynaraColor.accent)
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
        .synaraDepth(.raised, cornerRadius: SynaraRadius.composer)
    }

    private var canSubmit: Bool {
        if editTarget != nil {
            return currentText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
        }
        return ComposerAttachmentDraftList.canSend(text: currentText, drafts: attachmentDrafts)
    }

    private var resolvedPlaceholder: String {
        if let editTarget {
            return editTarget.isLocalPending ? "Edit unsent message..." : "Edit message..."
        }
        return placeholder
    }

    private var composerSendAccessibilityHint: String {
        if editTarget != nil {
            return isSending ? "Saving the edited message" : "Saves the edited message"
        }
        return isSending
            ? "Sending the current message and attachments"
            : "Sends the current message and attachments"
    }

    private var remainingAttachmentSlots: Int {
        max(0, ComposerAttachmentDraftList.maxCount - attachmentDrafts.count)
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

    private var composerPromptMetrics: some View {
        HStack(spacing: SynaraSpacing.small) {
            Spacer()
            Text("\(currentText.count) chars · \(composerLineCount) line\(composerLineCount == 1 ? "" : "s")")
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
        let result = ComposerMarkdown.apply(format, to: currentText, selection: composerSelection)
        setLiveText(result.text)
        composerSelection = result.selection
        formattingRevision += 1
        isComposerFocused = true
    }

    private func submitMessage() {
        guard isSending == false else {
            return
        }
        #if canImport(UIKit)
            ComposerTextInputRegistry.dismissKeyboard()
        #endif
        isComposerFocused = false
        setOutgoingTyping(false)
        let messageBody = currentText
        text = messageBody
        onSend(messageBody)
    }

    private func updateOutgoingTyping() {
        let shouldType =
            isComposerFocused
            && currentText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            && SynaraSharedConstants.boolSetting(SynaraSharedConstants.hideActivityKey) == false
        setOutgoingTyping(shouldType)
    }

    private func setOutgoingTyping(_ typing: Bool) {
        guard lastOutgoingTyping != typing else {
            return
        }
        lastOutgoingTyping = typing
        Task {
            await environment.matrix.setOutgoingTyping(roomID: roomID, typing: typing)
        }
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
            HStack(spacing: SynaraSpacing.xSmall) {
                ForEach(ComposerMarkdownFormat.allCases) { format in
                    Button {
                        onFormat(format)
                    } label: {
                        Image(systemName: format.systemImage)
                            .font(.system(size: 15, weight: .semibold))
                            .frame(width: 36, height: 36)
                            .background(SynaraColor.surface)
                            .foregroundStyle(SynaraColor.primaryText)
                            .clipShape(
                                RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)
                            )
                            .synaraDepth(
                                .raised,
                                shape: RoundedRectangle(
                                    cornerRadius: SynaraRadius.control,
                                    style: .continuous
                                )
                            )
                            .frame(width: 44, height: 44)
                    }
                    .buttonStyle(SynaraTactileButtonStyle())
                    .contentShape(Rectangle())
                    .accessibilityLabel(format.accessibilityLabel)
                    .accessibilityIdentifier("ComposerFormat-\(format.rawValue)")
                }
            }
            .padding(.horizontal, SynaraSpacing.small)
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

    private let options: [AttachmentOption] = [
        AttachmentOption(title: "Photo or Video", systemImage: "photo", tint: SynaraColor.success, kind: .photo),
        AttachmentOption(title: "File", systemImage: "doc", tint: SynaraColor.accent, kind: .file),
        AttachmentOption(title: "Camera", systemImage: "camera", tint: SynaraColor.warning, kind: .camera),
    ]

    private var isUITestEnvironment: Bool {
        ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1"
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
                .accessibilityIdentifier("AttachmentOption-\(option.title)")
            } else if maxSelectionCount > 0 {
                PhotosPicker(
                    selection: $selectedPhotos,
                    maxSelectionCount: maxSelectionCount,
                    matching: .any(of: [.images, .videos])
                ) {
                    AttachmentOptionLabel(option: option)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("AttachmentOption-\(option.title)")
            } else {
                AttachmentOptionLabel(option: option)
                    .opacity(0.45)
                    .accessibilityIdentifier("AttachmentOption-\(option.title)")
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
            .accessibilityIdentifier("AttachmentOption-\(option.title)")
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
            .accessibilityIdentifier("AttachmentOption-\(option.title)")
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
    }

    let title: String
    let systemImage: String
    let tint: Color
    let kind: Kind

    var id: String { title }
}

private struct AttachmentOptionLabel: View {
    let option: AttachmentOption

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Image(systemName: option.systemImage)
                .font(.system(size: 18, weight: .semibold))
                .frame(width: 30, height: 30)
                .background(option.tint.opacity(0.14))
                .foregroundStyle(option.tint)
                .clipShape(Circle())

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
                .accessibilityLabel("Cancel \(target.kind == .edit ? "editing" : "reply")")
        }
        .padding(SynaraSpacing.small)
        .synaraCard(fill: SynaraColor.accent.opacity(0.08), stroke: SynaraColor.accent)
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
                    mediaErrorView(message: "Media could not be loaded.")
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
            errorMessage = "Encrypted media requires recovered keys before it can be opened."
            return
        }

        guard resource.authenticatedURL != nil else {
            errorMessage = "Media is unavailable."
            return
        }

        isLoading = true
        defer { isLoading = false }

        guard let data = await environment.mediaLoader.loadMediaData(for: resource),
              let loadedImage = UIImage(data: data)
        else {
            errorMessage = "Media could not be loaded."
            return
        }

        image = loadedImage
    }
}

private struct ZoomableMediaImage: View {
    let image: UIImage
    @Binding var scale: CGFloat
    @Binding var lastScale: CGFloat

    var body: some View {
        GeometryReader { geometry in
            ScrollView([.horizontal, .vertical], showsIndicators: false) {
                Image(uiImage: image)
                    .resizable()
                    .scaledToFit()
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

private extension Date {
    var timelineTime: String {
        let formatter = DateFormatter()
        formatter.dateStyle = .none
        if SynaraSharedConstants.boolSetting(SynaraSharedConstants.hour24ClockKey) {
            formatter.dateFormat = "HH:mm"
        } else {
            formatter.timeStyle = .short
        }
        return formatter.string(from: self)
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
            return "Deleted message"
        case .encryptedPlaceholder:
            return "Encrypted message"
        case let .unknown(type):
            return type
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
