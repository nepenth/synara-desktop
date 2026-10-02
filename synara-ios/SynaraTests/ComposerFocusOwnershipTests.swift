import XCTest
import SwiftUI
@testable import Synara
#if canImport(UIKit)
import UIKit

@MainActor
final class ComposerFocusOwnershipTests: XCTestCase {
    func testQueuedFocusReadsFalseBeforeAnotherRepresentableUpdate() async {
        let fixture = makeFixture(focused: true, responder: false)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.state.focused = false
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls, 0)
        XCTAssertEqual(fixture.editor.resignCalls, 0)
        XCTAssertFalse(fixture.state.focused)
    }

    func testQueuedDismissalReadsTrueBeforeAnotherRepresentableUpdate() async {
        let fixture = makeFixture(focused: false, responder: true)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.state.focused = true
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls, 0)
        XCTAssertEqual(fixture.editor.resignCalls, 0)
        XCTAssertTrue(fixture.state.focused)
    }

    func testOppositeRequestsCoalesceToOneCurrentFocusAttempt() async {
        let fixture = makeFixture(focused: true, responder: false)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.state.focused = false
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.state.focused = true
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls, 1)
        XCTAssertEqual(fixture.editor.resignCalls, 0)
        XCTAssertTrue(fixture.state.focused)
    }

    func testOppositeRequestsCoalesceToOneCurrentDismissalAttempt() async {
        let fixture = makeFixture(focused: false, responder: true)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.state.focused = true
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.state.focused = false
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls, 0)
        XCTAssertEqual(fixture.editor.resignCalls, 1)
        XCTAssertFalse(fixture.state.focused)
    }

    func testNativeBeginInvalidatesQueuedDismissalImmediately() async {
        let fixture = makeFixture(focused: false, responder: true)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.owner.textViewDidBeginEditing(fixture.editor)
        XCTAssertTrue(fixture.state.focused)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.resignCalls, 0)
    }

    func testNativeEndInvalidatesQueuedFocusImmediately() async {
        let fixture = makeFixture(focused: true, responder: false)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.owner.textViewDidEndEditing(fixture.editor)
        XCTAssertFalse(fixture.state.focused)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls, 0)
    }

    func testDetachedAndDifferentWindowRequestsAreDropped() async {
        let detached = makeFixture(focused: true, responder: false)
        detached.owner.requestFocusUpdate(for: detached.editor)
        detached.editor.attachedWindow = nil
        let replaced = makeFixture(focused: false, responder: true)
        let successorWindow = UIWindow()
        replaced.owner.requestFocusUpdate(for: replaced.editor)
        replaced.editor.attachedWindow = successorWindow
        await drainMainQueue()
        XCTAssertEqual(detached.editor.becomeCalls, 0)
        XCTAssertEqual(replaced.editor.resignCalls, 0)
    }

    func testEditorReplacementInvalidatesOldAttemptAndAdmitsNewEditor() async {
        let fixture = makeFixture(focused: true, responder: false)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        let replacement = ComposerFocusResponderSpy()
        replacement.attachedWindow = fixture.window
        replacement.delegate = fixture.owner
        fixture.owner.installFocusEditor(replacement)
        fixture.owner.requestFocusUpdate(for: replacement)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls, 0)
        XCTAssertEqual(replacement.becomeCalls, 1)
    }

    func testDismantledOwnerCannotRunOrReattachPendingWork() async {
        let fixture = makeFixture(focused: true, responder: false)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.owner.retireFocusOwner(for: fixture.editor)
        fixture.owner.installFocusEditor(fixture.editor)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls, 0)
        XCTAssertTrue(fixture.state.focused)
    }

    func testQueuedWorkDoesNotRetainOwnerOrEditor() async {
        let state = ComposerFocusStateBox()
        state.focused = true
        let window = UIWindow()
        var editor: ComposerFocusResponderSpy? = ComposerFocusResponderSpy()
        editor?.attachedWindow = window
        var owner: ComposerTextView.Coordinator? = ComposerTextView.Coordinator(parent: state.view())
        let lifetime = ComposerFocusWeakWitness(owner: owner, editor: editor)
        owner?.installFocusEditor(editor!)
        owner?.requestFocusUpdate(for: editor!)
        owner = nil
        editor = nil
        XCTAssertNil(lifetime.owner)
        XCTAssertNil(lifetime.editor)
        await drainMainQueue()
        XCTAssertTrue(state.focused)
    }

    func testFailedNativeCallsDoNotPublishSuccessOrRetry() async {
        let focus = makeFixture(focused: true, responder: false)
        let dismissal = makeFixture(focused: false, responder: true)
        focus.editor.allowsTransition = false
        dismissal.editor.allowsTransition = false
        focus.owner.requestFocusUpdate(for: focus.editor)
        dismissal.owner.requestFocusUpdate(for: dismissal.editor)
        await drainMainQueue()
        await drainMainQueue()
        XCTAssertEqual(focus.editor.becomeCalls, 1)
        XCTAssertEqual(dismissal.editor.resignCalls, 1)
        XCTAssertFalse(focus.editor.isFirstResponder)
        XCTAssertTrue(dismissal.editor.isFirstResponder)
        XCTAssertTrue(focus.state.focused)
        XCTAssertFalse(dismissal.state.focused)
    }

    func testOldBeginCannotCancelReplacementDismissalOrPublishFocus() async {
        let fixture = makeFixture(focused: false, responder: true)
        let replacement = ComposerFocusResponderSpy()
        replacement.nativeFocus = true
        replacement.attachedWindow = fixture.window
        replacement.delegate = fixture.owner
        fixture.owner.installFocusEditor(replacement)
        fixture.owner.requestFocusUpdate(for: replacement)
        fixture.owner.textViewDidBeginEditing(fixture.editor)
        XCTAssertFalse(fixture.state.focused)
        await drainMainQueue()
        XCTAssertEqual(replacement.resignCalls, 1)
        XCTAssertEqual(fixture.editor.resignCalls, 0)
    }

    func testOldEndCannotCancelReplacementFocusOrPublishText() async {
        let fixture = makeFixture(focused: true, responder: false)
        fixture.state.text = "current"
        fixture.editor.attributedText = NSAttributedString(string: "stale")
        let replacement = ComposerFocusResponderSpy()
        replacement.attachedWindow = fixture.window
        replacement.delegate = fixture.owner
        fixture.owner.installFocusEditor(replacement)
        fixture.owner.requestFocusUpdate(for: replacement)
        fixture.owner.textViewDidEndEditing(fixture.editor)
        XCTAssertTrue(fixture.state.focused)
        XCTAssertEqual(fixture.state.text, "current")
        await drainMainQueue()
        XCTAssertEqual(replacement.becomeCalls, 1)
        XCTAssertEqual(fixture.editor.becomeCalls, 0)
    }

    func testRetiredCallbacksCannotPublishFocusTextOrSelection() async {
        let fixture = makeFixture(focused: false, responder: false)
        fixture.state.text = "current"
        fixture.state.selection = ComposerTextSelection(location: 1, length: 2)
        let selection = fixture.state.selection
        fixture.editor.attributedText = NSAttributedString(string: "stale")
        fixture.editor.selectedRange = NSRange(location: 0, length: 5)
        fixture.owner.retireFocusOwner(for: fixture.editor)
        fixture.owner.textViewDidBeginEditing(fixture.editor)
        XCTAssertFalse(fixture.state.focused)
        fixture.state.focused = true
        fixture.owner.textViewDidEndEditing(fixture.editor)
        fixture.owner.textViewDidChange(fixture.editor)
        fixture.owner.textViewDidChangeSelection(fixture.editor)
        XCTAssertTrue(fixture.state.focused)
        XCTAssertEqual(fixture.state.text, "current")
        XCTAssertEqual(fixture.state.selection, selection)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls + fixture.editor.resignCalls, 0)
    }

    func testOldContentAndSelectionCallbacksCannotPublishReplacementState() async {
        let fixture = makeFixture(focused: false, responder: false)
        fixture.state.text = "current"
        fixture.state.selection = ComposerTextSelection(location: 1, length: 2)
        let selection = fixture.state.selection
        fixture.editor.attributedText = NSAttributedString(string: "stale")
        fixture.editor.selectedRange = NSRange(location: 0, length: 5)
        let replacement = ComposerFocusResponderSpy()
        replacement.attachedWindow = fixture.window
        fixture.owner.installFocusEditor(replacement)
        fixture.owner.textViewDidChange(fixture.editor)
        fixture.owner.textViewDidChangeSelection(fixture.editor)
        XCTAssertEqual(fixture.state.text, "current")
        XCTAssertEqual(fixture.state.selection, selection)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls + fixture.editor.resignCalls, 0)
    }

    func testForeignRequestAndOldDismantleCannotCancelCurrentFocus() async {
        let fixture = makeFixture(focused: true, responder: false)
        let foreign = ComposerFocusResponderSpy()
        foreign.attachedWindow = fixture.window
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.owner.requestFocusUpdate(for: foreign)
        fixture.owner.retireFocusOwner(for: foreign)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls, 1)
        XCTAssertEqual(foreign.becomeCalls + foreign.resignCalls, 0)
    }

    func testMatchingDetachedNativeEndRemainsAuthoritative() async {
        let fixture = makeFixture(focused: true, responder: false)
        fixture.owner.requestFocusUpdate(for: fixture.editor)
        fixture.editor.attachedWindow = nil
        fixture.owner.textViewDidEndEditing(fixture.editor)
        XCTAssertFalse(fixture.state.focused)
        await drainMainQueue()
        XCTAssertEqual(fixture.editor.becomeCalls, 0)
    }

    private func drainMainQueue() async {
        let drained = expectation(description: "Previously enqueued focus admission has run")
        DispatchQueue.main.async { drained.fulfill() }
        await fulfillment(of: [drained], timeout: 5)
    }

    private func makeFixture(focused: Bool, responder: Bool) -> ComposerFocusFixture {
        let state = ComposerFocusStateBox()
        state.focused = focused
        let editor = ComposerFocusResponderSpy()
        editor.nativeFocus = responder
        let window = UIWindow()
        editor.attachedWindow = window
        let owner = ComposerTextView.Coordinator(parent: state.view())
        owner.installFocusEditor(editor)
        editor.delegate = owner
        return ComposerFocusFixture(state: state, editor: editor, owner: owner, window: window)
    }
}

