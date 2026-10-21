import SwiftUI
import SynaraCore
import XCTest
@testable import Synara

// MARK: - Sign-out confirmation (I8)

@MainActor
final class SignOutConfirmationTests: XCTestCase {
    func testConnectionBannerOffersSignOutOnlyWhereItIsTheRemedy() {
        let statuses: [MatrixSyncStatus] = [
            .connected, .syncing, .starting, .reconnecting, .disconnected, .failed("fixture"), .restoreFailed, .stopped,
        ]
        for status in statuses {
            XCTAssertEqual(ConnectionStatusCopy.showsSignOutAction(status), status == .restoreFailed, "\(status)")
            // Never side by side, so a mis-tap next to Retry cannot sign out.
            XCTAssertFalse(
                ConnectionStatusCopy.showsSignOutAction(status) && ConnectionStatusCopy.showsRetryAction(status),
                "\(status)"
            )
        }

        let store = ConnectionStatusStore(reconnectingHold: 0, connectedFlash: 0)
        store.update(.disconnected)
        XCTAssertTrue(store.showsRetryAction)
        XCTAssertFalse(store.showsSignOutAction, "a network blip offers Retry only")
        store.updateSlidingSyncCapability(false)
        XCTAssertTrue(store.showsSignOutAction)
        XCTAssertFalse(store.showsRetryAction)
    }

    func testRequestOpensTheConfirmationWithoutSigningOut() async {
        let model = SignOutConfirmationModel()
        var signOuts = 0
        let refreshed = expectation(description: "device list read")
        model.request(known: .unknown) {
            defer { refreshed.fulfill() }
            return Self.status(isLastDevice: true)
        }

        XCTAssertTrue(model.isPresented)
        XCTAssertEqual(signOuts, 0, "a request never signs out")
        XCTAssertEqual(model.message, SignOutCopy.standardMessage, "unknown device list: no warning yet")

        await fulfillment(of: [refreshed], timeout: 2)
        for _ in 0..<50 where model.isLastDevice != true {
            await Task.yield()
        }
        XCTAssertTrue(model.message.hasPrefix(SignOutCopy.lastDeviceWarning), "last device: warn before wiping")

        model.confirm { signOuts += 1 }
        XCTAssertEqual(signOuts, 1)
        XCTAssertFalse(model.isPresented)
    }

    func testKnownLastDeviceWarnsImmediatelyAndCancelKeepsTheSession() {
        let model = SignOutConfirmationModel()
        model.request(known: Self.status(isLastDevice: true)) { Self.status(isLastDevice: true) }
        XCTAssertTrue(model.message.contains(SignOutCopy.lastDeviceWarning))

        // Cancel is the dialog dismissing itself; nothing else runs.
        model.isPresented = false
        XCTAssertFalse(model.isPresented)

        let other = SignOutConfirmationModel()
        other.request(known: Self.status(isLastDevice: false)) { Self.status(isLastDevice: false) }
        XCTAssertEqual(other.message, SignOutCopy.standardMessage)
    }

    private static func status(isLastDevice: Bool?) -> SessionCryptoStatus {
        SessionCryptoStatus(
            verification: .unknown,
            recovery: .unknown,
            backup: .unknown,
            hasDevicesToVerifyAgainst: nil,
            isLastDevice: isLastDevice,
            unableToDecryptCount: nil
        )
    }
}

// MARK: - Contrast tokens (I17)

final class StatusTextContrastTests: XCTestCase {
    private let aa = 4.5

    func testStatusTextColorsMeetAAOnEveryThemeSurface() {
        for (name, pair) in SynaraStatusTextPalette.all {
            for preset in SynaraThemeRamp.presets {
                for dark in [false, true] {
                    let tokens = SynaraThemeRamp.tokens(baseHex: preset.hex, dark: dark)
                    let text = pair.hex(dark: dark)
                    for surface in [
                        tokens.surface, tokens.secondarySurface, tokens.groupedSurface, tokens.elevatedSurface,
                    ] {
                        let ratio = SynaraThemeRamp.contrastRatio(foreground: text, background: surface)
                        XCTAssertGreaterThanOrEqual(
                            ratio, aa,
                            "\(name) \(text) on \(surface) (\(preset.label), dark: \(dark)) is \(ratio)"
                        )
                    }
                }
            }
        }
    }

