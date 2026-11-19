import SwiftUI

/// What the folder name sheet is for: a new folder (with the room it files,
/// from a room's "New folder…") or renaming one.
enum RoomFolderPrompt: Identifiable, Equatable {
    case create(roomID: String?)
    case rename(folderID: String, name: String)

    var id: String {
        switch self {
        case .create(let roomID):
            return "create:\(roomID ?? "")"
        case .rename(let folderID, _):
            return "rename:\(folderID)"
        }
    }
}

/// The one naming sheet for New folder and Rename (desktop's
/// `RoomFolderNameDialog`), in the app's short-entry sheet style (the New
/// room sheet): one field, Cancel and Create/Save, the save disabled while
/// the name is invalid, and a failed save says why under the field.
struct RoomFolderNameSheet: View {
    enum Result {
        case saved(RoomFoldersSnapshot)
        /// A newer client owns the format; the sheet stays open with the copy.
        case readOnly
    }

    let prompt: RoomFolderPrompt
    let service: RoomFoldersServicing
    let onResult: (Result) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var name: String
    @State private var isSaving = false
    @State private var errorMessage: String?
    @FocusState private var isFieldFocused: Bool

    init(prompt: RoomFolderPrompt, service: RoomFoldersServicing, onResult: @escaping (Result) -> Void) {
        self.prompt = prompt
        self.service = service
        self.onResult = onResult
        if case .rename(_, let name) = prompt {
            _name = State(initialValue: name)
        } else {
            _name = State(initialValue: "")
        }
    }

    private var isCreating: Bool {
        if case .create = prompt {
            return true
        }
        return false
    }

    private var isValid: Bool {
        RoomFolderName.isValid(name)
    }

    /// The length rule, once something was typed; Core's copy otherwise.
    private var shownMessage: String? {
        if let errorMessage {
            return errorMessage
        }
        if name.isEmpty == false, isValid == false {
            return RoomFolderError.message(forCode: RoomFolderError.invalidName)
        }
        return nil
    }

    private var footer: String? {
        switch prompt {
        case .create(let roomID?) where roomID.isEmpty == false:
            return String(localized: "The room moves into the new folder. Folders sync to your other devices.")
        case .create:
            return String(localized: "Folders group rooms in the list and sync to your other devices.")
        case .rename:
            return nil
        }
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField(String(localized: "Name"), text: $name, prompt: Text("Work"))
                        .focused($isFieldFocused)
                        .submitLabel(.done)
                        .onSubmit(save)
                        .disabled(isSaving)
                        .accessibilityLabel(Text("Name"))
                        .accessibilityIdentifier("RoomFolderNameField")
                } footer: {
                    if let footer {
                        Text(footer)
                    }
                }

                if let shownMessage {
                    Section {
                        Text(shownMessage)
                            .foregroundStyle(SynaraColor.criticalText)
                            .accessibilityIdentifier("RoomFolderNameError")
                    }
                }
            }
            .navigationTitle(isCreating ? String(localized: "New folder") : String(localized: "Rename folder"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                    .accessibilityIdentifier("RoomFolderNameCancelButton")
                }
                ToolbarItem(placement: .confirmationAction) {
                    if isSaving {
                        ProgressView()
                    } else {
                        Button(isCreating ? String(localized: "Create") : String(localized: "Save"), action: save)
                            .disabled(isValid == false)
                            .accessibilityIdentifier("RoomFolderNameSaveButton")
                    }
                }
            }
            .onChange(of: name) { _ in
                errorMessage = nil
            }
            .onAppear {
                isFieldFocused = true
            }
            .accessibilityIdentifier("RoomFolderNameSheet")
        }
        .presentationDetents([.medium, .large])
        .interactiveDismissDisabled(isSaving)
    }

    private func save() {
        guard isSaving == false, isValid else {
            return
        }
        if case .rename(_, let current) = prompt, RoomFolderName.normalized(name) == current {
            dismiss()
            return
        }
        isSaving = true
        errorMessage = nil
        let prompt = prompt
        let name = name
        let service = service
        Task {
            do {
                let snapshot: RoomFoldersSnapshot
                switch prompt {
                case .create(let roomID):
                    snapshot = try await service.create(name: name, roomID: roomID).snapshot
                case .rename(let folderID, _):
                    snapshot = try await service.rename(folderID: folderID, name: name)
                }
                await MainActor.run {
                    isSaving = false
                    onResult(.saved(snapshot))
                }
            } catch {
                let failure = RoomFolderError(error)
                await MainActor.run {
                    isSaving = false
                    errorMessage = failure.localizedDescription
                    isFieldFocused = true
                    if failure.isReadOnly {
                        onResult(.readOnly)
                    }
                }
            }
        }
    }
}
