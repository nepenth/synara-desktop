import PhotosUI
import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

#if canImport(UIKit)
enum ComposerTextMetrics {
    static let maxHeight: CGFloat = 240
    static let textContainerInset = UIEdgeInsets(top: 6, left: 0, bottom: 6, right: 0)

    static func singleLineHeight(font: UIFont) -> CGFloat {
        ceil(font.lineHeight) + textContainerInset.top + textContainerInset.bottom
    }
}

enum ComposerHeightMeasurementPolicy {
    static func canReuseCappedHeight(
        previousText: String?,
        currentText: String,
        previousHeight: CGFloat?,
        previousWidth: CGFloat,
        currentWidth: CGFloat,
        previousShowsPlaceholder: Bool?,
        currentShowsPlaceholder: Bool,
        previousFontPointSize: CGFloat,
        currentFontPointSize: CGFloat,
        force: Bool
    ) -> Bool {
        guard force == false,
              previousHeight == ComposerTextMetrics.maxHeight,
              abs(previousWidth - currentWidth) <= 0.5,
              previousShowsPlaceholder == false,
              currentShowsPlaceholder == false,
              abs(previousFontPointSize - currentFontPointSize) <= 0.1,
              let previousText
        else {
            return false
        }
        return currentText.hasPrefix(previousText)
            && currentText.utf16.count > previousText.utf16.count
    }
}

enum ComposerTextInputRegistry {
    private(set) static weak var activeTextView: UITextView?

    static func register(_ textView: UITextView) {
        activeTextView = textView
    }

    static func dismissKeyboard() {
        activeTextView?.resignFirstResponder()
        UIApplication.shared.sendAction(
            #selector(UIResponder.resignFirstResponder),
            to: nil,
            from: nil,
            for: nil
        )
    }

    static func selectionForFormatting(
        _ format: ComposerMarkdownFormat,
        fallback: ComposerTextSelection
    ) -> ComposerTextSelection {
        guard let textView = activeTextView as? ComposerPasteTextView else {
            return fallback
        }
        return textView.selectionForFormatting(format, fallback: fallback)
    }
}

