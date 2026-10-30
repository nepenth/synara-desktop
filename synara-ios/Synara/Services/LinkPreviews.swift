import Foundation
import LinkPresentation
import Network
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
        var isEnabled: Bool
        var isAgentApproval = false
        var hasInlineGif = false
        var hasXPost = false
    }

    static func previewURL(for row: Row) -> URL? {
        guard row.isEnabled,
              let body = row.body,
              body.contains("."),
              row.isAgentApproval == false,
              row.hasInlineGif == false,
              row.hasXPost == false,
              let first = SynaraPlainTextLinks.links(in: body).first?.url,
              isPreviewable(first)
        else {
            return nil
        }
        return first
    }

    /// An http(s) link without credentials whose host is not this device, the
    /// local network or a private address written as a literal IP.
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

/// What a preview card shows.
struct LinkPreview {
    let url: URL
    let title: String
    let siteName: String
    let image: UIImage?
}

/// Previews by URL for this session, fetched on this device with Apple's
/// LinkPresentation (nothing goes through the homeserver, so encrypted rooms
/// preview too). Failures are remembered for a few minutes.
@MainActor
final class LinkPreviewStore {
    static let shared = LinkPreviewStore()

    private static let timeout: TimeInterval = 8
    private static let failureTTL: TimeInterval = 5 * 60
    private static let maxImageSide: CGFloat = 600
    private static let maxTitleCharacters = 300

    private var previews = BoundedLRUCache<URL, LinkPreview>(capacity: 200)
    private var failures = BoundedLRUCache<URL, Date>(capacity: 200)
    private var inFlight: [URL: Task<LinkPreview?, Never>] = [:]

    func preview(for url: URL) async -> LinkPreview? {
        guard LinkPreviewPolicy.isPreviewable(url) else {
            return nil
        }
        if let cached = previews.value(forKey: url) {
            return cached
        }
        if let failed = failures.value(forKey: url), Date().timeIntervalSince(failed) < Self.failureTTL {
            return nil
        }
        if let task = inFlight[url] {
            return await task.value
        }
        let task = Task { await Self.fetch(url) }
        inFlight[url] = task
        let preview = await task.value
        inFlight[url] = nil
        if let preview {
            previews.insert(preview, forKey: url)
        } else {
            failures.insert(Date(), forKey: url)
        }
        return preview
    }

    private static func fetch(_ url: URL) async -> LinkPreview? {
        let provider = LPMetadataProvider()
        provider.timeout = timeout
        guard let metadata = try? await provider.startFetchingMetadata(for: url),
              let rawTitle = metadata.title?.trimmingCharacters(in: .whitespacesAndNewlines),
              rawTitle.isEmpty == false
        else {
            return nil
        }
        let title = rawTitle.count > maxTitleCharacters
            ? String(rawTitle.prefix(maxTitleCharacters - 1)) + "…"
            : rawTitle
        let image = await loadImage(metadata.imageProvider)
        return LinkPreview(url: url, title: title, siteName: siteName(for: url), image: image)
    }

    nonisolated static func siteName(for url: URL) -> String {
        let host = url.host?.lowercased() ?? url.absoluteString
        return host.hasPrefix("www.") ? String(host.dropFirst(4)) : host
    }

    private static func loadImage(_ provider: NSItemProvider?) async -> UIImage? {
        guard let provider, provider.canLoadObject(ofClass: UIImage.self) else {
            return nil
        }
        let image: UIImage? = await withCheckedContinuation { continuation in
            provider.loadObject(ofClass: UIImage.self) { object, _ in
                continuation.resume(returning: object as? UIImage)
            }
        }
        guard let image else { return nil }
        let side = max(image.size.width, image.size.height)
        guard side > maxImageSide else { return image }
        let scale = maxImageSide / side
        let size = CGSize(width: image.size.width * scale, height: image.size.height * scale)
        return UIGraphicsImageRenderer(size: size).image { _ in
            image.draw(in: CGRect(origin: .zero, size: size))
        }
    }
}