@MainActor
private struct ComposerFocusFixture {
    let state: ComposerFocusStateBox
    let editor: ComposerFocusResponderSpy
    let owner: ComposerTextView.Coordinator
    let window: UIWindow
}

@MainActor
private final class ComposerFocusStateBox {
    var text = ""
    var selection = ComposerTextSelection.empty
    var height: CGFloat = 34
    var focused = false

    func view() -> ComposerTextView {
        ComposerTextView(
            text: Binding(get: { self.text }, set: { self.text = $0 }),
            selection: Binding(get: { self.selection }, set: { self.selection = $0 }),
            height: Binding(get: { self.height }, set: { self.height = $0 }),
            placeholder: "Message",
            formattingRevision: 0,
            isFocused: Binding(get: { self.focused }, set: { self.focused = $0 })
        )
    }
}

@MainActor
private final class ComposerFocusWeakWitness {
    weak var owner: ComposerTextView.Coordinator?
    weak var editor: UITextView?

    init(owner: ComposerTextView.Coordinator?, editor: UITextView?) {
        self.owner = owner
        self.editor = editor
    }
}

// These spies never call super's focus methods or activate a window. They
// cannot perform a successful system-keyboard transition before original A.
@MainActor
private final class ComposerFocusResponderSpy: UITextView {
    weak var attachedWindow: UIWindow?
    var nativeFocus = false
    var allowsTransition = true
    private(set) var becomeCalls = 0
    private(set) var resignCalls = 0

    override var window: UIWindow? { attachedWindow }
    override var isFirstResponder: Bool { nativeFocus }

    override func becomeFirstResponder() -> Bool {
        becomeCalls += 1
        guard allowsTransition else { return false }
        nativeFocus = true
        delegate?.textViewDidBeginEditing?(self)
        return true
    }

    override func resignFirstResponder() -> Bool {
        resignCalls += 1
        guard allowsTransition else { return false }
        nativeFocus = false
        delegate?.textViewDidEndEditing?(self)
        return true
    }
}
#endif
