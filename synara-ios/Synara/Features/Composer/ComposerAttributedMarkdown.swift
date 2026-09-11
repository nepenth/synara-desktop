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
            if let html = pasteboard.value(forPasteboardType: UTType.html.identifier) as? String,
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
        project(attributed).markdown
    }

    /// Convert a visible UITextView range into UTF-16 offsets in the markdown draft.
    static func markdownSelection(
        from attributed: NSAttributedString,
        visibleRange: NSRange
    ) -> ComposerTextSelection {
        let projection = project(attributed)
        let start = clamp(visibleRange.location, max: attributed.length)
        let end = clamp(visibleRange.location + visibleRange.length, max: attributed.length)
        let mdStart = projection.attrToMarkdown[start]
        let mdEnd = projection.attrToMarkdown[end]
        return ComposerTextSelection(location: mdStart, length: max(0, mdEnd - mdStart))
    }

    /// Convert a markdown-draft range back into the visible attributed string.
    static func visibleRange(
        in attributed: NSAttributedString,
        markdownSelection: ComposerTextSelection
    ) -> NSRange {
        let projection = project(attributed)
        let mdLength = projection.attrToMarkdown.last ?? 0
        let mdStart = clamp(markdownSelection.location, max: mdLength)
        let mdEnd = clamp(markdownSelection.location + markdownSelection.length, max: mdLength)
        let start = attributedIndex(attrToMarkdown: projection.attrToMarkdown, markdownOffset: mdStart)
        let end = attributedIndex(attrToMarkdown: projection.attrToMarkdown, markdownOffset: mdEnd)
        return NSRange(location: start, length: max(0, end - start))
    }

    private struct MarkdownProjection {
        let markdown: String
        let attrToMarkdown: [Int]
    }

    private static func project(_ attributed: NSAttributedString) -> MarkdownProjection {
        guard attributed.length > 0 else {
            return MarkdownProjection(markdown: "", attrToMarkdown: [0])
        }
        var runs: [(attributes: [NSAttributedString.Key: Any], range: NSRange)] = []
        attributed.enumerateAttributes(
            in: NSRange(location: 0, length: attributed.length),
            options: []
        ) { attributes, range, _ in
            runs.append((attributes, range))
        }
        var output = ""
        var attrToMarkdown = Array(repeating: 0, count: attributed.length + 1)
        var mdOffset = 0
        for run in runs {
            let substring = (attributed.string as NSString).substring(with: run.range)
            let piece = wrappedMarkdown(substring, attributes: run.attributes)
            mapRun(
                attributedRange: run.range,
                original: substring,
                piece: piece,
                attrToMarkdown: &attrToMarkdown,
                mdOffset: &mdOffset
            )
            output += piece
        }
        attrToMarkdown[attributed.length] = mdOffset
        return MarkdownProjection(markdown: output, attrToMarkdown: attrToMarkdown)
    }

    private static func mapRun(
        attributedRange range: NSRange,
        original: String,
        piece: String,
        attrToMarkdown: inout [Int],
        mdOffset: inout Int
    ) {
        let pieceNS = piece as NSString
        let originalNS = original as NSString
        if piece == original {
            for index in 0 ..< range.length {
                attrToMarkdown[range.location + index] = mdOffset + index
            }
            mdOffset += pieceNS.length
            return
        }

        let escaped = escapeMarkdown(original)
        let escapedRange = pieceNS.range(of: escaped)
        let originalRange = pieceNS.range(of: original)
        let inner: String
        if escaped != original, escapedRange.location != NSNotFound {
            inner = escaped
        } else if originalRange.location != NSNotFound {
            inner = original
        } else {
            for index in 0 ..< range.length {
                attrToMarkdown[range.location + index] = mdOffset
            }
            mdOffset += pieceNS.length
            return
        }

        let prefixLen = pieceNS.range(of: inner).location
        if inner == original {
            for index in 0 ..< range.length {
                attrToMarkdown[range.location + index] = mdOffset + prefixLen + index
            }
        } else {
            var innerOffset = 0
            for index in 0 ..< originalNS.length {
                let character = originalNS.substring(with: NSRange(location: index, length: 1))
                let escapedCharacter = escapeMarkdown(character)
                attrToMarkdown[range.location + index] = mdOffset + prefixLen + innerOffset
                innerOffset += (escapedCharacter as NSString).length
            }
        }
        mdOffset += pieceNS.length
    }

    private static func attributedIndex(attrToMarkdown: [Int], markdownOffset: Int) -> Int {
        var result = 0
        for (index, mapped) in attrToMarkdown.enumerated() {
            if mapped <= markdownOffset {
                result = index
            } else {
                break
            }
        }
        return result
    }

    private static func clamp(_ value: Int, max upper: Int) -> Int {
        max(0, min(value, upper))
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
