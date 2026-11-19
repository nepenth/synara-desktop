import Foundation
import XCTest
@testable import Synara

/// Holds the Settings pages to `SettingsInventory`, as desktop's
/// `settingsInventory.test.ts` does: the root list comes from the inventory,
/// every declared row is still in the page sources, and the pinned lists
/// below fail when a page, row or stored key disappears or changes. Change
/// them only on purpose; see docs/settings-information-architecture.md.
final class SettingsInventoryTests: XCTestCase {
    // MARK: Pinned layout

    func testRootListIsGroupedLikeDesktop() {
        let root = SettingsInventory.rootGroups.map { group in
            "\(group.group.title): " + group.pages.map(\.title).joined(separator: " · ")
        }
        XCTAssertEqual(root, [
            "App: General · Appearance · Notifications",
            "Chat: Messages · Media & links · Privacy",
            "Account: Security & recovery · Sessions",
            "Advanced: Advanced · Diagnostics · About",
        ])
        XCTAssertEqual(SettingsInventory.page(.account)?.placement, .accountSummary)
        XCTAssertEqual(SettingsInventory.footerRows.map(\.identifier), ["LogoutButton"])
    }

    func testEveryPageIsDeclaredOnceAndReachable() {
        XCTAssertEqual(SettingsInventory.pages.map(\.page), SettingsPage.allCases)
        for page in SettingsInventory.pages {
            if case .subpage(let parent) = page.placement {
                let parentRows = SettingsInventory.page(parent)?.sections.flatMap(\.rows).map(\.identifier) ?? []
                XCTAssertTrue(
                    parentRows.contains(page.page.linkIdentifier),
                    "\(page.page) is a sub-page of \(parent) but \(parent) has no row for it"
                )
            }
        }
        XCTAssertEqual(Set(SettingsPage.allCases.map(\.linkIdentifier)).count, SettingsPage.allCases.count)
        XCTAssertEqual(Set(SettingsPage.allCases.map(\.screenIdentifier)).count, SettingsPage.allCases.count)
    }

    /// Every section and row, by page. A row that moves or disappears fails
    /// here first.
    func testPinnedSectionsAndRows() {
        let layout = SettingsInventory.pages.map { page in
            "\(page.page.rawValue): " + page.sections.map { section in
                "\(section.title) [" + section.rows.map(\.identifier).joined(separator: ", ") + "]"
            }.joined(separator: " | ")
        }
        XCTAssertEqual(layout, [
            "account: Account [SettingsAccountUser, SettingsAccountDevice, SettingsAccountHomeserver]"
                + " | Profile [SettingsDisplayNameField, SettingsSaveDisplayNameButton]"
                + " | Avatar [SettingsAvatarUploadButton]"
                + " | Contact [SettingsEmail-, SettingsEmailField, SettingsAddEmailButton, SettingsConfirmEmailButton]"
                + " | Presence [SettingsPresenceStatus, SettingsPresenceMessage]"
                + " | Status [SettingsUserStatusEmoji, SettingsUserStatusText, SettingsUserStatusSave, SettingsUserStatusClear]",
            "general: Date & time [AppearanceHour24Toggle]",
            "appearance: Theme [AppearanceThemeRow, AppearancePreset-, AppearanceBaseColorPicker, AppearanceBaseColorReset]"
                + " | Text [AppearanceTextSizeRow]"
                + " | Message display [AppearanceThreadDisplayPicker]",
            "notifications: Permission [NotificationPermissionStatus, NotificationPermissionButton]"
                + " | Direct messages [PushRuleDefault-dm, PushRuleDefault-dmEncrypted]"
                + " | Rooms [PushRuleDefault-group, PushRuleDefault-groupEncrypted]"
                + " | Mentions [PushRuleMention-userMention, PushRuleMention-displayName, PushRuleMention-userName,"
                + " PushRuleMention-roomMention, PushRuleMention-atRoom]"
                + " | Keywords [NotificationKeyword-, NotificationKeywordField, NotificationAddKeywordButton]"
                + " | Agent approvals [TimeSensitiveAgentApprovalsToggle, ApprovalNotificationDetailsToggle,"
                + " ApproveOnceWithoutUnlockToggle, AgentNotificationSettingsLink]"
                + " | Agent notifications › Agent accounts [AgentNotificationAgent-, AgentNotificationUserIDField,"
                + " AgentNotificationAddButton]"
                + " | Agent notifications › Message categories [AgentNotifyToolActivityToggle,"
                + " AgentNotifyCommentaryToggle, AgentNotifyFinalResponsesToggle, RefreshAgentNotificationPreferencesButton]"
                + " | Previews [LockScreenMessagePreviewsToggle]"
                + " | Troubleshooting [NotificationDiagnosticsLink]",
            "messages: Drafts [SyncDraftsToggle]",
            "mediaLinks: GIFs [RichLinkGifSearchToggle, RichLinkInlineGifToggle]"
                + " | Links [RichLinkLinkPreviewsToggle, RichLinkXPostsToggle, RichLinkShortenLinksToggle]",
            "privacy: Activity [AppearanceHideActivityToggle]"
                + " | Blocked users [SettingsBlockedUser-, SettingsIgnoreUserField, SettingsBlockUserButton]",
            "security: Verification [SecurityDeviceVerificationRow, SecurityCrossSigningRow,"
                + " RequestDeviceVerificationButton, RetryVerificationAvailabilityButton]"
                + " | Recovery [SecurityKeyRecoveryRow, SecurityKeyBackupRow, SecurityDecryptionIssuesRow,"
                + " RecoveryKeyField, RecoverKeysButton, SetUpRecoveryButton]"
                + " | Danger zone [ResetIdentityButton]",
            "sessions: Sessions [SettingsCurrentSessionRow, SettingsSessionRow-, VerifySessionButton-,"
                + " SignOutSessionButton-, RefreshSessionsButton]"
                + " | This session [AccountVerifyThisDeviceButton, AccountRetryVerificationAvailabilityButton]",
            "advanced: Calls [SettingsRtcTransportsDiagnostic, SettingsRtcTransportsRefresh]"
                + " | Security details [SecuritySessionStorageRow, SecurityMatrixSDKRow]",
            "diagnostics: Delivery [PushRegistrationStatusRow, PushRegistrationButton]"
                + " | Local delivery diagnostics [NotificationDiagnostic-, RefreshNotificationDiagnosticsButton,"
                + " CopyNotificationDiagnosticReportButton, ShareNotificationDiagnosticReportButton,"
                + " ClearNotificationDiagnosticsButton]",
            "about: Build [AboutVersionRow, AboutBuildRow, AboutBundleRow]"
                + " | Links [AboutWhatsNewLink, LicensesSettingsLink, PrivacyPolicySettingsLink, SupportSettingsLink]",
            "whatsNew: What's New in \\(version) [WhatsNewSettingsReleaseNotesLink]",
            "licenses: App [] | Dependencies []",
            "privacyPolicy: Privacy policy [PrivacyPolicyExternalLink] | Current data inventory []",
            "support: Support [SupportExternalLink] | Diagnostics [SupportNotificationDiagnosticsLink]",
        ])
    }