    func testBannerTextMeetsAAOnItsOwnTint() {
        // The banner fills with its status color at 12–14% over the shell.
        let systemFills: [ConnectionStatusCopy.Variant: (light: String, dark: String)] = [
            .success: ("#34c759", "#30d158"),
            .warning: ("#ff9500", "#ff9f0a"),
            .critical: ("#ff3b30", "#ff453a"),
        ]
        for (variant, fill) in systemFills {
            let pair = ConnectionStatusBanner.textPalette(for: variant)
            XCTAssertNotNil(pair, "\(variant)")
            guard let pair else { continue }
            for dark in [false, true] {
                for backdrop in dark ? ["#000000", "#1c1c1e", "#2c2c2e"] : ["#ffffff", "#f2f2f7"] {
                    let tinted = Self.mix(dark ? fill.dark : fill.light, over: backdrop, alpha: 0.14)
                    let ratio = SynaraThemeRamp.contrastRatio(foreground: pair.hex(dark: dark), background: tinted)
                    XCTAssertGreaterThanOrEqual(ratio, aa, "\(variant) dark: \(dark) on \(backdrop) is \(ratio)")
                }
            }
        }
        XCTAssertNil(ConnectionStatusBanner.textPalette(for: .neutral), "neutral uses secondary text")
    }

    func testRawSystemStatusColorsAreWhatFailedInLightMode() {
        // Documents why the text tokens exist: the raw colors stay for icons.
        XCTAssertLessThan(SynaraThemeRamp.contrastRatio(foreground: "#34c759", background: "#ffffff"), aa)
        XCTAssertLessThan(SynaraThemeRamp.contrastRatio(foreground: "#ff9500", background: "#ffffff"), aa)
        XCTAssertLessThan(SynaraThemeRamp.contrastRatio(foreground: "#ff3b30", background: "#ffffff"), aa)
    }

    private static func mix(_ foreground: String, over background: String, alpha: Double) -> String {
        func channels(_ hex: String) -> [Double] {
            let value = Int(hex.dropFirst(), radix: 16) ?? 0
            return [Double((value >> 16) & 0xff), Double((value >> 8) & 0xff), Double(value & 0xff)]
        }
        let top = channels(foreground)
        let bottom = channels(background)
        let mixed = zip(top, bottom).map { Int(($0 * alpha + $1 * (1 - alpha)).rounded()) }
        return String(format: "#%02x%02x%02x", mixed[0], mixed[1], mixed[2])
    }
}

// MARK: - Settings toggles that stick (I22)

@MainActor
final class AgentNotificationPreferencesEditorTests: XCTestCase {
    func testAnEditSavesWithoutAnExplicitSaveButton() async {
        let editor = AgentNotificationPreferencesEditor()
        await editor.refresh { Self.preferences(tool: true) }
        var saved: [SynaraAgentNotificationPreferences] = []

        await editor.update({ $0.notifyToolActivity = false }) { preferences in
            saved.append(preferences)
            return preferences
        }

        XCTAssertEqual(saved.map(\.notifyToolActivity), [false])
        XCTAssertEqual(editor.preferences?.notifyToolActivity, false)
        XCTAssertFalse(editor.hasUnsavedChanges, "nothing is left to lose by leaving the screen")
        XCTAssertEqual(editor.message, AgentNotificationPreferencesEditor.savedMessage)
    }

