import XCTest
@testable import Synara

/// Klipy search and parsing (desktop `gifProvider.test.ts`) and the build
/// key rules (desktop `runtime-config.test.mjs`).
final class KlipyGifProviderTests: XCTestCase {
    private let provider = KlipyGifProvider(apiKey: "klipy-key")

    private func klipyItem(_ id: Int, _ slug: String) -> [String: Any] {
        [
            "id": id,
            "slug": slug,
            "title": "GIF \(id)",
            "type": "gif",
            "file": [
                "md": ["gif": ["url": "https://static.klipy.com/ii/\(slug)/md.gif", "width": 320, "height": 240]],
                "sm": ["webp": ["url": "https://static.klipy.com/ii/\(slug)/sm.webp", "width": 160, "height": 120]],
            ],
        ]
    }

    private func components(_ url: URL?) throws -> URLComponents {
        try XCTUnwrap(URLComponents(url: try XCTUnwrap(url), resolvingAgainstBaseURL: false))
    }

    private func query(_ components: URLComponents, _ name: String) -> String? {
        components.queryItems?.first { $0.name == name }?.value
    }

    // MARK: URLs

    func testSearchAndTrendingURLsCarryTheKeyInThePathAndPageParams() throws {
        let search = try components(provider.url(for: .search(" high   five "), page: 2))
        XCTAssertEqual(search.scheme, "https")
        XCTAssertEqual(search.host, "api.klipy.com")
        XCTAssertEqual(search.path, "/api/v1/klipy-key/gifs/search")
        XCTAssertEqual(query(search, "q"), "high five")
        XCTAssertEqual(query(search, "page"), "2")
        XCTAssertEqual(query(search, "per_page"), "24")
        XCTAssertEqual(query(search, "content_filter"), "medium")
        XCTAssertEqual(query(search, "format_filter"), "gif,webp")

        let trending = try components(provider.url(for: .trending))
        XCTAssertEqual(trending.path, "/api/v1/klipy-key/gifs/trending")
        XCTAssertNil(query(trending, "q"))
        XCTAssertEqual(query(trending, "page"), "1")
    }

    func testSearchNeedsTwoCharactersAndEncodesTheQuery() throws {
        XCTAssertNil(provider.url(for: .search("a")))
        XCTAssertNil(provider.url(for: .search("   ")))
        XCTAssertFalse(KlipyGifProvider.isSearchable(" a "))
        XCTAssertTrue(KlipyGifProvider.isSearchable("ab"))
        XCTAssertEqual(KlipyGifProvider.normalizeQuery(String(repeating: "x", count: 100)).count, 80)

        let url = try XCTUnwrap(provider.url(for: .search("cats & dogs+1")))
        XCTAssertTrue(url.absoluteString.contains("q=cats%20%26%20dogs%2B1"), url.absoluteString)
        XCTAssertEqual(query(try components(url), "q"), "cats & dogs+1")
    }

    func testNoURLWithoutAValidKeyOrSafeEndpoint() {
        XCTAssertNil(KlipyGifProvider(apiKey: "").url(for: .trending))
        XCTAssertNil(KlipyGifProvider(apiKey: "bad key\"").url(for: .trending))
        XCTAssertNil(KlipyGifProvider(apiKey: "klipy-key", endpoint: "https://127.0.0.1/api").url(for: .trending))
        XCTAssertNil(KlipyGifProvider(apiKey: "klipy-key", endpoint: "http://api.klipy.com/api/v1").url(for: .trending))
        XCTAssertEqual(
            KlipyGifProvider(apiKey: "klipy-key", endpoint: "https://api.klipy.com/api/v1//").url(for: .trending)?.path,
            "/api/v1/klipy-key/gifs/trending"
        )
    }

    // MARK: Parsing

