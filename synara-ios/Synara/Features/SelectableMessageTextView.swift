import SwiftUI
#if canImport(UIKit)
    import UIKit
#endif

#if canImport(UIKit)
/// Selectable, non-editable message body for the Select Text sheet.
/// SwiftUI `Text` + `.textSelection(.enabled)` does not present the system
/// loupe/selection handles on long-press inside a sheet; `UITextView` does.
struct SelectableMessageTextView: UIViewRepresentable {
    let attributedText: NSAttributedString
    var accessibilityLabel: String
    var accessibilityHint: String = "Select and copy any part of this message"

    func makeUIView(context: Context) -> UITextView {
        let textView = UITextView()
        textView.backgroundColor = .clear
        textView.isEditable = false
        textView.isSelectable = true
        textView.isScrollEnabled = false
        textView.adjustsFontForContentSizeCategory = true
        textView.textContainerInset = .zero
        textView.textContainer.lineFragmentPadding = 0
        textView.dataDetectorTypes = []
        textView.isUserInteractionEnabled = true
        textView.tintColor = .systemBlue
        textView.allowsEditingTextAttributes = false
        textView.linkTextAttributes = [:]
        textView.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        textView.setContentHuggingPriority(.defaultLow, for: .horizontal)
        textView.accessibilityIdentifier = "MessageTextSelectionBody"
        textView.delegate = context.coordinator
        return textView
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        func textView(
            _: UITextView,
            shouldInteractWith _: URL,
            in _: NSRange,
            interaction _: UITextItemInteraction
        ) -> Bool {
            false
        }

        func textView(
            _: UITextView,
            shouldInteractWith _: NSTextAttachment,
            in _: NSRange,
            interaction _: UITextItemInteraction
        ) -> Bool {
            false
        }
    }

    func updateUIView(_ textView: UITextView, context: Context) {
        if textView.attributedText != attributedText {
            textView.attributedText = attributedText
        }
        textView.delegate = context.coordinator
        textView.accessibilityLabel = accessibilityLabel
        textView.accessibilityHint = accessibilityHint
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: UITextView, context: Context) -> CGSize? {
        let width = proposal.width ?? uiView.bounds.width
        guard width > 0 else {
            return nil
        }
        let fitted = uiView.sizeThatFits(CGSize(width: width, height: .greatestFiniteMagnitude))
        return CGSize(width: width, height: max(ceil(fitted.height), 1))
    }
}
#endif
