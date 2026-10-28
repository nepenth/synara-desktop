import Foundation

/// One photo, video or GIF of an X post. X's "GIFs" are short MP4s.
enum XPostMedia: Equatable, Hashable {
    case photo(url: String, width: Double?, height: Double?)
    case video(src: String, poster: String?, width: Double?, height: Double?)
    case gif(src: String, poster: String?, width: Double?, height: Double?)

    var aspectRatio: Double? {
        let size: (Double?, Double?)
        switch self {
        case let .photo(_, width, height),
             let .video(_, _, width, height),
             let .gif(_, _, width, height):
            size = (width, height)
        }
        guard let width = size.0, let height = size.1, width > 0, height > 0 else {
            return nil
        }
        return width / height
    }

    var isPhoto: Bool {
        if case .photo = self { return true }
        return false
    }
}

struct XPost: Equatable {
    let id: String
    let url: String
    let authorName: String
    let authorHandle: String
    let avatarURL: String?
    let text: String
    let createdAt: String?
    let media: [XPostMedia]

    /// What the card draws: up to four photos, or the one video.
    var cardMedia: [XPostMedia] {
        Array(media.prefix(media.first?.isPhoto == true ? 4 : 1))
    }
}

/// Matches desktop `xEmbed.ts` and the shell's `desktop_x_embed.rs`: finds
/// post links, derives X's embed token, fetches the post's public JSON from
/// the syndication endpoint, and keeps only what the card draws. Media must
/// come from X's own CDNs.
enum XPostEmbed {
    private static let hosts: Set<String> = [
        "x.com",
        "www.x.com",
        "mobile.x.com",
        "twitter.com",
        "www.twitter.com",
        "mobile.twitter.com",
    ]
    private static let statusPath = try? NSRegularExpression(
        pattern: #"^/(?:[A-Za-z0-9_]{1,15}|i(?:/web)?)/status(?:es)?/(\d{1,20})(?:/|$)"#
    )
    static let videoHost = "video.twimg.com"
    static let imageHost = "pbs.twimg.com"
    static let syndicationURL = "https://cdn.syndication.twimg.com/tweet-result"
    private static let maxTextCharacters = 1000
    private static let maxMedia = 4
    private static let maxPostIDDigits = 20
    private static let maxTokenCharacters = 32
    private static let maxBodyBytes = 1024 * 1024
    private static let requestTimeout: TimeInterval = 10

    /// The post id of an x.com or twitter.com status link.
    static func statusID(_ value: String) -> String? {
        guard let components = URLComponents(string: value),
              let scheme = components.scheme?.lowercased(),
              scheme == "https" || scheme == "http",
              (components.user ?? "").isEmpty,
              (components.password ?? "").isEmpty,
              let host = components.host?.lowercased(),
              hosts.contains(host),
              let statusPath
        else {
            return nil
        }
        let path = components.percentEncodedPath
        guard let match = statusPath.firstMatch(in: path, range: NSRange(path.startIndex..., in: path)),
              let range = Range(match.range(at: 1), in: path)
        else {
            return nil
        }
        return String(path[range])
    }

    /// The first X status link in a message body, as desktop picks the
    /// first link of a message for its preview.
    static func firstStatusLink(in body: String) -> String? {
        guard let first = SynaraPlainTextLinks.links(in: body).first else {
            return nil
        }
        let url = first.url.absoluteString
        return statusID(url) == nil ? nil : url
    }

    /// The token X's embed endpoint expects, as X's own embed script derives
    /// it: `((id / 1e15) * Math.PI).toString(36)` without zeros or the point.
    static func syndicationToken(_ id: String) -> String {
        let value = (Double(id) ?? .nan) / 1e15 * Double.pi
        return javaScriptRadixString(value, radix: 36)
            .filter { $0 != "0" && $0 != "." }
    }

    static func syndicationRequestURL(id: String, token: String) -> URL? {
        guard id.isEmpty == false,
              id.count <= maxPostIDDigits,
              id.allSatisfy({ $0.isASCII && $0.isNumber }),
              token.isEmpty == false,
              token.count <= maxTokenCharacters,
              token.allSatisfy({ $0.isASCII && ($0.isNumber || $0.isLowercase) })
        else {
            return nil
        }
        return URL(string: "\(syndicationURL)?id=\(id)&lang=en&token=\(token)")
    }

