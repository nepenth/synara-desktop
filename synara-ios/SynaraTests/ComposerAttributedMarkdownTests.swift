import XCTest
@testable import Synara
#if canImport(UIKit)
    import UIKit
#endif

final class ComposerAttributedMarkdownTests: XCTestCase {
    func testPlainAttributedStringStaysPlain() {
        let attributed = NSAttributedString(string: "hello world")
        XCTAssertEqual(ComposerAttributedMarkdown.markdown(from: attributed), "hello world")
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
                ComposerAttributedMarkdown.markdown(from: attributed),
                "hello **bold** `code`"
            )
        #else
            XCTFail("UIKit is required for composer attributed markdown")
        #endif
    }

    func testHTMLPasteBecomesVisibleTraitsAndMarkdownForSend() throws {
        #if canImport(UIKit)
            let html = "<p><strong>Ship</strong> <em>it</em></p>"
            let attributed = try XCTUnwrap(
                ComposerAttributedMarkdown.attributedString(
                    fromHTML: html,
                    baseFont: .preferredFont(forTextStyle: .callout)
                )
            )

            XCTAssertTrue(attributed.string.contains("Ship"))
            XCTAssertTrue(attributed.string.contains("it"))
            XCTAssertFalse(attributed.string.contains("<strong>"))
            XCTAssertFalse(attributed.string.contains("**"))

            let markdown = ComposerAttributedMarkdown.markdown(from: attributed)
            XCTAssertTrue(markdown.contains("**Ship**"), markdown)
            XCTAssertTrue(markdown.contains("_it_") || markdown.contains("*it*"), markdown)
        #else
            XCTFail("UIKit is required for HTML paste conversion")
        #endif
    }

    func testComposerPastePrefersHTMLOverVisibleMarkupCharacters() throws {
        let source = try Self.contents(of: "synara-ios/Synara/Features/Composer/ComposerTextView.swift")
        XCTAssertTrue(source.contains("ComposerPasteboard.htmlString()"))
        XCTAssertTrue(source.contains("ComposerAttributedMarkdown.attributedString(fromHTML:"))
        XCTAssertTrue(source.contains("ComposerAttributedMarkdown.markdown(from:"))
        XCTAssertTrue(
            source.contains("textView.isFirstResponder == false"),
            "Unfocused composer updates must not flatten rich paste until the markdown binding diverges"
        )
    }

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
