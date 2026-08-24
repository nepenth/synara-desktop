import SwiftUI

struct RoomNotesView: View {
    let roomID: String
    let roomTitle: String
    let onOpenMessage: (String) -> Void

    @Environment(\.appEnvironment) private var environment
    @State private var items: [SynaraRoomNoteItem] = []
    @State private var draftKind: RoomNoteKind = .note
    @State private var draftBody = ""
    @State private var isLoading = true
    @State private var isMutating = false
    @State private var errorMessage: String?
    @State private var editingItem: SynaraRoomNoteItem?
    @FocusState private var isDraftFocused: Bool

    var body: some View {
        List {
            composerSection

            if isLoading {
                Section {
                    SynaraSkeletonList(rowCount: 4, showsAvatar: false)
                        .listRowSeparator(.hidden)
                }
            } else if items.isEmpty {
                Section {
                    VStack(spacing: SynaraSpacing.medium) {
                        Image(systemName: "note.text")
                            .font(.system(size: 34, weight: .regular))
                            .foregroundStyle(SynaraColor.accent)
                            .accessibilityHidden(true)
                        Text("No personal notes yet")
                            .font(SynaraTypography.sectionTitle)
                            .foregroundStyle(SynaraColor.primaryText)
                        Text("Add a note or ToDo here, or pin a useful message from its menu.")
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .multilineTextAlignment(.center)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, SynaraSpacing.xLarge)
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("RoomNotesEmptyState")
                }
            } else {
                notesSection(title: "Active", items: items.filter { $0.isCompleted == false })
                notesSection(title: "Completed", items: items.filter { $0.isCompleted })
            }
        }
        .listStyle(.insetGrouped)
        .navigationTitle("Personal Notes")
        .navigationBarTitleDisplayMode(.inline)
        .accessibilityIdentifier("RoomNotesScreen")
        .refreshable { await load() }
        .task { await load() }
        .toolbar {
            ToolbarItemGroup(placement: .keyboard) {
                Spacer()
                Button("Done") { isDraftFocused = false }
            }
        }
        .sheet(item: $editingItem) { item in
            RoomNoteEditView(item: item) { body in
                await update(item, body: body)
            }
        }
        .alert("Personal Notes", isPresented: errorBinding) {
            Button("OK", role: .cancel) { errorMessage = nil }
        } message: {
            Text(errorMessage ?? "Try again.")
        }
    }

