import Foundation

/// The Settings pages. On iPhone a row pushes its page; on the iPad split
/// canvas the list is a sidebar and the selected page fills the detail column.
/// The root list is built from `SettingsInventory.pages`, so the list, the
/// pages and `SettingsInventoryTests` cannot drift apart.
enum SettingsPage: String, Hashable, CaseIterable, Identifiable {
    case account
    case general
    case appearance
    case notifications
    case messages
    case mediaLinks
    case privacy
    case security
    case sessions
    case advanced
    case diagnostics
    case about
    /// Pushed from About; not rows of the Settings list.
    case whatsNew
    case licenses
    case privacyPolicy
    case support

    var id: String { rawValue }
}

/// The root list's groups, in desktop's order (docs/settings-information-architecture.md).
enum SettingsGroup: String, CaseIterable {
    case app
    case chat
    case account
    case advanced

    var title: String {
        switch self {
        case .app: return String(localized: "App")
        case .chat: return String(localized: "Chat")
        case .account: return String(localized: "Account")
        case .advanced: return String(localized: "Advanced")
        }
    }
}

/// Where a page is reached from.
enum SettingsPagePlacement: Equatable {
    /// The account summary row at the top of the list, without a header.
    case accountSummary
    /// A row of the list under the group's header.
    case group(SettingsGroup)
    /// A row of another page.
    case subpage(of: SettingsPage)
}

extension SettingsPage {
    /// The page's title, on its row and in its navigation bar. Same names as
    /// desktop for the pages both have.
    var title: String {
        switch self {
        case .account: return String(localized: "Account")
        case .general: return String(localized: "General")
        case .appearance: return String(localized: "Appearance")
        case .notifications: return String(localized: "Notifications")
        case .messages: return String(localized: "Messages")
        case .mediaLinks: return String(localized: "Media & links")
        case .privacy: return String(localized: "Privacy")
        case .security: return String(localized: "Security & recovery")
        case .sessions: return String(localized: "Sessions")
        case .advanced: return String(localized: "Advanced")
        case .diagnostics: return String(localized: "Diagnostics")
        case .about: return String(localized: "About")
        case .whatsNew: return String(localized: "What's new")
        case .licenses: return String(localized: "Licenses")
        case .privacyPolicy: return String(localized: "Privacy policy")
        case .support: return String(localized: "Support")
        }
    }

    var systemImage: String {
        switch self {
        case .account: return "person.crop.circle.fill"
        case .general: return "gearshape"
        case .appearance: return "paintpalette"
        case .notifications: return "bell"
        case .messages: return "text.bubble"
        case .mediaLinks: return "play.rectangle"
        case .privacy: return "hand.raised"
        case .security: return "lock.shield"
        case .sessions: return "laptopcomputer.and.iphone"
        case .advanced: return "wrench.and.screwdriver"
        case .diagnostics: return "stethoscope"
        case .about: return "info.circle"
        case .whatsNew: return "sparkles"
        case .licenses: return "doc.text"
        case .privacyPolicy: return "doc.plaintext"
        case .support: return "questionmark.circle"
        }
    }

    /// The row's accessibility identifier. Rows that existed before the
    /// 3.3 regroup keep theirs (UI tests tap them).
    var linkIdentifier: String {
        switch self {
        case .account: return "AccountSettingsLink"
        case .general: return "GeneralSettingsLink"
        case .appearance: return "AppearanceSettingsLink"
        case .notifications: return "NotificationSettingsLink"
        case .messages: return "MessagesSettingsLink"
        case .mediaLinks: return "RichLinkSettingsLink"
        case .privacy: return "PrivacySettingsLink"
        case .security: return "SecuritySettingsLink"
        case .sessions: return "SessionsSettingsLink"
        case .advanced: return "AdvancedSettingsLink"
        case .diagnostics: return "DiagnosticsSettingsLink"
        case .about: return "AboutSettingsLink"
        case .whatsNew: return "AboutWhatsNewLink"
        case .licenses: return "LicensesSettingsLink"
        case .privacyPolicy: return "PrivacyPolicySettingsLink"
        case .support: return "SupportSettingsLink"
        }
    }

