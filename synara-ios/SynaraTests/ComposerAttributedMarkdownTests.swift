import XCTest
@testable import Synara
#if canImport(UIKit)
    import UIKit
#endif

final class ComposerAttributedMarkdownTests: XCTestCase {
    func testComposerPresentationStylesMarksWithoutDuplicatingMarkdown() {
        #if canImport(UIKit)
            let draft = "**bold** _italic_ <u>under</u> <span data-mx-spoiler>secret</span>\n## Heading\n- item\n```\nlet x = 1\n```"
            let textView = ComposerPasteTextView()
            textView.font = .systemFont(ofSize: 17)
            textView.textColor = .label
            textView.text = draft

            textView.refreshQuotePresentation()

            XCTAssertEqual(
                ComposerAttributedMarkdown.markdown(from: textView.attributedText, baseFont: textView.font!),
                draft
            )
            let text = draft as NSString
            let bold = text.range(of: "bold")
            let under = text.range(of: "under")
            let heading = text.range(of: "Heading")
            let boldFont = textView.textStorage.attribute(.font, at: bold.location, effectiveRange: nil) as? UIFont
            XCTAssertTrue(boldFont?.fontDescriptor.symbolicTraits.contains(.traitBold) == true)
            XCTAssertNotNil(textView.textStorage.attribute(.underlineStyle, at: under.location, effectiveRange: nil))
            let headingFont = textView.textStorage.attribute(.font, at: heading.location, effectiveRange: nil) as? UIFont
            XCTAssertGreaterThan(headingFont?.pointSize ?? 0, 17)
            let markerColor = textView.textStorage.attribute(.foregroundColor, at: 0, effectiveRange: nil) as? UIColor
            XCTAssertEqual(markerColor, .clear)
        #else
            XCTFail("UIKit is required for composer presentation")
        #endif
    }

    func testBlockFormattingUsesWholeRecentPasteUntilCaretMoves() {
        #if canImport(UIKit)
            let textView = ComposerPasteTextView()
            textView.font = .systemFont(ofSize: 17)
            textView.text = "Before: "
            textView.selectedRange = NSRange(location: 8, length: 0)
            textView.insertComposerAttributedText(NSAttributedString(string: "one\ntwo\nthree"))

            let fallback = ComposerTextSelection(location: (textView.text as NSString).length, length: 0)
            let quoteSelection = textView.selectionForFormatting(.blockquote, fallback: fallback)
            XCTAssertEqual(quoteSelection, ComposerTextSelection(location: 8, length: 13))
            XCTAssertEqual(textView.selectionForFormatting(.bold, fallback: fallback), fallback)

            let result = ComposerMarkdown.apply(.blockquote, to: textView.text, selection: quoteSelection)
            XCTAssertEqual(result.text, "> Before: one\n> two\n> three")

            textView.selectedRange = NSRange(location: 9, length: 0)
            textView.invalidateRecentPasteIfSelectionMoved()
            XCTAssertEqual(textView.selectionForFormatting(.blockquote, fallback: fallback), fallback)
        #else
            XCTFail("UIKit is required for paste selection")
        #endif
    }

    func testQuotePresentationKeepsMarkdownAndSelectionWhileHidingMarkers() {
        #if canImport(UIKit)
            let textView = ComposerPasteTextView()
            textView.font = .systemFont(ofSize: 17)
            textView.textColor = .label
            textView.text = "> first line\n> second line"
            textView.selectedRange = NSRange(location: 2, length: 22)

            textView.refreshQuotePresentation()

            XCTAssertEqual(textView.text, "> first line\n> second line")
            XCTAssertEqual(textView.selectedRange, NSRange(location: 2, length: 22))
            XCTAssertEqual(
                ComposerAttributedMarkdown.markdown(from: textView.attributedText, baseFont: textView.font!),
                "> first line\n> second line"
            )
            let prefixColor = textView.textStorage.attribute(.foregroundColor, at: 0, effectiveRange: nil) as? UIColor
            let contentColor = textView.textStorage.attribute(.foregroundColor, at: 2, effectiveRange: nil) as? UIColor
            XCTAssertEqual(prefixColor, .clear)
            XCTAssertEqual(contentColor, .label)
        #else
            XCTFail("UIKit is required for quote presentation")
        #endif
    }

    func testPlainAttributedStringStaysPlain() {
        let attributed = NSAttributedString(string: "hello world")
        XCTAssertEqual(ComposerAttributedMarkdown.markdown(from: attributed), "hello world")
    }

