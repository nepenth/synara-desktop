import XCTest
@testable import Synara

final class SynaraThemeRampTests: XCTestCase {
    func testContentDepthRemainsFlatAcrossAccessibilityModes() {
        for dark in [false, true] {
            for increasedContrast in [false, true] {
                for reduceTransparency in [false, true] {
                    let metrics = SynaraDepthLevel.content.metrics(
                        dark: dark,
                        increasedContrast: increasedContrast,
                        reduceTransparency: reduceTransparency
                    )
                    XCTAssertEqual(metrics.shadowOpacity, 0)
                    XCTAssertEqual(metrics.shadowRadius, 0)
                    XCTAssertEqual(metrics.shadowOffsetY, 0)
                    XCTAssertEqual(metrics.edgeOpacity, 0)
                    XCTAssertEqual(metrics.boundaryOpacity, 0)
                    XCTAssertEqual(metrics.boundaryWidth, 0)
                }
            }
        }
    }

    func testDepthLevelsFormAQuietVisualHierarchy() {
        for dark in [false, true] {
            let avatar = SynaraDepthLevel.avatar.metrics(
                dark: dark,
                increasedContrast: false,
                reduceTransparency: false
            )
            let layered = SynaraDepthLevel.layered.metrics(
                dark: dark,
                increasedContrast: false,
                reduceTransparency: false
            )
            let raised = SynaraDepthLevel.raised.metrics(
                dark: dark,
                increasedContrast: false,
                reduceTransparency: false
            )
            let floating = SynaraDepthLevel.floating.metrics(
                dark: dark,
                increasedContrast: false,
                reduceTransparency: false
            )
            let critical = SynaraDepthLevel.critical.metrics(
                dark: dark,
                increasedContrast: false,
                reduceTransparency: false
            )

            XCTAssertGreaterThan(layered.shadowOpacity, 0)
            XCTAssertGreaterThan(layered.edgeOpacity, 0)
            XCTAssertGreaterThan(layered.boundaryWidth, 0)
            XCTAssertLessThan(layered.shadowRadius, raised.shadowRadius)
            XCTAssertLessThan(layered.shadowOpacity, raised.shadowOpacity)
            XCTAssertLessThan(avatar.shadowRadius, raised.shadowRadius)
            XCTAssertLessThanOrEqual(avatar.shadowOpacity, raised.shadowOpacity)
            XCTAssertLessThanOrEqual(avatar.edgeOpacity, 0.18)
            XCTAssertLessThanOrEqual(avatar.boundaryOpacity, raised.boundaryOpacity)
            XCTAssertLessThan(raised.shadowRadius, floating.shadowRadius)
            XCTAssertLessThan(floating.shadowRadius, critical.shadowRadius)
            XCTAssertLessThan(raised.shadowOpacity, floating.shadowOpacity)
            XCTAssertLessThan(floating.shadowOpacity, critical.shadowOpacity)
            XCTAssertLessThan(raised.shadowOffsetY, floating.shadowOffsetY)
            XCTAssertLessThan(floating.shadowOffsetY, critical.shadowOffsetY)
            XCTAssertLessThanOrEqual(raised.edgeOpacity, 0.24)
            XCTAssertLessThanOrEqual(floating.edgeOpacity, 0.24)
        }
    }

    func testLightDepthRemainsQuieterThanDarkDepth() {
        for level in SynaraDepthLevel.allCases where level != .content {
            let light = level.metrics(
                dark: false,
                increasedContrast: false,
                reduceTransparency: false
            )
            let dark = level.metrics(
                dark: true,
                increasedContrast: false,
                reduceTransparency: false
            )

            XCTAssertLessThan(light.shadowOpacity, dark.shadowOpacity)
            XCTAssertLessThanOrEqual(light.shadowRadius, dark.shadowRadius)
            XCTAssertLessThanOrEqual(light.shadowOffsetY, dark.shadowOffsetY)
        }
    }

