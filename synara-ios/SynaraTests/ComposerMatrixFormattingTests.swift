import XCTest
import SwiftUI
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
    @MainActor
    func testHostedComposerSynchronizesStateWithUIKitFocusAndFormattedTyping() async throws {
        let appeared = expectation(description: "Composer bindings installed in a hosted view")
        var controls: ComposerFocusTestControls?
        let controller = UIHostingController(rootView: ComposerFocusTestHarness { bindings in
            controls = bindings
            appeared.fulfill()
        })
        let scene = try XCTUnwrap(
            UIApplication.shared.connectedScenes
                .compactMap { $0 as? UIWindowScene }
                .first { $0.activationState == .foregroundActive },
            "Use the hosted application's active window scene"
        )
        let previousKeyWindow = scene.windows.first { $0.isKeyWindow }
        let window = UIWindow(windowScene: scene)
        window.frame = CGRect(x: 0, y: 0, width: 390, height: 844)
        window.rootViewController = controller
        window.makeKeyAndVisible()
        defer {
            window.endEditing(true)
            window.isHidden = true
            previousKeyWindow?.makeKey()
            window.rootViewController = nil
            window.windowScene = nil
        }
        await fulfillment(of: [appeared], timeout: 5)
        let bindings = try XCTUnwrap(controls)
        // onAppear installs bindings before every UIKit attachment/layout is
        // necessarily complete. The global registry may hold a detached editor;
        // ownership comes from this controller's actual hosted view hierarchy.
        let editorReady = XCTNSPredicateExpectation(
            predicate: NSPredicate { _, _ in
                let editors = hostedComposerEditors(in: controller.view)
                return editors.count == 1
                    && editors[0].window === window
                    && editors[0].isDescendant(of: controller.view)
                    && editors[0].bounds.width > 0
                    && editors[0].bounds.height > 0
                    && window.isKeyWindow
                    && !editors[0].isFirstResponder
            },
            object: controller
        )
        await fulfillment(of: [editorReady], timeout: 5)
        let editors = hostedComposerEditors(in: controller.view)
        let textView = try XCTUnwrap(
            editors.count == 1 ? editors.first : nil,
            "Require one production editor in this hosted controller"
        )
        guard textView.window === window,
              textView.isDescendant(of: controller.view),
              textView.bounds.width > 0,
              textView.bounds.height > 0,
              window.isKeyWindow,
              !textView.isFirstResponder
        else {
            XCTFail("Hosted production editor was not laid out in the key test window: count=\(editors.count)")
            struct HostedEditorNotReady: Error {}
            throw HostedEditorNotReady()
        }

        // A toolbar request must reach UIKit through the real representable update.
        bindings.isFocused.wrappedValue = true
        let focused = XCTNSPredicateExpectation(
            predicate: NSPredicate { _, _ in textView.isFirstResponder },
            object: textView
        )
        await fulfillment(of: [focused], timeout: 5)
        textView.insertText("draft")
        XCTAssertEqual(bindings.text.wrappedValue, "draft")

        // A programmatic dismissal and a subsequent native editing action must
        // both update the same state, as happens around attachment presentation.
        bindings.isFocused.wrappedValue = false
        let dismissed = XCTNSPredicateExpectation(
            predicate: NSPredicate { _, _ in !textView.isFirstResponder },
            object: textView
        )
        await fulfillment(of: [dismissed], timeout: 5)
        XCTAssertTrue(textView.becomeFirstResponder())
        XCTAssertTrue(bindings.isFocused.wrappedValue)

        let result = ComposerMarkdown.apply(
            .bold,
            to: bindings.text.wrappedValue,
            selection: ComposerTextSelection(location: 0, length: 5)
        )
        bindings.text.wrappedValue = result.text
        bindings.selection.wrappedValue = result.selection
        bindings.formattingRevision.wrappedValue += 1
        let formatted = XCTNSPredicateExpectation(
            predicate: NSPredicate { _, _ in
                textView.isFirstResponder && textView.text == "**draft**"
            },
            object: textView
        )
        await fulfillment(of: [formatted], timeout: 5)
        textView.insertText("replacement")
        XCTAssertEqual(bindings.text.wrappedValue, "**replacement**")
        XCTAssertTrue(textView.isFirstResponder)
    }

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

#if canImport(UIKit)
@MainActor
private func hostedComposerEditors(in view: UIView) -> [ComposerPasteTextView] {
    let current = (view as? ComposerPasteTextView).map { [$0] } ?? []
    return current + view.subviews.flatMap { hostedComposerEditors(in: $0) }
}

private struct ComposerFocusTestControls {
    let text: Binding<String>
    let selection: Binding<ComposerTextSelection>
    let formattingRevision: Binding<Int>
    let isFocused: Binding<Bool>
}

private struct ComposerFocusTestHarness: View {
    let onReady: (ComposerFocusTestControls) -> Void
    @State private var text = ""
    @State private var selection = ComposerTextSelection.empty
    @State private var height: CGFloat = 34
    @State private var formattingRevision = 0
    @State private var isFocused = false

    var body: some View {
        ComposerTextView(
            text: $text,
            selection: $selection,
            height: $height,
            placeholder: "Message",
            formattingRevision: formattingRevision,
            isFocused: $isFocused
        )
        .frame(height: height)
        .onAppear {
            onReady(ComposerFocusTestControls(
                text: $text,
                selection: $selection,
                formattingRevision: $formattingRevision,
                isFocused: $isFocused
            ))
        }
    }
}
#endif