struct ComposerTextView: UIViewRepresentable {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.synaraThemeBaseHex) private var themeBaseHex
    @Binding var text: String
    @Binding var selection: ComposerTextSelection
    @Binding var height: CGFloat
    var placeholder: String
    var formattingRevision: Int
    var isFocused: FocusState<Bool>.Binding
    var onPasteImages: ([UIImage]) -> Void = { _ in }

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    func makeUIView(context: Context) -> ComposerTextContainer {
        let container = ComposerTextContainer()
        let textView = container.textView
        textView.backgroundColor = .clear
        textView.font = .preferredFont(forTextStyle: .callout)
        textView.adjustsFontForContentSizeCategory = true
        textView.textContainerInset = ComposerTextMetrics.textContainerInset
        textView.textContainer.lineFragmentPadding = 0
        textView.isScrollEnabled = false
        textView.keyboardDismissMode = .interactive
        textView.accessibilityIdentifier = "ComposerTextField"
        textView.accessibilityLabel = "Message"
        textView.accessibilityHint = "Enter a message for this room"
        textView.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        container.placeholderLabel.text = placeholder
        container.placeholderLabel.font = textView.font
        container.placeholderLabel.adjustsFontForContentSizeCategory = true
        container.placeholderLabel.textColor = .placeholderText
        context.coordinator.container = container
        context.coordinator.lastFormattingRevision = formattingRevision
        context.coordinator.lastPlaceholder = placeholder
        context.coordinator.performProgrammaticUpdate {
            textView.text = text
            applyTextAppearance(to: textView)
            textView.refreshQuotePresentation()
            applySelection(to: textView)
        }
        context.coordinator.lastAppearanceKey = appearanceKey
        textView.delegate = context.coordinator
        textView.onPasteImages = onPasteImages
        container.onWidthChange = { [weak coordinator = context.coordinator] in
            guard let coordinator, let textView = coordinator.container?.textView else {
                return
            }
            coordinator.updateHeight(for: textView)
        }
        ComposerTextInputRegistry.register(textView)
        context.coordinator.syncPlaceholder()
        context.coordinator.updateHeight(for: textView, force: true)
        return container
    }

    func updateUIView(_ uiView: ComposerTextContainer, context: Context) {
        let textView = uiView.textView
        context.coordinator.parent = self
        textView.onPasteImages = onPasteImages
        context.coordinator.performProgrammaticUpdate {
            var replacedText = false
            if context.coordinator.lastPlaceholder != placeholder {
                context.coordinator.lastPlaceholder = placeholder
                uiView.placeholderLabel.text = placeholder
                context.coordinator.refreshAccessibilityPlaceholder()
            }

            if uiView.placeholderLabel.font != textView.font {
                uiView.placeholderLabel.font = textView.font
            }

            if context.coordinator.lastFormattingRevision != formattingRevision {
                context.coordinator.lastFormattingRevision = formattingRevision
                textView.text = text
                replacedText = true
                applySelection(to: textView)
                context.coordinator.syncPlaceholder()
            } else if textView.isFirstResponder == false,
                      ComposerAttributedMarkdown.markdown(
                          from: textView.attributedText,
                          baseFont: ComposerAttributedMarkdown.composerBaseFont(for: textView)
                      ) != text
            {
                textView.text = text
                replacedText = true
                applySelection(to: textView)
                context.coordinator.syncPlaceholder()
            }
            let appearanceChanged = context.coordinator.lastAppearanceKey != appearanceKey
            if replacedText || appearanceChanged {
                applyTextAppearance(to: textView)
                context.coordinator.lastAppearanceKey = appearanceKey
                textView.refreshQuotePresentation()
            }
        }

        if isFocused.wrappedValue, textView.isFirstResponder == false {
            textView.becomeFirstResponder()
        }

        context.coordinator.syncPlaceholder()
        context.coordinator.updateHeight(for: textView)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: ComposerTextContainer, context: Context) -> CGSize? {
        let width = proposal.width ?? uiView.bounds.width
        guard width > 0 else {
            return nil
        }
        // The coordinator measures after content or width changes and publishes
        // `height`. Measuring again here forces a second synchronous text
        // layout during the same keystroke.
        return CGSize(width: width, height: height)
    }

    private func applySelection(to textView: UITextView) {
        let desiredRange = ComposerAttributedMarkdown.visibleRange(
            in: textView.attributedText,
            markdownSelection: selection,
            baseFont: ComposerAttributedMarkdown.composerBaseFont(for: textView)
        )
        guard desiredRange.upperBound <= textView.attributedText.length else {
            return
        }
        textView.selectedRange = desiredRange
    }

    private var appearanceKey: String {
        "\(themeBaseHex):\(colorScheme == .dark ? "dark" : "light")"
    }

    private func applyTextAppearance(to textView: UITextView) {
        let font = textView.font ?? .preferredFont(forTextStyle: .callout)
        // SwiftUI's room can use a preferred color scheme that does not match
        // the UIKit text view's inherited trait collection. Resolve against
        // the same theme and scheme as the composer surface.
        let color = SynaraThemeRamp.uiColor(
            \.primaryText,
            baseHex: themeBaseHex,
            dark: colorScheme == .dark
        )
        textView.font = font
        textView.textColor = color
        textView.tintColor = color
        textView.linkTextAttributes = [
            .foregroundColor: color
        ]
        if textView.textStorage.length > 0 {
            textView.textStorage.addAttribute(
                .foregroundColor,
                value: color,
                range: NSRange(location: 0, length: textView.textStorage.length)
            )
        }
        var typingAttributes = textView.typingAttributes
        typingAttributes[.font] = typingAttributes[.font] ?? font
        typingAttributes[.foregroundColor] = color
        textView.typingAttributes = typingAttributes
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        var parent: ComposerTextView
        weak var container: ComposerTextContainer?
        var lastFormattingRevision = -1
        var lastPlaceholder = ""
        var lastAppearanceKey = ""
        private var isApplyingProgrammaticState = false
        private var lastMeasuredText: String?
        private var lastMeasuredWidth: CGFloat = 0
        private var lastMeasuredShowsPlaceholder: Bool?
        private var lastMeasuredFontPointSize: CGFloat = 0
        private var lastMeasuredHeight: CGFloat?
        private var lastAccessibilityShowsPlaceholder: Bool?

        init(parent: ComposerTextView) {
            self.parent = parent
        }

        func publishContent(from textView: UITextView) {
            let markdown = ComposerAttributedMarkdown.markdown(
                from: textView.attributedText,
                baseFont: ComposerAttributedMarkdown.composerBaseFont(for: textView)
            )
            if parent.text != markdown {
                parent.text = markdown
            }
            updateSelection(from: textView)
            syncPlaceholder()
        }

        func textViewDidChange(_ textView: UITextView) {
            guard isApplyingProgrammaticState == false else {
                return
            }
            let traceID = PerformanceTrace.begin("ComposerTextChange")
            defer { PerformanceTrace.end("ComposerTextChange", id: traceID) }
            (textView as? ComposerPasteTextView)?.refreshQuotePresentation()
            publishContent(from: textView)
            updateHeight(for: textView)
        }

        func scrollViewDidScroll(_ scrollView: UIScrollView) {
            (scrollView as? ComposerPasteTextView)?.updateQuoteBars()
        }

        func textViewDidChangeSelection(_ textView: UITextView) {
            guard isApplyingProgrammaticState == false else {
                return
            }
            (textView as? ComposerPasteTextView)?.invalidateRecentPasteIfSelectionMoved()
            updateSelection(from: textView)
        }

        func textViewDidBeginEditing(_ textView: UITextView) {
            if parent.isFocused.wrappedValue == false {
                parent.isFocused.wrappedValue = true
            }
            syncPlaceholder()
            updateHeight(for: textView)
        }

        func textViewDidEndEditing(_ textView: UITextView) {
            if parent.isFocused.wrappedValue {
                parent.isFocused.wrappedValue = false
            }
            parent.text = ComposerAttributedMarkdown.markdown(
                from: textView.attributedText,
                baseFont: ComposerAttributedMarkdown.composerBaseFont(for: textView)
            )
            updateSelection(from: textView)
            syncPlaceholder()
            updateHeight(for: textView)
        }

        func performProgrammaticUpdate(_ update: () -> Void) {
            isApplyingProgrammaticState = true
            update()
            isApplyingProgrammaticState = false
        }

        func updateHeight(for textView: UITextView, force: Bool = false) {
            guard let container = container else {
                return
            }
            let width = container.bounds.width
            guard width > 0 else {
                return
            }

            let measuredHeight = preferredHeight(for: textView, width: width, force: force)
            let showsPlaceholder = textView.text.isEmpty
            textView.isScrollEnabled = showsPlaceholder == false
                && measuredHeight >= ComposerTextMetrics.maxHeight

            guard abs(parent.height - measuredHeight) > 0.5 else {
                return
            }
            DispatchQueue.main.async { [weak self] in
                guard let self else {
                    return
                }
                self.parent.height = measuredHeight
            }
        }

        func preferredHeight(for textView: UITextView, width: CGFloat, force: Bool = false) -> CGFloat {
            guard let container else { return parent.height }
            let showsPlaceholder = textView.text.isEmpty
            let fontPointSize = textView.font?.pointSize ?? 0
            let canReuseCappedHeight = ComposerHeightMeasurementPolicy.canReuseCappedHeight(
                previousText: lastMeasuredText,
                currentText: textView.text,
                previousHeight: lastMeasuredHeight,
                previousWidth: lastMeasuredWidth,
                currentWidth: width,
                previousShowsPlaceholder: lastMeasuredShowsPlaceholder,
                currentShowsPlaceholder: showsPlaceholder,
                previousFontPointSize: lastMeasuredFontPointSize,
                currentFontPointSize: fontPointSize,
                force: force
            )
            if canReuseCappedHeight {
                lastMeasuredText = textView.text
                return ComposerTextMetrics.maxHeight
            }
            guard force
                || lastMeasuredText != textView.text
                || abs(lastMeasuredWidth - width) > 0.5
                || lastMeasuredShowsPlaceholder != showsPlaceholder
                || abs(lastMeasuredFontPointSize - fontPointSize) > 0.1
                || lastMeasuredHeight == nil
            else {
                return lastMeasuredHeight ?? parent.height
            }
            lastMeasuredText = textView.text
            lastMeasuredWidth = width
            lastMeasuredShowsPlaceholder = showsPlaceholder
            lastMeasuredFontPointSize = fontPointSize
            let traceID = PerformanceTrace.begin("ComposerHeightMeasure")
            defer { PerformanceTrace.end("ComposerHeightMeasure", id: traceID) }
            let measuredHeight = container.preferredHeight(
                forWidth: width,
                showsPlaceholder: showsPlaceholder
            )
            lastMeasuredHeight = measuredHeight
            return measuredHeight
        }

        func syncPlaceholder() {
            let isEmpty = container?.textView.text.isEmpty ?? true
            container?.placeholderLabel.isHidden = isEmpty == false
            if let textView = container?.textView {
                guard lastAccessibilityShowsPlaceholder != isEmpty else { return }
                lastAccessibilityShowsPlaceholder = isEmpty
                if isEmpty {
                    textView.accessibilityValue = parent.placeholder
                } else {
                    textView.accessibilityValue = nil
                }
            }
        }

        func refreshAccessibilityPlaceholder() {
            lastAccessibilityShowsPlaceholder = nil
        }

        private func updateSelection(from textView: UITextView) {
            let selection = ComposerAttributedMarkdown.markdownSelection(
                from: textView.attributedText,
                visibleRange: textView.selectedRange,
                baseFont: ComposerAttributedMarkdown.composerBaseFont(for: textView)
            )
            if parent.selection != selection {
                parent.selection = selection
            }
        }
    }
}