    func testResponsesMapMdGifsToSendURLsAndSmallWebpsToPreviews() {
        let page = KlipyGifProvider.parsePage(
            [
                "result": true,
                "data": [
                    "data": [klipyItem(1, "wave"), ["id": 2, "slug": "broken", "file": [String: Any]()] as [String: Any]],
                    "current_page": 1,
                    "per_page": 24,
                    "has_next": true,
                ],
            ] as [String: Any],
            page: 1
        )
        XCTAssertEqual(page.results, [
            GifResult(
                id: "1",
                title: "GIF 1",
                url: "https://static.klipy.com/ii/wave/md.gif",
                previewURL: "https://static.klipy.com/ii/wave/sm.webp",
                width: 320,
                height: 240,
                sourceURL: "https://klipy.com/gifs/wave"
            ),
        ])
        XCTAssertEqual(page.nextPage, 2)
        XCTAssertNil(KlipyGifProvider.parsePage(["data": ["data": [Any](), "has_next": false] as [String: Any]] as [String: Any]).nextPage)
        XCTAssertNil(KlipyGifProvider.parsePage(["data": ["data": [Any](), "has_next": 1] as [String: Any]] as [String: Any]).nextPage)
        XCTAssertEqual(KlipyGifProvider.parsePage("not an object").results, [])
    }

    func testRenditionFallbacksIdsAndUnsafeURLs() {
        let hdOnly = KlipyGifProvider.parseItem([
            "slug": "hd-only",
            "file": ["hd": ["gif": ["url": "https://static.klipy.com/hd.gif"]]],
        ])
        XCTAssertEqual(hdOnly?.url, "https://static.klipy.com/hd.gif")
        XCTAssertEqual(hdOnly?.previewURL, "https://static.klipy.com/hd.gif", "the sent GIF previews itself")
        XCTAssertEqual(hdOnly?.id, "hd-only", "the slug stands in for a missing id")
        XCTAssertEqual(hdOnly?.title, "GIF")

        let smallGif = KlipyGifProvider.parseItem([
            "id": "abc",
            "file": [
                "sm": ["gif": ["url": "https://static.klipy.com/sm.gif"]],
                "xs": ["gif": ["url": "https://static.klipy.com/xs.gif"]],
            ],
        ])
        XCTAssertEqual(smallGif?.url, "https://static.klipy.com/sm.gif")
        XCTAssertEqual(smallGif?.previewURL, "https://static.klipy.com/sm.gif")
        XCTAssertEqual(smallGif?.id, "abc")
        XCTAssertNil(smallGif?.sourceURL)

        XCTAssertNil(KlipyGifProvider.parseItem([
            "id": 3,
            "file": ["md": ["gif": ["url": "http://127.0.0.1/private.gif"]]],
        ]))
        let unsafePreview = KlipyGifProvider.parseItem([
            "id": 4,
            "file": [
                "md": ["gif": ["url": "https://static.klipy.com/md.gif", "width": true]],
                "sm": ["webp": ["url": "https://10.0.0.1/sm.webp"]],
            ],
        ])
        XCTAssertNil(unsafePreview?.previewURL)
        XCTAssertNil(unsafePreview?.width, "a boolean is not a size")
    }

    func testKlipyWordingAndAttribution() {
        XCTAssertEqual(KlipyGifProvider.searchPlaceholder, "Search KLIPY")
        XCTAssertEqual(KlipyGifProvider.attribution, "Powered by KLIPY")
    }

    // MARK: Fetching

    func testTrendingLoadsPageByPageWithoutAQueryOrCookies() async throws {
        let session = StubURLProtocol.session { request in
            let body = try JSONSerialization.data(withJSONObject: [
                "data": ["data": [self.klipyItem(9, "nine")], "has_next": false],
            ])
            return (200, ["Content-Type": "application/json"], body)
        }
        let page = try await provider.page(.trending, page: 3, session: session)
        let request = try XCTUnwrap(StubURLProtocol.requests.last)
        let sent = try components(request.url)
        XCTAssertEqual(query(sent, "page"), "3")
        XCTAssertNil(query(sent, "q"))
        XCTAssertNil(request.value(forHTTPHeaderField: "Cookie"))
        XCTAssertEqual(page.results.first?.url, "https://static.klipy.com/ii/nine/md.gif")
        XCTAssertNil(page.nextPage)
    }

