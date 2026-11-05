import Foundation

/// UTF-16 range in the composer markdown draft, not the visible attributed
/// string after a rich paste.
struct ComposerTextSelection: Equatable {
    var location: Int
    var length: Int

    static let empty = ComposerTextSelection(location: 0, length: 0)

    var upperBound: Int {
        location + length
    }
}

enum ComposerMarkdownFormat: String, CaseIterable, Identifiable {
    case bold
    case italic
    case underline
    case strikethrough
    case inlineCode
    case spoiler
    case blockquote
    case codeBlock
    case numberedList
    case bulletList
    case heading1
    case heading2
    case heading3

    var id: String { rawValue }

    var headingLabel: String? {
        switch self {
        case .heading1: return "H1"
        case .heading2: return "H2"
        case .heading3: return "H3"
        default: return nil
        }
    }

    var accessibilityLabel: String {
        switch self {
        case .bold:
            return String(localized: "Bold")
        case .italic:
            return String(localized: "Italic")
        case .underline:
            return String(localized: "Underline")
        case .strikethrough:
            return String(localized: "Strikethrough")
        case .spoiler:
            return String(localized: "Spoiler")
        case .inlineCode:
            return String(localized: "Inline code")
        case .codeBlock:
            return String(localized: "Code block")
        case .blockquote:
            return String(localized: "Quote")
        case .bulletList:
            return String(localized: "Bulleted list")
        case .numberedList:
            return String(localized: "Numbered list")
        case .heading1:
            return String(localized: "Heading 1")
        case .heading2:
            return String(localized: "Heading 2")
        case .heading3:
            return String(localized: "Heading 3")
        }
    }

    var systemImage: String {
        switch self {
        case .bold:
            return "bold"
        case .italic:
            return "italic"
        case .underline:
            return "underline"
        case .strikethrough:
            return "strikethrough"
        case .spoiler:
            return "eye.slash"
        case .inlineCode:
            return "chevron.left.forwardslash.chevron.right"
        case .codeBlock:
            return "curlybraces"
        case .blockquote:
            return "text.quote"
        case .bulletList:
            return "list.bullet"
        case .numberedList:
            return "list.number"
        case .heading1:
            return "textformat.size.larger"
        case .heading2:
            return "textformat.size"
        case .heading3:
            return "textformat.size.smaller"
        }
    }
}

enum ComposerMarkdown {
    static func apply(
        _ format: ComposerMarkdownFormat,
        to text: String,
        selection: ComposerTextSelection
    ) -> (text: String, selection: ComposerTextSelection) {
        let nsText = text as NSString
        let clampedLocation = max(0, min(selection.location, nsText.length))
        let maxLength = max(0, nsText.length - clampedLocation)
        let clampedLength = max(0, min(selection.length, maxLength))
        let clampedSelection = ComposerTextSelection(location: clampedLocation, length: clampedLength)

        switch format {
        case .bold:
            return wrap(text: text, selection: clampedSelection, prefix: "**", suffix: "**", placeholder: "bold text")
        case .italic:
            return wrap(text: text, selection: clampedSelection, prefix: "_", suffix: "_", placeholder: "italic text")
        case .underline:
            return wrap(text: text, selection: clampedSelection, prefix: "<u>", suffix: "</u>", placeholder: "underlined text")
        case .strikethrough:
            return wrap(text: text, selection: clampedSelection, prefix: "~~", suffix: "~~", placeholder: "strikethrough")
        case .spoiler:
            return wrap(text: text, selection: clampedSelection, prefix: "<span data-mx-spoiler>", suffix: "</span>", placeholder: "spoiler")
        case .inlineCode:
            return wrap(text: text, selection: clampedSelection, prefix: "`", suffix: "`", placeholder: "code")
        case .codeBlock:
            return applyCodeBlock(to: text, selection: clampedSelection)
        case .blockquote:
            return prefixLines(in: text, selection: clampedSelection, prefix: "> ", placeholder: "quoted text")
        case .bulletList:
            return prefixLines(in: text, selection: clampedSelection, prefix: "- ", placeholder: "list item")
        case .numberedList:
            return applyNumberedList(to: text, selection: clampedSelection)
        case .heading1:
            return applyHeading(level: 1, to: text, selection: clampedSelection)
        case .heading2:
            return applyHeading(level: 2, to: text, selection: clampedSelection)
        case .heading3:
            return applyHeading(level: 3, to: text, selection: clampedSelection)
        }
    }