final class ComposerPasteTextView: UITextView {
    var onPasteImages: (([UIImage]) -> Void)?
    private let quoteBars = CAShapeLayer()
    private var isRefreshingQuotePresentation = false
    private var recentPasteRange: NSRange?
    private var recentPasteText: String?

    override init(frame: CGRect, textContainer: NSTextContainer?) {
        super.init(frame: frame, textContainer: textContainer)
        quoteBars.fillColor = UIColor.systemTeal.cgColor
        quoteBars.zPosition = 1
        layer.addSublayer(quoteBars)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        nil
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        updateQuoteBars()
    }

    /// Keep the sendable draft in text storage. Presentation attributes hide
    /// syntax and style its content without changing selection offsets.
    func refreshQuotePresentation() {
        guard isRefreshingQuotePresentation == false else { return }
        isRefreshingQuotePresentation = true
        defer { isRefreshingQuotePresentation = false }
        let content = textStorage.string as NSString
        guard content.length > 0 else {
            quoteBars.path = nil
            return
        }
        textStorage.beginEditing()
        let fullRange = NSRange(location: 0, length: content.length)
        var previousPresentationRanges: [NSRange] = []
        textStorage.enumerateAttribute(
            ComposerAttributedMarkdown.presentationAttribute,
            in: fullRange
        ) { value, range, _ in
            if value != nil { previousPresentationRanges.append(range) }
        }
        for range in previousPresentationRanges {
            textStorage.addAttribute(.font, value: font ?? UIFont.preferredFont(forTextStyle: .callout), range: range)
            textStorage.removeAttribute(.underlineStyle, range: range)
            textStorage.removeAttribute(.strikethroughStyle, range: range)
            textStorage.removeAttribute(.obliqueness, range: range)
            textStorage.removeAttribute(.backgroundColor, range: range)
        }
        textStorage.removeAttribute(ComposerAttributedMarkdown.presentationAttribute, range: fullRange)
        textStorage.addAttribute(
            .foregroundColor,
            value: textColor ?? UIColor.label,
            range: fullRange
        )
        for range in quoteLineRanges(in: content) {
            textStorage.addAttribute(ComposerAttributedMarkdown.presentationAttribute, value: true, range: range)
            textStorage.addAttribute(
                .foregroundColor,
                value: UIColor.clear,
                range: NSRange(location: range.location, length: 2)
            )
            let contentRange = NSRange(location: range.location + 2, length: range.length - 2)
            textStorage.addAttribute(.obliqueness, value: 0.15, range: contentRange)
        }
        for range in lineRanges(in: content, prefix: "- ") {
            textStorage.addAttribute(ComposerAttributedMarkdown.presentationAttribute, value: true, range: range)
            textStorage.addAttribute(
                .foregroundColor,
                value: UIColor.clear,
                range: NSRange(location: range.location, length: 1)
            )
        }
        applyInlinePresentation(in: content)
        applyHeadingPresentation(in: content)
        applyCodeBlockPresentation(in: content)
        textStorage.endEditing()
        updateQuoteBars()
    }