    func testRateLimitsAndFailuresAreReported() async {
        let limited = StubURLProtocol.session { _ in (429, [:], Data()) }
        do {
            _ = try await provider.page(.search("cats"), session: limited)
            XCTFail("a 429 throws")
        } catch {
            XCTAssertEqual(error as? GifProviderError, .rateLimited)
        }
        let failing = StubURLProtocol.session { _ in (500, [:], Data()) }
        do {
            _ = try await provider.page(.search("cats"), session: failing)
            XCTFail("a 500 throws")
        } catch {
            XCTAssertEqual(error as? GifProviderError, .failed)
        }
    }

    // MARK: Build key

    func testKeyValidationMatchesTheDesktopBuild() {
        XCTAssertEqual(KlipyAPIKey.validated(" key_ABC-123 "), "key_ABC-123")
        XCTAssertEqual(KlipyAPIKey.validated("\nkey_ABC-123\n"), "key_ABC-123")
        XCTAssertEqual(KlipyAPIKey.validated(String(repeating: "a", count: 128))?.count, 128)
        XCTAssertNil(KlipyAPIKey.validated(nil))
        XCTAssertNil(KlipyAPIKey.validated(""))
        XCTAssertNil(KlipyAPIKey.validated("   "))
        XCTAssertNil(KlipyAPIKey.validated("short"))
        XCTAssertNil(KlipyAPIKey.validated(String(repeating: "a", count: 129)))
        XCTAssertNil(KlipyAPIKey.validated("bad key\""))
        XCTAssertNil(KlipyAPIKey.validated("$(SYNARA_KLIPY_API_KEY)"), "an unexpanded build setting is no key")
    }

    func testTheAppReadsItsKeyFromAnExpandedInfoPlistEntry() {
        let raw = Bundle.main.object(forInfoDictionaryKey: KlipyAPIKey.infoDictionaryKey)
        XCTAssertNotNil(raw as? String, "Info.plist carries SynaraKlipyAPIKey, empty without a key")
        XCTAssertFalse((raw as? String ?? "").contains("$("), "the build setting is expanded")
    }

    func testABuildWithoutAKeyDoesNotOfferGifSearch() {
        // A bundle without a key: the setting alone cannot turn search on.
        let settings = InMemorySettingsStore()
        XCTAssertNil(KlipyGifProvider.configured(bundle: Bundle(for: Self.self)))
        XCTAssertEqual(GifSearchAvailability.resolve(settings: settings, bundle: Bundle(for: Self.self)), .unconfigured)
        XCTAssertEqual(
            GifSearchAvailability.unconfigured.commandUnavailableMessage,
            "GIF search is not set up for this app."
        )
    }
}

/// Answers every request of its session from `handler`.
final class StubURLProtocol: URLProtocol {
    typealias Handler = (URLRequest) throws -> (Int, [String: String], Data)

    private static let lock = NSLock()
    private static var handler: Handler?
    private static var recorded: [URLRequest] = []

    static var requests: [URLRequest] {
        lock.lock()
        defer { lock.unlock() }
        return recorded
    }

    static func session(_ handler: @escaping Handler) -> URLSession {
        lock.lock()
        self.handler = handler
        recorded = []
        lock.unlock()
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [StubURLProtocol.self]
        return URLSession(configuration: configuration)
    }

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        Self.lock.lock()
        Self.recorded.append(request)
        let handler = Self.handler
        Self.lock.unlock()
        guard let handler, let url = request.url else {
            client?.urlProtocol(self, didFailWithError: URLError(.badURL))
            return
        }
        do {
            let (status, headers, body) = try handler(request)
            let response = HTTPURLResponse(url: url, statusCode: status, httpVersion: "HTTP/1.1", headerFields: headers)!
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: body)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}
