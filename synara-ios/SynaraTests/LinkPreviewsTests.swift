import Network
import UIKit
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

    func testTheCardNamesTheSiteOrItsHost() throws {
        let url = try XCTUnwrap(URL(string: "https://www.Example.org/a"))
        XCTAssertEqual(LinkPreviewStore.hostName(for: url), "example.org")
        let named = LinkPreviewStore.makePreview(LinkPreviewContent(url: url, title: "T", siteName: "Example"))
        XCTAssertEqual(named.siteName, "Example")
        let unnamed = LinkPreviewStore.makePreview(
            LinkPreviewContent(url: url, title: "T", description: "", siteName: "")
        )
        XCTAssertEqual(unnamed.siteName, "example.org")
        XCTAssertNil(unnamed.description)
        XCTAssertNil(unnamed.image)
    }

    func testImagesAreThumbnailedAndNonImagesDropped() throws {
        let png = UIGraphicsImageRenderer(size: CGSize(width: 800, height: 400)).pngData { context in
            UIColor.red.setFill()
            context.fill(CGRect(x: 0, y: 0, width: 800, height: 400))
        }
        let thumbnail = try XCTUnwrap(LinkPreviewStore.thumbnail(from: png))
        let longest = max(thumbnail.size.width * thumbnail.scale, thumbnail.size.height * thumbnail.scale)
        XCTAssertLessThanOrEqual(longest, CGFloat(LinkPreviewStore.maxImagePixels))
        XCTAssertNil(LinkPreviewStore.thumbnail(from: Data("<svg onload=alert(1)/>".utf8)))
        XCTAssertNil(LinkPreviewStore.thumbnail(from: Data()))
    }

    func testTheStoreAsksCoreOnlyForPreviewableLinks() async throws {
        let url = try XCTUnwrap(URL(string: "https://example.org/a"))
        let local = try XCTUnwrap(URL(string: "http://192.168.1.1/"))
        let fetcher = CountingLinkPreviewFetcher(previews: [
            url: LinkPreviewContent(url: url, title: "Example page", description: "About it", siteName: "Example"),
            local: LinkPreviewContent(url: local, title: "Router"),
        ])
        let store = LinkPreviewStore()
        let loaded = await store.preview(for: url, using: fetcher)
        XCTAssertEqual(loaded?.title, "Example page")
        XCTAssertEqual(loaded?.description, "About it")
        XCTAssertEqual(loaded?.siteName, "Example")
        // Kept for the session: scrolling back does not ask Core again.
        _ = await store.preview(for: url, using: fetcher)
        let refused = await store.preview(for: local, using: fetcher)
        XCTAssertNil(refused)
        XCTAssertEqual(fetcher.requested, [url])
    }

    func testFailedPreviewsAreLeftToCore() async throws {
        let url = try XCTUnwrap(URL(string: "https://example.org/missing"))
        let fetcher = CountingLinkPreviewFetcher(previews: [:])
        let store = LinkPreviewStore()
        let first = await store.preview(for: url, using: fetcher)
        let second = await store.preview(for: url, using: fetcher)
        XCTAssertNil(first)
        XCTAssertNil(second)
        XCTAssertEqual(fetcher.requested, [url, url], "Core remembers failures, so the app does not")
    }

    func testFormattedMessagesPreviewTheirFirstPlainTextLink() {
        let body = "See https://example.org/first, and https://example.org/second."
        XCTAssertEqual(LinkPreviewPolicy.previewURL(for: row(body))?.absoluteString, "https://example.org/first")
        XCTAssertNil(LinkPreviewPolicy.previewURL(for: row("go to http://192.168.1.1/admin then https://example.org/")))
    }
}

/// Hands out fixed previews and records which links Core was asked for.
private final class CountingLinkPreviewFetcher: LinkPreviewFetching {
    private let previews: [URL: LinkPreviewContent]
    private let lock = NSLock()
    private var asked: [URL] = []

    init(previews: [URL: LinkPreviewContent]) {
        self.previews = previews
    }

    var requested: [URL] {
        lock.withLock { asked }
    }

    func preview(for url: URL) async -> LinkPreviewContent? {
        lock.withLock { asked.append(url) }
        return previews[url]
    }
}
