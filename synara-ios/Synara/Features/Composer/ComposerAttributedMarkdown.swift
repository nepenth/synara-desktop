import Foundation
import UniformTypeIdentifiers
#if canImport(UIKit)
    import UIKit
#endif

enum ComposerPasteboard {
    static func htmlString() -> String? {
        #if canImport(UIKit)
            let pasteboard = UIPasteboard.general
            if let data = pasteboard.data(forPasteboardType: UTType.html.identifier),
               let html = String(data: data, encoding: .utf8),
               html.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            {
                return html
            }
            if let html = pasteboard.string(forPasteboardType: UTType.html.identifier),
               html.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            {
                return html
            }
        #endif
        return nil
    }
}

enum ComposerAttributedMarkdown {
    static func markdown(from attributed: NSAttributedString) -> String {
        guard attributed.length > 0 else {
            return ""
        }
        var output = ""
        attributed.enumerateAttributes(
            in: NSRange(location: 0, length: attributed.length),
            options: []
        ) { attributes, range, _ in
            let substring = (attributed.string as NSString).substring(with: range)
            output += wrappedMarkdown(substring, attributes: attributes)
        }
        return output
    }

    #if canImport(UIKit)
        static func attributedString(fromHTML html: String, baseFont: UIFont) -> NSAttributedString? {
            guard let sanitized = MatrixHTMLRenderer.sanitizedHTMLForClipboard(html: html),
                  let data = sanitized.data(using: .utf8)
            else {
                return nil
            }
            guard let parsed = try? NSMutableAttributedString(
                data: data,
                options: [
                    .documentType: NSAttributedString.DocumentType.html,
                    .characterEncoding: String.Encoding.utf8.rawValue,
                ],
                documentAttributes: nil
            ) else {
                return nil
            }

            while parsed.length > 0 {
                let last = parsed.attributedSubstring(
                    from: NSRange(location: parsed.length - 1, length: 1)
                ).string
                if last == "\n" || last == "\r" {
                    parsed.deleteCharacters(in: NSRange(location: parsed.length - 1, length: 1))
                } else {
                    break
                }
            }

            parsed.enumerateAttribute(
                .font,
                in: NSRange(location: 0, length: parsed.length)
            ) { value, range, _ in
                let current = (value as? UIFont) ?? baseFont
                var traits = current.fontDescriptor.symbolicTraits
                if current.fontDescriptor.symbolicTraits.contains(.traitBold) {
                    traits.insert(.traitBold)
                }
                if current.fontDescriptor.symbolicTraits.contains(.traitItalic) {
                    traits.insert(.traitItalic)
                }
                if current.fontDescriptor.symbolicTraits.contains(.traitMonoSpace) {
                    traits.insert(.traitMonoSpace)
                }
                let descriptor = baseFont.fontDescriptor.withSymbolicTraits(traits) ?? baseFont.fontDescriptor
                parsed.addAttribute(.font, value: UIFont(descriptor: descriptor, size: baseFont.pointSize), range: range)
            }
            if parsed.length > 0 {
                var attachmentRanges: [NSRange] = []
                parsed.enumerateAttribute(
                    .attachment,
                    in: NSRange(location: 0, length: parsed.length)
                ) { value, range, _ in
                    if value != nil {
                        attachmentRanges.append(range)
                    }
                }
                for range in attachmentRanges.reversed() {
                    parsed.deleteCharacters(in: range)
                }
            }
            return parsed
        }
    #endif

    private static func wrappedMarkdown(_ text: String, attributes: [NSAttributedString.Key: Any]) -> String {
        if text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return text
        }

        let font = attributes[.font] as? UIFont
        let traits = font?.fontDescriptor.symbolicTraits ?? []
        let weight = fontWeight(font)
        let isBold = traits.contains(.traitBold) || weight >= 0.2
        let obliqueness = (attributes[.obliqueness] as? NSNumber)?.doubleValue ?? 0
        let isItalic = traits.contains(.traitItalic) || abs(obliqueness) > 0.01
        let isMono = traits.contains(.traitMonoSpace)
        let strikeValue = attributes[.strikethroughStyle] as? Int ?? 0
        let isStrike = strikeValue != 0

        if isMono, text.contains(where: { $0.isNewline }) {
            return "\n```\n\(text)\n```\n"
        }

        var inner = text
        if isBold || isItalic || isStrike || isMono {
            inner = escapeMarkdown(text)
        }

        if isMono {
            inner = "`\(inner)`"
        }
        if isStrike {
            inner = "~~\(inner)~~"
        }
        if isItalic {
            inner = "_\(inner)_"
        }
        if isBold {
            inner = "**\(inner)**"
        }
        if let href = safeLink(attributes) {
            let label = inner.replacingOccurrences(of: "]", with: "\\]")
            inner = "[\(label)](\(href))"
        }
        return inner
    }

    private static func fontWeight(_ font: UIFont?) -> CGFloat {
        guard let font else {
            return 0
        }
        let traits = font.fontDescriptor.object(forKey: UIFontDescriptor.AttributeName.traits)
            as? [UIFontDescriptor.TraitKey: Any]
        return traits?[.weight] as? CGFloat ?? 0
    }

    private static func safeLink(_ attributes: [NSAttributedString.Key: Any]) -> String? {
        let raw: String?
        if let url = attributes[.link] as? URL {
            raw = url.absoluteString
        } else {
            raw = attributes[.link] as? String
        }
        guard let href = raw, href.isEmpty == false else {
            return nil
        }
        guard href.unicodeScalars.allSatisfy({ scalar in
            let value = scalar.value
            return value > 0x1F && value != 0x7F
        }) else {
            return nil
        }
        guard let scheme = URLComponents(string: href)?.scheme?.lowercased() else {
            return nil
        }
        return ["https", "http", "ftp", "mailto", "magnet"].contains(scheme) ? href : nil
    }

    private static func escapeMarkdown(_ text: String) -> String {
        text
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "`", with: "\\`")
            .replacingOccurrences(of: "*", with: "\\*")
            .replacingOccurrences(of: "_", with: "\\_")
    }
}
