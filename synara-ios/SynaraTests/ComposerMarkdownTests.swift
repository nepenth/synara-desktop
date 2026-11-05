import XCTest
@testable import Synara

final class ComposerMarkdownTests: XCTestCase {
    func testDesktopInlineControlsWrapAndToggle() {
        let cases: [(ComposerMarkdownFormat, String, String)] = [
            (.underline, "<u>word</u>", "word"),
            (.spoiler, "<span data-mx-spoiler>word</span>", "word"),
        ]
        for (format, expected, original) in cases {
            let applied = ComposerMarkdown.apply(
                format,
                to: original,
                selection: ComposerTextSelection(location: 0, length: 4)
            )
            XCTAssertEqual(applied.text, expected)
            let removed = ComposerMarkdown.apply(format, to: applied.text, selection: applied.selection)
            XCTAssertEqual(removed.text, original)
        }
    }

    func testHeadingLevelsSwitchAndToggle() {
        let first = ComposerMarkdown.apply(
            .heading1,
            to: "Release plan",
            selection: ComposerTextSelection(location: 0, length: 12)
        )
        XCTAssertEqual(first.text, "# Release plan")
        let second = ComposerMarkdown.apply(.heading2, to: first.text, selection: first.selection)
        XCTAssertEqual(second.text, "## Release plan")
        let removed = ComposerMarkdown.apply(.heading2, to: second.text, selection: second.selection)
        XCTAssertEqual(removed.text, "Release plan")
    }

