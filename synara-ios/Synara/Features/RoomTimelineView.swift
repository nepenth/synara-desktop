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

enum RoomTimelineReadAcknowledgementPolicy {
    static func shouldSchedule(
        isLive: Bool,
        isConfirmedPinned: Bool,
        isJumpingToLatest: Bool,
        isUserInteracting: Bool,
        eventID: String,
        lastMarkedEventID: String?
    ) -> Bool {
        isLive
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

    static func flushCandidate(
        pendingEventID: String?,
        lastCandidateEventID: String?,
        lastMarkedEventID: String?
    ) -> String? {
        let candidate = pendingEventID ?? lastCandidateEventID
        guard let candidate,
              candidate != lastMarkedEventID,
              MatrixServerEventIDPolicy.canAcknowledge(candidate)
        else {
            return nil
        }
        return candidate
    }
}

enum RoomTimelineReadMarkerTaskPolicy {
    static func ownsInstalledTask(installedGeneration: UInt64, currentGeneration: UInt64) -> Bool {
        installedGeneration == currentGeneration
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
    @State private var state: TimelineViewState = .idle
    @State private var draft: String = ""
    @State private var replyTarget: ComposerRelationTarget?
    @State private var editTarget: ComposerRelationTarget?
    @State private var sendError: String?
    @State private var hasAnchoredEvent = false
    @State private var uploadState: MediaUploadState = .idle
    @State private var viewerResource: MediaResource?
    @State private var selectedPhotos: [PhotosPickerItem] = []
    @State private var attachmentDrafts: [ComposerAttachmentDraft] = []
    @State private var isSendingMessage = false
    @State private var agentActionMessage: String?
    @State private var cryptoStatus: RoomCryptoStatus = .unknown
    @State private var cryptoActionMessage: String?
    @State private var isCryptoBannerDismissed = false
    @State private var isRoomDetailsPresented = false
    @State private var isStickerPackPresented = false
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
    @State private var lastAcknowledgementCandidateEventID: String?
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
    private let timelineLogger = AppLogger()
    @Environment(\.openURL) private var openURL
    @Environment(\.dismiss) private var dismiss
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass

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
            if let typingText = RoomTypingPresentation.text(for: typingUserIDs) {
                RoomTypingIndicator(text: typingText)
            }
            Divider()
            ComposerView(
                text: $draft,
                placeholder: isAgentRoom ? "Reply to the agent workflow..." : "Send a message...",
                showsPromptMetrics: isAgentRoom,
                replyTarget: replyTarget,
                editTarget: editTarget,
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
                onOpenStickers: { isStickerPackPresented = true },
                selectedPhotos: $selectedPhotos,
                attachmentDrafts: $attachmentDrafts,
                isSending: isSendingMessage,
                onPasteImages: draftPastedImages,
                isFocusedExternally: $isComposerFocused
            )
            .background(SynaraChrome.composer)
            .shadow(color: Color.black.opacity(isAgentRoom ? 0.22 : 0.06), radius: 10, x: 0, y: -3)
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
                }
            )
        }
        .sheet(isPresented: $isStickerPackPresented) {
            StickerPackSheet(roomID: roomID, onSend: sendSticker)
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
        .task(id: timelineTaskID) {
            resetTimelineState()
            let roomOpenSignpostID = PerformanceTrace.begin("RoomOpen")
            defer {
                PerformanceTrace.end("RoomOpen", id: roomOpenSignpostID)
            }
            Task {
                _ = await loadCryptoStatus()
            }
            startTypingUpdates()
            startVerificationAutoRetry()
            await loadTimeline()
            applyOutgoingQueueToTimeline()
            flushOutgoingSendsIfReady(environment.connectionStatus.status)
            _ = await loadCryptoStatus()
        }
        .onDisappear {
            dismissKeyboard()
            stopTimelineUpdates(reason: "view-disappeared")
            stopTypingUpdates()
            cancelTimelineScroll()
            flushMarkFullyRead()
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
                                    animateSend: sendAnimationItemIDs.contains(item.id),
                                    replyPreview: item.replyToEventID.flatMap { replyPreviewsByEventID[$0] },
                                    replyCount: threadReplyCounts[item.eventID] ?? 0,
                                    availability: environment.eventActions.availability(for: item, currentUserID: currentUserID),
                                    onReply: {
                                        replyTarget = ComposerRelationTarget(
                                            item: item,
                                            kind: .reply,
                                            currentUserID: currentUserID
                                        )
                                    },
                                    onOpenThread: { openThread(root: item) },
                                    onEdit: { beginEdit(item) },
                                    onRedact: { applyAction(.redact, to: item) },
                                    onReact: { applyAction(.react("👍"), to: item) },
                                    onOpenMedia: { resource in viewerResource = resource },
                                    onAgentAction: { action in
                                        executeAgentAction(action, sourceEventID: item.eventID)
                                    },
                                    onAgentApprovalReaction: { reactionKey in
                                        submitAgentApprovalReaction(reactionKey: reactionKey, sourceEventID: item.eventID)
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
                animateSend: eventRow.animateSend,
                replyPreview: eventRow.replyPreview,
                replyCount: eventRow.replyCount,
                availability: eventRow.availability,
                onReply: {
                    replyTarget = ComposerRelationTarget(
                        item: eventRow.item,
                        kind: .reply,
                        currentUserID: currentUserID
                    )
                },
                onOpenThread: { openThread(root: eventRow.item) },
                onEdit: { beginEdit(eventRow.item) },
                onRedact: { applyAction(.redact, to: eventRow.item) },
                onReact: { applyAction(.react("👍"), to: eventRow.item) },
                onOpenMedia: { resource in viewerResource = resource },
                onAgentAction: { action in
                    executeAgentAction(action, sourceEventID: eventRow.item.eventID)
                },
                onAgentApprovalReaction: { reactionKey in
                    submitAgentApprovalReaction(reactionKey: reactionKey, sourceEventID: eventRow.item.eventID)
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
                            animateSend: sendAnimationItemIDs.contains(item.id),
                            replyPreview: item.replyToEventID.flatMap { previews[$0] },
                            replyCount: replyCounts[item.eventID] ?? 0,
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

    private func isGroupedWithPrevious(index: Int, items: [TimelineItem]) -> Bool {
        guard index > 0 else {
            return false
        }

        let previous = items[index - 1]
        let current = items[index]
        return previous.senderID == current.senderID
            && current.timestamp.timeIntervalSince(previous.timestamp) < 5 * 60
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
        state = .idle
        draft = environment.drafts.draft(roomID: roomID)
        replyTarget = nil
        editTarget = nil
        sendError = nil
        hasAnchoredEvent = false
        uploadState = .idle
        viewerResource = nil
        selectedPhotos = []
        attachmentDrafts = []
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
        lastAcknowledgementCandidateEventID = nil
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
        let shouldRemainPaginating = isPaginating || currentTimelineIsPaginating
        switch outcome {
        case let .loaded(items):
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
        case let .failed(message):
            if case let .loaded(currentItems, _) = state, currentItems.isEmpty == false {
                state = .loaded(currentItems, isPaginating: shouldRemainPaginating)
                sendError = message
                logTimelineEvent("snapshot-failed-preserved", fields: ["rendered": "\(currentItems.count)"])
            } else {
                state = .failed(message)
                logTimelineEvent("snapshot-failed", fields: [:])
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
                        if case .loading = state {
                            state = .empty
                            logTimelineEvent("stream-empty-initial")
                        } else {
                            logTimelineEvent("stream-empty-ignored")
                        }
                    case let .failed(message):
                        if case .loaded = state {
                            logTimelineEvent("stream-failed-preserved")
                            return
                        }
                        state = .failed(message)
                        logTimelineEvent("stream-failed")
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

        sendError = nil
        isSendingMessage = true
        if drafts.isEmpty {
            performSend(body: rawBody, replyToEventID: replyTarget?.eventID, editEventID: editTarget?.eventID)
            isSendingMessage = false
            return
        }

        Task {
            let signpostID = PerformanceTrace.begin("ComposerAttachmentDraftSend")
            defer {
                PerformanceTrace.end("ComposerAttachmentDraftSend", id: signpostID)
            }
            let uploaded = await ComposerAttachmentSend.uploadAll(
                drafts,
                roomID: roomID,
                uploader: environment.mediaUploader,
                onState: { state in
                    uploadState = state
                },
                onUploaded: { draft, item in
                    attachmentDrafts = ComposerAttachmentDraftList.remove(id: draft.id, from: attachmentDrafts)
                    append(item)
                }
            )
            await MainActor.run {
                if uploaded {
                    if trimmed.isEmpty == false {
                        performSend(
                            body: rawBody,
                            replyToEventID: replyTarget?.eventID,
                            editEventID: editTarget?.eventID
                        )
                    } else {
                        uploadState = .idle
                    }
                }
                isSendingMessage = false
            }
        }
    }

    private func sendSticker(_ sticker: SharedCoreSticker) {
        let request = StickerSendRequest(
            roomID: roomID,
            body: sticker.body,
            mxc: sticker.mxc,
            width: sticker.width,
            height: sticker.height,
            mimetype: sticker.mimetype,
            size: sticker.size,
            replyToEventID: replyTarget?.eventID,
            threadRoot: nil
        )
        Task {
            do {
                _ = try await environment.messageSender.sendSticker(request)
                await MainActor.run {
                    sendError = nil
                    clearComposerRelation()
                }
            } catch {
                await MainActor.run {
                    sendError = MessageSendError.failed.localizedDescription
                }
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

    private func performSend(
        body rawBody: String,
        replyToEventID: String?,
        editEventID: String?
    ) {
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
            editEventID: editEventID
        )
        let isEditing = request.editEventID != nil

        if isEditing {
            Task {
                do {
                    let signpostID = PerformanceTrace.begin("MessageSend")
                    defer {
                        PerformanceTrace.end("MessageSend", id: signpostID)
                    }
                    let item = try await environment.messageSender.send(request)
                    await MainActor.run {
                        replace(item)
                        draft = ""
                        environment.drafts.clearDraft(roomID: roomID)
                        clearComposerRelation()
                        sendError = nil
                    }
                } catch {
                    await MainActor.run {
                        sendError = MessageSendError.failed.localizedDescription
                        SynaraHaptics.trigger(.warning)
                    }
                }
            }
            return
        }

        let queued = environment.outgoingSends.enqueue(
            localID: "$pending-\(UUID().uuidString)",
            roomID: roomID,
            body: body,
            formattedBody: request.formattedBody,
            replyToEventID: replyToEventID,
            senderID: currentUserID,
            timestamp: Date()
        )
        registerSendAnimation(for: queued.id, isRetry: false)
        applyOutgoingQueueToTimeline()

        draft = ""
        environment.drafts.clearDraft(roomID: roomID)
        clearComposerRelation()
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
                    hasReachedOldestMessages = true
                    paginationScrollAnchorID = nil
                    state = .loaded(currentItems, isPaginating: false)
                    logTimelineEvent("pagination-reached-start")
                case let .failed(message):
                    paginationScrollAnchorID = nil
                    state = .loaded(currentItems, isPaginating: false)
                    sendError = message
                    logTimelineEvent("pagination-failed")
                }
            }
        }
        return true
    }

    private func scheduleMarkFullyRead(eventID: String) {
        guard RoomTimelineReadAcknowledgementPolicy.shouldSchedule(
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
        lastAcknowledgementCandidateEventID = eventID
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

            pendingMarkFullyReadEventID = nil
            firstPendingMarkFullyReadAt = nil
            // Resolve the SDK-authoritative live tail when this task executes.
            // A delayed candidate must never move the shared marker backwards.
            let acknowledgedEventID = await environment.readMarkers.markRoomAsRead(roomID: roomID)
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
                if let acknowledgedEventID {
                    lastMarkedFullyReadEventID = acknowledgedEventID
                    initialReadMarkerEventID = acknowledgedEventID
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

    private func flushMarkFullyRead() {
        let eventID = RoomTimelineReadMarkerQueuePolicy.flushCandidate(
            pendingEventID: pendingMarkFullyReadEventID,
            lastCandidateEventID: lastAcknowledgementCandidateEventID,
            lastMarkedEventID: lastMarkedFullyReadEventID
        )
        cancelMarkFullyRead()
        guard eventID != nil else {
            return
        }
        let readMarkers = environment.readMarkers
        let disappearingRoomID = roomID
        Task {
            _ = await readMarkers.markRoomAsRead(roomID: disappearingRoomID)
        }
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
                    sendError = "Could not load the latest messages. Try again."
                    logTimelineEvent("jump-latest-empty-preserved", fields: ["rendered": "\(currentItems.count)"])
                case let .failed(message):
                    isJumpingToLatest = false
                    showJumpToLatest = true
                    sendError = message
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
                    sendError = "Could not load the latest messages. Try again."
                    logTimelineEvent("jump-latest-empty-preserved", fields: ["rendered": "\(currentItems.count)"])
                case let .failed(message):
                    isJumpingToLatest = false
                    showJumpToLatest = true
                    sendError = message
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
                sendError = "That message is not available in this timeline. Showing the latest messages instead."
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

    private func beginEdit(_ item: TimelineItem) {
        editTarget = ComposerRelationTarget(
            item: item,
            kind: .edit,
            currentUserID: currentUserID
        )
        if case let .text(body) = item.kind {
            draft = body
            environment.drafts.setDraft(body, roomID: roomID)
        } else if case let .formattedText(body, _) = item.kind {
            draft = body
            environment.drafts.setDraft(body, roomID: roomID)
        }
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
        replyTarget = nil
        editTarget = nil
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

    private func submitAgentApprovalReaction(reactionKey: String, sourceEventID: String) {
        Task {
            do {
                try await environment.agentApprovalReactions.submitReaction(
                    SynaraAgentApprovalReactionRequest(
                        roomID: roomID,
                        sourceEventID: sourceEventID,
                        reactionKey: reactionKey
                    )
                )
                await MainActor.run {
                    if reactionKey == SynaraAgentApprovalPromptReaction.deny.reactionKey {
                        SynaraHaptics.trigger(.lightImpact)
                    } else {
                        SynaraHaptics.trigger(.success)
                    }
                    agentActionMessage = "Approval reaction sent."
                }
            } catch let error as SynaraAgentApprovalError {
                await MainActor.run {
                    agentActionMessage = error.errorDescription ?? "Approval reaction could not be submitted."
                }
            } catch {
                await MainActor.run {
                    agentActionMessage = "Approval reaction could not be submitted."
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
                        .foregroundStyle(SynaraColor.primaryText)
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
    @State private var isSendingMessage = false
    @State private var isComposerFocused = false
    @State private var threadUpdatesTask: Task<Void, Never>?

    var body: some View {
        VStack(spacing: 0) {
            ThreadHeader(
                subtitle: rootTitle ?? roomTitle ?? "Room message",
                onBack: { dismiss() }
            )

            threadContent

            Divider()

            ComposerView(
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
                        ThreadMessageRow(item: item)
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
                    case let .failed(message):
                        if case .loaded = state {
                            return
                        }
                        state = .failed(message)
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
        case let .failed(message):
            state = .failed(message)
        }
    }

    private func threadItems(from items: [TimelineItem]) -> [TimelineItem] {
        items
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
        Task {
            var uploaded = true
            if drafts.isEmpty == false {
                let uploadSignpostID = PerformanceTrace.begin("ThreadComposerAttachmentDraftSend")
                uploaded = await ComposerAttachmentSend.uploadAll(
                    drafts,
                    roomID: roomID,
                    uploader: environment.mediaUploader,
                    onState: { state in
                        uploadState = state
                    },
                    onUploaded: { draft, item in
                        attachmentDrafts = ComposerAttachmentDraftList.remove(id: draft.id, from: attachmentDrafts)
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

            guard body.isEmpty == false else {
                await MainActor.run {
                    uploadState = .idle
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
                        body: body,
                        formattedBody: ComposerMatrixFormatting.formattedBody(for: body),
                        replyToEventID: rootEventID,
                        editEventID: nil
                    )
                )
                await MainActor.run {
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
                                ReactionPill(title: reaction, count: item.reactions[reaction] ?? 0, animationIndex: index)
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
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("ThreadItem-\(item.eventID)")
    }

    @ViewBuilder
    private var threadBody: some View {
        switch item.kind {
        case let .text(body):
            Text(body)
                .font(SynaraTypography.messageBody)
                .foregroundStyle(SynaraColor.primaryText)
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

private struct MatrixFormattedMessageView: View {
    let fallbackBody: String
    let font: Font
    private let segments: [MatrixHTMLRenderer.Segment]

    init(fallbackBody: String, html: String, font: Font) {
        self.fallbackBody = fallbackBody
        self.font = font
        segments = MatrixHTMLRenderer.segments(body: fallbackBody, html: html)
    }

    var body: some View {
        if segments.count == 1, case let .markdown(markdown) = segments[0] {
            markdownText(markdown)
        } else {
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                ForEach(Array(segments.enumerated()), id: \.offset) { _, segment in
                    segmentView(segment)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    @ViewBuilder
    private func segmentView(_ segment: MatrixHTMLRenderer.Segment) -> some View {
        switch segment {
        case let .markdown(markdown):
            markdownText(markdown)
        case let .code(code):
            MatrixCodeBlockView(code: code)
        case let .quote(markdown):
            MatrixQuoteBlockView(markdown: markdown, font: font)
        case let .details(block):
            MatrixDetailsBlockView(block: block)
        }
    }

    private func markdownText(_ markdown: String) -> some View {
        let displayMarkdown = MatrixDisplayMarkdown.normalize(markdown.isEmpty ? fallbackBody : markdown)
        return Text(attributedMarkdown(displayMarkdown))
            .font(font)
            .foregroundStyle(SynaraColor.primaryText)
            .lineLimit(nil)
            .frame(maxWidth: .infinity, alignment: .leading)
            .fixedSize(horizontal: false, vertical: true)
    }

    private func attributedMarkdown(_ markdown: String) -> AttributedString {
        (try? AttributedString(
            markdown: markdown,
            options: AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        )) ?? AttributedString(markdown)
    }
}

private struct MatrixQuoteBlockView: View {
    let markdown: String
    let font: Font

    var body: some View {
        Text(attributedMarkdown(MatrixDisplayMarkdown.normalize(markdown)))
            .font(font)
            .foregroundStyle(SynaraColor.secondaryText)
            .lineLimit(nil)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.leading, SynaraSpacing.medium)
            .overlay(alignment: .leading) {
                RoundedRectangle(cornerRadius: 2, style: .continuous)
                    .fill(SynaraColor.secondaryText.opacity(0.75))
                    .frame(width: 3)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func attributedMarkdown(_ markdown: String) -> AttributedString {
        (try? AttributedString(
            markdown: markdown,
            options: AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        )) ?? AttributedString(markdown)
    }
}

private struct MatrixDetailsBlockView: View {
    let block: MatrixHTMLRenderer.DetailsBlock
    @State private var isExpanded = true

    var body: some View {
        DisclosureGroup(isExpanded: $isExpanded) {
            VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                if let code = block.code {
                    MatrixCodeBlockView(code: code)
                }

                if block.body.isEmpty == false {
                    Text(block.body)
                        .font(SynaraTypography.messageBody)
                        .foregroundStyle(SynaraColor.primaryText)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .padding(.top, SynaraSpacing.xSmall)
        } label: {
            Text(block.summary)
                .font(SynaraTypography.messageBody.weight(.semibold))
                .foregroundStyle(SynaraColor.primaryText)
                .lineLimit(nil)
        }
        .tint(SynaraColor.secondaryText)
    }
}

private struct MatrixCodeBlockView: View {
    let code: String

    private var lineNumbers: String {
        let count = MatrixHTMLRenderer.codeLineCount(code)
        return (1 ... count).map(String.init).joined(separator: "\n")
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Code")
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
            .background(SynaraColor.elevatedSurface)

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
            .background(SynaraColor.secondarySurface)
        }
        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous)
                .stroke(SynaraColor.separator.opacity(0.8), lineWidth: 1)
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
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @State private var details: RoomDetails?
    @State private var profileName = ""
    @State private var profileTopic = ""
    @State private var canonicalAlias = ""
    @State private var alternativeAliases = ""
    @State private var selectedAvatarPhoto: PhotosPickerItem?
    @State private var inviteUserID = ""
    @State private var notificationMode: SynaraRoomNotificationMode = .allMessages
    @State private var isApplyingLoadedNotificationMode = false
    @State private var message: String?
    @State private var isLoading = false
    @State private var isLeaveConfirmationPresented = false

    var body: some View {
        NavigationStack {
            Form {
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
            notificationMode = loadedDetails?.notificationMode ?? .allMessages
            isApplyingLoadedNotificationMode = false
            profileName = loadedDetails?.name ?? fallbackTitle
            profileTopic = loadedDetails?.topic ?? ""
            let aliases = loadedDetails?.aliases ?? []
            canonicalAlias = aliases.first ?? ""
            alternativeAliases = aliases.dropFirst().joined(separator: ", ")
        }
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

private struct StickerPackSheet: View {
    let roomID: String
    let onSend: (SharedCoreSticker) -> Void
    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @State private var stickers: [SharedCoreSticker] = []

    var body: some View {
        NavigationStack {
            List {
                if stickers.isEmpty {
                    Text("No sticker packs are available.")
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("StickerPackEmpty")
                } else {
                    ForEach(stickers) { sticker in
                        Button {
                            onSend(sticker)
                            dismiss()
                        } label: {
                            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                                Text(sticker.body)
                                    .font(SynaraTypography.body)
                                Text(sticker.packName)
                                    .font(SynaraTypography.supporting)
                                    .foregroundStyle(SynaraColor.secondaryText)
                            }
                        }
                        .accessibilityIdentifier("StickerPackItem")
                    }
                }
            }
            .navigationTitle("Stickers")
            .accessibilityIdentifier("StickerPackSheet")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") { dismiss() }
                }
            }
            .task(id: roomID) {
                stickers = await environment.roomManagement.stickers(roomID: roomID)
            }
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
        .synaraCard(fill: SynaraColor.warning.opacity(0.10), stroke: SynaraColor.warning.opacity(0.30))
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
    let animateSend: Bool
    let replyPreview: TimelineReplyPreview?
    let replyCount: Int
    let availability: EventActionAvailability
    let onReply: () -> Void
    let onOpenThread: () -> Void
    let onEdit: () -> Void
    let onRedact: () -> Void
    let onReact: () -> Void
    let onOpenMedia: (MediaResource) -> Void
    let onAgentAction: (SynaraAgentCardAction) -> Void
    let onAgentApprovalReaction: (String) -> Void
    let onRetryFailedSend: () -> Void
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass

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
                    HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                        Text(senderDisplayName)
                            .font(SynaraTypography.emphasis)
                            .foregroundStyle(SynaraColor.primaryText)
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
                }

                messageContent
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.top, isGroupedWithPrevious ? 0 : 7)
        .contextMenu {
            if availability.canReply {
                Button("Reply", action: onReply)
            }
            if replyCount > 0 {
                Button("Open Thread", action: onOpenThread)
            }
            if availability.canEdit {
                Button("Edit", action: onEdit)
            }
            if availability.canReact {
                Button("React", action: onReact)
            }
            if availability.canRedact {
                Button("Redact", role: .destructive, action: onRedact)
            }
        }
        .accessibilityElement(children: accessibilityChildBehavior)
        .accessibilityLabel(accessibilitySummary)
        .accessibilityHint(accessibilityHint)
        .accessibilityIdentifier("TimelineItem-\(item.eventID)")

        withFailedRetryAccessibilityAction(row)
            .synaraSendSlideIn(isEnabled: animateSend, fromTrailing: isOutgoing)
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
    private var messageContent: some View {
        let content = VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            if let replyToEventID = item.replyToEventID {
                replyQuoteLabel(for: replyToEventID)
            }

            bubbleWrappedBodyContent

            if item.reactions.isEmpty == false {
                HStack(spacing: SynaraSpacing.xSmall) {
                    ForEach(Array(item.reactions.keys.sorted().enumerated()), id: \.element) { index, reaction in
                        ReactionPill(title: reaction, count: item.reactions[reaction] ?? 0, animationIndex: index)
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
        if let approvalPrompt {
            SynaraMessageBubble(
                alignment: bubbleAlignment,
                variant: .agent,
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
                    isGrouped: isGroupedWithPrevious,
                    showsBackground: false,
                    deliveryStatus: item.deliveryStatus,
                    statusEventID: item.eventID,
                    onRetryFailedSend: item.deliveryStatus == .failed ? onRetryFailedSend : nil
                ) {
                    MatrixFormattedMessageView(
                        fallbackBody: body,
                        html: html,
                        font: SynaraTypography.messageBody
                    )
                }
            case .encryptedPlaceholder:
                SynaraMessageBubble(
                    alignment: bubbleAlignment,
                    variant: .encrypted,
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
        switch item.kind {
        case let .text(body):
            return "\(item.senderID): \(body)"
        case let .formattedText(body, _):
            return "\(item.senderID): \(body)"
        case let .mediaPlaceholder(resource):
            if resource.isEncrypted {
                return "\(item.senderID) sent encrypted media that cannot be opened until keys are available"
            }
            return "\(item.senderID) sent \(resource.safeDescription)"
        case .redacted:
            return "\(item.senderID): message deleted"
        case .encryptedPlaceholder:
            return "\(item.senderID): encrypted message unavailable"
        case let .unknown(type):
            return "\(item.senderID): unsupported event \(type)"
        case let .agentCard(card):
            let status = card.status.map { ", status \($0)" } ?? ""
            let primaryAction = card.actions.first(where: SynaraAgentCardActionResolver.shouldRender)
                .map { ", primary action \($0.title)" } ?? ""
            return "\(item.senderID): agent card: \(card.title)\(status)\(primaryAction)"
        }
    }

    private var accessibilityChildBehavior: AccessibilityChildBehavior {
        TimelineRowAccessibility.containsChildren(
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
            return "Tap Retry to send this message again"
        }

        switch item.kind {
        case .agentCard:
            return "Review available agent actions"
        default:
            return "Long press for message actions"
        }
    }

    private var isOutgoing: Bool {
        item.senderID == currentUserID
    }

    private var approvalPrompt: SynaraAgentApprovalPrompt? {
        SynaraAgentApprovalPromptDetector.detect(in: item)
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
                    .font(SynaraTypography.messageMeta.weight(.semibold))
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(1)
                Text(preview.snippet)
                    .font(SynaraTypography.messageMeta)
                    .foregroundStyle(SynaraColor.tertiaryText)
                    .lineLimit(2)
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

private struct ReactionPill: View {
    let title: String
    let count: Int?
    var isSystemImage = false
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
        .background(SynaraColor.elevatedSurface)
        .clipShape(Capsule())
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
                .background(SynaraColor.surface.opacity(0.72))
                .overlay(
                    RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)
                        .stroke(SynaraColor.critical.opacity(0.22), lineWidth: 1)
                )
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
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
                .background(SynaraColor.surface.opacity(0.55))
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
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
                            onReaction(SynaraAgentApprovalPromptReaction.approveAlways.reactionKey)
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
                .background(SynaraColor.critical.opacity(0.08))
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
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
                                onReaction(action.reactionKey)
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
                        }
                        .buttonStyle(.plain)
                        .foregroundStyle(action.tint)
                        .background(action.tint.opacity(action == .deny ? 0.08 : 0.10))
                        .overlay(
                            RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)
                                .stroke(action.tint.opacity(0.68), lineWidth: 1)
                        )
                        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
                        .accessibilityLabel("\(action.title) \(action.reactionKey)")
                        .accessibilityIdentifier("AgentApprovalPromptReaction-\(action.accessibilityIdentifierSuffix)-\(eventID)")
                    }
                }
            }
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
        .synaraCard(fill: SynaraColor.surface.opacity(0.7), stroke: SynaraColor.agent.opacity(0.2))
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
        .synaraCard(fill: SynaraColor.surface.opacity(0.65), stroke: SynaraColor.agent.opacity(0.22))
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
    var onOpenStickers: (() -> Void)? = nil
    @Binding var selectedPhotos: [PhotosPickerItem]
    @Binding var attachmentDrafts: [ComposerAttachmentDraft]
    var isSending = false
    #if canImport(UIKit)
        var onPasteImages: ([UIImage]) -> Void = { _ in }
    #endif
    @Binding var isFocusedExternally: Bool
    @State private var isAttachmentSheetPresented = false
    @State private var isFileImporterPresented = false
    #if canImport(UIKit)
        @State private var isCameraPresented = false
    #endif
    @State private var isFormattingBarVisible = false
    @State private var composerSelection = ComposerTextSelection.empty
    @State private var formattingRevision = 0
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
                ComposerRelationBanner(target: replyTarget, onCancel: onCancelRelation)
            }

            if let editTarget {
                ComposerRelationBanner(target: editTarget, onCancel: onCancelRelation)
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

            HStack(alignment: .center, spacing: SynaraSpacing.xSmall) {
                Button {
                    isAttachmentSheetPresented = true
                } label: {
                    Image(systemName: "plus")
                        .font(.system(size: 16, weight: .semibold))
                        .frame(width: 34, height: 34)
                        .background(SynaraColor.secondarySurface)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .clipShape(Circle())
                        .overlay(
                            Circle()
                                .stroke(SynaraColor.separator.opacity(0.45), lineWidth: 0.5)
                                .allowsHitTesting(false)
                        )
                }
                .buttonStyle(.plain)
                .contentShape(Rectangle())
                .disabled(isSending)
                .accessibilityLabel("Attach")
                .accessibilityIdentifier("AttachmentButton")

                if let onOpenStickers {
                    Button(action: onOpenStickers) {
                        Image(systemName: "face.smiling")
                            .font(.system(size: 16, weight: .semibold))
                            .frame(width: 34, height: 34)
                            .background(SynaraColor.secondarySurface)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .clipShape(Circle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Stickers")
                    .accessibilityIdentifier("StickerPackButton")
                }

                HStack(alignment: .center, spacing: SynaraSpacing.xSmall) {
                    composerField

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
                    }
                    .buttonStyle(.plain)
                    .contentShape(Rectangle())
                    .accessibilityLabel(isFormattingBarVisible ? "Hide formatting toolbar" : "Show formatting toolbar")
                    .accessibilityAddTraits(isFormattingBarVisible ? .isSelected : [])
                    .accessibilityIdentifier("ComposerFormattingToggle")
                }
                .padding(.leading, SynaraSpacing.small)
                .padding(.trailing, SynaraSpacing.xSmall)
                .padding(.vertical, 5)
                .background {
                    RoundedRectangle(cornerRadius: SynaraRadius.composer, style: .continuous)
                        .fill(SynaraColor.surface)
                }
                .overlay(
                    RoundedRectangle(cornerRadius: SynaraRadius.composer, style: .continuous)
                        .stroke(SynaraColor.separator.opacity(0.35), lineWidth: 0.5)
                        .allowsHitTesting(false)
                )

                if canSubmit {
                    Button(action: submitMessage) {
                        Image(systemName: "paperplane.fill")
                            .font(.system(size: 16, weight: .semibold))
                            .frame(width: 34, height: 34)
                            .background(sendButtonTint)
                            .foregroundStyle(Color.white)
                            .clipShape(Circle())
                    }
                    .buttonStyle(.plain)
                    .contentShape(Rectangle())
                    .disabled(isSending)
                    .accessibilityLabel("Send")
                    .accessibilityHint(
                        isSending
                            ? "Sending the current message and attachments"
                            : "Sends the current message and attachments"
                    )
                    .accessibilityIdentifier("ComposerSendButton")
                }
            }

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
        }
        .onAppear {
            isFocusedExternally = isComposerFocused
        }
        .onDisappear {
            isFocusedExternally = false
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

    private var canSubmit: Bool {
        ComposerAttachmentDraftList.canSend(text: text, drafts: attachmentDrafts)
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
        text.isEmpty == false || isComposerFocused
    }

    private var composerLineCount: Int {
        max(1, text.components(separatedBy: .newlines).count)
    }

    private var composerPromptMetrics: some View {
        HStack(spacing: SynaraSpacing.small) {
            Spacer()
            Text("\(text.count) chars · \(composerLineCount) line\(composerLineCount == 1 ? "" : "s")")
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
                text: $text,
                selection: $composerSelection,
                height: $composerFieldHeight,
                placeholder: placeholder,
                formattingRevision: formattingRevision,
                isFocused: $isComposerFocused,
                onPasteImages: onPasteImages
            )
            .frame(height: composerFieldHeight)
        #else
            TextField(placeholder, text: $text, axis: .vertical)
                .font(SynaraTypography.body)
                .focused($isComposerFocused)
                .lineLimit(1 ... 5)
                .frame(minHeight: composerFieldHeight)
                .accessibilityIdentifier("ComposerTextField")
                .onChange(of: text) { _ in
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
            if text.isEmpty, isComposerFocused == false {
                composerFieldHeight = singleLineHeight
                return
            }

            let lineCount = max(1, text.components(separatedBy: .newlines).count)
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
        let result = ComposerMarkdown.apply(format, to: text, selection: composerSelection)
        text = result.text
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
        let messageBody = text
        text = messageBody
        onSend(messageBody)
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
                            .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control))
                            .overlay(
                                RoundedRectangle(cornerRadius: SynaraRadius.control)
                                    .stroke(SynaraColor.separator.opacity(0.45), lineWidth: 0.5)
                                    .allowsHitTesting(false)
                            )
                    }
                    .buttonStyle(.plain)
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
        .overlay(
            RoundedRectangle(cornerRadius: SynaraRadius.composer, style: .continuous)
                .stroke(SynaraColor.separator.opacity(0.55), lineWidth: 0.5)
                .allowsHitTesting(false)
        )
        .shadow(color: Color.black.opacity(0.08), radius: 6, x: 0, y: 2)
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
    let onCancel: () -> Void

    var body: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.small) {
            VStack(alignment: .leading, spacing: 2) {
                Label(target.bannerTitle, systemImage: target.kind == .edit ? "pencil" : "arrowshape.turn.up.left")
                    .font(SynaraTypography.supporting.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(1)
                Text(target.snippet)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(2)
            }
            Spacer(minLength: SynaraSpacing.small)
            Button("Cancel", action: onCancel)
                .accessibilityLabel("Cancel \(target.kind == .edit ? "editing" : "reply")")
        }
        .padding(SynaraSpacing.small)
        .synaraCard(fill: SynaraColor.accent.opacity(0.08), stroke: SynaraColor.accent.opacity(0.18))
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
        formatter.timeStyle = .short
        formatter.dateStyle = .none
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