    /// The device-local keys behind the rows. The regroup moved rows, never
    /// their keys; a changed key would silently reset a user's setting.
    func testStoredKeysAreUnchanged() {
        var keys: [String: String] = [:]
        for row in SettingsInventory.pages.flatMap(\.sections).flatMap(\.rows) {
            guard let key = row.key else { continue }
            keys[row.identifier] = key
        }
        XCTAssertEqual(keys, [
            "AppearanceHour24Toggle": "synara.settings.hour24Clock",
            "AppearancePreset-": "themeBaseColor",
            "AppearanceBaseColorPicker": "themeBaseColor",
            "AppearanceBaseColorReset": "themeBaseColor",
            "AppearanceThreadDisplayPicker": "synara.settings.threadDisplay",
            "TimeSensitiveAgentApprovalsToggle": "synara.settings.timeSensitiveAgentApprovals",
            "ApprovalNotificationDetailsToggle": "synara.settings.showApprovalNotificationDetails",
            "ApproveOnceWithoutUnlockToggle": "synara.settings.approveOnceWithoutUnlock",
            "LockScreenMessagePreviewsToggle": "synara.settings.lockScreenMessagePreviews",
            "SyncDraftsToggle": "synara.settings.syncDrafts",
            "RichLinkGifSearchToggle": "synara.settings.gifSearch",
            "RichLinkInlineGifToggle": "synara.settings.inlineGifLinks",
            "RichLinkLinkPreviewsToggle": "synara.settings.linkPreviews",
            "RichLinkXPostsToggle": "synara.settings.xEmbeds",
            "RichLinkShortenLinksToggle": "synara.settings.shortenLinks",
            "AppearanceHideActivityToggle": "synara.settings.hideActivity",
        ])
    }

    // MARK: Sources held to the inventory

    /// Each declared row's identifier, each section header and each page's
    /// row and screen identifiers are in the Settings sources, so a row
    /// cannot be deleted from a page while the inventory still lists it.
    func testEveryDeclaredRowIsInThePageSources() throws {
        let source = try settingsSources()
        for page in SettingsInventory.pages {
            for section in page.sections {
                let header = section.title.components(separatedBy: " › ").last ?? section.title
                XCTAssertTrue(source.contains("\"\(header)\""), "\(page.page): section header \"\(header)\"")
                for row in section.rows {
                    // A row that opens another page is a `SettingsPageLink`.
                    if let linked = SettingsPage.allCases.first(where: { $0.linkIdentifier == row.identifier }) {
                        XCTAssertTrue(
                            source.contains("SettingsPageLink(page: .\(linked.rawValue))"),
                            "\(page.page) › \(section.title): no link row to \(linked)"
                        )
                        continue
                    }
                    XCTAssertTrue(
                        Self.source(source, contains: row.identifier),
                        "\(page.page) › \(section.title): no row with identifier \(row.identifier)"
                    )
                }
            }
        }
        XCTAssertTrue(source.contains("\"LogoutButton\""))
        // Rows and screens get their identifiers from `SettingsPage`.
        XCTAssertTrue(source.contains(".accessibilityIdentifier(page.linkIdentifier)"))
        for page in SettingsPage.allCases {
            XCTAssertTrue(
                source.contains("SettingsPage.\(page.rawValue).screenIdentifier")
                    || source.contains("\"\(page.screenIdentifier)\""),
                "\(page) page has no screen identifier"
            )
        }
    }