    func testBoldWrapsSelectedText() {
        let result = ComposerMarkdown.apply(
            .bold,
            to: "hello world",
            selection: ComposerTextSelection(location: 6, length: 5)
        )

        XCTAssertEqual(result.text, "hello **world**")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 8, length: 5))
    }

    func testItalicInsertsPlaceholderWhenSelectionEmpty() {
        let result = ComposerMarkdown.apply(
            .italic,
            to: "prefix",
            selection: ComposerTextSelection(location: 6, length: 0)
        )

        XCTAssertEqual(result.text, "prefix_italic text_")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 7, length: 11))
    }

    func testBulletListPrefixesCurrentLine() {
        let result = ComposerMarkdown.apply(
            .bulletList,
            to: "line one\nline two",
            selection: ComposerTextSelection(location: 0, length: 0)
        )

        XCTAssertEqual(result.text, "- line one\nline two")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 2, length: 8))
    }

    func testNumberedListPrefixesMultipleLines() {
        let result = ComposerMarkdown.apply(
            .numberedList,
            to: "alpha\nbeta",
            selection: ComposerTextSelection(location: 0, length: 9)
        )

        XCTAssertEqual(result.text, "1. alpha\n2. beta")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 3, length: 13))
    }

    func testCodeBlockWrapsSelection() {
        let result = ComposerMarkdown.apply(
            .codeBlock,
            to: "let value = 1",
            selection: ComposerTextSelection(location: 0, length: 13)
        )

        XCTAssertEqual(result.text, "```\nlet value = 1\n```")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 4, length: 13))
    }

    func testBlockquotePrefixesLine() {
        let result = ComposerMarkdown.apply(
            .blockquote,
            to: "quoted",
            selection: ComposerTextSelection(location: 0, length: 6)
        )

        XCTAssertEqual(result.text, "> quoted")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 2, length: 6))
    }

    func testBlockquotePreservesEveryPastedLineAndSelection() {
        let source = "  first line\n\nsecond line\nlast line"
        let result = ComposerMarkdown.apply(
            .blockquote,
            to: source,
            selection: ComposerTextSelection(location: 0, length: (source as NSString).length)
        )

        XCTAssertEqual(result.text, ">   first line\n> \n> second line\n> last line")
        XCTAssertEqual(result.selection.location, 2)
        XCTAssertEqual(result.selection.upperBound, (result.text as NSString).length)
    }

    func testBlockquoteDoesNotConsumeNextLineAtSelectionBoundary() {
        let result = ComposerMarkdown.apply(
            .blockquote,
            to: "first\nsecond",
            selection: ComposerTextSelection(location: 0, length: 6)
        )

        XCTAssertEqual(result.text, "> first\nsecond")
    }

    func testBlockquoteKeepsTrailingNewline() {
        let result = ComposerMarkdown.apply(
            .blockquote,
            to: "first\n",
            selection: ComposerTextSelection(location: 0, length: 6)
        )

        XCTAssertEqual(result.text, "> first\n")
    }

    func testBlockquoteTogglesOffForEverySelectedLine() {
        let source = "> first\n> second"
        let result = ComposerMarkdown.apply(
            .blockquote,
            to: source,
            selection: ComposerTextSelection(location: 2, length: 14)
        )

        XCTAssertEqual(result.text, "first\nsecond")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 0, length: 12))
    }

    func testNumberedListTogglesOffForEverySelectedLine() {
        let source = "1. first\n2. second"
        let result = ComposerMarkdown.apply(
            .numberedList,
            to: source,
            selection: ComposerTextSelection(location: 3, length: 15)
        )

        XCTAssertEqual(result.text, "first\nsecond")
    }

    func testInlineCodeWrapsPartialSelection() {
        let result = ComposerMarkdown.apply(
            .inlineCode,
            to: "use npm install here",
            selection: ComposerTextSelection(location: 4, length: 11)
        )

        XCTAssertEqual(result.text, "use `npm install` here")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 5, length: 11))
    }

    func testNumberedListPrefixesOnlySelectedLines() {
        let result = ComposerMarkdown.apply(
            .numberedList,
            to: "alpha\nbeta\ngamma",
            selection: ComposerTextSelection(location: 6, length: 4)
        )

        XCTAssertEqual(result.text, "alpha\n1. beta\ngamma")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 9, length: 4))
    }

    func testItalicOnMappedWordAfterBoldMarkdownKeepsEarlierMarks() {
        let result = ComposerMarkdown.apply(
            .italic,
            to: "**hello** world",
            selection: ComposerTextSelection(location: 10, length: 5)
        )

        XCTAssertEqual(result.text, "**hello** _world_")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 11, length: 5))
    }

    // MARK: - Behaviour shared with desktop (docs/composer-formatting-model.md)

    func testCodeBlockBreaksLinesOnlyWhereNeeded() {
        let result = ComposerMarkdown.apply(
            .codeBlock,
            to: "see x here",
            selection: ComposerTextSelection(location: 4, length: 1)
        )
        XCTAssertEqual(result.text, "see \n```\nx\n```\n here")
    }

    func testNumberedListContinuesTheListAbove() {
        let text = "1. one\n2. two\na note\nthree"
        let result = ComposerMarkdown.apply(
            .numberedList,
            to: text,
            selection: ComposerTextSelection(location: (text as NSString).length, length: 0)
        )
        XCTAssertEqual(result.text, "1. one\n2. two\na note\n3. three")
        XCTAssertEqual(result.selection.location, ("1. one\n2. two\na note\n3. " as NSString).length)
    }

    func testNumberedListRestartsAfterAHeadingOrBullets() {
        for separator in ["## Next", "- bullet"] {
            let text = "1. one\n\(separator)\nthree"
            let result = ComposerMarkdown.apply(
                .numberedList,
                to: text,
                selection: ComposerTextSelection(location: (text as NSString).length, length: 0)
            )
            XCTAssertEqual(result.text, "1. one\n\(separator)\n1. three")
        }
    }

    func testBlockFormatsReplaceTheLinesMarker() {
        let numbered = ComposerMarkdown.apply(
            .numberedList,
            to: "- item",
            selection: ComposerTextSelection(location: 3, length: 0)
        )
        XCTAssertEqual(numbered.text, "1. item")
        let quoted = ComposerMarkdown.apply(.blockquote, to: numbered.text, selection: numbered.selection)
        XCTAssertEqual(quoted.text, "> item")
        let heading = ComposerMarkdown.apply(.heading3, to: quoted.text, selection: quoted.selection)
        XCTAssertEqual(heading.text, "### item")
        let bullet = ComposerMarkdown.apply(.bulletList, to: heading.text, selection: heading.selection)
        XCTAssertEqual(bullet.text, "- item")
    }

    private func applying(_ edit: ComposerMarkdown.LineBreakEdit?, to text: String) -> String? {
        guard let edit else { return nil }
        return (text as NSString).replacingCharacters(in: edit.range, with: edit.replacement)
    }

    func testReturnContinuesListsAndQuotes() {
        let cases: [(String, String)] = [
            ("- one", "- one\n- "),
            ("  * one", "  * one\n  * "),
            ("9. nine", "9. nine\n10. "),
            ("3) three", "3) three\n4) "),
            ("> quoted", "> quoted\n> "),
        ]
        for (text, expected) in cases {
            let length = (text as NSString).length
            let edit = ComposerMarkdown.lineBreak(in: text, at: length)
            XCTAssertEqual(applying(edit, to: text), expected, text)
            XCTAssertEqual(edit?.caret, (expected as NSString).length, text)
        }
    }

    func testReturnSplitsAnItemAtTheCaret() {
        let text = "1. abcd"
        let edit = ComposerMarkdown.lineBreak(in: text, at: 5)
        XCTAssertEqual(applying(edit, to: text), "1. ab\n2. cd")
        XCTAssertEqual(edit?.caret, ("1. ab\n2. " as NSString).length)
    }

    func testReturnOnAnEmptyItemLeavesTheList() {
        let text = "- one\n- "
        let edit = ComposerMarkdown.lineBreak(in: text, at: (text as NSString).length)
        XCTAssertEqual(applying(edit, to: text), "- one\n")
        XCTAssertEqual(edit?.caret, ("- one\n" as NSString).length)
    }

    func testReturnIsAPlainNewLineElsewhere() {
        XCTAssertNil(ComposerMarkdown.lineBreak(in: "plain text", at: 10))
        XCTAssertNil(ComposerMarkdown.lineBreak(in: "## Heading", at: 10))
        // Before the marker, and inside a code fence, Return is a new line.
        XCTAssertNil(ComposerMarkdown.lineBreak(in: "- one", at: 1))
        let fenced = "```\n- not a list"
        XCTAssertNil(ComposerMarkdown.lineBreak(in: fenced, at: (fenced as NSString).length))
    }
}
