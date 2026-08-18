import XCTest
@testable import Synara

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
}