    func testStandardTextBubbleIsLayeredByDefault() {
        let bubble = SynaraMessageBubble(
            text: "Readable message",
            alignment: .other
        )

        XCTAssertTrue(bubble.showsBackground)
        XCTAssertEqual(bubble.depth, .layered)
    }

    func testCoreSurfaceRolesRemainVisiblyDimensional() {
        XCTAssertEqual(SynaraSurfaceDepthRole.roomRow, .layered)
        XCTAssertEqual(SynaraSurfaceDepthRole.standardMessage, .layered)
        XCTAssertEqual(SynaraSurfaceDepthRole.emphasizedMessage, .raised)
        XCTAssertTrue(SynaraSurfaceDepthRole.standardMessageShowsBackground)
    }

    func testReduceTransparencyShiftsDepthFromShadowTowardBoundary() {
        let standard = SynaraDepthLevel.floating.metrics(
            dark: true,
            increasedContrast: false,
            reduceTransparency: false
        )
        let reduced = SynaraDepthLevel.floating.metrics(
            dark: true,
            increasedContrast: false,
            reduceTransparency: true
        )

        XCTAssertLessThan(reduced.shadowOpacity, standard.shadowOpacity)
        XCTAssertGreaterThan(reduced.boundaryOpacity, standard.boundaryOpacity)
        XCTAssertEqual(reduced.shadowRadius, standard.shadowRadius)
    }

    func testIncreasedContrastUsesBoundariesAndPreservesSemanticHierarchy() {
        let raised = SynaraDepthLevel.raised.metrics(
            dark: true,
            increasedContrast: true,
            reduceTransparency: false
        )
        let layered = SynaraDepthLevel.layered.metrics(
            dark: true,
            increasedContrast: true,
            reduceTransparency: false
        )
        let floating = SynaraDepthLevel.floating.metrics(
            dark: true,
            increasedContrast: true,
            reduceTransparency: false
        )
        let critical = SynaraDepthLevel.critical.metrics(
            dark: true,
            increasedContrast: true,
            reduceTransparency: false
        )

        XCTAssertEqual(layered.shadowOpacity, 0)
        XCTAssertEqual(raised.shadowOpacity, 0)
        XCTAssertEqual(floating.shadowOpacity, 0)
        XCTAssertEqual(critical.shadowOpacity, 0)
        XCTAssertLessThan(layered.boundaryWidth, raised.boundaryWidth)
        XCTAssertLessThan(raised.boundaryWidth, floating.boundaryWidth)
        XCTAssertLessThan(floating.boundaryWidth, critical.boundaryWidth)
        XCTAssertLessThan(raised.boundaryOpacity, floating.boundaryOpacity)
        XCTAssertLessThan(floating.boundaryOpacity, critical.boundaryOpacity)
    }

    func testTabBarScrollTailTracksActualVisibleOcclusion() {
        XCTAssertEqual(
            SynaraTabRootContentReachability.scrollTailHeight(
                windowBounds: CGRect(x: 0, y: 0, width: 390, height: 844),
                tabBarFrame: CGRect(x: 0, y: 760, width: 390, height: 84),
                isVisible: true
            ),
            92
        )
    }

    func testTabBarScrollTailDisappearsWithHiddenOrOffscreenBar() {
        let window = CGRect(x: 0, y: 0, width: 390, height: 844)
        XCTAssertEqual(
            SynaraTabRootContentReachability.scrollTailHeight(
                windowBounds: window,
                tabBarFrame: CGRect(x: 0, y: 760, width: 390, height: 84),
                isVisible: false
            ),
            0
        )
        XCTAssertEqual(
            SynaraTabRootContentReachability.scrollTailHeight(
                windowBounds: window,
                tabBarFrame: CGRect(x: 0, y: 900, width: 390, height: 84),
                isVisible: true
            ),
            0
        )
    }

