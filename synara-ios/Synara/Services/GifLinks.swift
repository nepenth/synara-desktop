import Foundation

/// A Klipy or GIPHY media link that plays in place of its text.
struct InlineGifLink: Equatable, Hashable {
    /// Exactly as sent: the provider's parameters carry delivery data.
    let url: String
    let host: String

    var resolvedURL: URL? {
        URL(string: url)
    }
}

/// Matches desktop `gifLinks.ts`. Klipy and GIPHY require their media to be
/// loaded straight from these hosts, so a GIF sent from the picker travels
/// as its link and the reader plays it.
enum GifLinks {
    private static let inlineHosts: [NSRegularExpression] = [
        #"^static\d*\.klipy\.com$"#,
        #"^media\d*\.giphy\.com$"#,
        #"^i\.giphy\.com$"#,
    ].compactMap { try? NSRegularExpression(pattern: $0) }
    private static let imageExtension = try? NSRegularExpression(
        pattern: #"\.(gif|webp)$"#,
        options: [.caseInsensitive]
    )

    static func inlineGifLink(_ value: String) -> InlineGifLink? {
        guard let components = RemoteContentURL.safeComponents(value),
              let rawHost = components.host
        else {
            return nil
        }
        let host = rawHost.lowercased()
        guard inlineHosts.contains(where: { matches($0, host) }),
              let imageExtension,
              matches(imageExtension, components.percentEncodedPath)
        else {
            return nil
        }
        return InlineGifLink(url: value, host: host)
    }

    /// A message whose whole body is one inline GIF link, as the picker sends it.
    static func messageInlineGif(_ body: String) -> InlineGifLink? {
        let trimmed = body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false,
              trimmed.unicodeScalars.contains(where: { CharacterSet.whitespacesAndNewlines.contains($0) }) == false
        else {
            return nil
        }
        return inlineGifLink(trimmed)
    }

    private static func matches(_ expression: NSRegularExpression, _ value: String) -> Bool {
        expression.firstMatch(in: value, range: NSRange(value.startIndex..., in: value)) != nil
    }
}

/// `/gif` or `/gif <search>` typed as the whole message opens the GIF
/// picker instead of sending; anything else (`/gifs are fun`) is a message.
enum GifCommand {
    static let name = "/gif"

    /// The search to open the picker with, or `nil` when `text` is not the command.
    static func query(in text: String) -> String? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.hasPrefix(name) else {
            return nil
        }
        let rest = trimmed.dropFirst(name.count)
        guard let first = rest.unicodeScalars.first else {
            return ""
        }
        guard CharacterSet.whitespacesAndNewlines.contains(first) else {
            return nil
        }
        return rest.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

/// The rich-link settings desktop has under Settings → General. All four
/// default on; each one controls whether this device contacts a third party.
enum RichLinkSettings {
    static let inlineGifLinksKey = "synara.settings.inlineGifLinks"
    static let gifSearchKey = "synara.settings.gifSearch"
    static let xEmbedsKey = "synara.settings.xEmbeds"
    /// Desktop `linkPreviews`: a title and image card for a message's first
    /// link, fetched from this device.
    static let linkPreviewsKey = "synara.settings.linkPreviews"

    static var registeredDefaults: [String: Bool] {
        [
            inlineGifLinksKey: true,
            gifSearchKey: true,
            xEmbedsKey: true,
            linkPreviewsKey: true,
            DraftSyncSettings.syncDraftsKey: true,
        ]
    }
}
