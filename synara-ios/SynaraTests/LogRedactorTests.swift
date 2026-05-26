import XCTest
@testable import Synara

final class LogRedactorTests: XCTestCase {
    func testRedactsBearerAndQueryTokens() {
        let input = "Bearer abc123TOKEN access_token=secret refresh_token=another"
        let output = LogRedactor.redact(input)

        XCTAssertFalse(output.contains("abc123TOKEN"))
        XCTAssertFalse(output.contains("secret"))
        XCTAssertFalse(output.contains("another"))
        XCTAssertTrue(output.contains("<redacted:token>"))
    }

    func testRedactsMatrixIdentifiersAndEventIDs() {
        let input = "user @alice:example.org room !abcdef:example.org event $eventid:example.org"
        let output = LogRedactor.redact(input)

        XCTAssertFalse(output.contains("@alice:example.org"))
        XCTAssertFalse(output.contains("!abcdef:example.org"))
        XCTAssertFalse(output.contains("$eventid:example.org"))
    }

    func testRedactsURLsAndAPNsTokens() {
        let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        let input = "url https://matrix.example.org/_matrix/client token \(token)"
        let output = LogRedactor.redact(input)

        XCTAssertFalse(output.contains("matrix.example.org"))
        XCTAssertFalse(output.contains(token))
        XCTAssertTrue(output.contains("<redacted:url>"))
        XCTAssertTrue(output.contains("<redacted:apns-token>"))
    }

    func testMockLoggerStoresRedactedEntries() {
        let logger = MockLoggingService()

        logger.info("token=secret", category: .auth)

        XCTAssertEqual(logger.entries, ["[auth] token=<redacted:token>"])
    }
}