    // MARK: Parsing

    /// The card's view of a syndication response, or `nil` for a deleted or
    /// unavailable post.
    static func parse(_ json: Any, url: String) -> XPost? {
        guard let post = json as? [String: Any],
              post["__typename"] as? String == "Tweet",
              let id = JSONValue.string(post["id_str"])
        else {
            return nil
        }
        let user = JSONValue.object(post["user"])
        guard let handle = JSONValue.string(user?["screen_name"]) else {
            return nil
        }
        let media = (post["mediaDetails"] as? [Any] ?? [])
            .compactMap(parseMedia)
            .prefix(maxMedia)
        return XPost(
            id: id,
            url: url,
            authorName: JSONValue.string(user?["name"]) ?? handle,
            authorHandle: handle,
            avatarURL: onHost(user?["profile_image_url_https"], imageHost),
            text: displayText(post),
            createdAt: JSONValue.string(post["created_at"]),
            media: Array(media)
        )
    }

    private static func positive(_ value: Any?) -> Double? {
        guard let number = JSONValue.number(value), number > 0 else {
            return nil
        }
        return number
    }

    private static func onHost(_ value: Any?, _ host: String) -> String? {
        guard let url = JSONValue.string(value),
              let safeHost = RemoteContentURL.safeHost(url),
              safeHost == host
        else {
            return nil
        }
        return url
    }

    /// The highest-bitrate MP4 variant; HLS playlists are skipped.
    private static func bestMP4(_ videoInfo: [String: Any]?) -> String? {
        var best: (url: String, bitrate: Double)?
        for raw in videoInfo?["variants"] as? [Any] ?? [] {
            guard let variant = JSONValue.object(raw),
                  variant["content_type"] as? String == "video/mp4",
                  let url = onHost(variant["url"], videoHost)
            else {
                continue
            }
            let bitrate = JSONValue.number(variant["bitrate"]) ?? 0
            if best == nil || bitrate > best!.bitrate {
                best = (url, bitrate)
            }
        }
        return best?.url
    }

    private static func parseMedia(_ raw: Any) -> XPostMedia? {
        guard let media = JSONValue.object(raw) else {
            return nil
        }
        let info = JSONValue.object(media["original_info"])
        let width = positive(info?["width"])
        let height = positive(info?["height"])
        let image = onHost(media["media_url_https"], imageHost)
        switch media["type"] as? String {
        case "photo":
            return image.map { .photo(url: $0, width: width, height: height) }
        case "video":
            return bestMP4(JSONValue.object(media["video_info"]))
                .map { .video(src: $0, poster: image, width: width, height: height) }
        case "animated_gif":
            return bestMP4(JSONValue.object(media["video_info"]))
                .map { .gif(src: $0, poster: image, width: width, height: height) }
        default:
            return nil
        }
    }

    private static func displayText(_ post: [String: Any]) -> String {
        let raw = JSONValue.string(post["text"]) ?? ""
        var text = raw
        // `display_text_range` counts code points, as `Array.from` does.
        if let range = post["display_text_range"] as? [Any],
           range.count >= 2,
           let start = JSONValue.number(range[0]),
           let end = JSONValue.number(range[1])
        {
            let scalars = Array(raw.unicodeScalars)
            let lower = min(max(0, Int(start)), scalars.count)
            let upper = min(max(0, Int(end)), scalars.count)
            var view = String.UnicodeScalarView()
            if lower < upper {
                view.append(contentsOf: scalars[lower ..< upper])
            }
            text = String(view)
        }
        // t.co short links read as the links they stand for.
        for entry in JSONValue.object(post["entities"])?["urls"] as? [Any] ?? [] {
            let link = JSONValue.object(entry)
            if let short = JSONValue.string(link?["url"]),
               let expanded = JSONValue.string(link?["expanded_url"])
            {
                text = text.replacingOccurrences(of: short, with: expanded)
            }
        }
        text = text
            .replacingOccurrences(of: "&amp;", with: "&")
            .replacingOccurrences(of: "&lt;", with: "<")
            .replacingOccurrences(of: "&gt;", with: ">")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard text.utf16.count > maxTextCharacters else {
            return text
        }
        var clipped = ""
        var units = 0
        for character in text {
            units += character.utf16.count
            if units > maxTextCharacters { break }
            clipped.append(character)
        }
        return clipped + "…"
    }

