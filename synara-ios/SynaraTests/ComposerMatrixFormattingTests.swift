import XCTest
@testable import Synara
#if canImport(UIKit)
import UIKit
#endif

final class ComposerMatrixFormattingTests: XCTestCase {
    func testPlainTextOmitsFormattedBody() {
        XCTAssertNil(ComposerMatrixFormatting.formattedBody(for: "hello world"))
    }

    func testToolbarMarkdownProducesMatrixHTML() {
        let html = ComposerMatrixFormatting.formattedBody(for: "- **Ship it**\n- `verify`")

        XCTAssertNotNil(html)
        XCTAssertTrue(html?.contains("<ul>") == true)
        XCTAssertTrue(html?.contains("<strong>Ship it</strong>") == true)
        XCTAssertTrue(html?.contains("<code>verify</code>") == true)
    }

    func testMultilineQuoteProducesOneFormattedBlock() {
        let body = "> first line\n> second line\n> third line"
        let html = ComposerMatrixFormatting.formattedBody(for: body)

        XCTAssertNotNil(html)
        XCTAssertTrue(html?.contains("<blockquote>") == true, html ?? "nil")
        for line in ["first line", "second line", "third line"] {
            XCTAssertTrue(html?.contains(line) == true, html ?? "nil")
        }
    }

    func testMatrixMarkdownRetainsSupportedInlineHTML() {
        let html = ComposerMatrixFormatting.formattedBody(
            for: #"<u>under</u> <span data-mx-spoiler>hidden</span>"#
        )
        XCTAssertTrue(html?.contains("<u>under</u>") == true, html ?? "nil")
        XCTAssertTrue(html?.contains("data-mx-spoiler") == true, html ?? "nil")
    }

    func testDesktopFormattingControlsSendReadablePlainBody() {
        let draft = "## Plan\n<u>under</u> and <span data-mx-spoiler>secret</span>"
        let html = ComposerMatrixFormatting.formattedBody(for: draft)
        XCTAssertTrue(html?.contains("<h2>") == true, html ?? "nil")
        XCTAssertTrue(html?.contains("<u>under</u>") == true, html ?? "nil")
        XCTAssertTrue(html?.contains("data-mx-spoiler") == true, html ?? "nil")

        let plain = ComposerMatrixFormatting.plainBody(for: draft, formattedBody: html)
        XCTAssertTrue(plain.contains("Plan"), plain)
        XCTAssertTrue(plain.contains("under"), plain)
        XCTAssertTrue(plain.contains("[spoiler]"), plain)
        XCTAssertFalse(plain.contains("<u>"), plain)
        XCTAssertFalse(plain.contains("data-mx-spoiler"), plain)
    }

    #if canImport(UIKit)
    func testEmptyComposerMeasuresWrappedPlaceholderAtAccessibilityScale() {
        let container = ComposerTextContainer()
        let accessibilityFont = UIFont.systemFont(ofSize: 31)
        container.textView.font = accessibilityFont
        container.placeholderLabel.font = accessibilityFont
        container.placeholderLabel.text = "Send an encrypted message to this room"

        let width: CGFloat = 180
        let height = container.preferredHeight(forWidth: width, showsPlaceholder: true)

        XCTAssertGreaterThan(height, ComposerTextMetrics.singleLineHeight(font: accessibilityFont))
        XCTAssertLessThanOrEqual(height, ComposerTextMetrics.maxHeight)

        container.frame = CGRect(x: 0, y: 0, width: width, height: height)
        container.layoutIfNeeded()

        XCTAssertTrue(container.clipsToBounds)
        XCTAssertGreaterThanOrEqual(container.placeholderLabel.frame.minY, container.bounds.minY)
        XCTAssertLessThanOrEqual(container.placeholderLabel.frame.maxY, container.bounds.maxY + 0.5)
    }

    func testNonemptyComposerStillCapsLongTextHeight() {
        let container = ComposerTextContainer()
        container.textView.font = UIFont.systemFont(ofSize: 31)
        container.textView.text = String(repeating: "A long message line ", count: 100)

        XCTAssertEqual(
            container.preferredHeight(forWidth: 180, showsPlaceholder: false),
            ComposerTextMetrics.maxHeight
        )
    }

    func testCappedComposerReusesHeightOnlyForAppendOnlyTyping() {
        let longText = String(repeating: "Long message text ", count: 40)
        let common = (
            previousHeight: Optional(ComposerTextMetrics.maxHeight),
            previousWidth: CGFloat(320),
            currentWidth: CGFloat(320),
            previousShowsPlaceholder: Optional(false),
            currentShowsPlaceholder: false,
            previousFontPointSize: CGFloat(17),
            currentFontPointSize: CGFloat(17),
            force: false
        )

        XCTAssertTrue(
            ComposerHeightMeasurementPolicy.canReuseCappedHeight(
                previousText: longText,
                currentText: longText + "a",
                previousHeight: common.previousHeight,
                previousWidth: common.previousWidth,
                currentWidth: common.currentWidth,
                previousShowsPlaceholder: common.previousShowsPlaceholder,
                currentShowsPlaceholder: common.currentShowsPlaceholder,
                previousFontPointSize: common.previousFontPointSize,
                currentFontPointSize: common.currentFontPointSize,
                force: common.force
            )
        )
        XCTAssertFalse(
            ComposerHeightMeasurementPolicy.canReuseCappedHeight(
                previousText: longText,
                currentText: String(longText.dropLast()),
                previousHeight: common.previousHeight,
                previousWidth: common.previousWidth,
                currentWidth: common.currentWidth,
                previousShowsPlaceholder: common.previousShowsPlaceholder,
                currentShowsPlaceholder: common.currentShowsPlaceholder,
                previousFontPointSize: common.previousFontPointSize,
                currentFontPointSize: common.currentFontPointSize,
                force: common.force
            )
        )
        XCTAssertFalse(
            ComposerHeightMeasurementPolicy.canReuseCappedHeight(
                previousText: longText,
                currentText: longText + "a",
                previousHeight: common.previousHeight,
                previousWidth: common.previousWidth,
                currentWidth: 280,
                previousShowsPlaceholder: common.previousShowsPlaceholder,
                currentShowsPlaceholder: common.currentShowsPlaceholder,
                previousFontPointSize: common.previousFontPointSize,
                currentFontPointSize: common.currentFontPointSize,
                force: common.force
            )
        )
    }
    #endif
}