    private static func wrap(
        text: String,
        selection: ComposerTextSelection,
        prefix: String,
        suffix: String,
        placeholder: String
    ) -> (text: String, selection: ComposerTextSelection) {
        let nsText = text as NSString
        if selection.length > 0 {
            let prefixLength = (prefix as NSString).length
            let suffixLength = (suffix as NSString).length
            if selection.location >= prefixLength,
               selection.upperBound + suffixLength <= nsText.length,
               nsText.substring(with: NSRange(
                   location: selection.location - prefixLength,
                   length: prefixLength
               )) == prefix,
               nsText.substring(with: NSRange(
                   location: selection.upperBound,
                   length: suffixLength
               )) == suffix
            {
                let selected = nsText.substring(with: NSRange(
                    location: selection.location,
                    length: selection.length
                ))
                let updated = nsText.replacingCharacters(
                    in: NSRange(
                        location: selection.location - prefixLength,
                        length: prefixLength + selection.length + suffixLength
                    ),
                    with: selected
                )
                return (
                    updated,
                    ComposerTextSelection(location: selection.location - prefixLength, length: selection.length)
                )
            }
            let selected = nsText.substring(with: NSRange(location: selection.location, length: selection.length))
            let wrapped = "\(prefix)\(selected)\(suffix)"
            let updated = nsText.replacingCharacters(in: NSRange(location: selection.location, length: selection.length), with: wrapped)
            let newSelection = ComposerTextSelection(
                location: selection.location + prefix.count,
                length: selected.utf16.count
            )
            return (updated, newSelection)
        }

        let insertion = "\(prefix)\(placeholder)\(suffix)"
        let updated = nsText.replacingCharacters(in: NSRange(location: selection.location, length: 0), with: insertion)
        let newSelection = ComposerTextSelection(
            location: selection.location + prefix.count,
            length: placeholder.utf16.count
        )
        return (updated, newSelection)
    }

    private static func applyCodeBlock(
        to text: String,
        selection: ComposerTextSelection
    ) -> (text: String, selection: ComposerTextSelection) {
        let nsText = text as NSString
        let selected = selection.length > 0
            ? nsText.substring(with: NSRange(location: selection.location, length: selection.length))
            : "code"
        // The fence needs lines of its own: break only where the text does not.
        let before = selection.location > 0
            && nsText.substring(with: NSRange(location: selection.location - 1, length: 1)) != "\n"
            ? "\n" : ""
        let after = selection.upperBound < nsText.length
            && nsText.substring(with: NSRange(location: selection.upperBound, length: 1)) != "\n"
            ? "\n" : ""
        let opening = "\(before)```\n"
        let block = "\(opening)\(selected)\n```\(after)"
        let updated = nsText.replacingCharacters(in: NSRange(location: selection.location, length: selection.length), with: block)
        let contentStart = selection.location + opening.utf16.count
        return (updated, ComposerTextSelection(location: contentStart, length: selected.utf16.count))
    }

    private static func prefixLines(
        in text: String,
        selection: ComposerTextSelection,
        prefix: String,
        placeholder: String
    ) -> (text: String, selection: ComposerTextSelection) {
        let nsText = text as NSString
        let lineRange = selectedLineRange(in: nsText, selection: selection)
        let selectedLines = nsText.substring(with: lineRange)
        let hasTrailingNewline = selectedLines.hasSuffix("\n")
        let content = hasTrailingNewline ? String(selectedLines.dropLast()) : selectedLines

        let lines: [String]
        if content.isEmpty {
            lines = [placeholder]
        } else {
            lines = content.components(separatedBy: "\n")
        }

        if content.isEmpty == false, lines.allSatisfy({ $0.hasPrefix(prefix) }) {
            let unprefixed = lines.map { String($0.dropFirst(prefix.count)) }.joined(separator: "\n")
            let updated = nsText.replacingCharacters(
                in: lineRange,
                with: unprefixed + (hasTrailingNewline ? "\n" : "")
            )
            let length = selection.length > 0
                ? (unprefixed as NSString).length
                : (lines.first.map { String($0.dropFirst(prefix.count)) } ?? "").utf16.count
            return (updated, ComposerTextSelection(location: lineRange.location, length: length))
        }

        // A line keeps one block marker: a bullet made a quote loses its bullet.
        let prefixed = lines
            .map { line in
                "\(prefix)\(strippingBlockMarker(line))"
            }
            .joined(separator: "\n")

        let replacement = prefixed + (hasTrailingNewline ? "\n" : "")
        let updated = nsText.replacingCharacters(in: lineRange, with: replacement)
        let contentStart = lineRange.location + prefix.utf16.count
        let selectedLength = selection.length > 0
            ? (prefixed as NSString).length - prefix.utf16.count
            : (lines.first ?? placeholder).utf16.count
        return (updated, ComposerTextSelection(location: contentStart, length: selectedLength))
    }

