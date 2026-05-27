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
    @Environment(\.openURL) private var openURL

    init(roomID: String, roomTitle: String?, focusedEventID: String? = nil) {
        self.roomID = roomID
        self.roomTitle = roomTitle
        self.focusedEventID = focusedEventID
    }

    var body: some View {
        VStack(spacing: 0) {
            timelineContent
            Divider()
            ComposerView(
                text: $draft,
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
        .navigationTitle(roomTitle ?? "Room")
        .sheet(item: $viewerResource) { resource in
            MediaViewer(resource: resource)
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
        .task(id: roomID) {
            hasAnchoredEvent = false
            draft = environment.drafts.draft(roomID: roomID)
            await loadTimeline()
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
                    LazyVStack(alignment: .leading, spacing: SynaraSpacing.medium) {
                        if isPaginating {
                            ProgressView()
                                .frame(maxWidth: .infinity)
                        }

                        Button {
                            loadOlderTimeline(before: items.first?.eventID)
                        } label: {
                            Label("Load Older", systemImage: "arrow.up")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.bordered)
                        .disabled(isPaginating || items.isEmpty)
                        .accessibilityIdentifier("LoadOlderTimelineButton")

                        ForEach(Array(items.enumerated()), id: \.element.id) { index, item in
                            TimelineRow(
                                item: item,
                                currentUserID: currentUserID,
                                isGroupedWithPrevious: isGroupedWithPrevious(index: index, items: items),
                                availability: environment.eventActions.availability(for: item, currentUserID: currentUserID),
                                onReply: { replyTarget = item },
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
                    .padding(SynaraSpacing.large)
                }
                .accessibilityIdentifier("TimelineList")
                .onAppear {
                    scrollToAnchoredEvent(items: items, proxy: proxy)
                }
                .onChange(of: state) { currentState in
                    guard case .loaded(let updatedItems, _) = currentState else {
                        return
                    }
                    scrollToAnchoredEvent(items: updatedItems, proxy: proxy)
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
            }
        }
    }

    private var currentUserID: String {
        if case .signedIn(let session) = environment.session.currentState {
            return session.userID
        }
        return "@local:matrix.org"
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

    private func loadTimeline() async {
        state = .loading
        let items = await environment.timeline.loadInitialTimeline(roomID: roomID)
        await MainActor.run {
            state = items.isEmpty ? .empty : .loaded(items, isPaginating: false)
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
            let older = await environment.timeline.loadOlderTimeline(roomID: roomID, before: eventID)
            await MainActor.run {
                let existingIDs = Set(items.map(\.id))
                let uniqueOlder = older.filter { existingIDs.contains($0.id) == false }
                state = .loaded(uniqueOlder + items, isPaginating: false)
            }
        }
    }

    private func beginEdit(_ item: TimelineItem) {
        editTarget = item
        if case .text(let body) = item.kind {
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

private struct TimelineRow: View {
    let item: TimelineItem
    let currentUserID: String
    let isGroupedWithPrevious: Bool
    let availability: EventActionAvailability
    let onReply: () -> Void
    let onEdit: () -> Void
    let onRedact: () -> Void
    let onReact: () -> Void
    let onOpenMedia: (MediaResource) -> Void
    let onAgentAction: (SynaraAgentCardAction) -> Void

    var body: some View {
        HStack(alignment: .bottom, spacing: SynaraSpacing.small) {
            if isOutgoing {
                Spacer(minLength: 40)
            } else if isGroupedWithPrevious {
                Color.clear
                    .frame(width: 34, height: 1)
            } else {
                SynaraAvatar(title: item.senderID, tint: avatarTint, size: 34)
            }

            VStack(alignment: isOutgoing ? .trailing : .leading, spacing: SynaraSpacing.xSmall) {
                if isGroupedWithPrevious == false {
                    HStack(spacing: SynaraSpacing.xSmall) {
                        Text(senderDisplayName)
                            .font(.caption.weight(.semibold))
                            .foregroundStyle(SynaraColor.secondaryText)
                            .lineLimit(1)
                        Text(item.timestamp.timelineTime)
                            .font(.caption2)
                            .foregroundStyle(SynaraColor.tertiaryText)
                        if item.isEdited {
                            Text("edited")
                                .font(.caption2)
                                .foregroundStyle(SynaraColor.tertiaryText)
                        }
                    }
                }

                VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                    if let replyToEventID = item.replyToEventID {
                        Label("Replying to \(replyToEventID)", systemImage: "arrowshape.turn.up.left")
                            .font(.caption)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .lineLimit(2)
                    }

                    bodyContent

                    if item.reactions.isEmpty == false {
                        HStack {
                            ForEach(item.reactions.keys.sorted(), id: \.self) { reaction in
                                Text("\(reaction) \(item.reactions[reaction] ?? 0)")
                                    .font(.caption)
                                    .padding(.horizontal, SynaraSpacing.small)
                                    .padding(.vertical, SynaraSpacing.xSmall)
                                    .background(SynaraColor.elevatedSurface)
                                    .clipShape(Capsule())
                            }
                        }
                    }
                }
                .padding(SynaraSpacing.medium)
                .frame(maxWidth: 520, alignment: .leading)
                .synaraCard(fill: bubbleFill, stroke: bubbleStroke)
            }

            if isOutgoing == false {
                Spacer(minLength: 40)
            }
        }
        .padding(.top, isGroupedWithPrevious ? 0 : SynaraSpacing.small)
        .contextMenu {
            if availability.canReply {
                Button("Reply", action: onReply)
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
    private var bodyContent: some View {
        switch item.kind {
        case .text(let body):
            Text(body)
                .font(SynaraTypography.body)
                .lineLimit(nil)
        case .mediaPlaceholder(let resource):
            Button {
                onOpenMedia(resource)
            } label: {
                Label(resource.safeDescription, systemImage: "photo")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .accessibilityIdentifier("MediaPlaceholder-\(resource.filename)")
        case .redacted:
            Text("Message deleted")
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.secondaryText)
        case .encryptedPlaceholder:
            Label("Encrypted message unavailable", systemImage: "lock")
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
        case .mediaPlaceholder(let resource):
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

        return item.senderID
            .dropFirst()
            .split(separator: ":")
            .first
            .map(String.init) ?? item.senderID
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
}

private struct AgentCardTimelineRow: View {
    let card: SynaraAgentCard
    let onAction: (SynaraAgentCardAction) -> Void

    var body: some View {
        let visibleActions = card.actions.filter { SynaraAgentCardActionResolver.shouldRender($0) }

        VStack(alignment: .leading, spacing: SynaraSpacing.medium) {
            HStack(alignment: .center, spacing: SynaraSpacing.small) {
                SynaraAvatar(title: "Agent", systemImage: "sparkles", tint: SynaraColor.agent, size: 32)

                VStack(alignment: .leading, spacing: 2) {
                    Text(card.title)
                        .font(SynaraTypography.body.weight(.semibold))
                        .foregroundStyle(SynaraColor.primaryText)
                        .accessibilityIdentifier("AgentCardTitle")
                    Text("Agent workflow")
                        .font(.caption)
                        .foregroundStyle(SynaraColor.secondaryText)
                }

                Spacer()

                if let status = card.status {
                    SynaraStatusChip(title: status, tint: SynaraColor.agent, systemImage: "circle.dashed")
                }
            }

            if let summary = card.summary {
                Text(summary)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(nil)
            }

            if visibleActions.isEmpty == false {
                VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                    Text("Actions")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(SynaraColor.secondaryText)

                    ForEach(visibleActions, id: \.id) { action in
                        Button {
                            onAction(action)
                        } label: {
                            HStack {
                                Image(systemName: action.systemImage)
                                    .accessibilityHidden(true)
                                Text(action.title)
                                    .font(.callout.weight(.semibold))
                                Spacer()
                            }
                            .frame(maxWidth: .infinity)
                        }
                        .disabled(SynaraAgentCardActionResolver.shouldRender(action) == false)
                        .buttonStyle(.borderedProminent)
                        .tint(action.tint)
                        .controlSize(.regular)
                        .accessibilityHint("Performs \(action.title)")
                        .accessibilityIdentifier("AgentCardAction-\(action.id)")
                    }
                }
            }
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
    let replyTarget: TimelineItem?
    let editTarget: TimelineItem?
    let uploadState: MediaUploadState
    let sendError: String?
    let onCancelRelation: () -> Void
    let onSend: () -> Void
    let onUpload: () -> Void
    @Binding var selectedPhoto: PhotosPickerItem?

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
                if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1" {
                    SynaraActionIconButton(systemImage: "paperclip", accessibilityLabel: "Attach", tint: SynaraColor.secondaryText, action: onUpload)
                        .accessibilityIdentifier("AttachmentButton")
                } else {
                    PhotosPicker(selection: $selectedPhoto, matching: .images) {
                        Image(systemName: "paperclip")
                            .font(.system(size: 17, weight: .semibold))
                            .frame(width: 44, height: 44)
                            .background(SynaraColor.secondaryText.opacity(0.12))
                            .foregroundStyle(SynaraColor.secondaryText)
                            .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control))
                    }
                    .buttonStyle(.plain)
                    .contentShape(Rectangle())
                    .accessibilityLabel("Attach")
                    .accessibilityIdentifier("AttachmentButton")
                }

                TextField("Message", text: $text, axis: .vertical)
                    .lineLimit(1...4)
                    .padding(SynaraSpacing.small)
                    .background(SynaraColor.elevatedSurface)
                    .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control))
                    .accessibilityLabel("Message")
                    .accessibilityHint("Enter a message for this room")
                    .accessibilityIdentifier("ComposerTextField")

                Button(action: onSend) {
                    Image(systemName: "paperplane.fill")
                        .font(.system(size: 17, weight: .semibold))
                        .frame(width: 44, height: 44)
                        .background(sendButtonTint)
                        .foregroundStyle(Color.white)
                        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control))
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
        .padding(SynaraSpacing.medium)
        .background(.regularMaterial)
    }

    private var sendButtonTint: Color {
        text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? SynaraColor.secondaryText : SynaraColor.accent
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

struct RoomTimelineView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            RoomTimelineView(roomID: "!project:matrix.org", roomTitle: "Project")
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
