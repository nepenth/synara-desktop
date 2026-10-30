import Network
import XCTest
@testable import Synara

/// Link preview gating and first-link rules, as desktop's
/// `nativeLinkUnfurl.test.ts`.
final class LinkPreviewsTests: XCTestCase {
    private func row(
        _ body: String?,
        enabled: Bool = true,
        approval: Bool = false,
        gif: Bool = false,
        xPost: Bool = false
    ) -> LinkPreviewPolicy.Row {
        LinkPreviewPolicy.Row(
            body: body,
            isEnabled: enabled,
            isAgentApproval: approval,
            hasInlineGif: gif,
            hasXPost: xPost
        )
    }

    func testOnlyTheFirstLinkGetsAPreview() {
        let body = "read https://example.org/a then https://example.org/b"
        XCTAssertEqual(LinkPreviewPolicy.previewURL(for: row(body))?.absoluteString, "https://example.org/a")
        XCTAssertNil(LinkPreviewPolicy.previewURL(for: row("no links here")))
        XCTAssertNil(LinkPreviewPolicy.previewURL(for: row("")))
    }

    func testTheSettingGatesThePreview() {
        XCTAssertNil(LinkPreviewPolicy.previewURL(for: row("https://example.org/a", enabled: false)))
    }

    func testOtherCardsAndMediaSkipThePreview() {
        let body = "https://example.org/a"
        XCTAssertNil(LinkPreviewPolicy.previewURL(for: row(nil)), "media, agent cards and placeholders have no body")
        XCTAssertNil(LinkPreviewPolicy.previewURL(for: row(body, approval: true)))
        XCTAssertNil(LinkPreviewPolicy.previewURL(for: row(body, gif: true)))
        XCTAssertNil(LinkPreviewPolicy.previewURL(for: row(body, xPost: true)))
    }

    func testTheSettingDefaultsOn() {
        XCTAssertEqual(RichLinkSettings.registeredDefaults[RichLinkSettings.linkPreviewsKey], true)
        let store = InMemorySettingsStore()
        XCTAssertTrue(store.bool(for: RichLinkSettings.linkPreviewsKey))
        store.set(false, for: RichLinkSettings.linkPreviewsKey)
        XCTAssertFalse(store.bool(for: RichLinkSettings.linkPreviewsKey))
    }

    func testOnlyPublicWebLinksArePreviewed() throws {
        let previewable = { (value: String) -> Bool in
            LinkPreviewPolicy.isPreviewable(try XCTUnwrap(URL(string: value)))
        }
        XCTAssertTrue(try previewable("https://example.org/a?b=c"))
        XCTAssertTrue(try previewable("http://example.org/"))
        XCTAssertTrue(try previewable("https://8.8.8.8/"))
        XCTAssertTrue(try previewable("https://[2606:4700:4700::1111]/"))
        for refused in [
            "mailto:someone@example.org",
            "file:///etc/passwd",
            "ftp://example.org/",
            "https://user:pass@example.org/",
            "http://localhost/",
            "http://printer.local/",
            "http://router.lan/",
            "http://127.0.0.1/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://169.254.169.254/latest/meta-data/",
            "http://100.64.0.1/",
            "http://0.0.0.0/",
            "http://2130706433/",
            "http://0x7f.1/",
            "http://[::1]/",
            "http://[::ffff:127.0.0.1]/",
            "http://[fd00::1]/",
            "http://[fe80::1]/",
            "http://[ff02::1]/",
        ] {
            XCTAssertFalse(try previewable(refused), refused)
        }
    }

    func testSiteNameDropsWww() throws {
        XCTAssertEqual(LinkPreviewStore.siteName(for: try XCTUnwrap(URL(string: "https://www.Example.org/a"))), "example.org")
    }
}
