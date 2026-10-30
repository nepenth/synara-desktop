import Foundation
import ImageIO
import Network
import SynaraCore
import UIKit

/// Which link of a message gets a preview card (Settings → GIFs & Link
/// Previews → Link previews), with desktop's rules: only the first link, and
/// none when the setting is off, for media, agent cards and approvals, or
/// when the message already shows an inline GIF or an X post.
enum LinkPreviewPolicy {
    /// What a timeline row knows when it decides.
    struct Row: Equatable {
        /// The message text, or `nil` for media, agent cards and placeholders.
        var body: String?
        /// The formatted body (`org.matrix.custom.html`), when the message has one.
        var html: String? = nil
        var isEnabled: Bool
        var isAgentApproval = false
        var hasInlineGif = false
        var hasXPost = false
    }

    static func previewURL(for row: Row) -> URL? {
        guard row.isEnabled,
              let body = row.body,
              row.isAgentApproval == false,
              row.hasInlineGif == false,
              row.hasXPost == false,
              let first = firstLink(body: body, html: row.html),
              isPreviewable(first)
        else {
            return nil
        }
        return first
    }

    /// Desktop's `firstTimelinePreviewUrl`: the formatted body's first web
    /// link (its `href`, which the plain body may not spell out), else the
    /// plain body's first link.
    static func firstLink(body: String, html: String?) -> URL? {
        if let html, html.isEmpty == false,
           let formatted = MatrixHTMLRenderer.richText(body: body, html: html).runs
               .lazy.compactMap(\.link).first(where: isCandidate)
        {
            return formatted
        }
        guard body.contains(".") else {
            return nil
        }
        return SynaraPlainTextLinks.links(in: body).first?.url
    }

    /// An http(s) link without credentials, as desktop's `isPreviewCandidateUrl`.
    static func isCandidate(_ url: URL) -> Bool {
        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
              let scheme = components.scheme?.lowercased()
        else {
            return false
        }
        return (scheme == "https" || scheme == "http")
            && (components.user ?? "").isEmpty
            && (components.password ?? "").isEmpty
            && url.absoluteString.count <= 2048
    }

    /// An http(s) link without credentials whose host is not this device, the
    /// local network or a private address written as a literal IP. This only
    /// spares Core a call: Core's fetcher checks every address it connects
    /// to, including where names resolve and redirects lead.
    static func isPreviewable(_ url: URL) -> Bool {
        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
              let scheme = components.scheme?.lowercased(),
              scheme == "https" || scheme == "http",
              (components.user ?? "").isEmpty,
              (components.password ?? "").isEmpty,
              let host = components.host?.lowercased(),
              host.isEmpty == false,
              url.absoluteString.count <= 2048
        else {
            return false
        }
        let bare = host.hasPrefix("[") && host.hasSuffix("]") ? String(host.dropFirst().dropLast()) : host
        if let v4 = IPv4Address(bare), bare.contains(":") == false {
            return isPublic(v4)
        }
        if let v6 = IPv6Address(bare) {
            return isPublic(v6)
        }
        // A name whose last label is a number ("2130706433", "0x7f.1") is an
        // IPv4 address in another notation that the network stack may accept.
        let lastLabel = host.split(separator: ".").last.map(String.init) ?? ""
        let numericLabel = lastLabel.allSatisfy(\.isNumber)
            || (lastLabel.hasPrefix("0x") && lastLabel.dropFirst(2).allSatisfy(\.isHexDigit))
        let localSuffixes = [".localhost", ".local", ".internal", ".home.arpa", ".lan"]
        return host != "localhost"
            && host.contains(".")
            && numericLabel == false
            && localSuffixes.contains(where: host.hasSuffix) == false
    }

    static func isPublic(_ address: IPv4Address) -> Bool {
        let bytes = [UInt8](address.rawValue)
        guard bytes.count == 4 else { return false }
        let (a, b, c) = (bytes[0], bytes[1], bytes[2])
        return !(a == 0 || a == 10 || a == 127
            || (a == 100 && (64 ..< 128).contains(b))
            || (a == 169 && b == 254)
            || (a == 172 && (16 ..< 32).contains(b))
            || (a == 192 && b == 0 && (c == 0 || c == 2))
            || (a == 192 && b == 168)
            || (a == 198 && (b == 18 || b == 19))
            || (a == 198 && b == 51 && c == 100)
            || (a == 203 && b == 0 && c == 113)
            || a >= 224)
    }

    static func isPublic(_ address: IPv6Address) -> Bool {
        let bytes = [UInt8](address.rawValue)
        guard bytes.count == 16 else { return false }
        // IPv4-mapped and IPv4-compatible addresses (including :: and ::1).
        if bytes[0 ..< 10].allSatisfy({ $0 == 0 }) {
            guard bytes[10] == 0xff, bytes[11] == 0xff,
                  let v4 = IPv4Address(Data(bytes[12 ..< 16]))
            else {
                return false
            }
            return isPublic(v4)
        }
        return !((bytes[0] & 0xfe) == 0xfc // unique local
            || (bytes[0] == 0xfe && (bytes[1] & 0xc0) == 0x80) // link-local
            || (bytes[0] == 0xfe && (bytes[1] & 0xc0) == 0xc0) // site-local
            || bytes[0] == 0xff // multicast
            || (bytes[0] == 0x20 && bytes[1] == 0x01 && bytes[2] == 0x0d && bytes[3] == 0xb8)
            || (bytes[0] == 0x00 && bytes[1] == 0x64 && bytes[2] == 0xff && bytes[3] == 0x9b))
    }
}