    /// Each stored key is written by the page that declares it.
    func testEachKeyIsWrittenOnItsPage() throws {
        let source = try settingsSources()
        let writers: [String: String] = [
            SynaraSharedConstants.hour24ClockKey: "SynaraSharedConstants.hour24ClockKey",
            SynaraThemeRamp.storageKey: "SynaraThemeRamp.storageKey",
            ThreadDisplayMode.settingsKey: "ThreadDisplayMode.settingsKey",
            SynaraSharedConstants.timeSensitiveAgentApprovalsKey: "SynaraSharedConstants.timeSensitiveAgentApprovalsKey",
            SynaraSharedConstants.approvalNotificationDetailsKey: "SynaraSharedConstants.approvalNotificationDetailsKey",
            SynaraSharedConstants.approveOnceWithoutUnlockKey: "SynaraSharedConstants.approveOnceWithoutUnlockKey",
            SynaraSharedConstants.lockScreenMessagePreviewsKey: "SynaraSharedConstants.lockScreenMessagePreviewsKey",
            DraftSyncSettings.syncDraftsKey: "DraftSyncSettings.syncDraftsKey",
            RichLinkSettings.gifSearchKey: "RichLinkSettings.gifSearchKey",
            RichLinkSettings.inlineGifLinksKey: "RichLinkSettings.inlineGifLinksKey",
            RichLinkSettings.linkPreviewsKey: "RichLinkSettings.linkPreviewsKey",
            RichLinkSettings.xEmbedsKey: "RichLinkSettings.xEmbedsKey",
            SynaraShortLinks.settingKey: "SynaraShortLinks.settingKey",
            SynaraSharedConstants.hideActivityKey: "SynaraSharedConstants.hideActivityKey",
        ]
        for page in SettingsInventory.pages {
            let keys = Set(page.sections.flatMap(\.rows).compactMap(\.key))
            guard keys.isEmpty == false else { continue }
            let body = try XCTUnwrap(pageSource(page.page, in: source), "\(page.page): page view not found")
            for key in keys {
                let symbol = try XCTUnwrap(writers[key], "no writer symbol for \(key)")
                XCTAssertTrue(body.contains("for: \(symbol)"), "\(page.page) does not write \(symbol)")
            }
        }
    }

    // MARK: Helpers

    private static let pageViews: [SettingsPage: String] = [
        .general: "GeneralSettingsView",
        .appearance: "AppearanceSettingsView",
        .notifications: "NotificationSettingsView",
        .messages: "MessagesSettingsView",
        .mediaLinks: "RichLinkSettingsView",
        .privacy: "PrivacySettingsView",
    ]

    /// The source of one page's view struct, up to the next top-level type.
    private func pageSource(_ page: SettingsPage, in source: String) -> String? {
        guard let name = Self.pageViews[page],
              let start = source.range(of: "private struct \(name): View {")
        else { return nil }
        let rest = source[start.upperBound...]
        let end = rest.range(of: "\nprivate struct ")?.lowerBound
            ?? rest.range(of: "\nstruct ")?.lowerBound
            ?? rest.endIndex
        return String(rest[..<end])
    }

    /// `Prefix-` matches an interpolated `"Prefix-\(…)"`; `Prefix-suffix`
    /// matches the literal or a `"Prefix-\(…)"` fed with `"suffix"`.
    private static func source(_ source: String, contains identifier: String) -> Bool {
        if identifier.hasSuffix("-") {
            return source.contains("\"\(identifier)\\(")
        }
        if source.contains("\"\(identifier)\"") {
            return true
        }
        guard let dash = identifier.lastIndex(of: "-") else { return false }
        let prefix = identifier[...dash]
        let suffix = identifier[identifier.index(after: dash)...]
        return source.contains("\"\(prefix)\\(") && source.contains("\"\(suffix)\"")
    }

    private func settingsSources() throws -> String {
        let root = URL(fileURLWithPath: repositoryRoot())
        return try [
            "synara-ios/Synara/Features/SettingsView.swift",
            "synara-ios/Synara/Features/AgentNotificationSettingsView.swift",
            "synara-ios/Synara/Features/WhatsNewCard.swift",
        ]
        .map { try String(contentsOf: root.appendingPathComponent($0), encoding: .utf8) }
        .joined(separator: "\n")
    }

    private func repositoryRoot() -> String {
        var url = URL(fileURLWithPath: #filePath)
        while url.pathComponents.count > 1 {
            url.deleteLastPathComponent()
            if FileManager.default.fileExists(atPath: url.appendingPathComponent("synara-ios").path) {
                return url.path
            }
        }
        return url.path
    }
}