    /// The page's own accessibility identifier.
    var screenIdentifier: String {
        switch self {
        case .account: return "AccountSettingsScreen"
        case .general: return "GeneralSettingsScreen"
        case .appearance: return "AppearanceSettingsScreen"
        case .notifications: return "NotificationSettingsScreen"
        case .messages: return "MessagesSettingsScreen"
        case .mediaLinks: return "RichLinkSettingsScreen"
        case .privacy: return "PrivacySettingsScreen"
        case .security: return "SecuritySettingsScreen"
        case .sessions: return "SessionsSettingsScreen"
        case .advanced: return "AdvancedSettingsScreen"
        case .diagnostics: return "NotificationDiagnosticsScreen"
        case .about: return "AboutSettingsScreen"
        case .whatsNew: return "WhatsNewSettingsScreen"
        case .licenses: return "LicensesSettingsScreen"
        case .privacyPolicy: return "PrivacyPolicySettingsScreen"
        case .support: return "SupportSettingsScreen"
        }
    }
}

/// One control or readout on a Settings page.
struct SettingsInventoryRow: Equatable {
    /// The title the row shows.
    let title: String
    /// Its accessibility identifier. `Prefix-\(…)` identifiers are written
    /// as the prefix plus one example suffix after the last "-", or as the
    /// bare prefix ending in "-" for per-item rows.
    let identifier: String
    /// The `environment.settings` key it reads and writes on this device.
    var key: String? = nil
    /// What else it reads or writes (account, homeserver, system).
    var stores: String? = nil
    /// Where it was before the 3.3 regroup, when that differs.
    var was: String? = nil
}

struct SettingsInventorySection: Equatable {
    /// The section header as shown. Sections of a pushed sub-page are
    /// written "Sub-page › Header".
    let title: String
    let rows: [SettingsInventoryRow]
}

struct SettingsInventoryPage: Equatable {
    let page: SettingsPage
    let placement: SettingsPagePlacement
    let sections: [SettingsInventorySection]
}

/// Every iOS Settings page, section and row, in the order shown, with the
/// key each row writes. The root list is built from it, and
/// `SettingsInventoryTests` holds the page sources to it, so a page or row
/// cannot disappear or move without changing this list on purpose. Mirrors
/// desktop's `settingsInventory.ts`; see docs/settings-information-architecture.md.
enum SettingsInventory {
    typealias Row = SettingsInventoryRow
    typealias Section = SettingsInventorySection

