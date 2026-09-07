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
        for level in SynaraDepthLevel.allCases where level != .content && level != .collection {
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

    func testCollectionRowsStayOnTheReadingPlane() {
        for dark in [false, true] {
            for increasedContrast in [false, true] {
                let row = SynaraSurfaceDepthRole.roomRow.metrics(
                    dark: dark,
                    increasedContrast: increasedContrast,
                    reduceTransparency: false
                )
                XCTAssertEqual(row.shadowOpacity, 0)
                XCTAssertEqual(row.shadowRadius, 0)
                XCTAssertGreaterThan(row.boundaryWidth, 0)
                if increasedContrast {
                    XCTAssertEqual(row.edgeOpacity, 0)
                    XCTAssertGreaterThanOrEqual(row.boundaryOpacity, 0.78)
                } else {
                    XCTAssertLessThanOrEqual(row.edgeOpacity, 0.035)
                    XCTAssertLessThanOrEqual(row.boundaryOpacity, 0.20)
                }
            }
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

    func testOwnAndOtherGroupedBubblesShareTheLeadingContentColumn() {
        XCTAssertEqual(
            SynaraMessageBubbleMetrics.cornerRadii(alignment: .own, isGrouped: true),
            SynaraMessageBubbleMetrics.cornerRadii(alignment: .other, isGrouped: true)
        )
        XCTAssertTrue(SynaraMessageBubbleMetrics.usesLeadingContentColumn(alignment: .own))
        XCTAssertTrue(SynaraMessageBubbleMetrics.usesLeadingContentColumn(alignment: .other))
        XCTAssertTrue(SynaraMessageBubbleMetrics.usesLeadingStatusOverlay(alignment: .own))
        XCTAssertTrue(SynaraMessageBubbleMetrics.usesLeadingStatusOverlay(alignment: .other))
    }

    func testCoreSurfaceRolesRemainVisiblyDimensional() {
        XCTAssertEqual(SynaraSurfaceDepthRole.roomRow, .collection)
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
            96
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

    func testTabBarScrollTailRejectsInvalidTransitionGeometry() {
        let window = CGRect(x: 0, y: 0, width: 390, height: 844)

        XCTAssertEqual(
            SynaraTabRootContentReachability.scrollTailHeight(
                windowBounds: window,
                tabBarFrame: CGRect(x: 0, y: CGFloat.nan, width: 390, height: 84),
                isVisible: true
            ),
            0
        )
        XCTAssertEqual(
            SynaraTabRootContentReachability.scrollTailHeight(
                windowBounds: window,
                tabBarFrame: CGRect(x: 0, y: 760, width: CGFloat.infinity, height: 84),
                isVisible: true
            ),
            0
        )
        XCTAssertEqual(
            SynaraTabRootContentReachability.scrollTailHeight(
                windowBounds: CGRect(x: 0, y: 0, width: 0, height: 844),
                tabBarFrame: CGRect(x: 0, y: 760, width: 390, height: 84),
                isVisible: true
            ),
            0
        )
        XCTAssertEqual(SynaraTabRootContentReachability.sanitizedScrollTailHeight(CGFloat.nan), 0)
        XCTAssertEqual(SynaraTabRootContentReachability.sanitizedScrollTailHeight(CGFloat.infinity), 0)
        XCTAssertEqual(SynaraTabRootContentReachability.sanitizedScrollTailHeight(-12), 0)
        XCTAssertEqual(SynaraTabRootContentReachability.sanitizedScrollTailHeight(92), 92)
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

    func testEveryPresetHasDistinctReadableRichTextRoles() {
        for preset in SynaraThemeRamp.presets {
            for dark in [false, true] {
                for increasedContrast in [false, true] {
                    let tokens = SynaraThemeRamp.tokens(
                        baseHex: preset.hex,
                        dark: dark,
                        increasedContrast: increasedContrast
                    )
                    XCTAssertNotEqual(
                        tokens.richTextInlineCodeBackground,
                        tokens.surface,
                        "\(preset.label) inline code must not collapse into the message canvas"
                    )
                    XCTAssertNotEqual(tokens.richTextCodeBlockBackground, tokens.surface)
                    XCTAssertNotEqual(tokens.richTextSpoilerBackground, tokens.surface)
                    XCTAssertNotEqual(tokens.richTextTableHeader, tokens.richTextTableOdd)
                    XCTAssertNotEqual(tokens.richTextTableOdd, tokens.richTextTableEven)
                    XCTAssertGreaterThanOrEqual(
                        SynaraThemeRamp.contrastRatio(
                            foreground: tokens.richTextInlineCodeForeground,
                            background: tokens.richTextInlineCodeBackground
                        ),
                        increasedContrast ? 7 : 4.5,
                        "\(preset.label) inline code text contrast"
                    )
                    XCTAssertGreaterThanOrEqual(
                        SynaraThemeRamp.contrastRatio(
                            foreground: tokens.primaryText,
                            background: tokens.richTextCodeBlockBackground
                        ),
                        increasedContrast ? 7 : 4.5,
                        "\(preset.label) code block text contrast"
                    )
                    XCTAssertGreaterThanOrEqual(
                        SynaraThemeRamp.contrastRatio(
                            foreground: tokens.primaryText,
                            background: tokens.richTextSpoilerBackground
                        ),
                        increasedContrast ? 7 : 4.5,
                        "\(preset.label) spoiler text contrast"
                    )
                }
            }
        }
    }

    func testIncreasedContrastStrengthensRichTextBoundaries() {
        for preset in SynaraThemeRamp.presets {
            for dark in [false, true] {
                let standard = SynaraThemeRamp.tokens(baseHex: preset.hex, dark: dark)
                let increased = SynaraThemeRamp.tokens(
                    baseHex: preset.hex,
                    dark: dark,
                    increasedContrast: true
                )
                XCTAssertGreaterThan(
                    SynaraThemeRamp.contrastRatio(
                        foreground: increased.richTextInlineCodeBorder,
                        background: increased.surface
                    ),
                    SynaraThemeRamp.contrastRatio(
                        foreground: standard.richTextInlineCodeBorder,
                        background: standard.surface
                    )
                )
                XCTAssertGreaterThanOrEqual(
                    SynaraThemeRamp.contrastRatio(
                        foreground: increased.richTextInlineCodeBorder,
                        background: increased.surface
                    ),
                    3
                )
                XCTAssertGreaterThanOrEqual(
                    SynaraThemeRamp.contrastRatio(
                        foreground: increased.richTextSpoilerBorder,
                        background: increased.surface
                    ),
                    3
                )
            }
        }
    }

    func testInlineCodeBoundaryIsQuietNormallyAndMeasurableInIncreasedContrast() {
        for preset in SynaraThemeRamp.presets {
            for dark in [false, true] {
                let standard = SynaraThemeRamp.tokens(baseHex: preset.hex, dark: dark)
                XCTAssertEqual(
                    SynaraRichTextColorPolicy.inlineCodeBoundary(
                        tokens: standard,
                        increasedContrast: false
                    ),
                    standard.richTextInlineCodeBackground,
                    "\(preset.label) standard inline-code boundary must be optically absent"
                )

                let increased = SynaraThemeRamp.tokens(
                    baseHex: preset.hex,
                    dark: dark,
                    increasedContrast: true
                )
                let increasedBoundary = SynaraRichTextColorPolicy.inlineCodeBoundary(
                    tokens: increased,
                    increasedContrast: true
                )
                XCTAssertEqual(increasedBoundary, increased.richTextInlineCodeBorder)
                XCTAssertGreaterThanOrEqual(
                    SynaraThemeRamp.contrastRatio(
                        foreground: increasedBoundary,
                        background: increased.surface
                    ),
                    3,
                    "\(preset.label) Increased Contrast boundary"
                )
            }
        }
    }

    func testRichTextColorPolicyPreservesSafeAuthoredColors() {
        let result = SynaraRichTextColorPolicy.resolve(
            authoredForeground: "#ffffff",
            authoredBackground: "#000000",
            fallbackForeground: "#111111",
            fallbackBackground: "#ffffff",
            increasedContrast: false
        )

        XCTAssertEqual(result.foreground, "#ffffff")
        XCTAssertEqual(result.background, "#000000")
        XCTAssertTrue(result.preservedAuthoredForeground)
        XCTAssertTrue(result.preservedAuthoredBackground)
    }

    func testRichTextColorPolicyMinimallyClampsUnsafeForeground() {
        let result = SynaraRichTextColorPolicy.resolve(
            authoredForeground: "#dddddd",
            authoredBackground: nil,
            fallbackForeground: "#111111",
            fallbackBackground: "#ffffff",
            increasedContrast: false
        )

        XCTAssertEqual(result.background, "#ffffff")
        XCTAssertFalse(result.preservedAuthoredForeground)
        XCTAssertGreaterThanOrEqual(
            SynaraThemeRamp.contrastRatio(
                foreground: result.foreground,
                background: result.background
            ),
            SynaraRichTextColorPolicy.standardTextContrast
        )
        XCTAssertNotEqual(result.foreground, "#111111", "The authored color should be clamped, not discarded")
    }

    func testRichTextColorPolicyDropsBackgroundThatCannotMeetIncreasedContrast() {
        let result = SynaraRichTextColorPolicy.resolve(
            authoredForeground: "#777777",
            authoredBackground: "#777777",
            fallbackForeground: "#ffffff",
            fallbackBackground: "#000000",
            increasedContrast: true
        )

        XCTAssertEqual(result.background, "#000000")
        XCTAssertFalse(result.preservedAuthoredBackground)
        XCTAssertGreaterThanOrEqual(
            SynaraThemeRamp.contrastRatio(
                foreground: result.foreground,
                background: result.background
            ),
            SynaraRichTextColorPolicy.increasedTextContrast
        )
    }

    func testAuthoredColorsAreResolvedAgainstActualOwnMessageSurface() {
        for dark in [false, true] {
            for increasedContrast in [false, true] {
                let tokens = SynaraThemeRamp.tokens(
                    baseHex: SynaraThemeRamp.defaultBaseHex,
                    dark: dark,
                    increasedContrast: increasedContrast
                )
                for reduceTransparency in [false, true] {
                    let bubbleSurface = SynaraRichTextColorPolicy.standardOwnMessageBackground(
                        tokens: tokens,
                        dark: dark,
                        reduceTransparency: reduceTransparency
                    )
                    let resolved = SynaraRichTextColorPolicy.resolve(
                        authoredForeground: dark ? "#555555" : "#bbbbbb",
                        authoredBackground: nil,
                        fallbackForeground: tokens.primaryText,
                        fallbackBackground: bubbleSurface,
                        increasedContrast: increasedContrast
                    )
                    XCTAssertEqual(resolved.background, bubbleSurface)
                    XCTAssertGreaterThanOrEqual(
                        SynaraThemeRamp.contrastRatio(
                            foreground: resolved.foreground,
                            background: bubbleSurface
                        ),
                        increasedContrast ? 7 : 4.5
                    )
                }
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