    private static func applyNumberedList(
        to text: String,
        selection: ComposerTextSelection
    ) -> (text: String, selection: ComposerTextSelection) {
        let nsText = text as NSString
        let lineRange = selectedLineRange(in: nsText, selection: selection)
        let selectedLines = nsText.substring(with: lineRange)
        let hasTrailingNewline = selectedLines.hasSuffix("\n")
        let content = hasTrailingNewline ? String(selectedLines.dropLast()) : selectedLines

        let lines: [String]
        if content.isEmpty {
            lines = ["list item"]
        } else {
            lines = content.components(separatedBy: "\n")
        }

        let numberedPrefixes = lines.map { line -> String? in
            guard let match = line.range(of: #"^\d+\. "#, options: .regularExpression) else {
                return nil
            }
            return String(line[match])
        }
        if content.isEmpty == false, numberedPrefixes.allSatisfy({ $0 != nil }) {
            let unprefixed = zip(lines, numberedPrefixes).map { pair in
                String(pair.0.dropFirst(pair.1?.count ?? 0))
            }.joined(separator: "\n")
            let updated = nsText.replacingCharacters(
                in: lineRange,
                with: unprefixed + (hasTrailingNewline ? "\n" : "")
            )
            let firstPrefixLength = numberedPrefixes.first.flatMap { $0 }?.count ?? 0
            let length = selection.length > 0
                ? (unprefixed as NSString).length
                : (lines.first.map { String($0.dropFirst(firstPrefixLength)) } ?? "").utf16.count
            return (updated, ComposerTextSelection(location: lineRange.location, length: length))
        }

        let first = continuedListNumber(in: nsText, before: lineRange.location)
        let prefixed = lines.enumerated().map { index, line in
            "\(first + index). \(strippingBlockMarker(line))"
        }.joined(separator: "\n")

        let replacement = prefixed + (hasTrailingNewline ? "\n" : "")
        let updated = nsText.replacingCharacters(in: lineRange, with: replacement)
        let prefix = "\(first). "
        let contentStart = lineRange.location + prefix.utf16.count
        let selectedLength = selection.length > 0
            ? (prefixed as NSString).length - prefix.utf16.count
            : (lines.first ?? "list item").utf16.count
        return (updated, ComposerTextSelection(location: contentStart, length: selectedLength))
    }

    private static func applyHeading(
        level: Int,
        to text: String,
        selection: ComposerTextSelection
    ) -> (text: String, selection: ComposerTextSelection) {
        let nsText = text as NSString
        let lineRange = selectedLineRange(in: nsText, selection: selection)
        let selectedLines = nsText.substring(with: lineRange)
        let trailingNewline = selectedLines.hasSuffix("\n")
        let content = trailingNewline ? String(selectedLines.dropLast()) : selectedLines
        let lines = content.isEmpty ? ["Heading"] : content.components(separatedBy: "\n")
        // Heading 1-3 are <h1>-<h3> on the wire, as desktop sends them.
        let requestedPrefix = String(repeating: "#", count: level) + " "
        let allAtRequestedLevel = content.isEmpty == false
            && lines.allSatisfy { $0.hasPrefix(requestedPrefix) }
        let transformed = lines.map { line -> String in
            let bare = strippingBlockMarker(line)
            return allAtRequestedLevel ? bare : requestedPrefix + bare
        }.joined(separator: "\n")
        let updated = nsText.replacingCharacters(
            in: lineRange,
            with: transformed + (trailingNewline ? "\n" : "")
        )
        let firstPrefixLength = allAtRequestedLevel ? 0 : requestedPrefix.utf16.count
        let length = selection.length > 0
            ? (transformed as NSString).length - firstPrefixLength
            : (lines.first ?? "Heading").utf16.count
        return (
            updated,
            ComposerTextSelection(location: lineRange.location + firstPrefixLength, length: length)
        )
    }

