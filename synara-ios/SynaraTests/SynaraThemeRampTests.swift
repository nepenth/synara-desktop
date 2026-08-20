import XCTest
@testable import Synara

final class SynaraThemeRampTests: XCTestCase {
    func testNormalizeAcceptsOnlyFullHexColors() {
        XCTAssertEqual(SynaraThemeRamp.normalize("#AABBCC"), "#aabbcc")
        XCTAssertEqual(SynaraThemeRamp.normalize(" #2b2d31 "), "#2b2d31")
        XCTAssertNil(SynaraThemeRamp.normalize("#abc"))
        XCTAssertNil(SynaraThemeRamp.normalize("red"))
        XCTAssertEqual(SynaraThemeRamp.resolve(nil), SynaraThemeRamp.defaultBaseHex)
    }

    func testDefaultDarkRampStacksSidebarDarkerThanChat() {
        let tokens = SynaraThemeRamp.tokens(baseHex: SynaraThemeRamp.defaultBaseHex, dark: true)

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
        XCTAssertGreaterThan(SynaraThemeRamp.relativeLuminance(hex: tokens.surface), 0.9)
        XCTAssertGreaterThan(
            SynaraThemeRamp.contrastRatio(foreground: tokens.primaryText, background: tokens.surface),
            7
        )
    }

    func testSaturatedBaseColorTintsRampsWithoutBreakingContrast() {
        let dark = SynaraThemeRamp.tokens(baseHex: "#5865f2", dark: true)
        let light = SynaraThemeRamp.tokens(baseHex: "#5865f2", dark: false)
        let defaultDark = SynaraThemeRamp.tokens(baseHex: SynaraThemeRamp.defaultBaseHex, dark: true)

        XCTAssertNotEqual(dark.surface, defaultDark.surface)
        XCTAssertLessThan(
            SynaraThemeRamp.relativeLuminance(hex: dark.groupedSurface),
            SynaraThemeRamp.relativeLuminance(hex: dark.surface)
        )
        XCTAssertGreaterThan(
            SynaraThemeRamp.contrastRatio(foreground: dark.primaryText, background: dark.surface),
            4.5
        )
        XCTAssertGreaterThan(
            SynaraThemeRamp.contrastRatio(foreground: light.primaryText, background: light.surface),
            4.5
        )
    }
}