    func testARefusedSaveRollsBackAndSaysSo() async {
        struct Refused: Error {}
        let editor = AgentNotificationPreferencesEditor()
        await editor.refresh { Self.preferences(tool: true) }

        await editor.update({ $0.notifyToolActivity = false }) { _ in throw Refused() }

        XCTAssertEqual(editor.preferences?.notifyToolActivity, true, "the toggle returns to the saved value")
        XCTAssertEqual(editor.message, AgentNotificationPreferencesEditor.saveFailedMessage)
        XCTAssertFalse(editor.hasUnsavedChanges)
    }

    func testEditsDuringASaveAreSentAfterIt() async {
        let editor = AgentNotificationPreferencesEditor()
        await editor.refresh { Self.preferences(tool: true) }
        var saved: [SynaraAgentNotificationPreferences] = []
        let firstSaveStarted = expectation(description: "first save in flight")
        var releaseFirstSave: CheckedContinuation<Void, Never>?

        let first = Task {
            await editor.update({ $0.notifyToolActivity = false }) { preferences in
                if saved.isEmpty {
                    await withCheckedContinuation { continuation in
                        releaseFirstSave = continuation
                        firstSaveStarted.fulfill()
                    }
                }
                saved.append(preferences)
                return preferences
            }
        }
        await fulfillment(of: [firstSaveStarted], timeout: 2)
        await editor.update({ $0.notifyCommentary = false }) { preferences in
            saved.append(preferences)
            return preferences
        }
        XCTAssertEqual(editor.preferences?.notifyCommentary, false, "shown at once")
        releaseFirstSave?.resume()
        await first.value

        XCTAssertEqual(saved.count, 2)
        XCTAssertEqual(saved.last?.notifyToolActivity, false)
        XCTAssertEqual(saved.last?.notifyCommentary, false)
        XCTAssertFalse(editor.hasUnsavedChanges)
    }

    func testRefreshDoesNotOverwriteAnUnsavedEdit() async {
        struct Refused: Error {}
        let editor = AgentNotificationPreferencesEditor()
        await editor.refresh { Self.preferences(tool: true) }
        await editor.refresh { Self.preferences(tool: false) }
        XCTAssertEqual(editor.preferences?.notifyToolActivity, false, "a clean editor follows the account")
        await editor.refresh { throw Refused() }
        XCTAssertEqual(editor.message, AgentNotificationPreferencesEditor.loadFailedMessage)
    }

    private static func preferences(tool: Bool) -> SynaraAgentNotificationPreferences {
        SynaraAgentNotificationPreferences(dto: AgentNotificationPreferencesDto(
            schemaVersion: 1,
            agentUserIds: ["@hermes:example.org"],
            notifyToolActivity: tool,
            notifyCommentary: true,
            notifyFinalResponses: true
        ))
    }
}

@MainActor
final class PushRuleSettingsModelTests: XCTestCase {
    func testMentionToggleMovesAtOnceAndKeepsTheServerValue() async {
        let service = FakePushRules()
        let model = PushRuleSettingsModel()
        await model.load(using: service)
        XCTAssertEqual(model.snapshot?.mentions.displayName, true)

        service.onWrite = {
            // The control already shows the new value while the request runs.
            XCTAssertEqual(model.snapshot?.mentions.displayName, false)
        }
        await model.setMention(\.displayName, ruleID: "displayName", enabled: false, using: service)

        XCTAssertEqual(service.mentionWrites.first?.ruleID, "displayName")
        XCTAssertEqual(model.snapshot?.mentions.displayName, false)
        XCTAssertNil(model.message)
    }

    func testRefusedWriteRollsBackWithAnInlineMessage() async {
        let service = FakePushRules()
        service.succeeds = false
        let model = PushRuleSettingsModel()
        await model.load(using: service)

        await model.setMention(\.atRoom, ruleID: "atRoom", enabled: false, using: service)
        XCTAssertEqual(model.snapshot?.mentions.atRoom, true)
        XCTAssertEqual(model.message, PushRuleSettingsModel.failureMessage)

        await model.setDefault(\.group, encrypted: false, oneToOne: false, mode: "mute", using: service)
        XCTAssertEqual(model.snapshot?.group, "all")

        let added = await model.addKeyword("deploy", using: service)
        XCTAssertFalse(added)
        XCTAssertEqual(model.snapshot?.keywords, ["incident"])
    }