    private enum InlineStyle: Equatable {
        case bold, italic, underline, strike, code, spoiler
    }

    private func applyInlinePresentation(in content: NSString) {
        let fullRange = NSRange(location: 0, length: content.length)
        let fencedRanges = codeBlockMatches(in: content).map(\.range)
        let inlineCodeRanges = (try? NSRegularExpression(pattern: #"`([^`\n]+)`"#))?
            .matches(in: content as String, range: fullRange)
            .map(\.range) ?? []
        let patterns: [(String, InlineStyle)] = [
            (#"\*\*([^*\n]+)\*\*"#, .bold),
            (#"(?<!_)_([^_\n]+)_(?!_)"#, .italic),
            (#"<u>([\s\S]+?)</u>"#, .underline),
            (#"~~([^~\n]+)~~"#, .strike),
            (#"`([^`\n]+)`"#, .code),
            (#"<span data-mx-spoiler>([\s\S]+?)</span>"#, .spoiler),
        ]
        for (pattern, style) in patterns {
            guard let regex = try? NSRegularExpression(pattern: pattern) else { continue }
            for match in regex.matches(in: content as String, range: fullRange) {
                if fencedRanges.contains(where: { NSIntersectionRange($0, match.range).length > 0 }) {
                    continue
                }
                if style != .code,
                   inlineCodeRanges.contains(where: { NSIntersectionRange($0, match.range).length > 0 })
                {
                    continue
                }
                let inner = match.range(at: 1)
                guard inner.location != NSNotFound else { continue }
                textStorage.addAttribute(
                    ComposerAttributedMarkdown.presentationAttribute,
                    value: true,
                    range: match.range
                )
                let leading = NSRange(location: match.range.location, length: inner.location - match.range.location)
                let trailing = NSRange(location: NSMaxRange(inner), length: NSMaxRange(match.range) - NSMaxRange(inner))
                textStorage.addAttribute(.foregroundColor, value: UIColor.clear, range: leading)
                textStorage.addAttribute(.foregroundColor, value: UIColor.clear, range: trailing)
                switch style {
                case .bold:
                    applyFontTraits(.traitBold, in: inner)
                case .italic:
                    applyFontTraits(.traitItalic, in: inner)
                case .underline:
                    textStorage.addAttribute(.underlineStyle, value: NSUnderlineStyle.single.rawValue, range: inner)
                case .strike:
                    textStorage.addAttribute(.strikethroughStyle, value: NSUnderlineStyle.single.rawValue, range: inner)
                case .code:
                    textStorage.addAttribute(
                        .font,
                        value: UIFont.monospacedSystemFont(ofSize: font?.pointSize ?? 17, weight: .regular),
                        range: inner
                    )
                    textStorage.addAttribute(.backgroundColor, value: UIColor.tertiarySystemFill, range: inner)
                case .spoiler:
                    textStorage.addAttribute(.backgroundColor, value: UIColor.tertiarySystemFill, range: inner)
                }
            }
        }
    }

    private func applyHeadingPresentation(in content: NSString) {
        guard let regex = try? NSRegularExpression(pattern: #"^(#{1,4}) (.*)$"#, options: .anchorsMatchLines) else {
            return
        }
        let fullRange = NSRange(location: 0, length: content.length)
        let codeRanges = codeBlockMatches(in: content).map(\.range)
        for match in regex.matches(in: content as String, range: fullRange) {
            if codeRanges.contains(where: { NSIntersectionRange($0, match.range).length > 0 }) {
                continue
            }
            let markers = match.range(at: 1)
            let body = match.range(at: 2)
            let level = max(1, markers.length - 1)
            textStorage.addAttribute(ComposerAttributedMarkdown.presentationAttribute, value: true, range: match.range)
            textStorage.addAttribute(
                .foregroundColor,
                value: UIColor.clear,
                range: NSRange(location: markers.location, length: markers.length + 1)
            )
            let base = font ?? UIFont.preferredFont(forTextStyle: .callout)
            let scale: CGFloat = level == 1 ? 1.35 : (level == 2 ? 1.2 : 1.1)
            textStorage.addAttribute(
                .font,
                value: UIFont.systemFont(ofSize: base.pointSize * scale, weight: .semibold),
                range: body
            )
        }
    }

    private func applyCodeBlockPresentation(in content: NSString) {
        for match in codeBlockMatches(in: content) {
            let code = match.range(at: 1)
            guard code.location != NSNotFound else { continue }
            textStorage.addAttribute(ComposerAttributedMarkdown.presentationAttribute, value: true, range: match.range)
            textStorage.addAttribute(
                .foregroundColor,
                value: UIColor.clear,
                range: NSRange(location: match.range.location, length: code.location - match.range.location)
            )
            textStorage.addAttribute(
                .foregroundColor,
                value: UIColor.clear,
                range: NSRange(location: NSMaxRange(code), length: NSMaxRange(match.range) - NSMaxRange(code))
            )
            textStorage.addAttribute(
                .font,
                value: UIFont.monospacedSystemFont(ofSize: font?.pointSize ?? 17, weight: .regular),
                range: code
            )
            textStorage.addAttribute(.backgroundColor, value: UIColor.tertiarySystemFill, range: code)
        }
    }

    private func codeBlockMatches(in content: NSString) -> [NSTextCheckingResult] {
        guard let regex = try? NSRegularExpression(
            pattern: #"(?m)^```[^\n]*\n([\s\S]*?)\n```(?=\n|$)"#
        ) else {
            return []
        }
        return regex.matches(in: content as String, range: NSRange(location: 0, length: content.length))
    }

    private func applyFontTraits(_ traits: UIFontDescriptor.SymbolicTraits, in range: NSRange) {
        let current = (textStorage.attribute(.font, at: range.location, effectiveRange: nil) as? UIFont)
            ?? font ?? UIFont.preferredFont(forTextStyle: .callout)
        let combined = current.fontDescriptor.symbolicTraits.union(traits)
        let descriptor = current.fontDescriptor.withSymbolicTraits(combined) ?? current.fontDescriptor
        textStorage.addAttribute(.font, value: UIFont(descriptor: descriptor, size: current.pointSize), range: range)
    }

    func updateQuoteBars() {
        quoteBars.frame = bounds
        guard bounds.width > 0 else {
            quoteBars.path = nil
            return
        }
        let content = textStorage.string as NSString
        let path = UIBezierPath()
        for range in quoteLineRanges(in: content) {
            let glyphs = layoutManager.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
            let rect = layoutManager.boundingRect(forGlyphRange: glyphs, in: textContainer)
            let y = textContainerInset.top + rect.minY - contentOffset.y
            let height = max(rect.height, font?.lineHeight ?? 0)
            path.append(UIBezierPath(roundedRect: CGRect(
                x: textContainerInset.left + 2 - contentOffset.x,
                y: y,
                width: 3,
                height: height
            ), cornerRadius: 1.5))
        }
        for range in lineRanges(in: content, prefix: "- ") {
            let glyphs = layoutManager.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
            let rect = layoutManager.boundingRect(forGlyphRange: glyphs, in: textContainer)
            let y = textContainerInset.top + rect.minY - contentOffset.y
            path.append(UIBezierPath(ovalIn: CGRect(
                x: textContainerInset.left + 2 - contentOffset.x,
                y: y + max(0, (font?.lineHeight ?? 17) - 5) / 2,
                width: 5,
                height: 5
            )))
        }
        quoteBars.path = path.cgPath
    }

    private func quoteLineRanges(in content: NSString) -> [NSRange] {
        lineRanges(in: content, prefix: "> ")
    }

    private func lineRanges(in content: NSString, prefix: String) -> [NSRange] {
        var ranges: [NSRange] = []
        var position = 0
        let codeRanges = codeBlockMatches(in: content).map(\.range)
        while position < content.length {
            let range = content.lineRange(for: NSRange(location: position, length: 0))
            if range.length >= prefix.utf16.count,
               codeRanges.contains(where: { NSIntersectionRange($0, range).length > 0 }) == false,
               content.substring(with: NSRange(location: range.location, length: prefix.utf16.count)) == prefix
            {
                ranges.append(range)
            }
            position = NSMaxRange(range)
        }
        return ranges
    }

    func selectionForFormatting(
        _ format: ComposerMarkdownFormat,
        fallback: ComposerTextSelection
    ) -> ComposerTextSelection {
        guard fallback.length == 0,
              [.blockquote, .bulletList, .numberedList, .codeBlock,
               .heading1, .heading2, .heading3].contains(format),
              let recentPasteRange,
              recentPasteText == text,
              selectedRange.location == NSMaxRange(recentPasteRange),
              selectedRange.length == 0
        else {
            return fallback
        }
        return ComposerAttributedMarkdown.markdownSelection(
            from: attributedText,
            visibleRange: recentPasteRange,
            baseFont: font ?? .preferredFont(forTextStyle: .callout)
        )
    }

    func invalidateRecentPasteIfSelectionMoved() {
        guard let recentPasteRange else { return }
        if selectedRange.length != 0 || selectedRange.location != NSMaxRange(recentPasteRange) {
            self.recentPasteRange = nil
            recentPasteText = nil
        }
    }

    override func canPerformAction(_ action: Selector, withSender sender: Any?) -> Bool {
        if action == #selector(paste(_:)), pasteboardImages().isEmpty == false {
            return true
        }
        return super.canPerformAction(action, withSender: sender)
    }

    override func paste(_ sender: Any?) {
        let images = pasteboardImages()
        if images.isEmpty == false {
            onPasteImages?(images)
            return
        }
        if let html = ComposerPasteboard.htmlString(),
           let attributed = ComposerAttributedMarkdown.attributedString(
               fromHTML: html,
               baseFont: font ?? .preferredFont(forTextStyle: .callout)
           ),
           attributed.length > 0
        {
            insertComposerAttributedText(attributed)
            delegate?.textViewDidChange?(self)
            return
        }
        if let plain = UIPasteboard.general.string,
           plain.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
        {
            insertComposerAttributedText(NSAttributedString(string: plain))
            delegate?.textViewDidChange?(self)
        }
    }

    func insertComposerAttributedText(_ attributed: NSAttributedString) {
        let normalized = NSMutableAttributedString(attributedString: attributed)
        if normalized.length > 0 {
            normalized.addAttribute(
                .foregroundColor,
                value: textColor ?? .label,
                range: NSRange(location: 0, length: normalized.length)
            )
        }
        let mutable = NSMutableAttributedString(attributedString: attributedText)
        let range = selectedRange
        mutable.replaceCharacters(in: range, with: normalized)
        attributedText = mutable
        selectedRange = NSRange(location: range.location + normalized.length, length: 0)
        recentPasteRange = NSRange(location: range.location, length: normalized.length)
        recentPasteText = text
        refreshQuotePresentation()
        typingAttributes = [
            .font: font ?? .preferredFont(forTextStyle: .callout),
            .foregroundColor: textColor ?? .label,
        ]
    }

    private func pasteboardImages() -> [UIImage] {
        if let images = UIPasteboard.general.images, images.isEmpty == false {
            return images
        }
        if let image = UIPasteboard.general.image {
            return [image]
        }
        return []
    }
}

final class ComposerTextContainer: UIView {
    let textView = ComposerPasteTextView()
    let placeholderLabel = UILabel()
    private var lastMeasuredWidth: CGFloat = 0
    var onWidthChange: (() -> Void)?

    func preferredHeight(forWidth width: CGFloat, showsPlaceholder: Bool) -> CGFloat {
        let font = textView.font ?? .preferredFont(forTextStyle: .callout)
        let singleLineHeight = ComposerTextMetrics.singleLineHeight(font: font)
        if showsPlaceholder {
            let placeholderHeight = placeholderLabel.sizeThatFits(
                CGSize(width: max(width, 1), height: .greatestFiniteMagnitude)
            ).height + ComposerTextMetrics.textContainerInset.top + ComposerTextMetrics.textContainerInset.bottom
            return min(
                max(ceil(placeholderHeight), singleLineHeight),
                ComposerTextMetrics.maxHeight
            )
        }

        let fittingHeight = textView.sizeThatFits(
            CGSize(width: width, height: .greatestFiniteMagnitude)
        ).height
        return min(max(fittingHeight, singleLineHeight), ComposerTextMetrics.maxHeight)
    }

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear
        clipsToBounds = true

        placeholderLabel.numberOfLines = 0
        placeholderLabel.isUserInteractionEnabled = false
        placeholderLabel.translatesAutoresizingMaskIntoConstraints = false

        textView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(textView)
        addSubview(placeholderLabel)

        NSLayoutConstraint.activate([
            textView.leadingAnchor.constraint(equalTo: leadingAnchor),
            textView.trailingAnchor.constraint(equalTo: trailingAnchor),
            textView.topAnchor.constraint(equalTo: topAnchor),
            textView.bottomAnchor.constraint(equalTo: bottomAnchor),
            placeholderLabel.leadingAnchor.constraint(equalTo: leadingAnchor),
            placeholderLabel.trailingAnchor.constraint(equalTo: trailingAnchor),
            placeholderLabel.topAnchor.constraint(
                equalTo: topAnchor,
                constant: ComposerTextMetrics.textContainerInset.top
            ),
            placeholderLabel.bottomAnchor.constraint(
                lessThanOrEqualTo: bottomAnchor,
                constant: -ComposerTextMetrics.textContainerInset.bottom
            )
        ])
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        let width = bounds.width
        guard width > 0, abs(width - lastMeasuredWidth) > 0.5 else {
            return
        }
        lastMeasuredWidth = width
        onWidthChange?()
    }

    override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        guard previousTraitCollection?.preferredContentSizeCategory
            != traitCollection.preferredContentSizeCategory
        else {
            return
        }
        onWidthChange?()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        nil
    }
}
#endif
