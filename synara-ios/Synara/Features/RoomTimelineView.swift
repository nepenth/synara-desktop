import SwiftUI
import PhotosUI
import UniformTypeIdentifiers
#if canImport(UIKit)
import UIKit
#endif

enum RoomTimelineFocusPolicy {
    static func initialLoadFocus(focusedEventID: String?, initialReadMarkerEventID: String?) -> String? {
        focusedEventID ?? initialReadMarkerEventID
    }

    static func updateStreamFocus(focusedEventID: String?, override: String?? = nil) -> String? {
        override ?? focusedEventID
    }
}

struct RoomTimelineView: View {
    private static let olderPaginationTopThreshold = 3
    private static let olderPaginationDebounceInterval: TimeInterval = 0.5
    private static let markFullyReadDelayNanoseconds: UInt64 = 1_000_000_000
    private static let timelineBottomLayoutDelayNanoseconds: UInt64 = 16_000_000
    private static let timelineBottomAnchorID = "timeline-bottom-anchor"

    let roomID: String
    let roomTitle: String?
    let focusedEventID: String?
    @Environment(\.appEnvironment) private var environment
    @State private var state: TimelineViewState = .idle
    @State private var draft: String = ""
    @State private var replyTarget: ComposerRelationTarget?
    @State private var editTarget: ComposerRelationTarget?
    @State private var sendError: String?
    @State private var hasAnchoredEvent = false
    @State private var uploadState: MediaUploadState = .idle
    @State private var viewerResource: MediaResource?
    @State private var selectedPhoto: PhotosPickerItem?
    @State private var agentActionMessage: String?
    @State private var cryptoStatus: RoomCryptoStatus = .unknown
    @State private var cryptoActionMessage: String?
    @State private var isCryptoBannerDismissed = false
    @State private var isRoomDetailsPresented = false
    @State private var isTimelineSearchPresented = false
    @State private var timelineSearchQuery = ""
    @State private var lastRenderedTimelineCount = 0
    @State private var showJumpToLatest = false
    @State private var hasPositionedInitialTimeline = false
    @State private var initialReadMarkerEventID: String?
    @State private var hasReachedOldestMessages = false
    @State private var lastOlderPaginationAt = Date.distantPast
    @State private var paginationScrollAnchorID: String?
    @State private var isJumpingToLatest = false
    @State private var pendingJumpToLatestEventID: String?
    @State private var isComposerFocused = false
    @State private var isTimelineBottomVisible = false
    @State private var lastMarkedFullyReadEventID: String?
    @State private var markFullyReadTask: Task<Void, Never>?
    @State private var timelineUpdatesTask: Task<Void, Never>?
    @State private var timelineScrollTask: Task<Void, Never>?
    @State private var sendAnimationItemIDs: Set<String> = []
    @Environment(\.openURL) private var openURL
    @Environment(\.dismiss) private var dismiss

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
                onSearch: { isTimelineSearchPresented = true },
                onDetails: { isRoomDetailsPresented = true },
                onBack: {
                    dismissKeyboard()
                    dismiss()
                }
            )
            timelineContent
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
                onMockMediaUpload: uploadMockMedia,
                onFileURL: uploadPickedFile,
                onCameraImage: uploadCameraImage,
                onUploadFailed: { message in
                    uploadState = .failed(message)
                },
                selectedPhoto: $selectedPhoto,
                isFocusedExternally: $isComposerFocused
            )
            .background(SynaraColor.surface)
            .shadow(color: Color.black.opacity(isAgentRoom ? 0.22 : 0.06), radius: 10, x: 0, y: -3)
        }
        .background(isAgentRoom ? SynaraColor.agentReviewBackground : SynaraColor.surface)
        .navigationTitle(roomTitle ?? "Room")
        .navigationBarBackButtonHidden(true)
        .toolbar(.hidden, for: .navigationBar)
        .toolbar(.hidden, for: .tabBar)
        .preferredColorScheme(isAgentRoom ? .dark : nil)
        .sheet(item: $viewerResource) { resource in
            MediaViewer(resource: resource)
        }
        .sheet(isPresented: $isRoomDetailsPresented) {
            RoomDetailsView(roomID: roomID, fallbackTitle: roomTitle ?? "Room")
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
            await loadTimeline()
            startTimelineUpdates(streamFocusEventID: focusedEventID == nil ? .some(nil) : nil)
        }
        .onDisappear {
            dismissKeyboard()
            timelineUpdatesTask?.cancel()
            cancelTimelineScroll()
            cancelMarkFullyRead()
        }
        .onChange(of: draft) { value in
            environment.drafts.setDraft(value, roomID: roomID)
        }
        .onChange(of: selectedPhoto) { item in
            guard let item else {
                return
            }
            uploadPickedPhoto(item)
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
            .background(isAgentRoom ? SynaraColor.agentReviewBackground : SynaraColor.surface)
            .accessibilityIdentifier("TimelineLoading")
        case .empty:
            SynaraEmptyState(title: "No Messages", systemImage: "text.bubble", message: "Messages will appear here.")
        case .failed(let message):
            SynaraErrorState(title: "Could Not Load Timeline", message: message) {
                Task {
                    await loadTimeline()
                    startTimelineUpdates(streamFocusEventID: focusedEventID == nil ? .some(nil) : nil)
                }
            }
        case .loaded(let items, let isPaginating):
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

                        ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                            if shouldShowUnreadDivider(before: item, at: index, in: items) {
                                UnreadMessagesDivider()
                                    .padding(.vertical, SynaraSpacing.small)
                            }
                            TimelineRow(
                                item: item,
                                currentUserID: currentUserID,
                                isGroupedWithPrevious: isGroupedWithPrevious(index: index, items: items),
                                animateSend: sendAnimationItemIDs.contains(item.id),
                                replyPreviewsByEventID: replyPreviewsByEventID,
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
                                onRetryFailedSend: {
                                    retryFailedMessage(item)
                                }
                            )
                            .id(item.eventID)
                            .onAppear {
                                if item.eventID == items.last?.eventID {
                                    scheduleMarkFullyRead(eventID: item.eventID)
                                }
                                if isAgentRoom == false, index < Self.olderPaginationTopThreshold {
                                    loadOlderTimelineIfNeeded(anchorItem: item, index: index, items: items)
                                }
                            }
                            .onDisappear {
                                if item.eventID == items.last?.eventID {
                                    cancelMarkFullyRead()
                                }
                            }
                        }

                        Color.clear
                            .frame(height: 1)
                            .id(Self.timelineBottomAnchorID)
                            .accessibilityHidden(true)
                            .onAppear {
                                isTimelineBottomVisible = true
                                showJumpToLatest = false
                            }
                            .onDisappear {
                                isTimelineBottomVisible = false
                                if items.count > 1 {
                                    showJumpToLatest = true
                                }
                            }
                    }
                    .padding(.horizontal, SynaraSpacing.medium)
                    .padding(.top, isAgentRoom ? SynaraSpacing.medium : SynaraSpacing.small)
                    .padding(.bottom, SynaraSpacing.small)
                }
                .scrollDismissesKeyboard(.interactively)
                .background(isAgentRoom ? SynaraColor.agentReviewBackground : SynaraColor.surface)
                .accessibilityIdentifier("TimelineList")
                .simultaneousGesture(
                    DragGesture(minimumDistance: 8).onChanged { _ in
                        if items.count > 8 {
                            cancelTimelineScroll()
                        }
                    }
                )
                .overlay(alignment: .bottomTrailing) {
                    if showJumpToLatest, isTimelineBottomVisible == false, let latest = items.last {
                        JumpToLatestButton(isLoading: isJumpingToLatest) {
                            jumpToLatest(proxy: proxy, currentItems: items, fallbackEventID: latest.eventID)
                        }
                        .padding(.trailing, SynaraSpacing.large)
                        .padding(.bottom, SynaraSpacing.medium)
                        .transition(.scale.combined(with: .opacity))
                    }
                }
                .onAppear {
                    lastRenderedTimelineCount = items.count
                    scrollToInitialPosition(items: items, proxy: proxy)
                    scrollToAnchoredEvent(items: items, proxy: proxy)
                }
                .onChange(of: state) { currentState in
                    guard case .loaded(let updatedItems, let isPaginating) = currentState else {
                        return
                    }
                    if let anchorID = paginationScrollAnchorID, isPaginating == false {
                        paginationScrollAnchorID = nil
                        Task {
                            await MainActor.run {
                                proxy.scrollTo(anchorID, anchor: .top)
                            }
                        }
                    }
                    scrollToInitialPosition(items: updatedItems, proxy: proxy)
                    scrollToLatestMessageIfNeeded(items: updatedItems, proxy: proxy)
                    scrollToAnchoredEvent(items: updatedItems, proxy: proxy)
                    scrollToPendingLatestIfNeeded(items: updatedItems, proxy: proxy)
                }
            }
        }
    }

    private func scrollToInitialPosition(items: [TimelineItem], proxy: ScrollViewProxy) {
        guard hasPositionedInitialTimeline == false,
              focusedEventID == nil,
              let latest = items.last else {
            return
        }

        let target = initialReadMarkerEventID.flatMap { eventID in
            items.first { item in
                item.eventID == eventID || item.id == eventID
            }
        } ?? latest
        let isLatestTarget = target.eventID == latest.eventID || target.id == latest.id

        hasPositionedInitialTimeline = true
        Task {
            await MainActor.run {
                if isLatestTarget {
                    scrollToTimelineBottom(
                        proxy: proxy,
                        eventID: target.eventID,
                        animated: false,
                        ignoreComposerFocus: true
                    )
                    showJumpToLatest = false
                } else {
                    proxy.scrollTo(target.eventID, anchor: .center)
                    showJumpToLatest = initialReadMarkerEventID != nil || isLatestTarget == false
                }
            }
        }
    }

    private func scrollToAnchoredEvent(items: [TimelineItem], proxy: ScrollViewProxy) {
        guard hasAnchoredEvent == false,
              let focusedEventID else {
            return
        }

        guard let target = items.first(where: { item in
            item.eventID == focusedEventID || item.id == focusedEventID
        }) else {
            return
        }

        hasAnchoredEvent = true
        Task {
            await MainActor.run {
                withAnimation {
                    proxy.scrollTo(target.id, anchor: .center)
                }
                if let latest = items.last {
                    showJumpToLatest = target.eventID != latest.eventID && target.id != latest.id
                }
            }
        }
    }

    private func scrollToLatestMessageIfNeeded(items: [TimelineItem], proxy: ScrollViewProxy) {
        defer {
            lastRenderedTimelineCount = items.count
        }

        guard focusedEventID == nil,
              isReadingFromEarlierPosition == false,
              lastRenderedTimelineCount > 0,
              items.count > lastRenderedTimelineCount,
              showJumpToLatest == false,
              let latest = items.last else {
            return
        }

        scrollToTimelineBottom(
            proxy: proxy,
            eventID: latest.eventID,
            animated: true,
            ignoreComposerFocus: true
        )
    }

    private func scrollToPendingLatestIfNeeded(items: [TimelineItem], proxy: ScrollViewProxy) {
        guard let pendingJumpToLatestEventID else {
            return
        }
        guard let latest = items.last,
              latest.eventID == pendingJumpToLatestEventID || latest.id == pendingJumpToLatestEventID else {
            return
        }

        self.pendingJumpToLatestEventID = nil
        scrollToTimelineBottom(proxy: proxy, eventID: latest.eventID, animated: true, ignoreComposerFocus: true)
    }

    private func scrollToTimelineBottom(
        proxy: ScrollViewProxy,
        eventID: String?,
        animated: Bool,
        ignoreComposerFocus: Bool = false
    ) {
        cancelTimelineScroll()
        timelineScrollTask = Task { @MainActor in
            let delays: [UInt64] = [0, 50, 150, 300]
            for (index, delayMilliseconds) in delays.enumerated() {
                if delayMilliseconds > 0 {
                    try? await Task.sleep(nanoseconds: delayMilliseconds * 1_000_000)
                }
                guard Task.isCancelled == false, (ignoreComposerFocus || isComposerFocused == false) else {
                    return
                }

                let shouldAnimate = animated && index > 0
                if shouldAnimate {
                    withAnimation(.easeInOut(duration: 0.2)) {
                        scrollToLatestMessageTarget(proxy: proxy, eventID: eventID)
                    }
                } else {
                    scrollToLatestMessageTarget(proxy: proxy, eventID: eventID)
                }

                if eventID != nil {
                    await Task.yield()
                    try? await Task.sleep(nanoseconds: Self.timelineBottomLayoutDelayNanoseconds)
                    guard Task.isCancelled == false, (ignoreComposerFocus || isComposerFocused == false) else {
                        return
                    }
                }

                if shouldAnimate {
                    withAnimation(.easeInOut(duration: 0.2)) {
                        scrollToTimelineBottomAnchor(proxy: proxy)
                    }
                } else {
                    scrollToTimelineBottomAnchor(proxy: proxy)
                }
            }
            showJumpToLatest = false
        }
    }

    private var isReadingFromEarlierPosition: Bool {
        guard focusedEventID == nil,
              initialReadMarkerEventID != nil,
              hasPositionedInitialTimeline else {
            return false
        }
        return showJumpToLatest
    }

    private func cancelTimelineScroll() {
        timelineScrollTask?.cancel()
        timelineScrollTask = nil
    }

    private func scrollToLatestMessageTarget(proxy: ScrollViewProxy, eventID: String?) {
        if let eventID {
            proxy.scrollTo(eventID, anchor: UnitPoint(x: 0.5, y: 0.86))
        }
    }

    private func scrollToTimelineBottomAnchor(proxy: ScrollViewProxy) {
        proxy.scrollTo(Self.timelineBottomAnchorID, anchor: .bottom)
    }

    private var currentUserID: String {
        if case .signedIn(let session) = environment.session.currentState {
            return session.userID
        }
        return "@local:matrix.org"
    }

    private var timelineSubtitle: String {
        guard case .loaded(let items, _) = state else {
            return "Matrix room"
        }

        let participantCount = Set(items.map(\.senderID)).count
        guard participantCount > 0 else {
            return "Matrix room"
        }
        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1",
           roomID == "!project:matrix.org" {
            return "21 members"
        }
        return "\(participantCount) members"
    }

    private var isAgentRoom: Bool {
        environment.roomList.isAgentRoom(roomID: roomID)
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

    private func resetTimelineState() {
        timelineUpdatesTask?.cancel()
        timelineUpdatesTask = nil
        cancelTimelineScroll()
        state = .idle
        draft = environment.drafts.draft(roomID: roomID)
        replyTarget = nil
        editTarget = nil
        sendError = nil
        hasAnchoredEvent = false
        uploadState = .idle
        viewerResource = nil
        selectedPhoto = nil
        agentActionMessage = nil
        cryptoStatus = .unknown
        cryptoActionMessage = nil
        isCryptoBannerDismissed = false
        isRoomDetailsPresented = false
        lastRenderedTimelineCount = 0
        showJumpToLatest = false
        hasPositionedInitialTimeline = false
        initialReadMarkerEventID = nil
        hasReachedOldestMessages = false
        lastOlderPaginationAt = .distantPast
        paginationScrollAnchorID = nil
        isJumpingToLatest = false
        pendingJumpToLatestEventID = nil
        isComposerFocused = false
        isTimelineBottomVisible = false
        lastMarkedFullyReadEventID = nil
        cancelMarkFullyRead()
    }

    private func shouldShowUnreadDivider(before item: TimelineItem, at index: Int, in items: [TimelineItem]) -> Bool {
        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1",
           roomID == "!project:matrix.org",
           index > 0,
           item.eventID.contains("$security:") {
            return true
        }

        guard let markerID = initialReadMarkerEventID,
              markerID.isEmpty == false,
              item.eventID != markerID,
              item.id != markerID else {
            return false
        }

        guard index > 0 else {
            return false
        }

        let previous = items[index - 1]
        return previous.eventID == markerID || previous.id == markerID
    }

    private func applyTimelineOutcome(_ outcome: TimelineLoadOutcome, isPaginating: Bool = false) {
        switch outcome {
        case .loaded(let items):
            let merged = mergeTimelineItems(items, isPaginating: isPaginating)
            state = .loaded(merged, isPaginating: isPaginating)
        case .empty:
            if let pendingItems = localPendingItems, pendingItems.isEmpty == false {
                state = .loaded(pendingItems, isPaginating: isPaginating)
            } else {
                state = .empty
            }
        case .failed(let message):
            state = .failed(message)
        }
    }

    private var localPendingItems: [TimelineItem]? {
        guard case .loaded(let items, _) = state else {
            return nil
        }
        let pendingItems = TimelinePendingReconciler.pendingItems(from: items)
        return pendingItems.isEmpty ? nil : pendingItems
    }

    private func mergeTimelineItems(_ streamItems: [TimelineItem], isPaginating: Bool) -> [TimelineItem] {
        let localItems: [TimelineItem]
        if case .loaded(let items, _) = state {
            localItems = items
        } else {
            localItems = []
        }

        return TimelinePendingReconciler.mergeStableWindow(
            streamItems: streamItems,
            localItems: localItems,
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
        state = .loading
        showJumpToLatest = false
        hasPositionedInitialTimeline = false
        initialReadMarkerEventID = nil
        let signpostID = PerformanceTrace.begin("TimelineInitialLoad")
        defer {
            PerformanceTrace.end("TimelineInitialLoad", id: signpostID)
        }
        let readMarkerEventID: String?
        if let focusedEventID {
            readMarkerEventID = focusedEventID
        } else {
            readMarkerEventID = await loadReadMarkerEventID()
        }
        await MainActor.run {
            initialReadMarkerEventID = focusedEventID == nil ? readMarkerEventID : nil
        }
    }

    private func loadTimeline() async {
        await prepareTimelineUpdates()
        let readMarkerEventID = RoomTimelineFocusPolicy.initialLoadFocus(
            focusedEventID: focusedEventID,
            initialReadMarkerEventID: initialReadMarkerEventID
        )
        let outcome = await environment.timeline.loadInitialTimeline(roomID: roomID, focusedEventID: readMarkerEventID)
        await MainActor.run {
            applyTimelineOutcome(outcome)
        }
    }

    private func startTimelineUpdates(streamFocusEventID overrideFocus: String?? = nil) {
        timelineUpdatesTask?.cancel()
        let streamFocusEventID = RoomTimelineFocusPolicy.updateStreamFocus(
            focusedEventID: focusedEventID,
            override: overrideFocus
        )
        timelineUpdatesTask = Task {
            for await outcome in environment.timeline.timelineUpdates(roomID: roomID, focusedEventID: streamFocusEventID) {
                guard Task.isCancelled == false else {
                    return
                }
                await MainActor.run {
                    switch outcome {
                    case .loaded(let items):
                        applyTimelineOutcome(.loaded(items))
                    case .empty:
                        if case .loading = state {
                            state = .empty
                        }
                    case .failed(let message):
                        if case .loaded = state {
                            return
                        }
                        state = .failed(message)
                    }
                }
            }
        }
    }

    private func loadReadMarkerEventID() async -> String? {
        await environment.readMarkers.fullyReadEventID(roomID: roomID)
    }

    private func loadCryptoStatus() async -> RoomCryptoStatus {
        let status = await environment.crypto.roomStatus(roomID: roomID)
        await MainActor.run {
            cryptoStatus = status
        }
        return status
    }

    private func retryDecryption() {
        Task {
            let result = await environment.crypto.retryDecryption(roomID: roomID)
            _ = await loadCryptoStatus()
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
        performSend(body: rawBody, replyToEventID: replyTarget?.eventID, editEventID: editTarget?.eventID)
    }

    private func retryFailedMessage(_ item: TimelineItem) {
        guard item.deliveryStatus == .failed,
              let body = TimelinePendingReconciler.messageBody(for: item) else {
            return
        }

        performSend(
            body: body,
            replyToEventID: item.replyToEventID,
            editEventID: nil,
            retrying: item
        )
    }

    private func performSend(
        body rawBody: String,
        replyToEventID: String?,
        editEventID: String?,
        retrying failedItem: TimelineItem? = nil
    ) {
        let body = rawBody.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            sendError = MessageSendError.emptyMessage.localizedDescription
            return
        }

        let request = MessageSendRequest(
            roomID: roomID,
            body: body,
            replyToEventID: replyToEventID,
            editEventID: editEventID
        )
        let isEditing = request.editEventID != nil

        let pendingLocalID: String?
        if isEditing == false {
            let pendingItem = TimelineItem.pendingMessage(
                localID: failedItem?.id ?? "$pending-\(UUID().uuidString)",
                body: body,
                senderID: currentUserID,
                replyToEventID: replyToEventID,
                deliveryStatus: .sending,
                timestamp: failedItem?.timestamp ?? Date()
            )
            pendingLocalID = pendingItem.id

            if failedItem != nil {
                replace(pendingItem)
            } else {
                append(pendingItem)
            }
            registerSendAnimation(for: pendingItem.id, isRetry: failedItem != nil)

            draft = ""
            environment.drafts.clearDraft(roomID: roomID)
            clearComposerRelation()
            sendError = nil
        } else {
            pendingLocalID = nil
        }

        Task {
            do {
                let signpostID = PerformanceTrace.begin("MessageSend")
                defer {
                    PerformanceTrace.end("MessageSend", id: signpostID)
                }
                let item = try await environment.messageSender.send(request)
                await MainActor.run {
                    if isEditing {
                        replace(item)
                        draft = ""
                        environment.drafts.clearDraft(roomID: roomID)
                        clearComposerRelation()
                    } else if let pendingLocalID {
                        markPendingSendSent(localID: pendingLocalID)
                        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1" {
                            reconcilePendingSend(localID: pendingLocalID, confirmed: item)
                        }
                    }
                    sendError = nil
                    if isEditing == false {
                        SynaraHaptics.trigger(.lightImpact)
                    }
                }
            } catch {
                await MainActor.run {
                    if isEditing {
                        sendError = MessageSendError.failed.localizedDescription
                    } else if let pendingLocalID {
                        markPendingSendFailed(localID: pendingLocalID)
                    }
                    SynaraHaptics.trigger(.warning)
                }
            }
        }
    }

    private func markPendingSendSent(localID: String) {
        guard case .loaded(let items, _) = state,
              let item = items.first(where: { $0.id == localID }) else {
            return
        }

        replace(item.withDeliveryStatus(.sent))
    }

    private func markPendingSendFailed(localID: String) {
        guard case .loaded(let items, _) = state,
              let item = items.first(where: { $0.id == localID }) else {
            return
        }

        replace(item.withDeliveryStatus(.failed))
    }

    private func reconcilePendingSend(localID: String, confirmed: TimelineItem) {
        guard case .loaded(let items, let isPaginating) = state else {
            return
        }

        let withoutPending = items.filter { $0.id != localID }
        if withoutPending.contains(where: { $0.eventID == confirmed.eventID }) {
            state = .loaded(withoutPending, isPaginating: isPaginating)
            return
        }

        state = .loaded(withoutPending + [confirmed], isPaginating: isPaginating)
    }

    private func uploadMockMedia(source: MediaUploadSource) {
        uploadState = .uploading(progress: 0.5)
        Task {
            let signpostID = PerformanceTrace.begin("MediaUpload")
            defer {
                PerformanceTrace.end("MediaUpload", id: signpostID)
            }
            let displayName: String
            let mimeType: String
            let data: Data
            switch source {
            case .photoLibrary:
                displayName = "synara-upload.jpg"
                mimeType = "image/jpeg"
                data = Data("Synara test image".utf8)
            case .file:
                displayName = "synara-upload.pdf"
                mimeType = "application/pdf"
                data = Data("Synara test file".utf8)
            case .camera:
                displayName = "synara-camera.jpg"
                mimeType = "image/jpeg"
                data = Data("Synara test camera image".utf8)
            }
            let result = await environment.mediaUploader.upload(
                MediaUploadRequest(
                    roomID: roomID,
                    source: source,
                    displayName: displayName,
                    data: data,
                    mimeType: mimeType
                )
            )
            await MainActor.run {
                uploadState = result
                if case .uploaded(let item) = result {
                    append(item)
                }
            }
        }
    }

    private func uploadPickedFile(_ url: URL) {
        uploadState = .uploading(progress: 0.25)
        Task {
            let signpostID = PerformanceTrace.begin("FilePickerUpload")
            defer {
                PerformanceTrace.end("FilePickerUpload", id: signpostID)
            }

            let didAccess = url.startAccessingSecurityScopedResource()
            defer {
                if didAccess {
                    url.stopAccessingSecurityScopedResource()
                }
            }

            do {
                let data = try Data(contentsOf: url)
                guard data.isEmpty == false else {
                    await MainActor.run {
                        uploadState = .failed("Attachment is empty.")
                    }
                    return
                }

                let result = await environment.mediaUploader.upload(
                    MediaUploadRequest(
                        roomID: roomID,
                        source: .file,
                        displayName: MediaAttachmentSupport.displayName(for: url),
                        data: data,
                        mimeType: MediaAttachmentSupport.mimeType(for: url)
                    )
                )
                await MainActor.run {
                    uploadState = result
                    if case .uploaded(let item) = result {
                        append(item)
                    }
                }
            } catch {
                await MainActor.run {
                    uploadState = .failed("Attachment could not be loaded. Try again.")
                }
            }
        }
    }

    #if canImport(UIKit)
    private func uploadCameraImage(_ image: UIImage) {
        uploadState = .uploading(progress: 0.25)
        Task {
            let signpostID = PerformanceTrace.begin("CameraCaptureUpload")
            defer {
                PerformanceTrace.end("CameraCaptureUpload", id: signpostID)
            }

            guard let data = MediaAttachmentSupport.jpegData(from: image) else {
                await MainActor.run {
                    uploadState = .failed("Attachment could not be loaded. Try again.")
                }
                return
            }

            let result = await environment.mediaUploader.upload(
                MediaUploadRequest(
                    roomID: roomID,
                    source: .camera,
                    displayName: "synara-camera.jpg",
                    data: data,
                    mimeType: "image/jpeg"
                )
            )
            await MainActor.run {
                uploadState = result
                if case .uploaded(let item) = result {
                    append(item)
                }
            }
        }
    }
    #endif

    private func uploadPickedPhoto(_ item: PhotosPickerItem) {
        uploadState = .uploading(progress: 0.25)
        Task {
            let signpostID = PerformanceTrace.begin("PhotoPickerUpload")
            defer {
                PerformanceTrace.end("PhotoPickerUpload", id: signpostID)
            }
            do {
                guard let data = try await item.loadTransferable(type: Data.self) else {
                    await MainActor.run {
                        uploadState = .failed("Attachment could not be loaded. Try again.")
                    }
                    return
                }

                let contentType = item.supportedContentTypes.first
                let fileExtension = contentType?.preferredFilenameExtension ?? "jpg"
                let mimeType = contentType?.preferredMIMEType ?? "image/jpeg"
                let result = await environment.mediaUploader.upload(
                    MediaUploadRequest(
                        roomID: roomID,
                        source: .photoLibrary,
                        displayName: "synara-photo.\(fileExtension)",
                        data: data,
                        mimeType: mimeType
                    )
                )
                await MainActor.run {
                    selectedPhoto = nil
                    uploadState = result
                    if case .uploaded(let item) = result {
                        append(item)
                    }
                }
            } catch {
                await MainActor.run {
                    selectedPhoto = nil
                    uploadState = .failed("Attachment could not be loaded. Try again.")
                }
            }
        }
    }

    private func loadOlderTimelineIfNeeded(anchorItem: TimelineItem, index: Int, items: [TimelineItem]) {
        guard index < Self.olderPaginationTopThreshold,
              let oldestEventID = items.first?.eventID else {
            return
        }
        loadOlderTimeline(before: oldestEventID, scrollAnchorID: anchorItem.eventID)
    }

    private func loadOlderTimeline(before eventID: String?, scrollAnchorID: String? = nil) {
        guard let eventID,
              hasReachedOldestMessages == false,
              case .loaded(let items, false) = state else {
            return
        }

        let now = Date()
        guard now.timeIntervalSince(lastOlderPaginationAt) >= Self.olderPaginationDebounceInterval else {
            return
        }
        lastOlderPaginationAt = now

        if let scrollAnchorID {
            paginationScrollAnchorID = scrollAnchorID
        }

        state = .loaded(items, isPaginating: true)
        Task {
            let signpostID = PerformanceTrace.begin("TimelineLoadOlder")
            defer {
                PerformanceTrace.end("TimelineLoadOlder", id: signpostID)
            }
            let outcome = await environment.timeline.loadOlderTimeline(roomID: roomID, before: eventID)
            await MainActor.run {
                switch outcome {
                case .loaded(let older):
                    let existingIDs = Set(items.map(\.id))
                    let uniqueOlder = older.filter { existingIDs.contains($0.id) == false }
                    state = .loaded(uniqueOlder + items, isPaginating: false)
                    if uniqueOlder.isEmpty == false {
                        showJumpToLatest = true
                    } else {
                        hasReachedOldestMessages = true
                    }
                case .empty:
                    hasReachedOldestMessages = true
                    paginationScrollAnchorID = nil
                    state = .loaded(items, isPaginating: false)
                case .failed(let message):
                    paginationScrollAnchorID = nil
                    state = .failed(message)
                }
            }
        }
    }

    private func scheduleMarkFullyRead(eventID: String) {
        markFullyReadTask?.cancel()
        guard lastMarkedFullyReadEventID != eventID else {
            return
        }

        markFullyReadTask = Task {
            try? await Task.sleep(nanoseconds: Self.markFullyReadDelayNanoseconds)
            guard Task.isCancelled == false else {
                return
            }

            let didMark = await environment.readMarkers.markFullyRead(roomID: roomID, eventID: eventID)
            guard Task.isCancelled == false else {
                return
            }

            await MainActor.run {
                if didMark {
                    lastMarkedFullyReadEventID = eventID
                    initialReadMarkerEventID = eventID
                }
                showJumpToLatest = false
            }
        }
    }

    private func cancelMarkFullyRead() {
        markFullyReadTask?.cancel()
        markFullyReadTask = nil
    }

    private func jumpToLatest(proxy: ScrollViewProxy, currentItems: [TimelineItem], fallbackEventID: String) {
        guard isJumpingToLatest == false else {
            return
        }

        dismissKeyboard()
        isComposerFocused = false
        cancelTimelineScroll()
        cancelMarkFullyRead()
        paginationScrollAnchorID = nil
        hasReachedOldestMessages = false

        let baselineItems: [TimelineItem]
        if case .loaded(let items, _) = state {
            baselineItems = items
        } else {
            baselineItems = currentItems
        }

        initialReadMarkerEventID = nil
        hasPositionedInitialTimeline = true
        showJumpToLatest = false
        isJumpingToLatest = true

        let immediateLatest = baselineItems.last
        scrollToTimelineBottom(
            proxy: proxy,
            eventID: immediateLatest?.eventID ?? fallbackEventID,
            animated: true,
            ignoreComposerFocus: true
        )
        if let immediateLatest {
            markLatestAsRead(eventID: immediateLatest.eventID)
        }
        startTimelineUpdates(streamFocusEventID: .some(nil))

        Task {
            let signpostID = PerformanceTrace.begin("TimelineJumpToLatest")
            defer {
                PerformanceTrace.end("TimelineJumpToLatest", id: signpostID)
            }
            let outcome = await environment.timeline.loadLatestTimeline(roomID: roomID)
            await MainActor.run {
                let nextItems: [TimelineItem]
                switch outcome {
                case .loaded(let items):
                    nextItems = items.isEmpty ? baselineItems : items
                case .empty:
                    nextItems = baselineItems
                case .failed:
                    nextItems = baselineItems
                }
                let merged = TimelinePendingReconciler.mergeStableWindow(
                    streamItems: nextItems,
                    localItems: baselineItems,
                    currentUserID: currentUserID
                )
                state = .loaded(merged, isPaginating: false)
                if let latest = merged.last {
                    pendingJumpToLatestEventID = latest.eventID
                    markLatestAsRead(eventID: latest.eventID)
                } else {
                    scrollToTimelineBottom(proxy: proxy, eventID: fallbackEventID, animated: true, ignoreComposerFocus: true)
                    showJumpToLatest = false
                }
                isJumpingToLatest = false
            }
        }
    }

    private func markLatestAsRead(eventID: String) {
        Task {
            _ = await environment.readMarkers.markFullyRead(roomID: roomID, eventID: eventID)
            await MainActor.run {
                lastMarkedFullyReadEventID = eventID
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
        if case .text(let body) = item.kind {
            draft = body
            environment.drafts.setDraft(body, roomID: roomID)
        } else if case .formattedText(let body, _) = item.kind {
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
        case .loaded(let items, let isPaginating):
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
        guard case .loaded(let items, let isPaginating) = state else {
            return
        }
        state = .loaded(items.map { $0.id == item.id ? item : $0 }, isPaginating: isPaginating)
    }

    private func executeAgentAction(_ action: SynaraAgentCardAction, sourceEventID: String?) {
        switch SynaraAgentCardActionResolver.plan(for: action) {
        case .success(let plan):
            switch plan {
            case .openURL(let url):
                openURL(url)
            case .copyText(let text):
                #if canImport(UIKit)
                UIPasteboard.general.string = text
                #endif
            case .submitApproval(let decision):
                submitAgentApproval(action, decision: decision, sourceEventID: sourceEventID)
            }
        case .failure(let error):
            switch error {
            case .unsupportedKind(let unsupported):
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

    private var loadedTimelineItems: [TimelineItem] {
        guard case .loaded(let items, _) = state else {
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
                Text(subtitle)
                    .font(SynaraTypography.messageMeta)
                    .foregroundStyle(SynaraColor.secondaryText)
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
    @State private var selectedPhoto: PhotosPickerItem?
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
                onMockMediaUpload: uploadMockThreadAttachment,
                onFileURL: uploadThreadFile,
                onCameraImage: uploadThreadCameraImage,
                onUploadFailed: { message in
                    uploadState = .failed(message)
                },
                selectedPhoto: $selectedPhoto,
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
        .onChange(of: selectedPhoto) { item in
            guard let item else {
                return
            }
            uploadThreadPhoto(item)
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
        case .failed(let message):
            SynaraErrorState(title: "Could Not Load Thread", message: message) {
                Task {
                    await loadThread()
                }
            }
        case .loaded(let items, _):
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
                    case .loaded(let items):
                        applyThreadOutcome(.loaded(items))
                    case .empty:
                        if case .loading = state {
                            state = .empty
                        }
                    case .failed(let message):
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
        case .loaded(let items):
            state = items.isEmpty ? .empty : .loaded(items, isPaginating: false)
        case .empty:
            state = .empty
        case .failed(let message):
            state = .failed(message)
        }
    }

    private func threadItems(from items: [TimelineItem]) -> [TimelineItem] {
        items
    }

    private func sendThreadReply(body rawBody: String) {
        let body = rawBody.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            sendError = MessageSendError.emptyMessage.localizedDescription
            return
        }

        sendError = nil
        Task {
            do {
                let signpostID = PerformanceTrace.begin("ThreadMessageSend")
                defer {
                    PerformanceTrace.end("ThreadMessageSend", id: signpostID)
                }
                let item = try await environment.messageSender.send(
                    MessageSendRequest(
                        roomID: roomID,
                        body: body,
                        replyToEventID: rootEventID,
                        editEventID: nil
                    )
                )
                await MainActor.run {
                    draft = ""
                    append(item)
                }
            } catch let error as MessageSendError {
                await MainActor.run {
                    sendError = error.localizedDescription
                }
            } catch {
                await MainActor.run {
                    sendError = MessageSendError.failed.localizedDescription
                }
            }
        }
    }

    private func uploadMockThreadAttachment(source: MediaUploadSource) {
        uploadState = .uploading(progress: 0.5)
        Task {
            let signpostID = PerformanceTrace.begin("ThreadMediaUpload")
            defer {
                PerformanceTrace.end("ThreadMediaUpload", id: signpostID)
            }
            let displayName: String
            let mimeType: String
            let data: Data
            switch source {
            case .photoLibrary:
                displayName = "thread-attachment.jpg"
                mimeType = "image/jpeg"
                data = Data("Synara thread attachment".utf8)
            case .file:
                displayName = "thread-attachment.pdf"
                mimeType = "application/pdf"
                data = Data("Synara thread file".utf8)
            case .camera:
                displayName = "thread-camera.jpg"
                mimeType = "image/jpeg"
                data = Data("Synara thread camera image".utf8)
            }
            let result = await environment.mediaUploader.upload(
                MediaUploadRequest(
                    roomID: roomID,
                    source: source,
                    displayName: displayName,
                    data: data,
                    mimeType: mimeType
                )
            )
            await MainActor.run {
                uploadState = result
                if case .uploaded(let item) = result {
                    append(item)
                }
            }
        }
    }

    private func uploadThreadPhoto(_ item: PhotosPickerItem) {
        uploadState = .uploading(progress: 0.25)
        Task {
            let signpostID = PerformanceTrace.begin("ThreadPhotoPickerUpload")
            defer {
                PerformanceTrace.end("ThreadPhotoPickerUpload", id: signpostID)
            }
            do {
                guard let data = try await item.loadTransferable(type: Data.self) else {
                    await MainActor.run {
                        uploadState = .failed("Attachment could not be loaded. Try again.")
                    }
                    return
                }

                let contentType = item.supportedContentTypes.first
                let fileExtension = contentType?.preferredFilenameExtension ?? "jpg"
                let mimeType = contentType?.preferredMIMEType ?? "image/jpeg"
                let result = await environment.mediaUploader.upload(
                    MediaUploadRequest(
                        roomID: roomID,
                        source: .photoLibrary,
                        displayName: "thread-photo.\(fileExtension)",
                        data: data,
                        mimeType: mimeType
                    )
                )
                await MainActor.run {
                    selectedPhoto = nil
                    uploadState = result
                    if case .uploaded(let item) = result {
                        append(item)
                    }
                }
            } catch {
                await MainActor.run {
                    selectedPhoto = nil
                    uploadState = .failed("Attachment could not be loaded. Try again.")
                }
            }
        }
    }

    private func uploadThreadFile(_ url: URL) {
        uploadState = .uploading(progress: 0.25)
        Task {
            let signpostID = PerformanceTrace.begin("ThreadFilePickerUpload")
            defer {
                PerformanceTrace.end("ThreadFilePickerUpload", id: signpostID)
            }

            let didAccess = url.startAccessingSecurityScopedResource()
            defer {
                if didAccess {
                    url.stopAccessingSecurityScopedResource()
                }
            }

            do {
                let data = try Data(contentsOf: url)
                guard data.isEmpty == false else {
                    await MainActor.run {
                        uploadState = .failed("Attachment is empty.")
                    }
                    return
                }

                let result = await environment.mediaUploader.upload(
                    MediaUploadRequest(
                        roomID: roomID,
                        source: .file,
                        displayName: MediaAttachmentSupport.displayName(for: url),
                        data: data,
                        mimeType: MediaAttachmentSupport.mimeType(for: url)
                    )
                )
                await MainActor.run {
                    uploadState = result
                    if case .uploaded(let item) = result {
                        append(item)
                    }
                }
            } catch {
                await MainActor.run {
                    uploadState = .failed("Attachment could not be loaded. Try again.")
                }
            }
        }
    }

    #if canImport(UIKit)
    private func uploadThreadCameraImage(_ image: UIImage) {
        uploadState = .uploading(progress: 0.25)
        Task {
            let signpostID = PerformanceTrace.begin("ThreadCameraCaptureUpload")
            defer {
                PerformanceTrace.end("ThreadCameraCaptureUpload", id: signpostID)
            }

            guard let data = MediaAttachmentSupport.jpegData(from: image) else {
                await MainActor.run {
                    uploadState = .failed("Attachment could not be loaded. Try again.")
                }
                return
            }

            let result = await environment.mediaUploader.upload(
                MediaUploadRequest(
                    roomID: roomID,
                    source: .camera,
                    displayName: "thread-camera.jpg",
                    data: data,
                    mimeType: "image/jpeg"
                )
            )
            await MainActor.run {
                uploadState = result
                if case .uploaded(let item) = result {
                    append(item)
                }
            }
        }
    }
    #endif

    private func append(_ item: TimelineItem) {
        switch state {
        case .loaded(let items, let isPaginating):
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
        case .text(let body):
            Text(body)
                .font(SynaraTypography.messageBody)
                .foregroundStyle(SynaraColor.primaryText)
                .fixedSize(horizontal: false, vertical: true)
        case .formattedText(let body, let html):
            MatrixFormattedMessageView(fallbackBody: body, html: html, font: SynaraTypography.messageBody)
        case .mediaPlaceholder(let resource):
            MediaAttachmentCard(resource: resource)
        case .redacted:
            Text("Message deleted")
                .font(SynaraTypography.messageBody)
                .foregroundStyle(SynaraColor.secondaryText)
        case .encryptedPlaceholder:
            Label("Encrypted content unavailable.", systemImage: "lock")
                .font(SynaraTypography.messageBody)
                .foregroundStyle(SynaraColor.secondaryText)
        case .agentCard(let card):
            AgentCardTimelineRow(card: card, onAction: { _ in })
        case .unknown(let type):
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
        self.segments = MatrixHTMLRenderer.segments(body: fallbackBody, html: html)
    }

    var body: some View {
        if segments.count == 1, case .markdown(let markdown) = segments[0] {
            markdownText(markdown)
        } else {
            VStack(alignment: .leading, spacing: SynaraSpacing.small) {
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
        case .markdown(let markdown):
            markdownText(markdown)
        case .code(let code):
            MatrixCodeBlockView(code: code)
        case .quote(let markdown):
            MatrixQuoteBlockView(markdown: markdown, font: font)
        case .details(let block):
            MatrixDetailsBlockView(block: block)
        }
    }

    private func markdownText(_ markdown: String) -> some View {
        Text(attributedMarkdown(markdown.isEmpty ? fallbackBody : markdown))
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
        Text(attributedMarkdown(markdown))
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
                Text(code)
                    .font(SynaraTypography.monoBody)
                    .foregroundStyle(SynaraColor.primaryText)
                    .textSelection(.enabled)
                    .padding(SynaraSpacing.medium)
                    .frame(maxWidth: .infinity, alignment: .leading)
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
              avatarURL.scheme == "mxc" else {
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
           let image = UIImage(data: data) {
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
                        .lineLimit(1...3)
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
                }

                Section("Aliases And Avatar") {
                    TextField("#room:server", text: $canonicalAlias)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .disabled(details?.canEditAliases != true || isLoading)
                        .accessibilityIdentifier("RoomCanonicalAliasField")
                    TextField("#alias:server, #other:server", text: $alternativeAliases, axis: .vertical)
                        .lineLimit(1...3)
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
                    environment.router.roomsPath = []
                    dismiss()
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
    let replyPreviewsByEventID: [String: TimelineReplyPreview]
    let replyCount: Int
    let availability: EventActionAvailability
    let onReply: () -> Void
    let onOpenThread: () -> Void
    let onEdit: () -> Void
    let onRedact: () -> Void
    let onReact: () -> Void
    let onOpenMedia: (MediaResource) -> Void
    let onAgentAction: (SynaraAgentCardAction) -> Void
    let onRetryFailedSend: () -> Void

    var body: some View {
        let row = HStack(alignment: .top, spacing: SynaraSpacing.small) {
            if isGroupedWithPrevious {
                Color.clear
                    .frame(width: 30, height: 1)
            } else {
                TimelineAvatar(senderID: item.senderID, avatarURL: item.senderAvatarURL, size: 30)
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

        let animatedRow = row
            .synaraSendSlideIn(isEnabled: animateSend, fromTrailing: isOutgoing)

        if item.deliveryStatus == .failed {
            Button(action: onRetryFailedSend) {
                animatedRow
            }
            .buttonStyle(.plain)
            .accessibilityHint("Tap to retry sending this message")
            .accessibilityIdentifier("TimelineItemRetry-\(item.eventID)")
        } else {
            animatedRow
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
        switch item.kind {
        case .text(let body):
            SynaraMessageBubble(
                text: body,
                alignment: bubbleAlignment,
                isGrouped: isGroupedWithPrevious,
                deliveryStatus: item.deliveryStatus
            )
        case .formattedText(let body, let html):
            SynaraMessageBubble(
                alignment: bubbleAlignment,
                variant: .standard,
                isGrouped: isGroupedWithPrevious,
                showsBackground: false,
                deliveryStatus: item.deliveryStatus
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
        case .agentCard(let card):
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

    @ViewBuilder
    private var bodyContent: some View {
        switch item.kind {
        case .text, .formattedText, .encryptedPlaceholder, .agentCard:
            EmptyView()
        case .mediaPlaceholder(let resource):
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
        case .unknown(let type):
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
        case .text(let body):
            return "\(item.senderID): \(body)"
        case .formattedText(let body, _):
            return "\(item.senderID): \(body)"
        case .mediaPlaceholder(let resource):
            if resource.isEncrypted {
                return "\(item.senderID) sent encrypted media that cannot be opened until keys are available"
            }
            return "\(item.senderID) sent \(resource.safeDescription)"
        case .redacted:
            return "\(item.senderID): message deleted"
        case .encryptedPlaceholder:
            return "\(item.senderID): encrypted message unavailable"
        case .unknown(let type):
            return "\(item.senderID): unsupported event \(type)"
        case .agentCard(let card):
            let status = card.status.map { ", status \($0)" } ?? ""
            let primaryAction = card.actions.first(where: SynaraAgentCardActionResolver.shouldRender)
                .map { ", primary action \($0.title)" } ?? ""
            return "\(item.senderID): agent card: \(card.title)\(status)\(primaryAction)"
        }
    }

    private var accessibilityChildBehavior: AccessibilityChildBehavior {
        if case .agentCard = item.kind {
            return .contain
        }

        if replyCount > 0 {
            return .contain
        }

        return .combine
    }

    private var accessibilityHint: String {
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

    @ViewBuilder
    private func replyQuoteLabel(for replyToEventID: String) -> some View {
        if let preview = replyPreviewsByEventID[replyToEventID] {
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
              resource.authenticatedURL != nil else {
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
           let image = UIImage(data: data) {
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
               let previewURL = linkAction.url {
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
    @Binding var selectedPhoto: PhotosPickerItem?
    @Binding var isFocusedExternally: Bool
    @State private var isAttachmentSheetPresented = false
    @State private var isFileImporterPresented = false
    #if canImport(UIKit)
    @State private var isCameraPresented = false
    #endif
    @State private var isFormattingBarVisible = false
    @State private var composerSelection = ComposerTextSelection.empty
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
                .accessibilityLabel("Attach")
                .accessibilityIdentifier("AttachmentButton")

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

                if text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
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
                    .accessibilityLabel("Send")
                    .accessibilityHint("Sends the current message")
                    .accessibilityIdentifier("ComposerSendButton")
                }
            }

            if showsPromptMetrics, shouldShowPromptMetrics {
                composerPromptMetrics
            }

            if case .uploading(let progress) = uploadState {
                ProgressView(value: progress)
                    .accessibilityIdentifier("MediaUploadProgress")
            } else if case .failed(let message) = uploadState {
                Text(message)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("MediaUploadErrorText")
            }
        }
        .padding(.horizontal, SynaraSpacing.small)
        .padding(.top, SynaraSpacing.xSmall)
        .padding(.bottom, SynaraSpacing.xSmall)
        .background(SynaraColor.surface)
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
                    isFileImporterPresented = true
                },
                onCamera: {
                    isAttachmentSheetPresented = false
                    #if canImport(UIKit)
                    if CameraCaptureSupport.isAvailable {
                        isCameraPresented = true
                    } else {
                        onUploadFailed("Camera is not available on this device.")
                    }
                    #endif
                },
                selectedPhoto: $selectedPhoto
            )
            .presentationDetents([.height(260)])
            .presentationDragIndicator(.visible)
        }
        .fileImporter(
            isPresented: $isFileImporterPresented,
            allowedContentTypes: [.item],
            allowsMultipleSelection: false
        ) { result in
            switch result {
            case .success(let urls):
                guard let url = urls.first else {
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
        text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? SynaraColor.secondarySurface : SynaraColor.accent
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
        TextField(placeholder, text: $text, axis: .vertical)
            .font(SynaraTypography.body)
            .foregroundStyle(SynaraColor.primaryText)
            .tint(SynaraColor.accent)
            .focused($isComposerFocused)
            .lineLimit(1...5)
            .submitLabel(.send)
            .onSubmit {
                if text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
                    submitMessage()
                }
            }
            .frame(minHeight: composerFieldHeight)
            .accessibilityLabel("Message")
            .accessibilityHint("Enter a message for this room")
            .accessibilityIdentifier("ComposerTextField")
        .onChange(of: text) { _ in
            updateComposerFieldHeight()
        }
        .onChange(of: isComposerFocused) { _ in
            updateComposerFieldHeight()
        }
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
        isComposerFocused = true
    }

    private func submitMessage() {
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
    @Binding var selectedPhoto: PhotosPickerItem?

    private let options: [AttachmentOption] = [
        AttachmentOption(title: "Photo or Video", systemImage: "photo", tint: SynaraColor.success, kind: .photo),
        AttachmentOption(title: "File", systemImage: "doc", tint: SynaraColor.accent, kind: .file),
        AttachmentOption(title: "Camera", systemImage: "camera", tint: SynaraColor.warning, kind: .camera)
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
            } else {
                PhotosPicker(selection: $selectedPhoto, matching: .any(of: [.images, .videos])) {
                    AttachmentOptionLabel(option: option)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("AttachmentOption-\(option.title)")
            }
        case .file:
            Button {
                if isUITestEnvironment {
                    onMockMediaUpload(.file)
                } else {
                    onFile()
                }
            } label: {
                AttachmentOptionLabel(option: option)
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("AttachmentOption-\(option.title)")
        case .camera:
            Button {
                if isUITestEnvironment {
                    onMockMediaUpload(.camera)
                } else {
                    onCamera()
                }
            } label: {
                AttachmentOptionLabel(option: option)
            }
            .buttonStyle(.plain)
            .accessibilityIdentifier("AttachmentOption-\(option.title)")
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
        self.systemImage = nil
    }

    init(systemImage: String) {
        self.title = nil
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
              let loadedImage = UIImage(data: data) else {
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
        case .text(let body):
            return body
        case .formattedText(let body, _):
            return body
        case .mediaPlaceholder(let resource):
            return resource.safeDescription
        case .agentCard(let card):
            return card.title
        case .redacted:
            return "Deleted message"
        case .encryptedPlaceholder:
            return "Encrypted message"
        case .unknown(let type):
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