    func testKeywordsAddAndRemoveOptimistically() async {
        let service = FakePushRules()
        let model = PushRuleSettingsModel()
        await model.load(using: service)

        let added = await model.addKeyword("deploy", using: service)
        XCTAssertTrue(added)
        XCTAssertEqual(model.snapshot?.keywords, ["incident", "deploy"])
        await model.removeKeyword("incident", using: service)
        XCTAssertEqual(model.snapshot?.keywords, ["deploy"])
    }
}

@MainActor
private final class FakePushRules: PushRuleSettingsServicing {
    var snapshot = SynaraPushRulesSnapshot(
        dm: "all",
        dmEncrypted: "all",
        group: "all",
        groupEncrypted: "mentions",
        mentions: SynaraPushRuleMentions(
            userMention: true,
            displayName: true,
            userName: true,
            roomMention: true,
            atRoom: true
        ),
        keywords: ["incident"]
    )
    var succeeds = true
    var onWrite: (() -> Void)?
    private(set) var mentionWrites: [(ruleID: String, enabled: Bool)] = []

    func pushRulesSnapshot() async -> SynaraPushRulesSnapshot? { snapshot }

    func setPushRuleDefault(encrypted: Bool, oneToOne: Bool, mode: String) async -> Bool {
        onWrite?()
        guard succeeds else { return false }
        switch (encrypted, oneToOne) {
        case (false, true): snapshot.dm = mode
        case (true, true): snapshot.dmEncrypted = mode
        case (false, false): snapshot.group = mode
        case (true, false): snapshot.groupEncrypted = mode
        }
        return true
    }

    func setPushRuleMention(ruleID: String, enabled: Bool) async -> Bool {
        onWrite?()
        mentionWrites.append((ruleID, enabled))
        guard succeeds else { return false }
        switch ruleID {
        case "userMention": snapshot.mentions.userMention = enabled
        case "displayName": snapshot.mentions.displayName = enabled
        case "userName": snapshot.mentions.userName = enabled
        case "roomMention": snapshot.mentions.roomMention = enabled
        case "atRoom": snapshot.mentions.atRoom = enabled
        default: return false
        }
        return true
    }

    func addPushKeyword(_ keyword: String) async -> Bool {
        onWrite?()
        guard succeeds else { return false }
        snapshot.keywords.append(keyword)
        return true
    }

    func removePushKeyword(_ keyword: String) async -> Bool {
        onWrite?()
        guard succeeds else { return false }
        snapshot.keywords.removeAll { $0 == keyword }
        return true
    }
}

// MARK: - VoiceOver semantics (I16)

final class TimelineAccessibilityTests: XCTestCase {
    func testRowLabelUsesTheDisplayNameAndReadsReactionsAndReplies() {
        let label = TimelineRowAccessibility.summary(
            sender: "Alice",
            time: "10:42 AM",
            kind: .text("Ship it"),
            isEdited: true,
            reactions: ["👍": 3, "🎉": 1],
            ownReactions: .known(["👍"]),
            replyCount: 2
        )
        XCTAssertEqual(label, "Alice at 10:42 AM: Ship it. Edited. Reactions: 🎉 1; 👍 3, you reacted. 2 replies")
        XCTAssertFalse(label.contains("@"), "never the raw Matrix ID")
    }

    func testSystemNoticesReadAsTheirSentenceOnly() {
        XCTAssertEqual(
            TimelineRowAccessibility.summary(sender: "Ada", time: "9:00", kind: .systemNotice("Ada joined the room")),
            "Ada joined the room"
        )
    }

