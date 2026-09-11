import SwiftUI
import UIKit

struct RoomMemberActionPlan: Equatable {
    var canMessage: Bool
    var canIgnore: Bool
    var canInvite: Bool
    var canCancelInvite: Bool
    var canRemove: Bool
    var canBan: Bool
    var canUnban: Bool
    var canEditPowerLevel: Bool
    var assignablePowerLevels: [Int]

    static func plan(
        member: RoomMemberSummary,
        ownUserID: String?,
        powerLevels: RoomPowerLevelSummary?
    ) -> RoomMemberActionPlan {
        let isSelf = member.userID == ownUserID
        let membership = member.membership
        let ownLevel = powerLevels?.ownUserLevel ?? 0
        let higherPower = ownLevel > Int64(member.powerLevel)
        let canKick = (powerLevels?.canKick ?? false) && higherPower
        let canBan = (powerLevels?.canBan ?? false) && higherPower
        let canInvite = powerLevels?.canInvite ?? false
        let canEditPower = (powerLevels?.canEditPowerLevels ?? false) && (isSelf || higherPower)
        let maxAssignable = isSelf ? ownLevel : min(ownLevel, 100)
        let levels = [0, 50, 100].filter { Int64($0) <= maxAssignable }

        return RoomMemberActionPlan(
            canMessage: isSelf == false,
            canIgnore: isSelf == false,
            canInvite: isSelf == false && membership == "leave" && canInvite,
            canCancelInvite: isSelf == false && membership == "invite" && canKick,
            canRemove: isSelf == false && canKick && (membership == "join" || membership.isEmpty),
            canBan: isSelf == false && canBan && membership != "ban",
            canUnban: isSelf == false && membership == "ban" && (powerLevels?.canBan ?? false),
            canEditPowerLevel: isSelf == false && canEditPower && membership != "ban",
            assignablePowerLevels: canEditPower ? levels : []
        )
    }
}

struct RoomMemberActionsView: View {
    let roomID: String
    let member: RoomMemberSummary
    let ownUserID: String?
    let powerLevels: RoomPowerLevelSummary?
    let onChanged: () async -> Void

    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @State private var reason = ""
    @State private var message: String?
    @State private var isLoading = false
    @State private var isIgnored = false
    @State private var selectedPowerLevel: Int
    @State private var confirmAction: ConfirmAction?

    private var plan: RoomMemberActionPlan {
        RoomMemberActionPlan.plan(member: member, ownUserID: ownUserID, powerLevels: powerLevels)
    }

    init(
        roomID: String,
        member: RoomMemberSummary,
        ownUserID: String?,
        powerLevels: RoomPowerLevelSummary?,
        onChanged: @escaping () async -> Void
    ) {
        self.roomID = roomID
        self.member = member
        self.ownUserID = ownUserID
        self.powerLevels = powerLevels
        self.onChanged = onChanged
        _selectedPowerLevel = State(initialValue: member.powerLevel)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Text(member.title)
                        .font(SynaraTypography.emphasis)
                    Text(member.userID)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .textSelection(.enabled)
                    Text(membershipLabel)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("RoomMemberMembershipLabel")
                }

                if let message {
                    Section {
                        Text(message)
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .accessibilityIdentifier("RoomMemberActionMessage")
                    }
                }

                Section("Actions") {
                    if plan.canMessage {
                        Button("Send message") {
                            Task {
                                do {
                                    _ = try await environment.roomManagement.createDirectMessage(
                                        DirectMessageCreateRequest(userID: member.userID, isEncrypted: true)
                                    )
                                    message = "Direct message created."
                                } catch let error as RoomManagementError {
                                    message = error.localizedDescription
                                } catch {
                                    message = RoomManagementError.failed.localizedDescription
                                }
                            }
                        }
                        .accessibilityIdentifier("RoomMemberSendMessageButton")
                    }
                    Button("Copy user ID") {
                        UIPasteboard.general.string = member.userID
                        message = "Copied user ID."
                    }
                    .accessibilityIdentifier("RoomMemberCopyUserIDButton")
                    if plan.canIgnore {
                        Button(isIgnored ? "Unignore" : "Ignore", role: .destructive, action: toggleIgnore)
                            .disabled(isLoading)
                            .accessibilityIdentifier("RoomMemberIgnoreButton")
                    }
                }

                if plan.canInvite || plan.canCancelInvite || plan.canRemove || plan.canBan || plan.canUnban {
                    Section("Moderation") {
                        TextField("Reason (optional)", text: $reason, axis: .vertical)
                            .lineLimit(1 ... 3)
                            .disabled(isLoading)
                            .accessibilityIdentifier("RoomMemberReasonField")
                        if plan.canInvite {
                            Button("Invite", action: invite)
                                .disabled(isLoading)
                                .accessibilityIdentifier("RoomMemberInviteButton")
                        }
                        if plan.canCancelInvite {
                            Button("Cancel invite", role: .destructive) {
                                confirmAction = .cancelInvite
                            }
                            .disabled(isLoading)
                            .accessibilityIdentifier("RoomMemberCancelInviteButton")
                        }
                        if plan.canRemove {
                            Button("Remove from room", role: .destructive) {
                                confirmAction = .remove
                            }
                            .disabled(isLoading)
                            .accessibilityIdentifier("RoomMemberRemoveButton")
                        }
                        if plan.canBan {
                            Button("Ban from room", role: .destructive) {
                                confirmAction = .ban
                            }
                            .disabled(isLoading)
                            .accessibilityIdentifier("RoomMemberBanButton")
                        }
                        if plan.canUnban {
                            Button("Unban", action: unban)
                                .disabled(isLoading)
                                .accessibilityIdentifier("RoomMemberUnbanButton")
                        }
                    }
                }

