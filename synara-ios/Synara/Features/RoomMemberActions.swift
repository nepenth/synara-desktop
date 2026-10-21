import SwiftUI
import SynaraCore
import UIKit

struct RoomMemberActionPlan: Equatable {
    var canMessage: Bool
    var canIgnore: Bool
    var canInvite: Bool
    var canCancelInvite: Bool
    var canAcceptKnock: Bool
    var canDenyKnock: Bool
    var canRemove: Bool
    var canBan: Bool
    var canUnban: Bool
    var canEditPowerLevel: Bool
    var assignablePowerLevels: [Int]

    /// Core owns the member-menu rules (`app::members::plan_member_actions`).
    static func plan(
        member: RoomMemberSummary,
        ownUserID: String?,
        powerLevels: RoomPowerLevelSummary?
    ) -> RoomMemberActionPlan {
        let core = planMemberActions(context: MemberActionContext(
            isSelf: member.userID == ownUserID,
            memberMembership: member.membership.wireLabel,
            memberPowerLevel: Int64(member.powerLevel),
            ownPowerLevel: powerLevels?.isCreator == true ? nil : (powerLevels?.ownUserLevel ?? 0),
            canInvite: powerLevels?.canInvite ?? false,
            canKick: powerLevels?.canKick ?? false,
            canBan: powerLevels?.canBan ?? false,
            canChangePowerLevels: powerLevels?.canEditPowerLevels ?? false
        ))
        return RoomMemberActionPlan(
            canMessage: core.canMessage,
            canIgnore: core.canIgnore,
            canInvite: core.canInvite,
            canCancelInvite: core.canCancelInvite,
            canAcceptKnock: core.canAcceptKnock,
            canDenyKnock: core.canDenyKnock,
            canRemove: core.canRemove,
            canBan: core.canBan,
            canUnban: core.canUnban,
            canEditPowerLevel: core.canEditPowerLevel,
            assignablePowerLevels: core.assignablePowerLevels.map { Int($0) }
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
                                    message = String(localized: "Direct message created.")
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
                        message = String(localized: "Copied user ID.")
                    }
                    .accessibilityIdentifier("RoomMemberCopyUserIDButton")
                    if plan.canIgnore {
                        Button(isIgnored ? "Unignore" : "Ignore", role: .destructive, action: toggleIgnore)
                            .disabled(isLoading)
                            .accessibilityIdentifier("RoomMemberIgnoreButton")
                    }
                }

                if plan.canInvite || plan.canCancelInvite || plan.canAcceptKnock || plan.canDenyKnock || plan.canRemove || plan.canBan || plan.canUnban {
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
                        if plan.canAcceptKnock {
                            Button("Accept knock", action: invite)
                                .disabled(isLoading)
                                .accessibilityIdentifier("RoomMemberAcceptKnockButton")
                        }
                        if plan.canCancelInvite {
                            Button("Cancel invite", role: .destructive) {
                                confirmAction = .cancelInvite
                            }
                            .disabled(isLoading)
                            .accessibilityIdentifier("RoomMemberCancelInviteButton")
                        }
                        if plan.canDenyKnock {
                            Button("Deny knock", role: .destructive) {
                                confirmAction = .denyKnock
                            }
                            .disabled(isLoading)
                            .accessibilityIdentifier("RoomMemberDenyKnockButton")
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
            .alert(confirmTitle, isPresented: confirmPresented) {
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
        member.membership.displayName
    }

    private enum ConfirmAction {
        case remove
        case ban
        case cancelInvite
        case denyKnock
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
            return String(localized: "Ban from room?")
        case .cancelInvite:
            return String(localized: "Cancel invite?")
        case .denyKnock:
            return String(localized: "Deny knock?")
        default:
            return String(localized: "Remove from room?")
        }
    }

    private var confirmButtonTitle: String {
        switch confirmAction {
        case .ban:
            return String(localized: "Ban from room")
        case .cancelInvite:
            return String(localized: "Cancel invite")
        case .denyKnock:
            return String(localized: "Deny knock")
        default:
            return String(localized: "Remove from room")
        }
    }

    private var confirmMessage: String {
        switch confirmAction {
        case .ban:
            return String(localized: "They will be removed and cannot rejoin until they are unbanned.")
        case .cancelInvite:
            return String(localized: "This withdraws their pending invitation.")
        case .denyKnock:
            return String(localized: "This rejects their request to join this room.")
        default:
            return String(localized: "They will be removed from this room and can rejoin if the room allows it.")
        }
    }

    private func powerLevelTitle(_ level: Int) -> String {
        switch level {
        case 100:
            return String(localized: "Admin (100)")
        case 50:
            return String(localized: "Moderator (50)")
        default:
            return String(localized: "Default (\(level))")
        }
    }

    private func trimmedReason() -> String? {
        let value = reason.trimmingCharacters(in: .whitespacesAndNewlines)
        return value.isEmpty ? nil : value
    }

    private func invite() {
        run(plan.canAcceptKnock ? String(localized: "Knock accepted.") : String(localized: "Invitation sent.")) {
            try await environment.roomManagement.inviteUser(roomID: roomID, userID: member.userID)
        }
    }

    private func unban() {
        run(String(localized: "User unbanned.")) {
            try await environment.roomManagement.unbanUser(roomID: roomID, userID: member.userID)
        }
    }

    private func savePowerLevel() {
        run(String(localized: "Power level updated.")) {
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
                message = ok
                    ? (isIgnored ? String(localized: "User ignored.") : String(localized: "User unignored."))
                    : RoomManagementError.failed.localizedDescription
                isLoading = false
            }
        }
    }

    private func performConfirmedAction() {
        switch confirmAction {
        case .ban:
            run(String(localized: "User banned.")) {
                try await environment.roomManagement.banUser(
                    roomID: roomID,
                    userID: member.userID,
                    reason: trimmedReason()
                )
            }
        case .cancelInvite:
            run(String(localized: "Invite cancelled.")) {
                try await environment.roomManagement.kickUser(
                    roomID: roomID,
                    userID: member.userID,
                    reason: trimmedReason()
                )
            }
        case .denyKnock:
            run(String(localized: "Knock denied.")) {
                try await environment.roomManagement.kickUser(
                    roomID: roomID,
                    userID: member.userID,
                    reason: trimmedReason()
                )
            }
        case .remove:
            run(String(localized: "User removed.")) {
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