    private var composerSection: some View {
        Section {
            Picker("Item type", selection: $draftKind) {
                Text("Note").tag(RoomNoteKind.note)
                Text("ToDo").tag(RoomNoteKind.todo)
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("RoomNotesKindPicker")

            TextEditor(text: $draftBody)
                .frame(minHeight: 88)
                .disabled(isMutating)
                .focused($isDraftFocused)
                .accessibilityLabel(draftKind == .todo ? "New ToDo" : "New private note")
                .accessibilityIdentifier("RoomNotesBodyEditor")

            Button {
                addItem()
            } label: {
                HStack {
                    Label(draftKind == .todo ? "Add ToDo" : "Add Note", systemImage: "plus")
                    Spacer()
                    if isMutating {
                        ProgressView()
                            .controlSize(.small)
                    }
                }
            }
            .disabled(isMutating || draftBody.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            .accessibilityIdentifier("RoomNotesAddButton")
        } header: {
            Text(roomTitle)
        } footer: {
            Text("Private to your account. Synced across Synara clients; never posted to the room.")
        }
    }

    @ViewBuilder
    private func notesSection(title: String, items sectionItems: [SynaraRoomNoteItem]) -> some View {
        if sectionItems.isEmpty == false {
            Section(title) {
                ForEach(sectionItems) { item in
                    RoomNoteRow(
                        item: item,
                        canMoveUp: canMove(item, direction: .up),
                        canMoveDown: canMove(item, direction: .down),
                        onToggle: { setCompleted(item, completed: item.isCompleted == false) },
                        onMove: { move(item, direction: $0) },
                        onEdit: { editingItem = item },
                        onDelete: { delete(item) },
                        onOpenMessage: { eventID in onOpenMessage(eventID) }
                    )
                    .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                        Button(role: .destructive) {
                            delete(item)
                        } label: {
                            Label("Delete", systemImage: "trash")
                        }
                        .accessibilityIdentifier("RoomNotesDelete-\(item.id)")
                    }
                }
                .onDelete { offsets in
                    for index in offsets {
                        delete(sectionItems[index])
                    }
                }
            }
        }
    }

    private var errorBinding: Binding<Bool> {
        Binding(
            get: { errorMessage != nil },
            set: { if $0 == false { errorMessage = nil } }
        )
    }

    private func load() async {
        let result = await environment.roomNotes.loadItems(roomID: roomID)
        await MainActor.run {
            apply(result, clearDraft: false)
            isLoading = false
        }
    }

    private func addItem() {
        let body = draftBody
        isMutating = true
        Task {
            let result = await environment.roomNotes.addItem(roomID: roomID, kind: draftKind, body: body)
            await MainActor.run {
                apply(result, clearDraft: true)
                isMutating = false
            }
        }
    }

    private func delete(_ item: SynaraRoomNoteItem) {
        mutate {
            await environment.roomNotes.deleteItem(roomID: roomID, itemID: item.id)
        }
    }

    private func update(_ item: SynaraRoomNoteItem, body: String) async -> Bool {
        let result = await environment.roomNotes.updateItem(item, body: body)
        return await MainActor.run {
            if case let .success(snapshot) = result {
                items = snapshot
                return true
            }
            return false
        }
    }

    private func setCompleted(_ item: SynaraRoomNoteItem, completed: Bool) {
        mutate {
            await environment.roomNotes.setTodoCompleted(roomID: roomID, itemID: item.id, completed: completed)
        }
    }

    private func move(_ item: SynaraRoomNoteItem, direction: RoomNoteMoveDirection) {
        mutate {
            await environment.roomNotes.moveTodo(roomID: roomID, itemID: item.id, direction: direction)
        }
    }

    private func mutate(
        _ operation: @escaping () async -> Result<[SynaraRoomNoteItem], RoomNotesError>
    ) {
        guard isMutating == false else { return }
        isMutating = true
        Task {
            let result = await operation()
            await MainActor.run {
                apply(result, clearDraft: false)
                isMutating = false
            }
        }
    }

    private func apply(_ result: Result<[SynaraRoomNoteItem], RoomNotesError>, clearDraft: Bool) {
        switch result {
        case .success(let snapshot):
            items = snapshot
            if clearDraft {
                draftBody = ""
                isDraftFocused = false
            }
        case .failure(let error):
            errorMessage = error.errorDescription ?? "Personal notes could not be synced."
        }
    }

    private func canMove(_ item: SynaraRoomNoteItem, direction: RoomNoteMoveDirection) -> Bool {
        guard item.kind == .todo else { return false }
        let group = items.filter { $0.kind == .todo && $0.isCompleted == item.isCompleted }
        guard let index = group.firstIndex(where: { $0.id == item.id }) else { return false }
        return direction == .up ? index > 0 : index < group.count - 1
    }
}

private struct RoomNoteRow: View {
    let item: SynaraRoomNoteItem
    let canMoveUp: Bool
    let canMoveDown: Bool
    let onToggle: () -> Void
    let onMove: (RoomNoteMoveDirection) -> Void
    let onEdit: () -> Void
    let onDelete: () -> Void
    let onOpenMessage: (String) -> Void