                if plan.canEditPowerLevel, plan.assignablePowerLevels.isEmpty == false {
                    Section("Power level") {
                        Picker("Role", selection: $selectedPowerLevel) {
                            ForEach(plan.assignablePowerLevels, id: \.self) { level in
                                Text(powerLevelTitle(level)).tag(level)
                            }
                        }
                        .accessibilityIdentifier("RoomMemberPowerLevelPicker")
                        Button("Save power level", action: savePowerLevel)
                            .disabled(isLoading || selectedPowerLevel == member.powerLevel)
                            .accessibilityIdentifier("RoomMemberSavePowerLevelButton")
                    }
                }
            }
            .navigationTitle("Member")
            .accessibilityIdentifier("RoomMemberActionsScreen")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .confirmationDialog(confirmTitle, isPresented: confirmPresented) {
                Button(confirmButtonTitle, role: .destructive, action: performConfirmedAction)
                    .accessibilityIdentifier("RoomMemberConfirmActionButton")
                Button("Cancel", role: .cancel) {}
            } message: {
                Text(confirmMessage)
            }
            .task {
                let ignored = await environment.matrix.ignoredUserIDs()
                isIgnored = ignored.contains(member.userID)
            }
        }
    }

    private var membershipLabel: String {
        switch member.membership {
        case "join":
            return "Joined"
        case "invite":
            return "Invited"
        case "ban":
            return "Banned"
        case "leave":
            return "Left"
        case "knock":
            return "Knocking"
        default:
            return member.membership
        }
    }

    private enum ConfirmAction {
        case remove
        case ban
        case cancelInvite
    }

    private var confirmPresented: Binding<Bool> {
        Binding(
            get: { confirmAction != nil },
            set: { if $0 == false { confirmAction = nil } }
        )
    }

    private var confirmTitle: String {
        switch confirmAction {
        case .ban:
            return "Ban from room?"
        case .cancelInvite:
            return "Cancel invite?"
        default:
            return "Remove from room?"
        }
    }

    private var confirmButtonTitle: String {
        switch confirmAction {
        case .ban:
            return "Ban from room"
        case .cancelInvite:
            return "Cancel invite"
        default:
            return "Remove from room"
        }
    }

    private var confirmMessage: String {
        switch confirmAction {
        case .ban:
            return "They will be removed and cannot rejoin until they are unbanned."
        case .cancelInvite:
            return "This withdraws their pending invitation."
        default:
            return "They will be removed from this room and can rejoin if the room allows it."
        }
    }

    private func powerLevelTitle(_ level: Int) -> String {
        switch level {
        case 100:
            return "Admin (100)"
        case 50:
            return "Moderator (50)"
        default:
            return "Default (\(level))"
        }
    }

    private func trimmedReason() -> String? {
        let value = reason.trimmingCharacters(in: .whitespacesAndNewlines)
        return value.isEmpty ? nil : value
    }

    private func invite() {
        run("Invitation sent.") {
            try await environment.roomManagement.inviteUser(roomID: roomID, userID: member.userID)
        }
    }

    private func unban() {
        run("User unbanned.") {
            try await environment.roomManagement.unbanUser(roomID: roomID, userID: member.userID)
        }
    }

    private func savePowerLevel() {
        run("Power level updated.") {
            try await environment.roomManagement.setMemberPowerLevel(
                roomID: roomID,
                userID: member.userID,
                powerLevel: selectedPowerLevel
            )
        }
    }

    private func toggleIgnore() {
        isLoading = true
        Task {
            let ok: Bool
            if isIgnored {
                ok = await environment.matrix.unignoreUser(member.userID)
            } else {
                ok = await environment.matrix.ignoreUser(member.userID)
            }
            let ignored = await environment.matrix.ignoredUserIDs()
            await MainActor.run {
                isIgnored = ignored.contains(member.userID)
                message = ok ? (isIgnored ? "User ignored." : "User unignored.") : RoomManagementError.failed.localizedDescription
                isLoading = false
            }
        }
    }

    private func performConfirmedAction() {
        switch confirmAction {
        case .ban:
            run("User banned.") {
                try await environment.roomManagement.banUser(
                    roomID: roomID,
                    userID: member.userID,
                    reason: trimmedReason()
                )
            }
        case .cancelInvite:
            run("Invite cancelled.") {
                try await environment.roomManagement.kickUser(
                    roomID: roomID,
                    userID: member.userID,
                    reason: trimmedReason()
                )
            }
        case .remove:
            run("User removed.") {
                try await environment.roomManagement.kickUser(
                    roomID: roomID,
                    userID: member.userID,
                    reason: trimmedReason()
                )
            }
        case .none:
            break
        }
        confirmAction = nil
    }

    private func run(_ success: String, operation: @escaping () async throws -> Void) {
        isLoading = true
        Task {
            do {
                try await operation()
                await onChanged()
                await MainActor.run {
                    message = success
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
}