    func testPlainVisibleSelectionMatchesMarkdownOffsets() {
        let attributed = NSAttributedString(string: "hello world")
        XCTAssertEqual(
            ComposerAttributedMarkdown.markdownSelection(
                from: attributed,
                visibleRange: NSRange(location: 6, length: 5)
            ),
            ComposerTextSelection(location: 6, length: 5)
        )
        XCTAssertEqual(
            ComposerAttributedMarkdown.visibleRange(
                in: attributed,
                markdownSelection: ComposerTextSelection(location: 6, length: 5)
            ),
            NSRange(location: 6, length: 5)
        )
    }

    func testRichPasteSelectionFormatsTheVisibleWord() {
        #if canImport(UIKit)
            let attributed = NSMutableAttributedString()
            attributed.append(
                NSAttributedString(
                    string: "hello",
                    attributes: [.font: UIFont.boldSystemFont(ofSize: 17)]
                )
            )
            attributed.append(NSAttributedString(string: " world"))

            XCTAssertEqual(
                ComposerAttributedMarkdown.markdown(from: attributed, baseFont: .systemFont(ofSize: 17)),
                "**hello** world"
            )

            let markdownSelection = ComposerAttributedMarkdown.markdownSelection(
                from: attributed,
                visibleRange: NSRange(location: 6, length: 5),
                baseFont: .systemFont(ofSize: 17)
            )
            XCTAssertEqual(markdownSelection, ComposerTextSelection(location: 10, length: 5))
            XCTAssertEqual(
                ComposerAttributedMarkdown.visibleRange(
                    in: attributed,
                    markdownSelection: markdownSelection,
                    baseFont: .systemFont(ofSize: 17)
                ),
                NSRange(location: 6, length: 5)
            )

            let result = ComposerMarkdown.apply(
                .italic,
                to: "**hello** world",
                selection: markdownSelection
            )
            XCTAssertEqual(result.text, "**hello** _world_")
            XCTAssertFalse(result.text.contains("hell_o"))
            XCTAssertFalse(result.text.contains("w_orld"))
        #else
            XCTFail("UIKit is required for rich paste selection mapping")
        #endif
    }

    func testHTMLPasteThenFormattingALaterWordUsesMarkdownOffsets() throws {
        #if canImport(UIKit)
            let html = "<p><strong>hello</strong> world</p>"
            let baseFont = UIFont.preferredFont(forTextStyle: .callout)
            let attributed = try XCTUnwrap(
                ComposerAttributedMarkdown.attributedString(
                    fromHTML: html,
                    baseFont: baseFont
                )
            )
            let visible = attributed.string as NSString
            let world = visible.range(of: "world")
            XCTAssertNotEqual(world.location, NSNotFound)
            XCTAssertEqual(world.length, 5)

            let markdown = ComposerAttributedMarkdown.markdown(from: attributed, baseFont: baseFont)
            let markdownSelection = ComposerAttributedMarkdown.markdownSelection(
                from: attributed,
                visibleRange: world,
                baseFont: baseFont
            )
            XCTAssertLessThanOrEqual(
                markdownSelection.upperBound,
                (markdown as NSString).length
            )
            let selected = (markdown as NSString).substring(
                with: NSRange(location: markdownSelection.location, length: markdownSelection.length)
            )
            XCTAssertEqual(selected, "world")

            let result = ComposerMarkdown.apply(.italic, to: markdown, selection: markdownSelection)
            XCTAssertTrue(result.text.contains("_world_"), result.text)
            XCTAssertFalse(result.text.contains("hell_o"))
            XCTAssertFalse(result.text.contains("w_orld"))
        #else
            XCTFail("UIKit is required for HTML paste selection mapping")
        #endif
    }

    func testBoldItalicAndCodeBecomeMarkdown() {
        #if canImport(UIKit)
            let attributed = NSMutableAttributedString()
            attributed.append(NSAttributedString(string: "hello "))
            attributed.append(
                NSAttributedString(
                    string: "bold",
                    attributes: [.font: UIFont.boldSystemFont(ofSize: 17)]
                )
            )
            attributed.append(NSAttributedString(string: " "))
            attributed.append(
                NSAttributedString(
                    string: "code",
                    attributes: [.font: UIFont.monospacedSystemFont(ofSize: 17, weight: .regular)]
                )
            )

            XCTAssertEqual(
                ComposerAttributedMarkdown.markdown(
                    from: attributed,
                    baseFont: .systemFont(ofSize: 17)
                ),
                "hello **bold** `code`"
            )
        #else
            XCTFail("UIKit is required for composer attributed markdown")
        #endif
    }

