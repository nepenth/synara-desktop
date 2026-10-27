import Foundation

/// One GIF from the picker. `url` is what is sent; `previewURL` is the
/// lighter rendition the grid shows.
struct GifResult: Equatable, Identifiable, Hashable {
    let id: String
    let title: String
    let url: String
    let previewURL: String?
    let width: Double?
    let height: Double?
    let sourceURL: String?
}

/// One page of results; `nextPage` is set while the provider has more.
struct GifPage: Equatable {
    var results: [GifResult]
    var nextPage: Int?
}

enum GifFeed: Equatable {
    case trending
    case search(String)
}

enum GifProviderError: Error, Equatable {
    case rateLimited
    case failed
}

/// The Klipy key ships inside the app, as client-side GIF keys do, but never
/// enters the repository: release builds inject it from `SYNARA_KLIPY_API_KEY`
/// (see `scripts/lib/klipy-api-key.sh`) into the `SynaraKlipyAPIKey`
/// Info.plist entry. Without a valid key, GIF search is not offered.
enum KlipyAPIKey {
    static let infoDictionaryKey = "SynaraKlipyAPIKey"
    private static let pattern = try? NSRegularExpression(pattern: #"^[A-Za-z0-9_-]{8,128}$"#)

    /// `raw` trimmed, when it is a well-formed key.
    static func validated(_ raw: String?) -> String? {
        guard let trimmed = raw?.trimmingCharacters(in: .whitespacesAndNewlines),
              trimmed.isEmpty == false,
              let pattern,
              pattern.firstMatch(in: trimmed, range: NSRange(trimmed.startIndex..., in: trimmed)) != nil
        else {
            return nil
        }
        return trimmed
    }

    static func bundled(_ bundle: Bundle = .main) -> String? {
        validated(bundle.object(forInfoDictionaryKey: infoDictionaryKey) as? String)
    }
}

/// Matches desktop `gifProvider.ts` for Klipy: trending and search feeds,
/// page by page, with the md GIF as the sent URL and the small webp as the
/// grid preview.
struct KlipyGifProvider: Equatable {
    static let pageSize = 24
    static let defaultEndpoint = "https://api.klipy.com/api/v1"
    /// Klipy's terms require this wording in the search field.
    static let searchPlaceholder = "Search KLIPY"
    static let attribution = "Powered by KLIPY"
    private static let maxQueryLength = 80
    private static let minQueryLength = 2
    private static let maxResponseBytes = 4 * 1024 * 1024

    let apiKey: String
    var endpoint = defaultEndpoint
    var contentFilter = "medium"

    /// The provider for this build, or `nil` when it was built without a key.
    static func configured(bundle: Bundle = .main) -> KlipyGifProvider? {
        KlipyAPIKey.bundled(bundle).map { KlipyGifProvider(apiKey: $0) }
    }

    static func normalizeQuery(_ query: String) -> String {
        let collapsed = query
            .split(whereSeparator: { $0.isWhitespace })
            .joined(separator: " ")
        return String(collapsed.prefix(maxQueryLength))
    }

    /// Whether `query` is long enough to search; an empty one shows trending.
    static func isSearchable(_ query: String) -> Bool {
        normalizeQuery(query).count >= minQueryLength
    }

    func url(for feed: GifFeed, page: Int = 1, limit: Int = pageSize) -> URL? {
        guard let key = KlipyAPIKey.validated(apiKey),
              RemoteContentURL.isSafeHTTPS(endpoint)
        else {
            return nil
        }
        var params: [(String, String)] = []
        let kind: String
        switch feed {
        case .trending:
            kind = "trending"
        case let .search(query):
            let q = Self.normalizeQuery(query)
            guard q.count >= Self.minQueryLength else {
                return nil
            }
            kind = "search"
            params.append(("q", q))
        }
        let safePage = page > 0 ? page : 1
        params.append(("page", String(safePage)))
        params.append(("per_page", String(limit)))
        params.append(("content_filter", contentFilter))
        params.append(("format_filter", "gif,webp"))

        var base = endpoint
        while base.hasSuffix("/") {
            base.removeLast()
        }
        let query = params
            .map { "\(Self.encodeComponent($0.0))=\(Self.encodeComponent($0.1))" }
            .joined(separator: "&")
        return URL(string: "\(base)/\(Self.encodeComponent(key))/gifs/\(kind)?\(query)")
    }

    func page(_ feed: GifFeed, page: Int = 1, session: URLSession = RemoteFetch.session) async throws -> GifPage {
        guard let url = url(for: feed, page: page) else {
            return GifPage(results: [])
        }
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        let (data, response) = try await session.data(for: request)
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        if status == 429 {
            throw GifProviderError.rateLimited
        }
        guard (200 ..< 300).contains(status), data.count <= Self.maxResponseBytes else {
            throw GifProviderError.failed
        }
        guard let json = try? JSONSerialization.jsonObject(with: data) else {
            throw GifProviderError.failed
        }
        return Self.parsePage(json, page: page)
    }

    // MARK: Parsing

    /// `{ result, data: { data: [...], current_page, per_page, has_next } }`
    static func parsePage(_ json: Any, page: Int = 1) -> GifPage {
        let body = JSONValue.object((json as? [String: Any])?["data"])
        let items = body?["data"] as? [Any] ?? []
        let results = items.compactMap { item -> GifResult? in
            guard let object = JSONValue.object(item) else { return nil }
            return parseItem(object)
        }
        let hasNext = JSONValue.isTrue(body?["has_next"])
        return GifPage(results: results, nextPage: hasNext ? page + 1 : nil)
    }

    static func parseItem(_ object: [String: Any]) -> GifResult? {
        let file = JSONValue.object(object["file"])
        let gif = format(file, "md", "gif") ?? format(file, "hd", "gif") ?? format(file, "sm", "gif")
        let preview = format(file, "sm", "webp")
            ?? format(file, "sm", "gif")
            ?? format(file, "xs", "gif")
            ?? gif
        let slug = JSONValue.string(object["slug"])
        let id = JSONValue.string(object["id"]) ?? JSONValue.number(object["id"]).map(JSONValue.numberText) ?? slug

        guard let rawURL = JSONValue.string(gif?["url"]),
              let url = RemoteContentURL.safe(rawURL)
        else {
            return nil
        }
        let previewURL = JSONValue.string(preview?["url"]).flatMap(RemoteContentURL.safe)
        let sourceURL = slug
            .map { "https://klipy.com/gifs/\(encodeComponent($0))" }
            .flatMap(RemoteContentURL.safe)
        return GifResult(
            id: (id?.isEmpty == false ? id : nil) ?? url,
            title: JSONValue.string(object["title"]) ?? "GIF",
            url: url,
            previewURL: previewURL,
            width: JSONValue.number(gif?["width"]),
            height: JSONValue.number(gif?["height"]),
            sourceURL: sourceURL
        )
    }

    private static func format(_ file: [String: Any]?, _ size: String, _ format: String) -> [String: Any]? {
        JSONValue.object(JSONValue.object(file?[size])?[format])
    }

    /// `encodeURIComponent`: everything but unreserved characters is escaped.
    static func encodeComponent(_ value: String) -> String {
        value.addingPercentEncoding(withAllowedCharacters: uriComponentCharacters) ?? value
    }

    private static let uriComponentCharacters = CharacterSet(
        charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()"
    )
}
