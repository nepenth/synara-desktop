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

    static func dismissKeyboard() {
        activeTextView?.resignFirstResponder()
        UIApplication.shared.sendAction(
            #selector(UIResponder.resignFirstResponder),
            to: nil,
            from: nil,
            for: nil
        )
    }
}

struct ComposerTextView: UIViewRepresentable {
    @Binding var text: String
    @Binding var selection: ComposerTextSelection
    @Binding var height: CGFloat
    var placeholder: String
    var formattingRevision: Int
    var isFocused: FocusState<Bool>.Binding
    var onPasteImages: ([UIImage]) -> Void = { _ in }

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    func makeUIView(context: Context) -> ComposerTextContainer {
        let container = ComposerTextContainer()
        let textView = container.textView
        textView.backgroundColor = .clear
        textView.font = .preferredFont(forTextStyle: .callout)
        textView.adjustsFontForContentSizeCategory = true
        applyTextAppearance(to: textView)
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
        container.placeholderLabel.adjustsFontForContentSizeCategory = true
        container.placeholderLabel.textColor = .placeholderText
        context.coordinator.container = container
        context.coordinator.lastFormattingRevision = formattingRevision
        context.coordinator.lastPlaceholder = placeholder
        context.coordinator.performProgrammaticUpdate {
            textView.text = text
            applySelection(to: textView)
        }
        textView.delegate = context.coordinator
        textView.onPasteImages = onPasteImages
        container.onWidthChange = { [weak coordinator = context.coordinator] in
            guard let coordinator, let textView = coordinator.container?.textView else {
                return
            }
            coordinator.updateHeight(for: textView)
        }
        ComposerTextInputRegistry.register(textView)
        context.coordinator.syncPlaceholder()
        context.coordinator.updateHeight(for: textView, force: true)
        return container
    }