/// A page's preview as Core fetched it.
struct LinkPreviewContent: Equatable {
    let url: URL
    let title: String
    var description: String?
    var siteName: String?
    /// The page's image as fetched: PNG, JPEG, GIF, WebP or AVIF, at most 5 MB.
    var imageData: Data?
}

/// Fetches a link's preview from this device.
protocol LinkPreviewFetching {
    func preview(for url: URL) async -> LinkPreviewContent?
}

/// Core's fetcher, the one desktop uses: it connects only to public
/// addresses it resolved itself, re-checks every redirect and bounds size and
/// time. Never `LPMetadataProvider`, which resolves, redirects and loads
/// subresources on its own, so a message link could reach the local network.
final class SharedCoreLinkPreviewService: LinkPreviewFetching {
    private let core: SharedCore

    init(core: SharedCore) {
        self.core = core
    }

    func preview(for url: URL) async -> LinkPreviewContent? {
        guard let dto = await core.linkPreview(url: url.absoluteString) else {
            return nil
        }
        return LinkPreviewContent(
            url: url,
            title: dto.title,
            description: dto.description,
            siteName: dto.siteName,
            imageData: dto.image
        )
    }
}

/// No previews, or the ones a test hands it.
struct MockLinkPreviewService: LinkPreviewFetching {
    var previews: [URL: LinkPreviewContent] = [:]

    func preview(for url: URL) async -> LinkPreviewContent? {
        previews[url]
    }
}

/// What a preview card shows. `host` always shows, because a page names
/// itself in `siteName` and could claim to be any site.
struct LinkPreview {
    let url: URL
    let title: String
    let description: String?
    let host: String
    /// The page's own name, when it says something the host does not.
    let siteName: String?
    let image: UIImage?

    /// "example.org · Example", or the host alone.
    var source: String {
        siteName.map { "\(host) · \($0)" } ?? host
    }
}

/// Previews by URL for this session, with their images decoded once, so
/// scrolling past a link never refetches or redecodes it. Core remembers
/// failures for a few minutes, so only previews stay here.
actor LinkPreviewStore {
    static let shared = LinkPreviewStore()

    /// Thumbnails are drawn at 64 pt; this covers 3x screens.
    static let maxImagePixels = 192
    /// Larger images are not decoded at all (a small file can declare a huge
    /// canvas).
    static let maxSourcePixels = 16_000_000

    private var previews = BoundedLRUCache<URL, LinkPreview>(capacity: 200)
    private var inFlight: [URL: Task<LinkPreview?, Never>] = [:]

    func preview(for url: URL, using fetcher: LinkPreviewFetching) async -> LinkPreview? {
        guard LinkPreviewPolicy.isPreviewable(url) else {
            return nil
        }
        if let cached = previews.value(forKey: url) {
            return cached
        }
        if let task = inFlight[url] {
            return await task.value
        }
        let task = Task { () -> LinkPreview? in
            guard let content = await fetcher.preview(for: url) else {
                return nil
            }
            // Decoding can be slow; off this actor, other previews keep going.
            return await Task.detached(priority: .utility) {
                Self.makePreview(content)
            }.value
        }
        inFlight[url] = task
        let preview = await task.value
        inFlight[url] = nil
        if let preview {
            previews.insert(preview, forKey: url)
        }
        return preview
    }

    static func makePreview(_ content: LinkPreviewContent) -> LinkPreview {
        let host = hostName(for: content.url)
        let siteName = content.siteName?.trimmingCharacters(in: .whitespacesAndNewlines)
        return LinkPreview(
            url: content.url,
            title: content.title,
            description: content.description.flatMap { $0.isEmpty ? nil : $0 },
            host: host,
            siteName: siteName.flatMap { name in
                name.isEmpty || name.caseInsensitiveCompare(host) == .orderedSame ? nil : name
            },
            image: content.imageData.flatMap(thumbnail(from:))
        )
    }

    static func hostName(for url: URL) -> String {
        let host = url.host?.lowercased() ?? url.absoluteString
        return host.hasPrefix("www.") ? String(host.dropFirst(4)) : host
    }

    /// A small thumbnail of `data`, or `nil` when it is not an image ImageIO
    /// reads or its canvas is too large to decode.
    static func thumbnail(from data: Data) -> UIImage? {
        guard let source = CGImageSourceCreateWithData(data as CFData, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let width = properties[kCGImagePropertyPixelWidth] as? Int,
              let height = properties[kCGImagePropertyPixelHeight] as? Int,
              width > 0, height > 0,
              width.multipliedReportingOverflow(by: height).overflow == false,
              width * height <= maxSourcePixels
        else {
            return nil
        }
        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceShouldCacheImmediately: true,
            kCGImageSourceThumbnailMaxPixelSize: maxImagePixels,
        ]
        guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else {
            return nil
        }
        return UIImage(cgImage: image)
    }
}