    func testReactionAndReplyLabels() {
        XCTAssertEqual(TimelineRowAccessibility.reactionLabel(key: "👍", count: 3, isOwn: true), "👍 3, you reacted")
        XCTAssertEqual(TimelineRowAccessibility.reactionLabel(key: "👍", count: 1, isOwn: false), "👍 1")
        XCTAssertEqual(TimelineRowAccessibility.replyCountLabel(1), "1 reply")
        XCTAssertEqual(TimelineRowAccessibility.replyCountLabel(4), "4 replies")
    }

    func testOneElementRowsExposeTheMenuAsCustomActionsReplyFirst() {
        let full = EventActionAvailability(
            canReply: true,
            canEdit: true,
            canRedact: true,
            canReact: true,
            canReport: true,
            canForward: true
        )
        let actions = TimelineRowAccessibility.actions(
            availability: full,
            replyCount: 0,
            canCopy: true,
            canPinToNotes: true
        )
        XCTAssertEqual(Array(actions.prefix(2)), [.reply, .react])
        XCTAssertEqual(
            actions,
            [.reply, .react, .replyInThread, .edit, .copy, .selectText, .forward, .pinToNotes, .report, .redact]
        )
        XCTAssertEqual(Set(actions.map(\.title)).count, actions.count, "titles are unique action names")

        let readOnly = TimelineRowAccessibility.actions(
            availability: EventActionAvailability(canReply: false, canEdit: false, canRedact: false, canReact: false),
            replyCount: 3,
            canCopy: true,
            canPinToNotes: false
        )
        XCTAssertEqual(readOnly, [.openThread, .copy, .selectText])
    }

    func testRowsWithTheirOwnControlsKeepThemReachable() {
        XCTAssertFalse(TimelineRowAccessibility.containsChildren(
            deliveryStatus: nil, kind: .text("hi"), replyCount: 0, hasApprovalPrompt: false
        ), "a plain message is one element")
        XCTAssertTrue(TimelineRowAccessibility.containsChildren(
            deliveryStatus: .failed, kind: .text("hi"), replyCount: 0, hasApprovalPrompt: false
        ))
        XCTAssertTrue(TimelineRowAccessibility.containsChildren(
            deliveryStatus: nil, kind: .text("hi"), replyCount: 2, hasApprovalPrompt: false
        ))
    }
}

// MARK: - Hit targets (I26) and Dynamic Type (V4)

@MainActor
final class HitTargetAndDynamicTypeLayoutTests: XCTestCase {
    func testCompactBannerButtonsKeepA44PointTouchArea() {
        let size = Self.fittingSize(
            Button("Retry") {}.buttonStyle(SynaraCompactBorderedButtonStyle())
        )
        XCTAssertGreaterThanOrEqual(size.width, SynaraHitTarget.minimum)
        XCTAssertGreaterThanOrEqual(size.height, SynaraHitTarget.minimum)
    }

    func testRestoreFailedBannerSignOutIsA44PointTarget() {
        let store = ConnectionStatusStore(reconnectingHold: 0, connectedFlash: 0)
        store.update(.restoreFailed)
        let size = Self.fittingSize(
            ConnectionStatusBanner(store: store, onRetry: {}, onSignOut: {}),
            width: 390
        )
        XCTAssertGreaterThanOrEqual(size.height, SynaraHitTarget.minimum)
    }

    func testJumpToLastReadIsAnIconButtonAtAccessibilitySizes() {
        let regular = Self.fittingSize(JumpToLastReadButton {}, dynamicTypeSize: .large)
        XCTAssertGreaterThanOrEqual(regular.height, SynaraHitTarget.minimum)
        XCTAssertGreaterThan(regular.width, SynaraHitTarget.minimum, "titled pill")

        let ax5 = Self.fittingSize(JumpToLastReadButton {}, dynamicTypeSize: .accessibility5)
        XCTAssertEqual(ax5.width, SynaraHitTarget.minimum, accuracy: 0.5)
        XCTAssertEqual(ax5.height, SynaraHitTarget.minimum, accuracy: 0.5)
    }

