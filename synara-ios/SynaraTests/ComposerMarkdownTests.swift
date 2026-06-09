import XCTest
@testable import Synara

final class ComposerMarkdownTests: XCTestCase {
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
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 3, length: 5))
    }

    func testCodeBlockWrapsSelection() {
        let result = ComposerMarkdown.apply(
            .codeBlock,
            to: "let value = 1",
            selection: ComposerTextSelection(location: 0, length: 13)
        )

        XCTAssertEqual(result.text, "\n```\nlet value = 1\n```\n")
        XCTAssertEqual(result.selection, ComposerTextSelection(location: 5, length: 13))
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
}