    func testRichUnderlinePasteRetainsMatrixFormatting() {
        #if canImport(UIKit)
            let attributed = NSAttributedString(
                string: "underlined",
                attributes: [
                    .font: UIFont.systemFont(ofSize: 17),
                    .underlineStyle: NSUnderlineStyle.single.rawValue,
                ]
            )
            XCTAssertEqual(
                ComposerAttributedMarkdown.markdown(from: attributed, baseFont: .systemFont(ofSize: 17)),
                "<u>underlined</u>"
            )
        #else
            XCTFail("UIKit is required for rich underline paste")
        #endif
    }

    func testRichQuoteAndSpoilerPasteKeepsBlockSemantics() throws {
        #if canImport(UIKit)
            let html = "<blockquote><p>one</p><p>two</p></blockquote>"
                + "<p><span data-mx-spoiler>secret</span></p>"
            let attributed = try XCTUnwrap(ComposerAttributedMarkdown.attributedString(
                fromHTML: html,
                baseFont: .systemFont(ofSize: 17)
            ))
            let draft = ComposerAttributedMarkdown.markdown(
                from: attributed,
                baseFont: .systemFont(ofSize: 17)
            )
            XCTAssertTrue(draft.contains("> one"), draft)
            XCTAssertTrue(draft.contains("> two"), draft)
            XCTAssertTrue(draft.contains("<span data-mx-spoiler>secret</span>"), draft)

            let nested = try XCTUnwrap(ComposerAttributedMarkdown.attributedString(
                fromHTML: "<blockquote><span data-mx-spoiler>quiet</span></blockquote>",
                baseFont: .systemFont(ofSize: 17)
            ))
            let nestedDraft = ComposerAttributedMarkdown.markdown(
                from: nested,
                baseFont: .systemFont(ofSize: 17)
            )
            XCTAssertTrue(nestedDraft.contains("> <span data-mx-spoiler>quiet</span>"), nestedDraft)
        #else
            XCTFail("UIKit is required for rich quote and spoiler paste")
        #endif
    }

    func testHTMLPasteBecomesVisibleTraitsAndMarkdownForSend() throws {
        #if canImport(UIKit)
            let html = "<p><strong>Ship</strong> <em>it</em></p>"
            let baseFont = UIFont.preferredFont(forTextStyle: .callout)
            let attributed = try XCTUnwrap(
                ComposerAttributedMarkdown.attributedString(
                    fromHTML: html,
                    baseFont: baseFont
                )
            )

            XCTAssertTrue(attributed.string.contains("Ship"))
            XCTAssertTrue(attributed.string.contains("it"))
            XCTAssertFalse(attributed.string.contains("<strong>"))
            XCTAssertFalse(attributed.string.contains("**"))

            let markdown = ComposerAttributedMarkdown.markdown(from: attributed, baseFont: baseFont)
            XCTAssertTrue(markdown.contains("**Ship**"), markdown)
            XCTAssertTrue(markdown.contains("_it_") || markdown.contains("*it*"), markdown)
        #else
            XCTFail("UIKit is required for HTML paste conversion")
        #endif
    }

    func testHTMLPasteRejectsRemoteImageTrackerAsLink() throws {
        #if canImport(UIKit)
            let html = "<p><img src=\"https://evil.example/track\" alt=\"Innocent summary\"></p>"
            let attributed = try XCTUnwrap(
                ComposerAttributedMarkdown.attributedString(
                    fromHTML: html,
                    baseFont: .preferredFont(forTextStyle: .callout)
                )
            )
            XCTAssertTrue(attributed.string.contains("Innocent summary"))
            XCTAssertFalse(attributed.string.contains("evil.example"))
            XCTAssertFalse(attributed.string.contains("\u{FFFC}"))
            XCTAssertFalse(ComposerAttributedMarkdown.markdown(from: attributed).contains("evil.example"))
        #else
            XCTFail("UIKit is required for HTML paste conversion")
        #endif
    }

    func testHTMLPasteRejectsJavascriptLinks() throws {
        #if canImport(UIKit)
            let html = #"<p><a href="javascript:alert(1)">click</a></p>"#
            let attributed = try XCTUnwrap(
                ComposerAttributedMarkdown.attributedString(
                    fromHTML: html,
                    baseFont: .preferredFont(forTextStyle: .callout)
                )
            )
            XCTAssertTrue(attributed.string.contains("click"))
            XCTAssertFalse(attributed.string.localizedCaseInsensitiveContains("javascript"))
            XCTAssertFalse(
                ComposerAttributedMarkdown.markdown(from: attributed).localizedCaseInsensitiveContains("javascript")
            )
        #else
            XCTFail("UIKit is required for HTML paste conversion")
        #endif
    }