    static let pages: [SettingsInventoryPage] = [
        SettingsInventoryPage(page: .account, placement: .accountSummary, sections: [
            Section(title: "Account", rows: [
                Row(title: "User", identifier: "SettingsAccountUser", stores: "session: Matrix user ID"),
                Row(title: "Device", identifier: "SettingsAccountDevice", stores: "session: device ID"),
                Row(title: "Homeserver", identifier: "SettingsAccountHomeserver", stores: "session: homeserver"),
            ]),
            Section(title: "Profile", rows: [
                Row(title: "Display name", identifier: "SettingsDisplayNameField",
                    stores: "homeserver profile: display name"),
                Row(title: "Save display name", identifier: "SettingsSaveDisplayNameButton",
                    stores: "homeserver profile: display name"),
            ]),
            Section(title: "Avatar", rows: [
                Row(title: "Upload avatar", identifier: "SettingsAvatarUploadButton",
                    stores: "homeserver profile: avatar"),
            ]),
            Section(title: "Contact", rows: [
                Row(title: "Email addresses", identifier: "SettingsEmail-", stores: "homeserver 3PIDs: remove"),
                Row(title: "Email", identifier: "SettingsEmailField", stores: "homeserver 3PIDs"),
                Row(title: "Add email", identifier: "SettingsAddEmailButton", stores: "homeserver 3PIDs"),
                Row(title: "Confirm email", identifier: "SettingsConfirmEmailButton",
                    stores: "homeserver 3PIDs (password step)"),
            ]),
            Section(title: "Presence", rows: [
                Row(title: "Status", identifier: "SettingsPresenceStatus", stores: "homeserver presence"),
                Row(title: "Message", identifier: "SettingsPresenceMessage", stores: "read-only: presence message"),
            ]),
            Section(title: "Status", rows: [
                Row(title: "Emoji", identifier: "SettingsUserStatusEmoji", stores: "homeserver user status"),
                Row(title: "What are you doing?", identifier: "SettingsUserStatusText",
                    stores: "homeserver user status"),
                Row(title: "Save status", identifier: "SettingsUserStatusSave", stores: "homeserver user status"),
                Row(title: "Clear status", identifier: "SettingsUserStatusClear", stores: "homeserver user status"),
            ]),
        ]),
        SettingsInventoryPage(page: .general, placement: .group(.app), sections: [
            Section(title: "Date & time", rows: [
                Row(title: "24-hour time", identifier: "AppearanceHour24Toggle",
                    key: SynaraSharedConstants.hour24ClockKey, was: "Appearance › Date & Time"),
            ]),
        ]),
        SettingsInventoryPage(page: .appearance, placement: .group(.app), sections: [
            Section(title: "Theme", rows: [
                Row(title: "Appearance", identifier: "AppearanceThemeRow", stores: "read-only: follows iOS"),
                Row(title: "Base color presets", identifier: "AppearancePreset-",
                    key: SynaraThemeRamp.storageKey),
                Row(title: "Custom", identifier: "AppearanceBaseColorPicker", key: SynaraThemeRamp.storageKey),
                Row(title: "Reset base color", identifier: "AppearanceBaseColorReset",
                    key: SynaraThemeRamp.storageKey),
            ]),
            Section(title: "Text", rows: [
                Row(title: "Text size", identifier: "AppearanceTextSizeRow", stores: "read-only: iOS Dynamic Type"),
            ]),
            Section(title: "Message display", rows: [
                Row(title: "Thread display", identifier: "AppearanceThreadDisplayPicker",
                    key: ThreadDisplayMode.settingsKey, was: "Appearance › Thread Display"),
            ]),
        ]),
        SettingsInventoryPage(page: .notifications, placement: .group(.app), sections: [
            Section(title: "Permission", rows: [
                Row(title: "Notification permission", identifier: "NotificationPermissionStatus",
                    stores: "read-only: iOS notification permission"),
                Row(title: "Enable notifications", identifier: "NotificationPermissionButton",
                    stores: "iOS notification permission, push registration"),
            ]),
            Section(title: "Direct messages", rows: [
                Row(title: "Unencrypted", identifier: "PushRuleDefault-dm", stores: "push rules: one-to-one",
                    was: "Notifications › 1-to-1 Chats"),
                Row(title: "Encrypted", identifier: "PushRuleDefault-dmEncrypted",
                    stores: "push rules: encrypted one-to-one", was: "Notifications › 1-to-1 Chats"),
            ]),
            Section(title: "Rooms", rows: [
                Row(title: "Unencrypted", identifier: "PushRuleDefault-group", stores: "push rules: group"),
                Row(title: "Encrypted", identifier: "PushRuleDefault-groupEncrypted",
                    stores: "push rules: encrypted group"),
            ]),
            Section(title: "Mentions", rows: [
                Row(title: "Mentions of your user ID", identifier: "PushRuleMention-userMention",
                    stores: "push rules: user mention"),
                Row(title: "Contains your display name", identifier: "PushRuleMention-displayName",
                    stores: "push rules: display name"),
                Row(title: "Contains your username", identifier: "PushRuleMention-userName",
                    stores: "push rules: user name"),
                Row(title: "Mentions of @room", identifier: "PushRuleMention-roomMention",
                    stores: "push rules: room mention"),
                Row(title: "Contains @room", identifier: "PushRuleMention-atRoom", stores: "push rules: @room"),
            ]),
            Section(title: "Keywords", rows: [
                Row(title: "Keywords", identifier: "NotificationKeyword-", stores: "push rules: keywords (remove)"),
                Row(title: "Keyword", identifier: "NotificationKeywordField", stores: "push rules: keywords"),
                Row(title: "Add keyword", identifier: "NotificationAddKeywordButton", stores: "push rules: keywords"),
            ]),
            Section(title: "Agent approvals", rows: [
                Row(title: "Urgent agent approvals", identifier: "TimeSensitiveAgentApprovalsToggle",
                    key: SynaraSharedConstants.timeSensitiveAgentApprovalsKey),
                Row(title: "Show approval details", identifier: "ApprovalNotificationDetailsToggle",
                    key: SynaraSharedConstants.approvalNotificationDetailsKey),
                Row(title: "Approve once without unlocking", identifier: "ApproveOnceWithoutUnlockToggle",
                    key: SynaraSharedConstants.approveOnceWithoutUnlockKey),
                Row(title: "Agent notifications", identifier: "AgentNotificationSettingsLink",
                    stores: "pushes Agent notifications"),
            ]),
            Section(title: "Agent notifications › Agent accounts", rows: [
                Row(title: "Agents", identifier: "AgentNotificationAgent-",
                    stores: "account data: agent notification preferences (remove)"),
                Row(title: "Agent Matrix user ID", identifier: "AgentNotificationUserIDField",
                    stores: "account data: agent notification preferences"),
                Row(title: "Add agent", identifier: "AgentNotificationAddButton",
                    stores: "account data: agent notification preferences"),
            ]),
            Section(title: "Agent notifications › Message categories", rows: [
                Row(title: "Notify for tool activity", identifier: "AgentNotifyToolActivityToggle",
                    stores: "account data: agent notification preferences"),
                Row(title: "Notify for commentary", identifier: "AgentNotifyCommentaryToggle",
                    stores: "account data: agent notification preferences"),
                Row(title: "Notify for final responses", identifier: "AgentNotifyFinalResponsesToggle",
                    stores: "account data: agent notification preferences"),
                Row(title: "Refresh settings", identifier: "RefreshAgentNotificationPreferencesButton",
                    stores: "account data: agent notification preferences (read)"),
            ]),
            Section(title: "Previews", rows: [
                Row(title: "Show message content in notifications", identifier: "LockScreenMessagePreviewsToggle",
                    key: SynaraSharedConstants.lockScreenMessagePreviewsKey, was: "Notifications › Privacy"),
            ]),
            Section(title: "Troubleshooting", rows: [
                Row(title: "Diagnostics", identifier: "NotificationDiagnosticsLink",
                    stores: "link to Diagnostics; shows push registration state"),
            ]),
        ]),
        SettingsInventoryPage(page: .messages, placement: .group(.chat), sections: [
            Section(title: "Drafts", rows: [
                Row(title: "Sync drafts across devices", identifier: "SyncDraftsToggle",
                    key: DraftSyncSettings.syncDraftsKey, stores: "core: draft sync, drafts key state (hint)",
                    was: "Security & Recovery"),
            ]),
        ]),
        SettingsInventoryPage(page: .mediaLinks, placement: .group(.chat), sections: [
            Section(title: "GIFs", rows: [
                Row(title: "GIF search", identifier: "RichLinkGifSearchToggle", key: RichLinkSettings.gifSearchKey,
                    was: "GIFs & Link Previews"),
                Row(title: "Play GIF links inline", identifier: "RichLinkInlineGifToggle",
                    key: RichLinkSettings.inlineGifLinksKey, was: "GIFs & Link Previews"),
            ]),
            Section(title: "Links", rows: [
                Row(title: "Link previews", identifier: "RichLinkLinkPreviewsToggle",
                    key: RichLinkSettings.linkPreviewsKey, was: "GIFs & Link Previews"),
                Row(title: "Show X posts", identifier: "RichLinkXPostsToggle", key: RichLinkSettings.xEmbedsKey,
                    was: "GIFs & Link Previews"),
                Row(title: "Shorten long links", identifier: "RichLinkShortenLinksToggle",
                    key: SynaraShortLinks.settingKey, was: "GIFs & Link Previews"),
            ]),
        ]),
        SettingsInventoryPage(page: .privacy, placement: .group(.chat), sections: [
            Section(title: "Activity", rows: [
                Row(title: "Hide typing & read receipts", identifier: "AppearanceHideActivityToggle",
                    key: SynaraSharedConstants.hideActivityKey, was: "Appearance › Privacy"),
            ]),
            Section(title: "Blocked users", rows: [
                Row(title: "Blocked users", identifier: "SettingsBlockedUser-",
                    stores: "account data: m.ignored_user_list (unblock)", was: "Account › Blocked Users"),
                Row(title: "User ID", identifier: "SettingsIgnoreUserField",
                    stores: "account data: m.ignored_user_list", was: "Account › Blocked Users"),
                Row(title: "Block user", identifier: "SettingsBlockUserButton",
                    stores: "account data: m.ignored_user_list", was: "Account › Blocked Users"),
            ]),
        ]),
        SettingsInventoryPage(page: .security, placement: .group(.account), sections: [
            Section(title: "Verification", rows: [
                Row(title: "Device verification", identifier: "SecurityDeviceVerificationRow",
                    stores: "read-only: device verification", was: "Security › Protection"),
                Row(title: "Cross-signing", identifier: "SecurityCrossSigningRow",
                    stores: "read-only: cross-signing", was: "Security › Protection"),
                Row(title: "Verify this device", identifier: "RequestDeviceVerificationButton",
                    stores: "device verification request"),
                Row(title: "Retry availability check", identifier: "RetryVerificationAvailabilityButton",
                    stores: "reads the device list"),
            ]),
            Section(title: "Recovery", rows: [
                Row(title: "Key recovery", identifier: "SecurityKeyRecoveryRow",
                    stores: "read-only: secret storage", was: "Security › Protection"),
                Row(title: "Key backup", identifier: "SecurityKeyBackupRow",
                    stores: "read-only: key backup", was: "Security › Protection"),
                Row(title: "Decryption issues", identifier: "SecurityDecryptionIssuesRow",
                    stores: "read-only: undecryptable messages", was: "Security › Protection"),
                Row(title: "Recovery key", identifier: "RecoveryKeyField", stores: "used once, never stored"),
                Row(title: "Recover keys", identifier: "RecoverKeysButton", stores: "secret storage, key backup"),
                Row(title: "Set up recovery", identifier: "SetUpRecoveryButton",
                    stores: "cross-signing, secret storage, key backup"),
            ]),
            Section(title: "Danger zone", rows: [
                Row(title: "Reset identity", identifier: "ResetIdentityButton",
                    stores: "cross-signing identity reset"),
            ]),
        ]),
        SettingsInventoryPage(page: .sessions, placement: .group(.account), sections: [
            Section(title: "Sessions", rows: [
                Row(title: "This device", identifier: "SettingsCurrentSessionRow", stores: "read-only: device list",
                    was: "Account › Sessions"),
                Row(title: "Other sessions", identifier: "SettingsSessionRow-", stores: "read-only: device list",
                    was: "Account › Sessions"),
                Row(title: "Verify", identifier: "VerifySessionButton-", stores: "device verification request",
                    was: "Account › Sessions"),
                Row(title: "Log out", identifier: "SignOutSessionButton-", stores: "homeserver: delete device",
                    was: "Account › Sessions"),
                Row(title: "Refresh sessions", identifier: "RefreshSessionsButton", stores: "reads the device list",
                    was: "Account (toolbar)"),
            ]),
            Section(title: "This session", rows: [
                Row(title: "Verify this device", identifier: "AccountVerifyThisDeviceButton",
                    stores: "device verification request", was: "Account"),
                Row(title: "Retry availability check", identifier: "AccountRetryVerificationAvailabilityButton",
                    stores: "reads the device list", was: "Account"),
            ]),
        ]),
        SettingsInventoryPage(page: .advanced, placement: .group(.advanced), sections: [
            Section(title: "Calls", rows: [
                Row(title: "Call service", identifier: "SettingsRtcTransportsDiagnostic",
                    stores: "read-only: MatrixRTC transports", was: "Account › Calls"),
                Row(title: "Refresh", identifier: "SettingsRtcTransportsRefresh",
                    stores: "MatrixRTC transports refresh", was: "Account › Calls"),
            ]),
            Section(title: "Security details", rows: [
                Row(title: "Session storage", identifier: "SecuritySessionStorageRow", stores: "read-only",
                    was: "Security › Protection"),
                Row(title: "Message security", identifier: "SecurityMatrixSDKRow", stores: "read-only",
                    was: "Security › Protection"),
            ]),
        ]),
        SettingsInventoryPage(page: .diagnostics, placement: .group(.advanced), sections: [
            Section(title: "Delivery", rows: [
                Row(title: "Status", identifier: "PushRegistrationStatusRow", stores: "read-only: push registration",
                    was: "Notifications › Diagnostics"),
                Row(title: "Register push", identifier: "PushRegistrationButton",
                    stores: "APNs and Matrix pusher registration", was: "Notifications › Diagnostics"),
            ]),
            Section(title: "Local delivery diagnostics", rows: [
                Row(title: "Delivery steps", identifier: "NotificationDiagnostic-",
                    stores: "read-only: notification diagnostics log", was: "Notifications › Diagnostics"),
                Row(title: "Refresh diagnostics", identifier: "RefreshNotificationDiagnosticsButton",
                    stores: "reads the notification diagnostics log", was: "Notifications › Diagnostics"),
                Row(title: "Copy diagnostic report", identifier: "CopyNotificationDiagnosticReportButton",
                    stores: "pasteboard", was: "Notifications › Diagnostics"),
                Row(title: "Share diagnostic report", identifier: "ShareNotificationDiagnosticReportButton",
                    stores: "share sheet", was: "Notifications › Diagnostics"),
                Row(title: "Clear diagnostics", identifier: "ClearNotificationDiagnosticsButton",
                    stores: "notification diagnostics log (clears)", was: "Notifications › Diagnostics"),
            ]),
        ]),
        SettingsInventoryPage(page: .about, placement: .group(.advanced), sections: [
            Section(title: "Build", rows: [
                Row(title: "Version", identifier: "AboutVersionRow", stores: "read-only"),
                Row(title: "Build", identifier: "AboutBuildRow", stores: "read-only"),
                Row(title: "Bundle", identifier: "AboutBundleRow", stores: "read-only"),
            ]),
            Section(title: "Links", rows: [
                Row(title: "What's new", identifier: "AboutWhatsNewLink", stores: "pushes What's new"),
                Row(title: "Licenses", identifier: "LicensesSettingsLink", stores: "pushes Licenses",
                    was: "Settings › About"),
                Row(title: "Privacy policy", identifier: "PrivacyPolicySettingsLink",
                    stores: "pushes Privacy policy", was: "Settings › About"),
                Row(title: "Support", identifier: "SupportSettingsLink", stores: "pushes Support",
                    was: "Settings › About"),
            ]),
        ]),
        SettingsInventoryPage(page: .whatsNew, placement: .subpage(of: .about), sections: [
            Section(title: "What's New in \\(version)", rows: [
                Row(title: "Full Release Notes", identifier: "WhatsNewSettingsReleaseNotesLink",
                    stores: "opens the release notes"),
            ]),
        ]),
        SettingsInventoryPage(page: .licenses, placement: .subpage(of: .about), sections: [
            Section(title: "App", rows: []),
            Section(title: "Dependencies", rows: []),
        ]),
        SettingsInventoryPage(page: .privacyPolicy, placement: .subpage(of: .about), sections: [
            Section(title: "Privacy policy", rows: [
                Row(title: "https://synara.app/privacy", identifier: "PrivacyPolicyExternalLink",
                    stores: "opens the privacy policy"),
            ]),
            Section(title: "Current data inventory", rows: []),
        ]),
        SettingsInventoryPage(page: .support, placement: .subpage(of: .about), sections: [
            Section(title: "Support", rows: [
                Row(title: "synara-support@whyland.com", identifier: "SupportExternalLink",
                    stores: "opens Mail"),
            ]),
            Section(title: "Diagnostics", rows: [
                Row(title: "Push delivery", identifier: "SupportNotificationDiagnosticsLink",
                    stores: "pushes Diagnostics"),
            ]),
        ]),
    ]

    /// Below the groups: Log out, with its confirmation.
    static let footerRows: [Row] = [
        Row(title: "Log out", identifier: "LogoutButton", stores: "local session wipe, remote logout"),
    ]

    static func page(_ page: SettingsPage) -> SettingsInventoryPage? {
        pages.first { $0.page == page }
    }

    /// The root list below the account summary: each group with its pages,
    /// in declaration order.
    static var rootGroups: [SettingsRootGroup] {
        SettingsGroup.allCases.compactMap { group in
            let grouped = pages.filter { $0.placement == .group(group) }.map(\.page)
            return grouped.isEmpty ? nil : SettingsRootGroup(group: group, pages: grouped)
        }
    }
}

/// One headed section of the root Settings list.
struct SettingsRootGroup: Identifiable, Equatable {
    let group: SettingsGroup
    let pages: [SettingsPage]

    var id: SettingsGroup { group }
}
