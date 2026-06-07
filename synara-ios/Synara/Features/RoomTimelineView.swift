import SwiftUI
import PhotosUI
#if canImport(UIKit)
import UIKit
#endif

struct RoomTimelineView: View {
    let roomID: String
    let roomTitle: String?
    let focusedEventID: String?
    @Environment(\.appEnvironment) private var environment
    @State private var state: TimelineViewState = .idle
    @State private var draft: String = ""
    @State private var replyTarget: TimelineItem?
    @State private var editTarget: TimelineItem?
    @State private var sendError: String?
    @State private var hasAnchoredEvent = false
    @State private var uploadState: MediaUploadState = .idle
    @State private var viewerResource: MediaResource?
    @State private var selectedPhoto: PhotosPickerItem?
    @State private var agentActionMessage: String?
    @State private var cryptoStatus: RoomCryptoStatus = .unknown
    @State private var cryptoActionMessage: String?
    @State private var isRoomDetailsPresented = false
    @State private var lastRenderedTimelineCount = 0
    @State private var showJumpToLatest = false
    @State private var hasPositionedInitialTimeline = false
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
                cryptoStatus: cryptoStatus,
                onDetails: { isRoomDetailsPresented = true },
                onBack: { dismiss() }
            )
            timelineContent
            Divider()
            ComposerView(
                text: $draft,
                placeholder: "Send a message...",
                replyTarget: replyTarget,
                editTarget: editTarget,
                uploadState: uploadState,
                sendError: sendError,
                onCancelRelation: clearComposerRelation,
                onSend: sendMessage,
                onUpload: uploadTestMedia,
                selectedPhoto: $selectedPhoto
            )
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
        .task(id: roomID) {
            hasAnchoredEvent = false
            draft = environment.drafts.draft(roomID: roomID)
            let roomOpenSignpostID = PerformanceTrace.begin("RoomOpen")
            defer {
                PerformanceTrace.end("RoomOpen", id: roomOpenSignpostID)
            }
            let status = await loadCryptoStatus()
            if status.encryption == .encrypted {
                let encryptedOpenSignpostID = PerformanceTrace.begin("EncryptedRoomOpen")
                defer {
                    PerformanceTrace.end("EncryptedRoomOpen", id: encryptedOpenSignpostID)
                }
                await loadTimeline()
            } else {
                await loadTimeline()
            }
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
    }

    @ViewBuilder
    private var timelineContent: some View {
        switch state {
        case .idle, .loading:
            SynaraLoadingState(title: "Loading timeline")
        case .empty:
            SynaraEmptyState(title: "No Messages", systemImage: "text.bubble", message: "Messages will appear here.")
        case .failed(let message):
            SynaraErrorState(title: "Could Not Load Timeline", message: message) {
                Task {
                    await loadTimeline()
                }
            }
        case .loaded(let items, let isPaginating):
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                        if isPaginating {
                            ProgressView()
                                .frame(maxWidth: .infinity)
                        }

                        if shouldShowRecoveryBanner(items: items) {
                            CryptoRecoveryBanner(
                                status: cryptoStatus,
                                onRetry: retryDecryption,
                                onReviewSecurity: { environment.router.route(to: .settings) }
                            )
                        }

                        if isAgentRoom == false {
                            Button {
                                loadOlderTimeline(before: items.first?.eventID)
                            } label: {
                                Label("Load older messages", systemImage: "chevron.up")
                                    .font(.caption.weight(.semibold))
                                    .foregroundStyle(SynaraColor.secondaryText)
                                    .frame(maxWidth: .infinity)
                            }
                            .buttonStyle(.plain)
                            .padding(.vertical, SynaraSpacing.xSmall)
                            .background(SynaraColor.secondarySurface.opacity(0.55))
                            .clipShape(Capsule())
                            .disabled(isPaginating || items.isEmpty)
                            .accessibilityIdentifier("LoadOlderTimelineButton")
                        }

                        let threadReplyCounts = replyCounts(for: items)

                        ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                            if shouldShowMockupUnreadDivider(before: item, index: index) {
                                UnreadMessagesDivider()
                                    .padding(.vertical, SynaraSpacing.small)
                            }
                            TimelineRow(
                                item: item,
                                currentUserID: currentUserID,
                                isGroupedWithPrevious: isGroupedWithPrevious(index: index, items: items),
                                replyCount: threadReplyCounts[item.eventID] ?? 0,
                                availability: environment.eventActions.availability(for: item, currentUserID: currentUserID),
                                onReply: { replyTarget = item },
                                onOpenThread: { openThread(root: item) },
                                onEdit: { beginEdit(item) },
                                onRedact: { applyAction(.redact, to: item) },
                                onReact: { applyAction(.react("👍"), to: item) },
                                onOpenMedia: { resource in viewerResource = resource },
                                onAgentAction: { action in
                                    executeAgentAction(action, sourceEventID: item.eventID)
                                }
                            )
                            .id(item.eventID)
                        }
                    }
                    .padding(.horizontal, SynaraSpacing.large)
                    .padding(.top, isAgentRoom ? SynaraSpacing.medium : SynaraSpacing.small)
                    .padding(.bottom, SynaraSpacing.small)
                }
                .background(isAgentRoom ? SynaraColor.agentReviewBackground : SynaraColor.surface)
                .accessibilityIdentifier("TimelineList")
                .simultaneousGesture(
                    DragGesture(minimumDistance: 8).onChanged { _ in
                        if items.count > 8 {
                            showJumpToLatest = true
                        }
                    }
                )
                .overlay(alignment: .bottomTrailing) {
                    if showJumpToLatest, let latest = items.last {
                        JumpToLatestButton {
                            withAnimation(.easeInOut(duration: 0.2)) {
                                proxy.scrollTo(latest.eventID, anchor: .bottom)
                            }
                            showJumpToLatest = false
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
                    guard case .loaded(let updatedItems, _) = currentState else {
                        return
                    }
                    scrollToInitialPosition(items: updatedItems, proxy: proxy)
                    scrollToLatestMessageIfNeeded(items: updatedItems, proxy: proxy)
                    scrollToAnchoredEvent(items: updatedItems, proxy: proxy)
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

        hasPositionedInitialTimeline = true
        Task {
            await MainActor.run {
                proxy.scrollTo(latest.eventID, anchor: .bottom)
                showJumpToLatest = false
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
            }
        }
    }

    private func scrollToLatestMessageIfNeeded(items: [TimelineItem], proxy: ScrollViewProxy) {
        defer {
            lastRenderedTimelineCount = items.count
        }

        guard focusedEventID == nil,
              lastRenderedTimelineCount > 0,
              items.count > lastRenderedTimelineCount,
              showJumpToLatest == false,
              let latest = items.last else {
            return
        }

        Task {
            await MainActor.run {
                withAnimation {
                    proxy.scrollTo(latest.eventID, anchor: .bottom)
                }
            }
        }
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
        (roomTitle ?? "").localizedCaseInsensitiveContains("agent")
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

    private func replyCounts(for items: [TimelineItem]) -> [String: Int] {
        Dictionary(grouping: items.compactMap(\.replyToEventID), by: { $0 })
            .mapValues(\.count)
    }

    private func shouldShowMockupUnreadDivider(before item: TimelineItem, index: Int) -> Bool {
        ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1"
            && roomID == "!project:matrix.org"
            && index > 0
            && item.eventID.contains("$security:")
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

    private func loadTimeline() async {
        state = .loading
        showJumpToLatest = false
        hasPositionedInitialTimeline = false
        let signpostID = PerformanceTrace.begin("TimelineInitialLoad")
        defer {
            PerformanceTrace.end("TimelineInitialLoad", id: signpostID)
        }
        let items = await environment.timeline.loadInitialTimeline(roomID: roomID)
        await MainActor.run {
            state = items.isEmpty ? .empty : .loaded(items, isPaginating: false)
        }
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

    private func shouldShowRecoveryBanner(items: [TimelineItem]) -> Bool {
        cryptoStatus.needsRecoveryAttention || items.contains { item in
            if case .encryptedPlaceholder = item.kind {
                return true
            }
            return false
        }
    }

    private func sendMessage() {
        let body = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            sendError = MessageSendError.emptyMessage.localizedDescription
            return
        }

        let request = MessageSendRequest(
            roomID: roomID,
            body: body,
            replyToEventID: replyTarget?.eventID,
            editEventID: editTarget?.eventID
        )

        Task {
            do {
                let signpostID = PerformanceTrace.begin("MessageSend")
                defer {
                    PerformanceTrace.end("MessageSend", id: signpostID)
                }
                let item = try await environment.messageSender.send(request)
                await MainActor.run {
                    append(item)
                    draft = ""
                    environment.drafts.clearDraft(roomID: roomID)
                    clearComposerRelation()
                    sendError = nil
                }
            } catch {
                await MainActor.run {
                    sendError = MessageSendError.failed.localizedDescription
                }
            }
        }
    }

    private func uploadTestMedia() {
        uploadState = .uploading(progress: 0.5)
        Task {
            let signpostID = PerformanceTrace.begin("MediaUpload")
            defer {
                PerformanceTrace.end("MediaUpload", id: signpostID)
            }
            let result = await environment.mediaUploader.upload(
                MediaUploadRequest(
                    roomID: roomID,
                    source: .photoLibrary,
                    displayName: "synara-upload.jpg",
                    data: Data("Synara test image".utf8),
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

    private func loadOlderTimeline(before eventID: String?) {
        guard let eventID,
              case .loaded(let items, false) = state else {
            return
        }

        state = .loaded(items, isPaginating: true)
        Task {
            let signpostID = PerformanceTrace.begin("TimelineLoadOlder")
            defer {
                PerformanceTrace.end("TimelineLoadOlder", id: signpostID)
            }
            let older = await environment.timeline.loadOlderTimeline(roomID: roomID, before: eventID)
            await MainActor.run {
                let existingIDs = Set(items.map(\.id))
                let uniqueOlder = older.filter { existingIDs.contains($0.id) == false }
                state = .loaded(uniqueOlder + items, isPaginating: false)
                if uniqueOlder.isEmpty == false {
                    showJumpToLatest = true
                }
            }
        }
    }

    private func beginEdit(_ item: TimelineItem) {
        editTarget = item
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
            let updated = await environment.eventActions.apply(action, to: item, currentUserID: currentUserID, roomID: roomID)
            await MainActor.run {
                replace(updated)
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
}

private enum TimelineViewState: Equatable {
    case idle
    case loading
    case empty
    case failed(String)
    case loaded([TimelineItem], isPaginating: Bool)
}

private struct JumpToLatestButton: View {
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: "arrow.down")
                .font(.system(size: 17, weight: .bold))
                .frame(width: 44, height: 44)
                .background(.ultraThinMaterial, in: Circle())
                .overlay(
                    Circle()
                        .stroke(SynaraColor.separator.opacity(0.8), lineWidth: 1)
                )
                .foregroundStyle(SynaraColor.accent)
                .shadow(color: .black.opacity(0.12), radius: 10, x: 0, y: 4)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Jump to latest")
        .accessibilityIdentifier("JumpToLatestButton")
    }
}

private struct TimelineHeader: View {
    let title: String
    let subtitle: String
    let cryptoStatus: RoomCryptoStatus
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
                        .font(.headline.weight(.semibold))
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(1)
                }
                Text(subtitle)
                    .font(.caption)
                    .foregroundStyle(SynaraColor.secondaryText)
            }

            Spacer()

            HStack(spacing: SynaraSpacing.small) {
                if cryptoStatus.encryption != .unknown && cryptoStatus.encryption != .notEncrypted {
                    CryptoStatusPill(status: cryptoStatus)
                }

                Image(systemName: "magnifyingglass")
                    .font(.system(size: 17, weight: .medium))
                    .accessibilityHidden(true)
                Image(systemName: "phone")
                    .font(.system(size: 17, weight: .medium))
                    .accessibilityHidden(true)
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
                onUpload: uploadThreadAttachment,
                selectedPhoto: $selectedPhoto
            )
        }
        .background(SynaraColor.surface)
        .navigationBarBackButtonHidden(true)
        .toolbar(.hidden, for: .navigationBar)
        .toolbar(.hidden, for: .tabBar)
        .task(id: roomID + rootEventID) {
            await loadThread()
        }
        .onChange(of: selectedPhoto) { item in
            guard item != nil else {
                return
            }
            uploadThreadAttachment()
            selectedPhoto = nil
        }
    }

    @ViewBuilder
    private var threadContent: some View {
        switch state {
        case .idle, .loading:
            SynaraLoadingState(title: "Loading thread")
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
        let items = await environment.timeline.loadInitialTimeline(roomID: roomID)
        await MainActor.run {
            state = threadItems(from: items).isEmpty ? .empty : .loaded(items, isPaginating: false)
        }
    }

    private func threadItems(from items: [TimelineItem]) -> [TimelineItem] {
        let root = items.first { $0.eventID == rootEventID }
        let replies = items
            .filter { $0.replyToEventID == rootEventID }
            .sorted { $0.timestamp < $1.timestamp }

        if let root {
            return [root] + replies
        }

        return replies
    }

    private func sendThreadReply() {
        let body = draft.trimmingCharacters(in: .whitespacesAndNewlines)
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

    private func uploadThreadAttachment() {
        uploadState = .uploading(progress: 0.5)
        Task {
            let signpostID = PerformanceTrace.begin("ThreadMediaUpload")
            defer {
                PerformanceTrace.end("ThreadMediaUpload", id: signpostID)
            }
            let result = await environment.mediaUploader.upload(
                MediaUploadRequest(
                    roomID: roomID,
                    source: .photoLibrary,
                    displayName: "thread-attachment.jpg",
                    data: Data("Synara thread attachment".utf8),
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
                    .font(.headline.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                    .accessibilityIdentifier("ThreadTimelineTitle")
                Text(subtitle)
                    .font(.caption)
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
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(SynaraColor.primaryText)
                        Text(item.timestamp.timelineTime)
                            .font(.caption)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }

                    threadBody

                    if item.reactions.isEmpty == false {
                        HStack(spacing: SynaraSpacing.xSmall) {
                            ForEach(item.reactions.keys.sorted(), id: \.self) { reaction in
                                ReactionPill(title: reaction, count: item.reactions[reaction] ?? 0)
                            }
                            ReactionPill(title: "face.smiling", count: nil, isSystemImage: true)
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
                .font(.body)
                .foregroundStyle(SynaraColor.primaryText)
                .fixedSize(horizontal: false, vertical: true)
        case .formattedText(let body, let html):
            Text(MatrixHTMLRenderer.attributedString(body: body, html: html))
                .font(.body)
                .foregroundStyle(SynaraColor.primaryText)
                .fixedSize(horizontal: false, vertical: true)
        case .mediaPlaceholder(let resource):
            MediaAttachmentCard(resource: resource)
        case .redacted:
            Text("Message deleted")
                .font(.body)
                .foregroundStyle(SynaraColor.secondaryText)
        case .encryptedPlaceholder:
            Label("Encrypted content unavailable.", systemImage: "lock")
                .font(.body)
                .foregroundStyle(SynaraColor.secondaryText)
        case .agentCard(let card):
            AgentCardTimelineRow(card: card, onAction: { _ in })
        case .unknown(let type):
            Text("Unsupported event: \(type)")
                .font(.body)
                .foregroundStyle(SynaraColor.secondaryText)
        }
    }
}

private struct TimelineAvatar: View {
    let senderID: String
    let avatarURL: URL?
    let size: CGFloat
    @Environment(\.appEnvironment) private var environment
    @State private var avatarImage: UIImage?

    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            avatarContent

            Circle()
                .fill(SynaraColor.success)
                .frame(width: size * 0.24, height: size * 0.24)
                .overlay(Circle().stroke(SynaraColor.surface, lineWidth: 1.4))
                .offset(x: size * 0.03, y: size * 0.03)
        }
        .frame(width: size, height: size)
        .task(id: avatarURL) {
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
              case .signedIn(let session) = environment.session.currentState,
              let requestURL = mediaThumbnailURL(for: avatarURL, session: session) else {
            return
        }

        var request = URLRequest(url: requestURL)
        request.httpMethod = "GET"
        request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
        request.setValue("image/*", forHTTPHeaderField: "Accept")

        do {
            let (data, response) = try await URLSession.shared.data(for: request)
            guard let http = response as? HTTPURLResponse,
                  (200...299).contains(http.statusCode),
                  let image = UIImage(data: data) else {
                return
            }
            avatarImage = image
        } catch {
            avatarImage = nil
        }
    }

    private func mediaThumbnailURL(for sourceURL: URL, session: AuthenticatedSession) -> URL? {
        guard sourceURL.scheme == "mxc" else {
            return sourceURL
        }

        guard let serverName = sourceURL.host else {
            return nil
        }

        let mediaID = sourceURL.path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        guard mediaID.isEmpty == false else {
            return nil
        }

        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v1")
        url.appendPathComponent("media")
        url.appendPathComponent("thumbnail")
        url.appendPathComponent(serverName)
        url.appendPathComponent(mediaID)

        var components = URLComponents(url: url, resolvingAgainstBaseURL: false)
        components?.queryItems = [
            URLQueryItem(name: "width", value: "\(Int(size * 3))"),
            URLQueryItem(name: "height", value: "\(Int(size * 3))"),
            URLQueryItem(name: "method", value: "crop")
        ]
        return components?.url
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
            notificationMode = loadedDetails?.notificationMode ?? .allMessages
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
                    .font(.caption)
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
                .font(.caption)
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

private struct CryptoStatusPill: View {
    let status: RoomCryptoStatus

    var body: some View {
        Label(title, systemImage: systemImage)
            .font(.caption.weight(.semibold))
            .lineLimit(1)
            .padding(.horizontal, SynaraSpacing.small)
            .padding(.vertical, SynaraSpacing.xSmall)
            .background(tint.opacity(0.14))
            .foregroundStyle(tint)
            .clipShape(Capsule())
            .accessibilityIdentifier("RoomCryptoStatusPill")
            .accessibilityLabel(title)
    }

    private var title: String {
        if status.encryption == .unavailable {
            return "Encryption Unknown"
        }
        if status.unableToDecryptCount > 0 || status.recovery == .disabled || status.recovery == .incomplete {
            return "Recovery Needed"
        }
        if status.backup == .unavailable {
            return "No Key Backup"
        }
        if status.verification == .unverified {
            return "Unverified"
        }
        return "Encrypted"
    }

    private var systemImage: String {
        title == "Encrypted" ? "lock.fill" : "exclamationmark.lock.fill"
    }

    private var tint: Color {
        title == "Encrypted" ? SynaraColor.success : SynaraColor.warning
    }
}

private struct CryptoRecoveryBanner: View {
    let status: RoomCryptoStatus
    let onRetry: () -> Void
    let onReviewSecurity: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            Label("Encrypted history needs attention", systemImage: "lock.trianglebadge.exclamationmark")
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(SynaraColor.primaryText)

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

    private var detail: String {
        if status.recovery == .disabled || status.backup == .unavailable {
            return "This room is encrypted, but key backup or recovery is not available on this device. Retry sync, or verify/recover this device from Settings."
        }
        if status.recovery == .incomplete {
            return "This room is encrypted, but recovery is incomplete. Verify another session or recover keys before acting on undecrypted messages."
        }
        return "Some encrypted events are missing keys. Retry decryption after sync, or review device verification and recovery in Settings."
    }
}

private struct TimelineRow: View {
    let item: TimelineItem
    let currentUserID: String
    let isGroupedWithPrevious: Bool
    let replyCount: Int
    let availability: EventActionAvailability
    let onReply: () -> Void
    let onOpenThread: () -> Void
    let onEdit: () -> Void
    let onRedact: () -> Void
    let onReact: () -> Void
    let onOpenMedia: (MediaResource) -> Void
    let onAgentAction: (SynaraAgentCardAction) -> Void

    var body: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.small) {
            if isGroupedWithPrevious {
                Color.clear
                    .frame(width: 34, height: 1)
            } else {
                TimelineAvatar(senderID: item.senderID, avatarURL: item.senderAvatarURL, size: 34)
            }

            VStack(alignment: .leading, spacing: 5) {
                if isGroupedWithPrevious == false {
                    HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                        Text(senderDisplayName)
                            .font(.subheadline.weight(.semibold))
                            .foregroundStyle(SynaraColor.primaryText)
                            .lineLimit(1)
                        Text(item.timestamp.timelineTime)
                            .font(.caption)
                            .foregroundStyle(SynaraColor.secondaryText)
                        if item.isEdited {
                            Text("edited")
                                .font(.caption)
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
    }

    @ViewBuilder
    private var messageContent: some View {
        let content = VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            if let replyToEventID = item.replyToEventID {
                Label("Replying to \(replyToEventID)", systemImage: "arrowshape.turn.up.left")
                    .font(.caption)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(1)
            }

            bodyContent

            if item.reactions.isEmpty == false {
                HStack(spacing: SynaraSpacing.xSmall) {
                    ForEach(item.reactions.keys.sorted(), id: \.self) { reaction in
                        ReactionPill(title: reaction, count: item.reactions[reaction] ?? 0)
                    }
                    ReactionPill(title: "face.smiling", count: nil, isSystemImage: true)
                }
            }

            if replyCount > 0 {
                Button(action: onOpenThread) {
                    HStack(spacing: SynaraSpacing.xSmall) {
                        Image(systemName: "bubble.left.and.bubble.right.fill")
                        Text(replyCount == 1 ? "1 reply" : "\(replyCount) replies")
                    }
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(SynaraColor.accent)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("ThreadButton-\(item.eventID)")
            }
        }

        if usesBubble {
            content
                .padding(SynaraSpacing.small)
                .frame(maxWidth: .infinity, alignment: .leading)
                .synaraCard(fill: bubbleFill, stroke: bubbleStroke)
        } else {
            content
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    @ViewBuilder
    private var bodyContent: some View {
        switch item.kind {
        case .text(let body):
            Text(body)
                .font(.callout)
                .foregroundStyle(SynaraColor.primaryText)
                .lineLimit(nil)
                .frame(maxWidth: .infinity, alignment: .leading)
                .fixedSize(horizontal: false, vertical: true)
        case .formattedText(let body, let html):
            Text(MatrixHTMLRenderer.attributedString(body: body, html: html))
                .font(.callout)
                .foregroundStyle(SynaraColor.primaryText)
                .lineLimit(nil)
                .frame(maxWidth: .infinity, alignment: .leading)
                .fixedSize(horizontal: false, vertical: true)
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
            Text("Message deleted")
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.secondaryText)
        case .encryptedPlaceholder:
            Label("Encrypted content unavailable. Actions and media downloads are blocked until keys are available.", systemImage: "lock")
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.secondaryText)
        case .unknown(let type):
            Text("Unsupported event: \(type)")
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.secondaryText)
        case .agentCard(let card):
            AgentCardTimelineRow(card: card, onAction: onAgentAction)
        }
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

    private var senderDisplayName: String {
        guard item.senderID.hasPrefix("@") else {
            return item.senderID
        }

        return item.senderDisplayName
    }

    private var avatarTint: Color {
        if case .agentCard = item.kind {
            return SynaraColor.agent
        }
        return isOutgoing ? SynaraColor.accent : SynaraColor.secondaryText
    }

    private var bubbleFill: Color {
        if case .agentCard = item.kind {
            return SynaraColor.agent.opacity(0.08)
        }
        return isOutgoing ? SynaraColor.accent.opacity(0.12) : SynaraColor.secondarySurface
    }

    private var bubbleStroke: Color {
        if case .agentCard = item.kind {
            return SynaraColor.agent.opacity(0.28)
        }
        return isOutgoing ? SynaraColor.accent.opacity(0.22) : SynaraColor.separator.opacity(0.35)
    }

    private var usesBubble: Bool {
        switch item.kind {
        case .text, .formattedText, .mediaPlaceholder:
            return false
        default:
            return true
        }
    }
}

private struct MediaAttachmentCard: View {
    let resource: MediaResource

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
            SynaraIconTile(
                title: resource.safeDescription,
                systemImage: resource.safeDescription.localizedCaseInsensitiveContains(".pdf") ? "doc.richtext.fill" : "doc.text.fill",
                tint: resource.safeDescription.localizedCaseInsensitiveContains(".pdf") ? SynaraColor.critical : SynaraColor.accent,
                size: 30
            )

            VStack(alignment: .leading, spacing: 3) {
                Text(resource.safeDescription)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(1)
                Text(resource.safeDescription.localizedCaseInsensitiveContains(".pdf") ? "1.2 MB" : (resource.requiresAuthentication ? "Authenticated file" : "Attachment"))
                    .font(.caption)
                    .foregroundStyle(SynaraColor.secondaryText)
                if resource.isEncrypted {
                    Text("Encrypted media requires recovered keys")
                        .font(.caption2)
                        .foregroundStyle(SynaraColor.warning)
                        .lineLimit(2)
                }
            }

            Spacer()

            Image(systemName: resource.isEncrypted ? "lock.fill" : "arrow.down.to.line")
                .font(.system(size: 17, weight: .medium))
                .foregroundStyle(SynaraColor.secondaryText)
        }
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.vertical, SynaraSpacing.small)
        .synaraCard(fill: SynaraColor.surface)
    }
}

private struct UnreadMessagesDivider: View {
    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Rectangle()
                .fill(SynaraColor.separator.opacity(0.55))
                .frame(height: 0.5)
            Text("Unread messages")
                .font(.caption.weight(.semibold))
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

    var body: some View {
        HStack(spacing: SynaraSpacing.xSmall) {
            if isSystemImage {
                Image(systemName: title)
                    .font(.caption)
            } else {
                Text(title)
                    .font(.caption)
            }
            if let count {
                Text("\(count)")
                    .font(.caption.weight(.semibold))
                    .monospacedDigit()
            }
        }
        .padding(.horizontal, 7)
        .padding(.vertical, 3)
        .background(SynaraColor.elevatedSurface)
        .clipShape(Capsule())
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
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(2)
                        .accessibilityIdentifier("AgentCardTitle")
                    Text("Agent workflow")
                        .font(.caption)
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
                    .font(.subheadline)
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(2)
            }

            AgentApprovalDetails(card: card)

            if let preview = visibleActions.first(where: { $0.url != nil })?.url {
                VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                    Text("Preview")
                        .font(.caption)
                        .foregroundStyle(SynaraColor.secondaryText)
                    HStack {
                        Text(preview)
                            .font(.caption)
                            .foregroundStyle(SynaraColor.accent)
                            .lineLimit(1)
                        Spacer()
                        Image(systemName: "arrow.up.right.square")
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                    HStack(spacing: SynaraSpacing.xSmall) {
                        Image(systemName: "shield")
                            .font(.caption)
                            .foregroundStyle(SynaraColor.success)
                        Text("Safe link · Verified domain")
                            .font(.caption)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                }
                .padding(SynaraSpacing.small)
                .synaraCard(fill: SynaraColor.surface.opacity(0.7), stroke: SynaraColor.agent.opacity(0.2))
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
                                .font(.subheadline.weight(.semibold))
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
                                    .font(.subheadline.weight(.semibold))
                                    .frame(maxWidth: .infinity)
                            }
                            .buttonStyle(.borderedProminent)
                            .tint(action.tint)
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
                .font(.caption)
                .foregroundStyle(SynaraColor.secondaryText)
                .frame(width: 76, alignment: .leading)
            Text(value)
                .font(.caption)
                .foregroundStyle(valueTint)
                .lineLimit(3)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

private extension SynaraAgentCardAction {
    var isApprovalDecision: Bool {
        switch kind {
        case .some("approve"), .some("reject"):
            return true
        default:
            return false
        }
    }

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
    let replyTarget: TimelineItem?
    let editTarget: TimelineItem?
    let uploadState: MediaUploadState
    let sendError: String?
    let onCancelRelation: () -> Void
    let onSend: () -> Void
    let onUpload: () -> Void
    @Binding var selectedPhoto: PhotosPickerItem?
    @State private var isAttachmentSheetPresented = false
    @State private var unavailableAttachmentMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            if let replyTarget {
                ComposerRelationBanner(title: "Replying", eventID: replyTarget.eventID, onCancel: onCancelRelation)
            }

            if let editTarget {
                ComposerRelationBanner(title: "Editing", eventID: editTarget.eventID, onCancel: onCancelRelation)
            }

            if let sendError {
                Text(sendError)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("ComposerErrorText")
            }

            HStack(alignment: .bottom, spacing: SynaraSpacing.small) {
                Button {
                    isAttachmentSheetPresented = true
                } label: {
                    Image(systemName: "plus")
                        .font(.system(size: 17, weight: .semibold))
                        .frame(width: 40, height: 40)
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

                TextField(placeholder, text: $text, axis: .vertical)
                    .lineLimit(1...4)
                    .padding(.horizontal, SynaraSpacing.medium)
                    .padding(.vertical, SynaraSpacing.medium)
                    .background(SynaraColor.surface)
                    .clipShape(Capsule())
                    .overlay(
                        Capsule()
                            .stroke(SynaraColor.separator.opacity(0.35), lineWidth: 0.5)
                            .allowsHitTesting(false)
                    )
                    .accessibilityLabel("Message")
                    .accessibilityHint("Enter a message for this room")
                    .accessibilityIdentifier("ComposerTextField")

                Button(action: onSend) {
                    Image(systemName: text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "mic" : "paperplane.fill")
                        .font(.system(size: 17, weight: .semibold))
                        .frame(width: 40, height: 40)
                        .background(sendButtonTint)
                        .foregroundStyle(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? SynaraColor.secondaryText : Color.white)
                        .clipShape(Circle())
                }
                .buttonStyle(.plain)
                .contentShape(Rectangle())
                .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .opacity(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? 0.45 : 1)
                .accessibilityLabel("Send")
                .accessibilityHint("Sends the current message")
                .accessibilityIdentifier("ComposerSendButton")
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
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.top, SynaraSpacing.small)
        .padding(.bottom, SynaraSpacing.small)
        .background(SynaraColor.surface)
        .sheet(isPresented: $isAttachmentSheetPresented) {
            AttachmentOptionsSheet(
                onMockPhotoUpload: {
                    isAttachmentSheetPresented = false
                    onUpload()
                },
                onUnavailable: { option in
                    isAttachmentSheetPresented = false
                    unavailableAttachmentMessage = "\(option) attachments are not available in this build yet."
                },
                selectedPhoto: $selectedPhoto
            )
            .presentationDetents([.height(336)])
            .presentationDragIndicator(.visible)
        }
        .alert("Attachment Unavailable", isPresented: Binding(
            get: { unavailableAttachmentMessage != nil },
            set: { if !$0 { unavailableAttachmentMessage = nil } }
        )) {
            Button("OK") {
                unavailableAttachmentMessage = nil
            }
        } message: {
            Text(unavailableAttachmentMessage ?? "")
        }
    }

    private var sendButtonTint: Color {
        text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? SynaraColor.secondarySurface : SynaraColor.accent
    }
}

private struct AttachmentOptionsSheet: View {
    let onMockPhotoUpload: () -> Void
    let onUnavailable: (String) -> Void
    @Binding var selectedPhoto: PhotosPickerItem?

    private let options: [AttachmentOption] = [
        AttachmentOption(title: "Photo or Video", systemImage: "photo", tint: .green, kind: .photo),
        AttachmentOption(title: "File", systemImage: "doc", tint: .indigo, kind: .unavailable),
        AttachmentOption(title: "Location", systemImage: "mappin.circle", tint: .red, kind: .unavailable),
        AttachmentOption(title: "Poll", systemImage: "chart.bar.doc.horizontal", tint: .blue, kind: .unavailable),
        AttachmentOption(title: "Code Snippet", systemImage: "chevron.left.forwardslash.chevron.right", tint: .purple, kind: .unavailable),
        AttachmentOption(title: "Camera", systemImage: "camera", tint: .orange, kind: .unavailable),
        AttachmentOption(title: "Voice Message", systemImage: "mic", tint: .mint, kind: .unavailable),
        AttachmentOption(title: "Contact", systemImage: "person.crop.circle", tint: .mint, kind: .unavailable)
    ]

    var body: some View {
        VStack(spacing: SynaraSpacing.medium) {
            LazyVGrid(columns: [GridItem(.flexible(), spacing: SynaraSpacing.small), GridItem(.flexible(), spacing: SynaraSpacing.small)], spacing: SynaraSpacing.small) {
                ForEach(options) { option in
                    switch option.kind {
                    case .photo:
                        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1" {
                            Button(action: onMockPhotoUpload) {
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
                    case .unavailable:
                        Button {
                            onUnavailable(option.title)
                        } label: {
                            AttachmentOptionLabel(option: option)
                        }
                        .buttonStyle(.plain)
                        .accessibilityHint("Not available in this build yet")
                        .accessibilityIdentifier("AttachmentOption-\(option.title)")
                    }
                }
            }
        }
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.top, SynaraSpacing.large)
        .padding(.bottom, SynaraSpacing.large)
        .background(SynaraColor.surface)
        .accessibilityIdentifier("AttachmentOptionsSheet")
    }
}

private struct AttachmentOption: Identifiable {
    enum Kind {
        case photo
        case unavailable
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
                .font(.callout.weight(.semibold))
                .foregroundStyle(SynaraColor.primaryText)
                .lineLimit(1)
                .minimumScaleFactor(0.68)
                .allowsTightening(true)

            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, minHeight: 56, alignment: .leading)
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
                    .font(.callout.weight(.medium))
            } else if let systemImage {
                Image(systemName: systemImage)
                    .font(.callout.weight(.medium))
            }
        }
        .foregroundStyle(SynaraColor.secondaryText)
        .accessibilityHidden(true)
    }
}

private struct ComposerRelationBanner: View {
    let title: String
    let eventID: String
    let onCancel: () -> Void

    var body: some View {
        HStack {
            Label("\(title) \(eventID)", systemImage: title == "Editing" ? "pencil" : "arrowshape.turn.up.left")
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
                .lineLimit(1)
            Spacer()
            Button("Cancel", action: onCancel)
                .accessibilityLabel("Cancel \(title.lowercased())")
        }
        .padding(SynaraSpacing.small)
        .synaraCard(fill: SynaraColor.accent.opacity(0.08), stroke: SynaraColor.accent.opacity(0.18))
    }
}

private struct MediaViewer: View {
    let resource: MediaResource
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            VStack(spacing: SynaraSpacing.large) {
                Image(systemName: "photo")
                    .font(.system(size: 64, weight: .semibold))
                    .foregroundStyle(SynaraColor.secondaryText)
                Text(resource.safeDescription)
                    .font(SynaraTypography.screenTitle)
                    .multilineTextAlignment(.center)
                if resource.requiresAuthentication {
                    Text("Authenticated media")
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                }
            }
            .padding(SynaraSpacing.xLarge)
            .navigationTitle("Media")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
        .accessibilityIdentifier("MediaViewer")
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
    var senderDisplayName: String {
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
            break
        }

        guard senderID.hasPrefix("@") else {
            return senderID
        }

        return senderID
            .dropFirst()
            .split(separator: ":")
            .first
            .map(String.init) ?? senderID
    }

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