    // MARK: Fetching

    /// The post for `url`, or `nil` when it is not a status link or X does
    /// not answer with a post in time. Requests carry only the id and token.
    static func fetch(_ url: String, session: URLSession = RemoteFetch.session) async -> XPost? {
        guard let id = statusID(url),
              let requestURL = syndicationRequestURL(id: id, token: syndicationToken(id)),
              let data = try? await RemoteFetch.data(
                  from: requestURL,
                  maxBytes: maxBodyBytes,
                  accept: "application/json",
                  acceptedTypes: ["application/json"],
                  timeout: requestTimeout,
                  session: session
              ),
              let json = try? JSONSerialization.jsonObject(with: data)
        else {
            return nil
        }
        return parse(json, url: url)
    }

    /// `Number.prototype.toString(radix)` for a finite double, as V8 writes
    /// it (`DoubleToRadixCString`): fraction digits only up to the input's
    /// precision, rounded half to even.
    static func javaScriptRadixString(_ input: Double, radix: Int) -> String {
        guard input.isFinite, (2 ... 36).contains(radix) else {
            return input.isNaN ? "NaN" : (input < 0 ? "-Infinity" : "Infinity")
        }
        let digits = Array("0123456789abcdefghijklmnopqrstuvwxyz")
        let base = Double(radix)
        let negative = input < 0
        let value = abs(input)
        var integer = value.rounded(.down)
        var fraction = value - integer
        var delta = max(Double.leastNonzeroMagnitude, 0.5 * (value.nextUp - value))
        var fractionDigits: [Int] = []
        if fraction >= delta {
            repeat {
                fraction *= base
                delta *= base
                let digit = Int(fraction)
                fractionDigits.append(digit)
                fraction -= Double(digit)
                if fraction > 0.5 || (fraction == 0.5 && digit & 1 == 1), fraction + delta > 1 {
                    // Round up, carrying through digits already written.
                    while true {
                        guard let last = fractionDigits.popLast() else {
                            integer += 1
                            break
                        }
                        if last + 1 < radix {
                            fractionDigits.append(last + 1)
                            break
                        }
                    }
                    break
                }
            } while fraction >= delta
        }

        var integerDigits: [Character] = []
        // Digits beyond a double's precision are written as zeros.
        while integer / base >= 9_007_199_254_740_992 {
            integer /= base
            integerDigits.append("0")
        }
        repeat {
            let remainder = fmod(integer, base)
            integerDigits.append(digits[Int(remainder)])
            integer = (integer - remainder) / base
        } while integer > 0

        var result = negative ? "-" : ""
        result += String(integerDigits.reversed())
        if fractionDigits.isEmpty == false {
            result += "." + String(fractionDigits.map { digits[$0] })
        }
        return result
    }
}

/// Posts by id for this session, so scrolling past a link never refetches
/// it. A failed fetch may succeed later, so only parsed posts stay.
actor XPostStore {
    static let shared = XPostStore()

    private var posts = BoundedLRUCache<String, XPost>(capacity: 200)
    private var inFlight: [String: Task<XPost?, Never>] = [:]

    func post(for url: String) async -> XPost? {
        guard let id = XPostEmbed.statusID(url) else {
            return nil
        }
        if let cached = posts.value(forKey: id) {
            return cached
        }
        if let task = inFlight[id] {
            return await task.value
        }
        let task = Task { await XPostEmbed.fetch(url) }
        inFlight[id] = task
        let post = await task.value
        inFlight[id] = nil
        if let post {
            posts.insert(post, forKey: id)
        }
        return post
    }
}
