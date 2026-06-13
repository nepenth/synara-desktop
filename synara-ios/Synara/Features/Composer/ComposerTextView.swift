import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

#if canImport(UIKit)
enum ComposerTextMetrics {
    static let maxHeight: CGFloat = 112
    static let textContainerInset = UIEdgeInsets(top: 6, left: 0, bottom: 6, right: 0)

    static func singleLineHeight(font: UIFont) -> CGFloat {
        ceil(font.lineHeight) + textContainerInset.top + textContainerInset.bottom
    }
}

enum ComposerTextInputRegistry {
    private(set) static weak var activeTextView: UITextView?

    static func register(_ textView: UITextView) {
        activeTextView = textView
    }

    static func currentText() -> String? {
        activeTextView?.text
    }

    static func dismissKeyboard() {
        activeTextView?.resignFirstResponder()
    }
}

struct ComposerTextView: UIViewRepresentable {
    @Binding var text: String
    @Binding var selection: ComposerTextSelection
    @Binding var height: CGFloat
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
        textView.font = .preferredFont(forTextStyle: .callout)
        textView.adjustsFontForContentSizeCategory = true
        textView.textContainerInset = ComposerTextMetrics.textContainerInset
        textView.textContainer.lineFragmentPadding = 0
        textView.isScrollEnabled = false
        textView.keyboardDismissMode = .interactive
        textView.accessibilityIdentifier = "ComposerTextField"
        textView.accessibilityLabel = "Message"
        textView.accessibilityHint = "Enter a message for this room"
        textView.setContentCompressionResistancePriority(.defaultLow, for: .horizontal)
        container.placeholderLabel.text = placeholder
        container.placeholderLabel.font = textView.font
        container.placeholderLabel.textColor = .placeholderText
        context.coordinator.container = container
        container.onWidthChange = { [weak coordinator = context.coordinator] in
            guard let coordinator, let textView = coordinator.container?.textView else {
                return
            }
            coordinator.updateHeight(for: textView)
        }
        ComposerTextInputRegistry.register(textView)
        context.coordinator.syncPlaceholder()
        context.coordinator.updateHeight(for: textView)
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
        context.coordinator.updateHeight(for: textView)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: ComposerTextContainer, context: Context) -> CGSize? {
        let width = proposal.width ?? uiView.bounds.width
        guard width > 0 else {
            return nil
        }
        let measuredHeight = uiView.preferredHeight(
            forWidth: width,
            collapsed: text.isEmpty && isFocused.wrappedValue == false
        )
        return CGSize(width: width, height: measuredHeight)
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
            updateHeight(for: textView)
        }

        func textViewDidChangeSelection(_ textView: UITextView) {
            updateSelection(from: textView)
        }

        func textViewDidBeginEditing(_ textView: UITextView) {
            parent.isFocused.wrappedValue = true
            syncPlaceholder()
            updateHeight(for: textView)
        }

        func textViewDidEndEditing(_ textView: UITextView) {
            parent.isFocused.wrappedValue = false
            parent.text = textView.text
            updateSelection(from: textView)
            syncPlaceholder()
            updateHeight(for: textView)
        }

        func updateHeight(for textView: UITextView) {
            guard let container = container else {
                return
            }
            let width = container.bounds.width
            guard width > 0 else {
                return
            }

            let collapsed = textView.text.isEmpty && textView.isFirstResponder == false
            let measuredHeight = container.preferredHeight(forWidth: width, collapsed: collapsed)
            textView.isScrollEnabled = collapsed == false && measuredHeight >= ComposerTextMetrics.maxHeight

            guard abs(parent.height - measuredHeight) > 0.5 else {
                return
            }
            DispatchQueue.main.async { [weak self] in
                guard let self else {
                    return
                }
                self.parent.height = measuredHeight
            }
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
    private var lastMeasuredWidth: CGFloat = 0
    var onWidthChange: (() -> Void)?

    func preferredHeight(forWidth width: CGFloat, collapsed: Bool) -> CGFloat {
        let font = textView.font ?? .preferredFont(forTextStyle: .callout)
        let singleLineHeight = ComposerTextMetrics.singleLineHeight(font: font)
        if collapsed {
            return singleLineHeight
        }

        let fittingHeight = textView.sizeThatFits(
            CGSize(width: width, height: .greatestFiniteMagnitude)
        ).height
        return min(max(fittingHeight, singleLineHeight), ComposerTextMetrics.maxHeight)
    }

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
            placeholderLabel.topAnchor.constraint(equalTo: topAnchor, constant: 6)
        ])
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        let width = bounds.width
        guard width > 0, abs(width - lastMeasuredWidth) > 0.5 else {
            return
        }
        lastMeasuredWidth = width
        onWidthChange?()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        nil
    }
}
#endif
