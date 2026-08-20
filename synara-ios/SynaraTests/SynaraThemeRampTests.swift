import XCTest
@testable import Synara

final class SynaraThemeRampTests: XCTestCase {
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

        XCTAssertLessThan(
            SynaraThemeRamp.relativeLuminance(hex: tokens.groupedSurface),
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
