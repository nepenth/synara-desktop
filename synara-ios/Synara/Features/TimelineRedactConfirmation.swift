import SwiftUI

/// The words of the redaction confirmation; the same as desktop's
/// `REDACT_CONFIRM_COPY` (synara/src/app/features/room/nativeTimelineRedact.ts).
enum TimelineRedactConfirmCopy {
    static var question: String { String(localized: "Remove this message? This can't be undone.") }
    static var remove: String { String(localized: "Remove") }
    static var cancel: String { String(localized: "Cancel") }
}

enum TimelineRedactDecision: Equatable {
    case remove
    case cancel
}

/// The one redaction a confirmation dialog is asking about. Redaction removes
/// a message for everyone and cannot be undone, so the row menu, the VoiceOver
/// action and the thread view only ask; `settle` runs the redaction for Remove
/// alone, and at most once.
struct TimelineRedactConfirmation<Target> {
    private(set) var pending: Target?
    /// Bound to the dialog. SwiftUI may clear it before or after a button's
    /// action runs, so dismissal alone never settles or redacts.
    var isPresented = false

    mutating func request(_ target: Target) {
        pending = target
        isPresented = true
    }

    /// Returns whether this call settled an open request.
    @discardableResult
    mutating func settle(_ decision: TimelineRedactDecision, redact: (Target) -> Void) -> Bool {
        isPresented = false
        guard let target = pending else { return false }
        pending = nil
        if decision == .remove {
            redact(target)
        }
        return true
    }
}

private struct TimelineRedactConfirmationDialog<Target>: ViewModifier {
    @Binding var confirmation: TimelineRedactConfirmation<Target>
    let onRemove: (Target) -> Void

    // An alert, not a confirmation dialog: on current iOS a confirmation
    // dialog shows as a popover with no Cancel button, and this question
    // needs both choices on screen, Cancel as the default, as on desktop.
    func body(content: Content) -> some View {
        content.alert(
            TimelineRedactConfirmCopy.question,
            isPresented: $confirmation.isPresented
        ) {
            Button(TimelineRedactConfirmCopy.remove, role: .destructive) {
                confirmation.settle(.remove, redact: onRemove)
            }
            .accessibilityIdentifier("ConfirmRedactMessageButton")
            Button(TimelineRedactConfirmCopy.cancel, role: .cancel) {
                confirmation.settle(.cancel) { _ in }
            }
            .accessibilityIdentifier("CancelRedactMessageButton")
        }
    }
}

extension View {
    /// Asks "Remove this message? This can't be undone." before `onRemove`
    /// redacts the requested message; Cancel is the default.
    func timelineRedactConfirmation<Target>(
        _ confirmation: Binding<TimelineRedactConfirmation<Target>>,
        onRemove: @escaping (Target) -> Void
    ) -> some View {
        modifier(TimelineRedactConfirmationDialog(confirmation: confirmation, onRemove: onRemove))
    }
}