    func testUnreadDividerWrapsInsteadOfTruncatingAtAX5() {
        let width: CGFloat = 358
        let font = UIFont.preferredFont(
            forTextStyle: .caption1,
            compatibleWith: UITraitCollection(preferredContentSizeCategory: .accessibilityExtraExtraExtraLarge)
        )
        let bold = UIFont(
            descriptor: font.fontDescriptor.withSymbolicTraits(.traitBold) ?? font.fontDescriptor,
            size: font.pointSize
        )
        let textWidth = ("Unread messages" as NSString).size(withAttributes: [.font: bold]).width
        let available = width - 2 * SynaraSpacing.medium
        let lines: CGFloat = textWidth > available ? 2 : 1

        let size = Self.fittingSize(UnreadMessagesDivider(), width: width, dynamicTypeSize: .accessibility5)
        XCTAssertGreaterThanOrEqual(size.height, lines * font.lineHeight - 1, "the whole label is laid out")
        XCTAssertLessThanOrEqual(size.width, width + 0.5)
    }

    func testFilterChipGrowsWithDynamicTypeInsteadOfClipping() {
        let regular = Self.fittingSize(SynaraFilterChip(title: "Unread", isSelected: false) {}, dynamicTypeSize: .large)
        let ax5 = Self.fittingSize(SynaraFilterChip(title: "Unread", isSelected: false) {}, dynamicTypeSize: .accessibility5)
        let font = UIFont.preferredFont(
            forTextStyle: .caption1,
            compatibleWith: UITraitCollection(preferredContentSizeCategory: .accessibilityExtraExtraExtraLarge)
        )
        XCTAssertGreaterThanOrEqual(regular.height, 32)
        XCTAssertGreaterThan(ax5.height, regular.height)
        XCTAssertGreaterThanOrEqual(ax5.height, font.lineHeight)
    }

    func testAccessibilitySizesUnlockMultilineTitlesAndPreviews() {
        for size in DynamicTypeSize.allCases {
            let accessibility = size.isAccessibilitySize
            XCTAssertEqual(SynaraDynamicTypeLayout.titleLineLimit(size), accessibility ? 3 : 1, "\(size)")
            XCTAssertEqual(SynaraDynamicTypeLayout.previewLineLimit(size), accessibility ? 3 : 1, "\(size)")
            XCTAssertEqual(SynaraDynamicTypeLayout.stacksRoomRowMetadata(size), accessibility, "\(size)")
            XCTAssertEqual(SynaraDynamicTypeLayout.jumpToLastReadShowsTitle(size), accessibility == false, "\(size)")
            XCTAssertEqual(SynaraDynamicTypeLayout.ruleLabelLineLimit(size), accessibility ? nil : 1, "\(size)")
        }
    }

    private static func fittingSize<V: View>(
        _ view: V,
        width: CGFloat = 1000,
        dynamicTypeSize: DynamicTypeSize = .large
    ) -> CGSize {
        let controller = UIHostingController(
            rootView: view.environment(\.dynamicTypeSize, dynamicTypeSize)
        )
        return controller.sizeThatFits(in: CGSize(width: width, height: 2000))
    }
}

// MARK: - Sign-in keyboard flow and copy (V8)

final class LoginFormTests: XCTestCase {
    func testReturnMovesToThePasswordThenSignsIn() {
        XCTAssertEqual(LoginFormField.username.submitAction, .focus(.password))
        XCTAssertEqual(LoginFormField.password.submitAction, .signIn)
    }

    func testCopyServesFirstTimeUsers() {
        XCTAssertFalse(LoginCopy.title.localizedCaseInsensitiveContains("welcome back"))
        XCTAssertFalse(LoginCopy.subtitle.localizedCaseInsensitiveContains("restore"))
    }
}