    func testComposerPastePrefersHTMLOverVisibleMarkupCharacters() throws {
        let source = try Self.contents(of: "synara-ios/Synara/Features/Composer/ComposerTextView.swift")
        let markdown = try Self.contents(of: "synara-ios/Synara/Features/Composer/ComposerAttributedMarkdown.swift")
        XCTAssertTrue(source.contains("ComposerPasteboard.htmlString()"))
        XCTAssertTrue(source.contains("ComposerAttributedMarkdown.attributedString("))
        XCTAssertTrue(source.contains("fromHTML: html"))
        XCTAssertTrue(source.contains("ComposerAttributedMarkdown.markdown("))
        XCTAssertTrue(source.contains("from: textView.attributedText"))
        XCTAssertFalse(
            source.contains("super.paste"),
            "Rich RTF/HTML must not enter the composer through UITextView.paste"
        )
        XCTAssertFalse(
            source.contains("func insertAttributedText"),
            "UITextView.insertAttributedText is public on current SDKs; keep a distinct helper name"
        )
        XCTAssertTrue(source.contains("insertComposerAttributedText"))
        XCTAssertTrue(source.contains("markdownSelection"))
        XCTAssertTrue(source.contains("visibleRange"))
        XCTAssertTrue(markdown.contains("MatrixHTMLRenderer.sanitizedHTMLForClipboard"))
        XCTAssertTrue(markdown.contains("deleteCharacters(in: range)"))
        XCTAssertTrue(markdown.contains("0x1F"))
        XCTAssertTrue(markdown.contains("mailto"))
        XCTAssertTrue(markdown.contains("value(forPasteboardType:"))
        XCTAssertFalse(markdown.contains("string(forPasteboardType:"))
        XCTAssertTrue(
            source.contains("textView.isFirstResponder == false"),
            "Unfocused composer updates must not flatten rich paste until the markdown binding diverges"
        )
    }

    func testRegularCalloutRunIsNotProjectedAsBold() {
        #if canImport(UIKit)
            let base = UIFont.preferredFont(forTextStyle: .callout)
            let attributed = NSAttributedString(string: "hello", attributes: [.font: base])
            XCTAssertEqual(
                ComposerAttributedMarkdown.markdown(from: attributed, baseFont: base),
                "hello"
            )
        #else
            XCTFail("UIKit is required for composer font trait projection")
        #endif
    }

    func testExplicitBoldTraitWrapsRelativeToComposerBase() {
        #if canImport(UIKit)
            let base = UIFont.systemFont(ofSize: 16, weight: .regular)
            let bold = fontByAddingTraits(.traitBold, to: base)
            XCTAssertTrue(bold.fontDescriptor.symbolicTraits.contains(.traitBold))
            let attributed = NSAttributedString(string: "hello", attributes: [.font: bold])
            XCTAssertEqual(
                ComposerAttributedMarkdown.markdown(from: attributed, baseFont: base),
                "**hello**"
            )
        #else
            XCTFail("UIKit is required for composer font trait projection")
        #endif
    }

    func testMatchingSemiboldBaseIsNotProjectedAsBold() {
        #if canImport(UIKit)
            let base = UIFont.systemFont(ofSize: 16, weight: .semibold)
            let matching = NSAttributedString(string: "hello", attributes: [.font: base])
            XCTAssertEqual(
                ComposerAttributedMarkdown.markdown(from: matching, baseFont: base),
                "hello"
            )

            let heavier = UIFont.systemFont(ofSize: 16, weight: .heavy)
            let bold = NSAttributedString(string: "hello", attributes: [.font: heavier])
            XCTAssertEqual(
                ComposerAttributedMarkdown.markdown(from: bold, baseFont: base),
                "**hello**"
            )
        #else
            XCTFail("UIKit is required for composer font trait projection")
        #endif
    }

    #if canImport(UIKit)
        private func fontByAddingTraits(
            _ traits: UIFontDescriptor.SymbolicTraits,
            to font: UIFont
        ) -> UIFont {
            let combined = font.fontDescriptor.symbolicTraits.union(traits)
            let descriptor = font.fontDescriptor.withSymbolicTraits(combined) ?? font.fontDescriptor
            return UIFont(descriptor: descriptor, size: font.pointSize)
        }
    #endif

    private static func contents(of relativePath: String) throws -> String {
        try String(contentsOfFile: "\(repositoryRoot())/\(relativePath)", encoding: .utf8)
    }

    private static func repositoryRoot() -> String {
        var url = URL(fileURLWithPath: #filePath)
        while url.pathComponents.count > 1 {
            url.deleteLastPathComponent()
            if FileManager.default.fileExists(atPath: url.appendingPathComponent("synara-ios").path) {
                return url.path
            }
        }
        return url.path
    }
}