    func testNormalizeAcceptsOnlyFullHashPrefixedHexColors() {
        XCTAssertEqual(SynaraThemeRamp.normalize("#AABBCC"), "#aabbcc")
        XCTAssertEqual(SynaraThemeRamp.normalize(" #2b2d31 "), "#2b2d31")
        XCTAssertNil(SynaraThemeRamp.normalize("#abc"))
        XCTAssertNil(SynaraThemeRamp.normalize("aabbcc"))
        XCTAssertNil(SynaraThemeRamp.normalize("red"))
        XCTAssertEqual(SynaraThemeRamp.resolve(nil), SynaraThemeRamp.defaultBaseHex)
    }

    func testChromeRolesUseDistinctStackedStops() {
        let tokens = SynaraThemeRamp.tokens(baseHex: SynaraThemeRamp.defaultBaseHex, dark: true)

        XCTAssertNotEqual(tokens.groupedSurface, tokens.secondarySurface)
        XCTAssertNotEqual(tokens.secondarySurface, tokens.surface)
        XCTAssertNotEqual(tokens.surface, tokens.elevatedSurface)
        XCTAssertLessThan(
            SynaraThemeRamp.relativeLuminance(hex: tokens.groupedSurface),
            SynaraThemeRamp.relativeLuminance(hex: tokens.secondarySurface)
        )
        XCTAssertLessThan(
            SynaraThemeRamp.relativeLuminance(hex: tokens.secondarySurface),
            SynaraThemeRamp.relativeLuminance(hex: tokens.surface)
        )
        XCTAssertGreaterThan(
            SynaraThemeRamp.contrastRatio(foreground: tokens.primaryText, background: tokens.surface),
            7
        )
    }

    func testDefaultLightRampIsARealLightTheme() {
        let tokens = SynaraThemeRamp.tokens(baseHex: SynaraThemeRamp.defaultBaseHex, dark: false)

        XCTAssertGreaterThan(SynaraThemeRamp.relativeLuminance(hex: tokens.groupedSurface), 0.88)
        XCTAssertGreaterThan(SynaraThemeRamp.relativeLuminance(hex: tokens.secondarySurface), 0.92)
        XCTAssertLessThan(
            SynaraThemeRamp.relativeLuminance(hex: tokens.groupedSurface),
            SynaraThemeRamp.relativeLuminance(hex: tokens.secondarySurface)
        )
        XCTAssertLessThan(
            SynaraThemeRamp.relativeLuminance(hex: tokens.secondarySurface),
            SynaraThemeRamp.relativeLuminance(hex: tokens.surface)
        )
        XCTAssertGreaterThan(SynaraThemeRamp.relativeLuminance(hex: tokens.surface), 0.8)
        XCTAssertGreaterThan(
            SynaraThemeRamp.contrastRatio(foreground: tokens.primaryText, background: tokens.surface),
            7
        )
        XCTAssertGreaterThan(
            SynaraThemeRamp.contrastRatio(foreground: tokens.tertiaryText, background: tokens.surface),
            4.5
        )
    }

    func testWhiteBlackAndSaturatedBasesProduceVisibleRamps() {
        let charcoal = SynaraThemeRamp.tokens(baseHex: SynaraThemeRamp.defaultBaseHex, dark: true)
        let white = SynaraThemeRamp.tokens(baseHex: "#ffffff", dark: true)
        let black = SynaraThemeRamp.tokens(baseHex: "#000000", dark: true)
        let blurple = SynaraThemeRamp.tokens(baseHex: "#5865f2", dark: true)

        XCTAssertNotEqual(white.secondarySurface, black.secondarySurface)
        XCTAssertNotEqual(white.secondarySurface, charcoal.secondarySurface)
        XCTAssertNotEqual(blurple.surface, charcoal.surface)
        XCTAssertGreaterThan(
            SynaraThemeRamp.contrastRatio(foreground: blurple.primaryText, background: blurple.surface),
            4.5
        )
    }

