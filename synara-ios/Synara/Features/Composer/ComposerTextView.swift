import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

#if canImport(UIKit)
enum ComposerTextInputRegistry {
    private(set) static weak var activeTextView: UITextView?

    static func register(_ textView: UITextView) {
        activeTextView = textView
    }

    static func currentText() -> String? {
        activeTextView?.text
    }
}

struct ComposerTextView: UIViewRepresentable {
    @Binding var text: String
    @Binding var selection: ComposerTextSelection
    var placeholder: String
    var formattingRevision: Int
    var flushToken: Int
    var isFocused: FocusState<Bool>.Binding

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    func makeUIView(context: Context) -> ComposerTextContainer {
        let container = ComposerTextContainer()
        let textView = container.textView
        textView.delegate = context.coordinator
        textView.backgroundColor = .clear
        textView.font = .preferredFont(forTextStyle: .body)
        textView.adjustsFontForContentSizeCategory = true
        textView.textContainerInset = UIEdgeInsets(top: 10, left: 0, bottom: 10, right: 0)
        textView.textContainer.lineFragmentPadding = 0
        textView.isScrollEnabled = true
        textView.keyboardDismissMode = .interactive
        textView.accessibilityIdentifier = "ComposerTextField"
        textView.accessibilityLabel = "Message"
        textView.accessibilityHint = "Enter a message for this room"
        textView.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        container.placeholderLabel.text = placeholder
        container.placeholderLabel.font = textView.font
        container.placeholderLabel.textColor = .placeholderText
        context.coordinator.container = container
        ComposerTextInputRegistry.register(textView)
        context.coordinator.syncPlaceholder()
        return container
    }

    func updateUIView(_ uiView: ComposerTextContainer, context: Context) {
        let textView = uiView.textView
        context.coordinator.parent = self
        uiView.placeholderLabel.text = placeholder

        if context.coordinator.lastFlushToken != flushToken {
            context.coordinator.lastFlushToken = flushToken
            context.coordinator.publishContent(from: textView)
        } else if context.coordinator.lastFormattingRevision != formattingRevision {
            context.coordinator.lastFormattingRevision = formattingRevision
            textView.text = text
            applySelection(to: textView)
            context.coordinator.syncPlaceholder()
        } else if textView.isFirstResponder == false, textView.text != text {
            textView.text = text
            applySelection(to: textView)
            context.coordinator.syncPlaceholder()
        }

        if isFocused.wrappedValue, textView.isFirstResponder == false {
            textView.becomeFirstResponder()
        } else if isFocused.wrappedValue == false, textView.isFirstResponder {
            textView.resignFirstResponder()
        }

        context.coordinator.syncPlaceholder()
    }

    private func applySelection(to textView: UITextView) {
        let desiredRange = NSRange(location: selection.location, length: selection.length)
        guard desiredRange.upperBound <= (textView.text as NSString).length else {
            return
        }
        textView.selectedRange = desiredRange
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        var parent: ComposerTextView
        weak var container: ComposerTextContainer?
        var lastFormattingRevision = -1
        var lastFlushToken = -1

        init(parent: ComposerTextView) {
            self.parent = parent
        }

        func publishContent(from textView: UITextView) {
            parent.text = textView.text
            updateSelection(from: textView)
            syncPlaceholder()
        }

        func textViewDidChange(_ textView: UITextView) {
            publishContent(from: textView)
        }

        func textViewDidChangeSelection(_ textView: UITextView) {
            updateSelection(from: textView)
        }

        func textViewDidBeginEditing(_ textView: UITextView) {
            parent.isFocused.wrappedValue = true
            syncPlaceholder()
        }

        func textViewDidEndEditing(_ textView: UITextView) {
            parent.isFocused.wrappedValue = false
            parent.text = textView.text
            updateSelection(from: textView)
            syncPlaceholder()
        }

        func syncPlaceholder() {
            container?.placeholderLabel.isHidden = (container?.textView.text.isEmpty == false)
            if let textView = container?.textView {
                if textView.text.isEmpty {
                    textView.accessibilityValue = parent.placeholder
                } else {
                    textView.accessibilityValue = textView.text
                }
            }
        }

        private func updateSelection(from textView: UITextView) {
            parent.selection = ComposerTextSelection(
                location: textView.selectedRange.location,
                length: textView.selectedRange.length
            )
        }
    }
}

final class ComposerTextContainer: UIView {
    let textView = UITextView()
    let placeholderLabel = UILabel()

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear

        placeholderLabel.numberOfLines = 0
        placeholderLabel.isUserInteractionEnabled = false
        placeholderLabel.translatesAutoresizingMaskIntoConstraints = false

        textView.translatesAutoresizingMaskIntoConstraints = false
        addSubview(textView)
        addSubview(placeholderLabel)

        NSLayoutConstraint.activate([
            textView.leadingAnchor.constraint(equalTo: leadingAnchor),
            textView.trailingAnchor.constraint(equalTo: trailingAnchor),
            textView.topAnchor.constraint(equalTo: topAnchor),
            textView.bottomAnchor.constraint(equalTo: bottomAnchor),
            placeholderLabel.leadingAnchor.constraint(equalTo: leadingAnchor),
            placeholderLabel.trailingAnchor.constraint(equalTo: trailingAnchor),
            placeholderLabel.topAnchor.constraint(equalTo: topAnchor, constant: 10)
        ])
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        nil
    }
}
#endif