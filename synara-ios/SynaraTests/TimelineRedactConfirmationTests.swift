import XCTest
@testable import Synara

/// N2: Redact asks "Remove this message? This can't be undone." first, with
/// the desktop dialog's words, and only Remove redacts.
final class TimelineRedactConfirmationTests: XCTestCase {
    func testRequestOnlyAsks() {
        var confirmation = TimelineRedactConfirmation<String>()
        XCTAssertFalse(confirmation.isPresented)

        confirmation.request("$event")

        XCTAssertTrue(confirmation.isPresented)
        XCTAssertEqual(confirmation.pending, "$event")
    }

    func testRemoveRedactsTheRequestedMessageOnce() {
        var confirmation = TimelineRedactConfirmation<String>()
        var redacted: [String] = []
        confirmation.request("$event")

        let settled = confirmation.settle(.remove) { redacted.append($0) }
        XCTAssertTrue(settled)
        XCTAssertEqual(redacted, ["$event"])
        XCTAssertFalse(confirmation.isPresented)
        XCTAssertNil(confirmation.pending)

        let settledAgain = confirmation.settle(.remove) { redacted.append($0) }
        XCTAssertFalse(settledAgain, "a second Remove has nothing to settle")
        XCTAssertEqual(redacted, ["$event"])
    }

    func testCancelDoesNothing() {
        var confirmation = TimelineRedactConfirmation<String>()
        var redacted: [String] = []
        confirmation.request("$event")

        let settled = confirmation.settle(.cancel) { redacted.append($0) }
        XCTAssertTrue(settled)
        XCTAssertEqual(redacted, [])
        XCTAssertFalse(confirmation.isPresented)
        XCTAssertNil(confirmation.pending)

        let removedLater = confirmation.settle(.remove) { redacted.append($0) }
        XCTAssertFalse(removedLater, "Remove after Cancel finds no request")
        XCTAssertEqual(redacted, [])
    }

    func testDismissingTheDialogDoesNotRedactAndALaterRequestReplacesIt() {
        var confirmation = TimelineRedactConfirmation<String>()
        var redacted: [String] = []
        confirmation.request("$first")
        // SwiftUI clears the binding when the dialog goes away; that alone
        // never redacts.
        confirmation.isPresented = false
        XCTAssertEqual(redacted, [])

        confirmation.request("$second")
        confirmation.settle(.remove) { redacted.append($0) }
        XCTAssertEqual(redacted, ["$second"])
    }

    func testCopyMatchesTheDesktopDialog() {
        XCTAssertEqual(TimelineRedactConfirmCopy.question, "Remove this message? This can't be undone.")
        XCTAssertEqual(TimelineRedactConfirmCopy.remove, "Remove")
        XCTAssertEqual(TimelineRedactConfirmCopy.cancel, "Cancel")
    }
}