    func updateUIView(_ uiView: ComposerTextContainer, context: Context) {
        let textView = uiView.textView
        context.coordinator.parent = self
        textView.onPasteImages = onPasteImages
        context.coordinator.performProgrammaticUpdate {
            if context.coordinator.lastPlaceholder != placeholder {
                context.coordinator.lastPlaceholder = placeholder
                uiView.placeholderLabel.text = placeholder
                context.coordinator.refreshAccessibilityPlaceholder()
            }

            if context.coordinator.lastFormattingRevision != formattingRevision {
                context.coordinator.lastFormattingRevision = formattingRevision
                textView.text = text
                applySelection(to: textView)
                context.coordinator.syncPlaceholder()
            } else if textView.isFirstResponder == false, textView.text != text {
                textView.text = text
                applySelection(to: textView)
                context.coordinator.syncPlaceholder()
            }
        }

        if isFocused.wrappedValue, textView.isFirstResponder == false {
            textView.becomeFirstResponder()
        }

        context.coordinator.syncPlaceholder()
        context.coordinator.updateHeight(for: textView)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, uiView: ComposerTextContainer, context: Context) -> CGSize? {
        let width = proposal.width ?? uiView.bounds.width
        guard width > 0 else {
            return nil
        }
        let measuredHeight = context.coordinator.preferredHeight(
            for: uiView.textView,
            width: width
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

    private func applyTextAppearance(to textView: UITextView) {
        let font = textView.font ?? .preferredFont(forTextStyle: .callout)
        textView.font = font
        textView.textColor = .label
        textView.tintColor = .label
        textView.linkTextAttributes = [
            .foregroundColor: UIColor.label
        ]
        textView.typingAttributes = [
            .font: font,
            .foregroundColor: UIColor.label
        ]
    }

    final class Coordinator: NSObject, UITextViewDelegate {
        var parent: ComposerTextView
        weak var container: ComposerTextContainer?
        var lastFormattingRevision = -1
        var lastPlaceholder = ""
        private var isApplyingProgrammaticState = false
        private var lastMeasuredText: String?
        private var lastMeasuredWidth: CGFloat = 0
        private var lastMeasuredShowsPlaceholder: Bool?
        private var lastMeasuredHeight: CGFloat?
        private var lastAccessibilityShowsPlaceholder: Bool?

        init(parent: ComposerTextView) {
            self.parent = parent
        }

        func publishContent(from textView: UITextView) {
            if parent.text != textView.text {
                parent.text = textView.text
            }
            updateSelection(from: textView)
            syncPlaceholder()
        }

        func textViewDidChange(_ textView: UITextView) {
            guard isApplyingProgrammaticState == false else {
                return
            }
            publishContent(from: textView)
            updateHeight(for: textView)
        }

        func textViewDidChangeSelection(_ textView: UITextView) {
            guard isApplyingProgrammaticState == false else {
                return
            }
            updateSelection(from: textView)
        }

        func textViewDidBeginEditing(_ textView: UITextView) {
            if parent.isFocused.wrappedValue == false {
                parent.isFocused.wrappedValue = true
            }
            syncPlaceholder()
            updateHeight(for: textView)
        }

        func textViewDidEndEditing(_ textView: UITextView) {
            if parent.isFocused.wrappedValue {
                parent.isFocused.wrappedValue = false
            }
            parent.text = textView.text
            updateSelection(from: textView)
            syncPlaceholder()
            updateHeight(for: textView)
        }

        func performProgrammaticUpdate(_ update: () -> Void) {
            isApplyingProgrammaticState = true
            update()
            isApplyingProgrammaticState = false
        }

        func updateHeight(for textView: UITextView, force: Bool = false) {
            guard let container = container else {
                return
            }
            let width = container.bounds.width
            guard width > 0 else {
                return
            }

            let measuredHeight = preferredHeight(for: textView, width: width, force: force)
            let showsPlaceholder = textView.text.isEmpty
            textView.isScrollEnabled = showsPlaceholder == false
                && measuredHeight >= ComposerTextMetrics.maxHeight

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

        func preferredHeight(for textView: UITextView, width: CGFloat, force: Bool = false) -> CGFloat {
            guard let container else { return parent.height }
            let showsPlaceholder = textView.text.isEmpty
            guard force
                || lastMeasuredText != textView.text
                || abs(lastMeasuredWidth - width) > 0.5
                || lastMeasuredShowsPlaceholder != showsPlaceholder
                || lastMeasuredHeight == nil
            else {
                return lastMeasuredHeight ?? parent.height
            }
            lastMeasuredText = textView.text
            lastMeasuredWidth = width
            lastMeasuredShowsPlaceholder = showsPlaceholder
            let measuredHeight = container.preferredHeight(
                forWidth: width,
                showsPlaceholder: showsPlaceholder
            )
            lastMeasuredHeight = measuredHeight
            return measuredHeight
        }

        func syncPlaceholder() {
            let isEmpty = container?.textView.text.isEmpty ?? true
            container?.placeholderLabel.isHidden = isEmpty == false
            if let textView = container?.textView {
                guard lastAccessibilityShowsPlaceholder != isEmpty else { return }
                lastAccessibilityShowsPlaceholder = isEmpty
                if isEmpty {
                    textView.accessibilityValue = parent.placeholder
                } else {
                    textView.accessibilityValue = nil
                }
            }
        }

        func refreshAccessibilityPlaceholder() {
            lastAccessibilityShowsPlaceholder = nil
        }

        private func updateSelection(from textView: UITextView) {
            let selection = ComposerTextSelection(
                location: textView.selectedRange.location,
                length: textView.selectedRange.length
            )
            if parent.selection != selection {
                parent.selection = selection
            }
        }
    }
}

final class ComposerPasteTextView: UITextView {
    var onPasteImages: (([UIImage]) -> Void)?

    override func canPerformAction(_ action: Selector, withSender sender: Any?) -> Bool {
        if action == #selector(paste(_:)), pasteboardImages().isEmpty == false {
            return true
        }
        return super.canPerformAction(action, withSender: sender)
    }

    override func paste(_ sender: Any?) {
        let images = pasteboardImages()
        if images.isEmpty == false {
            onPasteImages?(images)
            return
        }
        super.paste(sender)
    }

    private func pasteboardImages() -> [UIImage] {
        if let images = UIPasteboard.general.images, images.isEmpty == false {
            return images
        }
        if let image = UIPasteboard.general.image {
            return [image]
        }
        return []
    }
}

final class ComposerTextContainer: UIView {
    let textView = ComposerPasteTextView()
    let placeholderLabel = UILabel()
    private var lastMeasuredWidth: CGFloat = 0
    var onWidthChange: (() -> Void)?

    func preferredHeight(forWidth width: CGFloat, showsPlaceholder: Bool) -> CGFloat {
        let font = textView.font ?? .preferredFont(forTextStyle: .callout)
        let singleLineHeight = ComposerTextMetrics.singleLineHeight(font: font)
        if showsPlaceholder {
            let placeholderHeight = placeholderLabel.sizeThatFits(
                CGSize(width: max(width, 1), height: .greatestFiniteMagnitude)
            ).height + ComposerTextMetrics.textContainerInset.top + ComposerTextMetrics.textContainerInset.bottom
            return min(
                max(ceil(placeholderHeight), singleLineHeight),
                ComposerTextMetrics.maxHeight
            )
        }

        let fittingHeight = textView.sizeThatFits(
            CGSize(width: width, height: .greatestFiniteMagnitude)
        ).height
        return min(max(fittingHeight, singleLineHeight), ComposerTextMetrics.maxHeight)
    }

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = .clear
        clipsToBounds = true

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
            placeholderLabel.topAnchor.constraint(
                equalTo: topAnchor,
                constant: ComposerTextMetrics.textContainerInset.top
            ),
            placeholderLabel.bottomAnchor.constraint(
                lessThanOrEqualTo: bottomAnchor,
                constant: -ComposerTextMetrics.textContainerInset.bottom
            )
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
