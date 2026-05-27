import SwiftUI

struct RoomTimelineView: View {
    let roomID: String
    let roomTitle: String?
    @Environment(\.appEnvironment) private var environment
    @State private var state: TimelineViewState = .idle
    @State private var draft: String = ""
    @State private var replyTarget: TimelineItem?
    @State private var editTarget: TimelineItem?
    @State private var sendError: String?
    @State private var uploadState: MediaUploadState = .idle
    @State private var viewerResource: MediaResource?

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
                onUpload: uploadMockMedia
            )
        }
        .navigationTitle(roomTitle ?? "Room")
        .sheet(item: $viewerResource) { resource in
            MediaViewer(resource: resource)
        }
        .task(id: roomID) {
            draft = environment.drafts.draft(roomID: roomID)
            await loadTimeline()
        }
        .onChange(of: draft) { value in
            environment.drafts.setDraft(value, roomID: roomID)
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
            ScrollView {
                LazyVStack(alignment: .leading, spacing: SynaraSpacing.medium) {
                    if isPaginating {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                    }

                    ForEach(items) { item in
                        TimelineRow(
                            item: item,
                            currentUserID: currentUserID,
                            availability: environment.eventActions.availability(for: item, currentUserID: currentUserID),
                            onReply: { replyTarget = item },
                            onEdit: { beginEdit(item) },
                            onRedact: { applyAction(.redact, to: item) },
                            onReact: { applyAction(.react("👍"), to: item) },
                            onOpenMedia: { resource in viewerResource = resource }
                        )
                    }
                }
                .padding(SynaraSpacing.large)
            }
            .accessibilityIdentifier("TimelineList")
        }
    }

    private var currentUserID: String {
        if case .signedIn(let session) = environment.session.currentState {
            return session.userID
        }
        return "@local:matrix.org"
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

    private func uploadMockMedia() {
        uploadState = .uploading(progress: 0.5)
        Task {
            let result = await environment.mediaUploader.upload(
                MediaUploadRequest(roomID: roomID, source: .photoLibrary, displayName: "synara-upload.jpg")
            )
            await MainActor.run {
                uploadState = result
                if case .uploaded(let item) = result {
                    append(item)
                }
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
}

private enum TimelineViewState {
    case idle
    case loading
    case empty
    case failed(String)
    case loaded([TimelineItem], isPaginating: Bool)
}

private struct TimelineRow: View {
    let item: TimelineItem
    let currentUserID: String
    let availability: EventActionAvailability
    let onReply: () -> Void
    let onEdit: () -> Void
    let onRedact: () -> Void
    let onReact: () -> Void
    let onOpenMedia: (MediaResource) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            HStack {
                Text(item.senderID)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(SynaraColor.secondaryText)
                if item.isEdited {
                    Text("edited")
                        .font(.caption)
                        .foregroundStyle(SynaraColor.secondaryText)
                }
            }

            if let replyToEventID = item.replyToEventID {
                Text("Replying to \(replyToEventID)")
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
                            .background(SynaraColor.secondarySurface)
                            .clipShape(Capsule())
                    }
                }
            }
        }
        .padding(SynaraSpacing.medium)
        .background(SynaraColor.secondarySurface)
        .clipShape(RoundedRectangle(cornerRadius: 8))
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
        .accessibilityElement(children: .combine)
        .accessibilityLabel(accessibilitySummary)
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
        case .unknown(let type):
            Text("Unsupported event: \(type)")
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.secondaryText)
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
        case .unknown(let type):
            return "\(item.senderID): unsupported event \(type)"
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
                Button(action: onUpload) {
                    Image(systemName: "paperclip")
                        .frame(width: 24, height: 24)
                }
                .frame(width: 44, height: 44)
                .buttonStyle(.plain)
                .contentShape(Rectangle())
                .accessibilityLabel("Attach")
                .accessibilityIdentifier("AttachmentButton")

                TextField("Message", text: $text, axis: .vertical)
                    .lineLimit(1...4)
                    .padding(SynaraSpacing.small)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(SynaraColor.secondaryText.opacity(0.25))
                    )
                    .accessibilityIdentifier("ComposerTextField")

                Button(action: onSend) {
                    Image(systemName: "paperplane.fill")
                        .frame(width: 24, height: 24)
                }
                .frame(width: 44, height: 44)
                .buttonStyle(.plain)
                .contentShape(Rectangle())
                .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .accessibilityLabel("Send")
                .accessibilityIdentifier("ComposerSendButton")
            }

            if case .uploading(let progress) = uploadState {
                ProgressView(value: progress)
                    .accessibilityIdentifier("MediaUploadProgress")
            }
        }
        .padding(SynaraSpacing.medium)
    }
}

private struct ComposerRelationBanner: View {
    let title: String
    let eventID: String
    let onCancel: () -> Void

    var body: some View {
        HStack {
            Text("\(title) \(eventID)")
                .font(SynaraTypography.supporting)
                .lineLimit(1)
            Spacer()
            Button("Cancel", action: onCancel)
        }
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

struct RoomTimelineView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            RoomTimelineView(roomID: "!project:matrix.org", roomTitle: "Project")
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