    private static func selectedLineRange(in text: NSString, selection: ComposerTextSelection) -> NSRange {
        var length = selection.length
        // A selection ending at the start of the next line must not format
        // that unselected line as well.
        if length > 0, selection.upperBound < text.length,
           text.substring(with: NSRange(location: selection.upperBound - 1, length: 1)) == "\n"
        {
            length -= 1
        }
        return text.lineRange(for: NSRange(location: selection.location, length: length))
    }


    // MARK: - Block markers and line breaks

    private static let blockMarker = try? NSRegularExpression(
        pattern: #"^(#{1,6} |> |[-*+] |\d{1,9}[.)] )"#
    )
    private static let listMarker = try? NSRegularExpression(
        pattern: #"^([ \t]*)(?:([-*+])|(\d{1,9})([.)])|(>))( +)"#
    )

    /// The line without its heading, quote or list marker.
    static func strippingBlockMarker(_ line: String) -> String {
        let ns = line as NSString
        guard let match = blockMarker?.firstMatch(in: line, range: NSRange(location: 0, length: ns.length)) else {
            return line
        }
        return ns.substring(from: match.range.length)
    }

    /// The number a numbered list starting at `location` continues from:
    /// the nearest top-level numbered line above it, across plain lines,
    /// quotes and code. A heading or a bulleted line restarts at 1, as on
    /// desktop.
    static func continuedListNumber(in text: NSString, before location: Int) -> Int {
        let preceding = text.substring(to: max(0, min(location, text.length)))
        for line in preceding.components(separatedBy: "\n").reversed() {
            if let range = line.range(of: #"^\d{1,9}(?=[.)] )"#, options: .regularExpression),
               let number = Int(line[range])
            {
                return number + 1
            }
            if line.range(of: #"^(#{1,6} |[-*+] )"#, options: .regularExpression) != nil {
                return 1
            }
        }
        return 1
    }

    /// What Return does on a list or quote line: continue it with the next
    /// marker, or, on a line that holds only a marker, clear the marker and
    /// leave the list. `nil` means an ordinary new line.
    struct LineBreakEdit: Equatable {
        /// UTF-16 range to replace, and its replacement.
        var range: NSRange
        var replacement: String
        /// Caret after the edit.
        var caret: Int
    }

    static func lineBreak(in text: String, at location: Int) -> LineBreakEdit? {
        let ns = text as NSString
        guard location >= 0, location <= ns.length else { return nil }
        var lineRange = ns.lineRange(for: NSRange(location: location, length: 0))
        if lineRange.length > 0,
           ns.substring(with: NSRange(location: NSMaxRange(lineRange) - 1, length: 1)) == "\n"
        {
            lineRange.length -= 1
        }
        // Inside a code fence a new line is code.
        let fences = ns.substring(to: lineRange.location)
            .components(separatedBy: "\n")
            .filter { $0.trimmingCharacters(in: .whitespaces).hasPrefix("```") }
            .count
        guard fences % 2 == 0 else { return nil }

        let line = ns.substring(with: lineRange)
        let lineNS = line as NSString
        guard let match = listMarker?.firstMatch(in: line, range: NSRange(location: 0, length: lineNS.length)),
              location >= lineRange.location + match.range.length
        else {
            return nil
        }
        let rest = lineNS.substring(from: match.range.length)
        if rest.trimmingCharacters(in: .whitespaces).isEmpty {
            return LineBreakEdit(range: lineRange, replacement: "", caret: lineRange.location)
        }
        func group(_ index: Int) -> String? {
            let range = match.range(at: index)
            return range.location == NSNotFound ? nil : lineNS.substring(with: range)
        }
        let indent = group(1) ?? ""
        let gap = group(6) ?? " "
        let marker: String
        if let bullet = group(2) {
            marker = bullet
        } else if let number = group(3).flatMap(Int.init) {
            marker = "\(number + 1)\(group(4) ?? ".")"
        } else {
            marker = ">"
        }
        let insertion = "\n\(indent)\(marker)\(gap)"
        return LineBreakEdit(
            range: NSRange(location: location, length: 0),
            replacement: insertion,
            caret: location + (insertion as NSString).length
        )
    }
}