    func testEveryLightPresetKeepsChromeBrightAndTextLegible() {
        for preset in SynaraThemeRamp.presets {
            let tokens = SynaraThemeRamp.tokens(baseHex: preset.hex, dark: false)

            XCTAssertGreaterThan(
                SynaraThemeRamp.relativeLuminance(hex: tokens.groupedSurface),
                0.88,
                "\(preset.label) grouped chrome must not cast a gray veil"
            )
            XCTAssertGreaterThan(
                SynaraThemeRamp.relativeLuminance(hex: tokens.secondarySurface),
                0.92,
                "\(preset.label) room-list chrome must remain near white"
            )
            XCTAssertGreaterThan(
                SynaraThemeRamp.contrastRatio(
                    foreground: tokens.primaryText,
                    background: tokens.groupedSurface
                ),
                7,
                "\(preset.label) primary text must remain comfortably legible"
            )
        }
    }

    func testEveryPresetMaintainsReadableTextHierarchyInBothAppearances() {
        for preset in SynaraThemeRamp.presets {
            for dark in [false, true] {
                let tokens = SynaraThemeRamp.tokens(baseHex: preset.hex, dark: dark)
                XCTAssertGreaterThanOrEqual(
                    SynaraThemeRamp.contrastRatio(foreground: tokens.headingText, background: tokens.surface),
                    12,
                    "\(preset.label) heading contrast"
                )
                XCTAssertGreaterThanOrEqual(
                    SynaraThemeRamp.contrastRatio(foreground: tokens.primaryText, background: tokens.surface),
                    7,
                    "\(preset.label) primary contrast"
                )
                XCTAssertGreaterThanOrEqual(
                    SynaraThemeRamp.contrastRatio(foreground: tokens.secondaryText, background: tokens.surface),
                    4.5,
                    "\(preset.label) secondary contrast"
                )
                XCTAssertGreaterThanOrEqual(
                    SynaraThemeRamp.contrastRatio(foreground: tokens.tertiaryText, background: tokens.surface),
                    4.5,
                    "\(preset.label) tertiary contrast"
                )

                let increased = SynaraThemeRamp.tokens(
                    baseHex: preset.hex,
                    dark: dark,
                    increasedContrast: true
                )
                XCTAssertGreaterThan(
                    SynaraThemeRamp.contrastRatio(foreground: increased.primaryText, background: increased.surface),
                    SynaraThemeRamp.contrastRatio(foreground: tokens.primaryText, background: tokens.surface)
                )
            }
        }
    }

    func testChromeRolesReadThePassedBaseHexNotTheStoredDefault() {
        let stored = SynaraThemeRamp.colorHex(
            SynaraChrome.chatToken,
            baseHex: SynaraThemeRamp.defaultBaseHex,
            dark: true
        )
        let live = SynaraThemeRamp.colorHex(
            SynaraChrome.chatToken,
            baseHex: "#5865f2",
            dark: true
        )
        let tokens = SynaraThemeRamp.tokens(baseHex: "#5865f2", dark: true)

        XCTAssertNotEqual(stored, live)
        XCTAssertEqual(live, tokens.surface)
        XCTAssertEqual(
            SynaraThemeRamp.colorHex(SynaraChrome.roomListToken, baseHex: "#5865f2", dark: true),
            tokens.secondarySurface
        )
        XCTAssertEqual(
            SynaraThemeRamp.colorHex(SynaraChrome.composerToken, baseHex: "#5865f2", dark: true),
            tokens.elevatedSurface
        )
        XCTAssertEqual(
            SynaraThemeRamp.colorHex(SynaraChrome.settingsToken, baseHex: "#5865f2", dark: true),
            tokens.groupedSurface
        )
    }

    func testInvalidHexDoesNotClearStoredBaseColor() {
        let suiteName = "synara.theme.invalid.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        defaults.set("#5865f2", forKey: SynaraThemeRamp.storageKey)
        SynaraThemeRamp.persist("not-a-color", defaults: defaults)
        XCTAssertEqual(defaults.string(forKey: SynaraThemeRamp.storageKey), "#5865f2")

        SynaraThemeRamp.persist(nil, defaults: defaults)
        XCTAssertNil(defaults.string(forKey: SynaraThemeRamp.storageKey))
    }
}