    var body: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.medium) {
            if item.kind == .todo {
                Button(action: onToggle) {
                    Image(systemName: item.isCompleted ? "checkmark.circle.fill" : "circle")
                        .font(.system(size: 22))
                        .foregroundStyle(item.isCompleted ? SynaraColor.success : SynaraColor.accent)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(item.isCompleted ? "Mark ToDo active" : "Complete ToDo")
                .accessibilityIdentifier("RoomNotesToggle-\(item.id)")
            } else {
                Image(systemName: item.kind == .message ? "text.bubble" : "note.text")
                    .font(.system(size: 19))
                    .foregroundStyle(SynaraColor.accent)
                    .frame(width: 22)
                    .accessibilityHidden(true)
            }

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                HStack(alignment: .firstTextBaseline) {
                    Text(item.kind.title)
                        .font(SynaraTypography.supporting.weight(.semibold))
                        .foregroundStyle(item.isCompleted ? SynaraColor.success : SynaraColor.secondaryText)
                    Spacer()
                    Text(item.updatedAt.formatted(date: .abbreviated, time: .shortened))
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.tertiaryText)
                }

                if let body = item.body, body.isEmpty == false {
                    Text(body)
                        .font(SynaraTypography.body)
                        .foregroundStyle(item.isCompleted ? SynaraColor.secondaryText : SynaraColor.primaryText)
                        .strikethrough(item.isCompleted)
                        .fixedSize(horizontal: false, vertical: true)
                }

                if item.kind == .message, let eventID = item.eventID {
                    HStack {
                        if let senderID = item.senderID {
                            Text(senderID)
                                .font(SynaraTypography.messageMeta)
                                .foregroundStyle(SynaraColor.secondaryText)
                                .lineLimit(1)
                        }
                        Spacer()
                        Button("Open") { onOpenMessage(eventID) }
                            .font(SynaraTypography.supporting.weight(.semibold))
                            .accessibilityIdentifier("RoomNotesOpenMessage-\(item.id)")
                    }
                }
            }
        }
        .padding(.vertical, SynaraSpacing.xSmall)
        .contextMenu {
            if item.kind == .note || item.kind == .todo {
                Button("Edit", systemImage: "pencil", action: onEdit)
            }
            if item.kind == .todo {
                Button(item.isCompleted ? "Mark Active" : "Complete", action: onToggle)
                Button("Move Up") { onMove(.up) }
                    .disabled(canMoveUp == false)
                Button("Move Down") { onMove(.down) }
                    .disabled(canMoveDown == false)
            }
            Button("Delete", systemImage: "trash", role: .destructive, action: onDelete)
        }
        .accessibilityIdentifier("RoomNotesItem-\(item.id)")
    }
}

private struct RoomNoteEditView: View {
    let item: SynaraRoomNoteItem
    let onSave: (String) async -> Bool

    @Environment(\.dismiss) private var dismiss
    @State private var bodyText: String
    @State private var isSaving = false
    @State private var showsSaveError = false
    @FocusState private var isEditorFocused: Bool

    init(item: SynaraRoomNoteItem, onSave: @escaping (String) async -> Bool) {
        self.item = item
        self.onSave = onSave
        _bodyText = State(initialValue: item.body ?? "")
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextEditor(text: $bodyText)
                        .frame(minHeight: 150)
                        .focused($isEditorFocused)
                        .accessibilityIdentifier("RoomNotesEditBody")
                } footer: {
                    Text("Private to your account and synced across Synara clients.")
                }
            }
            .navigationTitle("Edit \(item.kind.title)")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSaving)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") { save() }
                        .disabled(isSaving || trimmedBody.isEmpty)
                        .accessibilityIdentifier("RoomNotesEditSave")
                }
                ToolbarItemGroup(placement: .keyboard) {
                    Spacer()
                    Button("Done") { isEditorFocused = false }
                }
            }
            .alert("Could Not Save Note", isPresented: $showsSaveError) {
                Button("OK", role: .cancel) {}
            } message: {
                Text("Personal notes could not be synced. Try again.")
            }
        }
    }

    private var trimmedBody: String {
        bodyText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func save() {
        guard isSaving == false, trimmedBody.isEmpty == false else { return }
        isSaving = true
        Task {
            let saved = await onSave(bodyText)
            await MainActor.run {
                isSaving = false
                if saved {
                    dismiss()
                } else {
                    showsSaveError = true
                }
            }
        }
    }
}
