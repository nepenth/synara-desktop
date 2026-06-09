import Foundation

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
    case strikethrough
    case inlineCode
    case codeBlock
    case blockquote
    case bulletList
    case numberedList

    var id: String { rawValue }

    var accessibilityLabel: String {
        switch self {
        case .bold:
            return "Bold"
        case .italic:
            return "Italic"
        case .strikethrough:
            return "Strikethrough"
        case .inlineCode:
            return "Inline code"
        case .codeBlock:
            return "Code block"
        case .blockquote:
            return "Quote"
        case .bulletList:
            return "Bulleted list"
        case .numberedList:
            return "Numbered list"
        }
    }

    var systemImage: String {
        switch self {
        case .bold:
            return "bold"
        case .italic:
            return "italic"
        case .strikethrough:
            return "strikethrough"
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
        case .strikethrough:
            return wrap(text: text, selection: clampedSelection, prefix: "~~", suffix: "~~", placeholder: "strikethrough")
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
        let block = "\n```\n\(selected)\n```\n"
        let updated = nsText.replacingCharacters(in: NSRange(location: selection.location, length: selection.length), with: block)
        let contentStart = selection.location + "\n```\n".utf16.count
        return (updated, ComposerTextSelection(location: contentStart, length: selected.utf16.count))
    }

    private static func prefixLines(
        in text: String,
        selection: ComposerTextSelection,
        prefix: String,
        placeholder: String
    ) -> (text: String, selection: ComposerTextSelection) {
        let nsText = text as NSString
        let lineRange = nsText.lineRange(for: NSRange(location: selection.location, length: selection.length))
        let selectedLines = nsText.substring(with: lineRange)
        let trimmed = selectedLines.trimmingCharacters(in: .newlines)

        let lines: [String]
        if trimmed.isEmpty {
            lines = [placeholder]
        } else {
            lines = splitLines(selectedLines)
        }

        let prefixed = lines
            .map { line in
                let stripped = line.trimmingCharacters(in: .whitespaces)
                if stripped.isEmpty {
                    return prefix
                }
                return "\(prefix)\(stripped)"
            }
            .joined(separator: "\n")

        let replacement = prefixed + lineEnding(from: nsText, lineRange: lineRange)
        let updated = nsText.replacingCharacters(in: lineRange, with: replacement)
        let firstLine = lines.first ?? placeholder
        let contentStart = lineRange.location + prefix.utf16.count
        return (updated, ComposerTextSelection(location: contentStart, length: firstLine.utf16.count))
    }

    private static func applyNumberedList(
        to text: String,
        selection: ComposerTextSelection
    ) -> (text: String, selection: ComposerTextSelection) {
        let nsText = text as NSString
        let lineRange = nsText.lineRange(for: NSRange(location: selection.location, length: selection.length))
        let selectedLines = nsText.substring(with: lineRange)
        let trimmed = selectedLines.trimmingCharacters(in: .newlines)

        let lines: [String]
        if trimmed.isEmpty {
            lines = ["list item"]
        } else {
            lines = splitLines(selectedLines)
        }

        let prefixed = lines.enumerated().map { index, line in
            let stripped = line.trimmingCharacters(in: .whitespaces)
            if stripped.isEmpty {
                return "\(index + 1). "
            }
            return "\(index + 1). \(stripped)"
        }.joined(separator: "\n")

        let replacement = prefixed + lineEnding(from: nsText, lineRange: lineRange)
        let updated = nsText.replacingCharacters(in: lineRange, with: replacement)
        let firstLine = lines.first ?? "list item"
        let prefix = "1. "
        let contentStart = lineRange.location + prefix.utf16.count
        return (updated, ComposerTextSelection(location: contentStart, length: firstLine.utf16.count))
    }

    private static func lineEnding(from text: NSString, lineRange: NSRange) -> String {
        guard lineRange.upperBound <= text.length else {
            return ""
        }
        if lineRange.upperBound == text.length {
            return ""
        }
        return text.substring(with: NSRange(location: lineRange.upperBound - 1, length: 1)) == "\n" ? "\n" : ""
    }

    private static func splitLines(_ value: String) -> [String] {
        var lines = value
            .split(separator: "\n", omittingEmptySubsequences: false)
            .map(String.init)
        if lines.last?.isEmpty == true {
            lines.removeLast()
        }
        return lines
    }
}