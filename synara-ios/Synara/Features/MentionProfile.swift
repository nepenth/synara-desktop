import SwiftUI

/// A user a mention points at, shown as the member sheet the room's member
/// list opens.
struct MentionProfileTarget: Identifiable, Equatable {
    let userID: String
    var id: String { userID }
}

private struct SynaraOpenUserProfileKey: EnvironmentKey {
    static let defaultValue: ((String) -> Void)? = nil
}

extension EnvironmentValues {
    /// Opens a user's profile inside Synara. Set by the room and thread
    /// screens; message text calls it for mention links instead of handing
    /// matrix.to to Safari.
    var synaraOpenUserProfile: ((String) -> Void)? {
        get { self[SynaraOpenUserProfileKey.self] }
        set { self[SynaraOpenUserProfileKey.self] = newValue }
    }
}

extension View {
    /// Mentions in message text open `target` in the app; other links keep
    /// their default handling.
    func synaraMentionProfiles(
        _ target: Binding<MentionProfileTarget?>,
        roomID: String,
        ownUserID: String
    ) -> some View {
        environment(\.synaraOpenUserProfile) { userID in
            target.wrappedValue = MentionProfileTarget(userID: userID)
        }
        .environment(\.openURL, OpenURLAction { url in
            guard let userID = TimelineMentionPolicy.userID(inPermalink: url) else {
                return .systemAction
            }
            target.wrappedValue = MentionProfileTarget(userID: userID)
            return .handled
        })
        .sheet(item: target) { profile in
            MentionProfileSheet(roomID: roomID, userID: profile.userID, ownUserID: ownUserID)
        }
    }
}

/// The member sheet for a mentioned user, or a short note when they are not
/// in this room.
struct MentionProfileSheet: View {
    let roomID: String
    let userID: String
    let ownUserID: String

    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @State private var details: RoomDetails?
    @State private var isLoaded = false

    var body: some View {
        Group {
            if let details, let member = details.members.first(where: { $0.userID == userID }) {
                RoomMemberActionsView(
                    roomID: roomID,
                    member: member,
                    ownUserID: ownUserID,
                    powerLevels: details.powerLevels,
                    onChanged: { await load() }
                )
            } else if isLoaded {
                NavigationStack {
                    VStack(spacing: SynaraSpacing.medium) {
                        Image(systemName: "person.crop.circle")
                            .font(.system(size: 44))
                            .foregroundStyle(SynaraColor.secondaryText)
                            .accessibilityHidden(true)
                        Text(userID)
                            .font(SynaraTypography.emphasis)
                            .foregroundStyle(SynaraColor.primaryText)
                            .textSelection(.enabled)
                        Text("Not a member of this room.")
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                    .padding()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .toolbar {
                        ToolbarItem(placement: .confirmationAction) {
                            Button("Done") { dismiss() }
                        }
                    }
                }
                .accessibilityIdentifier("MentionProfileNotMember")
            } else {
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .task(id: userID) { await load() }
    }

    private func load() async {
        details = await environment.roomManagement.roomDetails(roomID: roomID)
        isLoaded = true
    }
}
