import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

enum SynaraHaptics {
    enum Kind: Equatable {
        case lightImpact
        case success
        case warning
        case selection
    }

    static func trigger(_ kind: Kind) {
        #if canImport(UIKit)
        guard shouldPlayHaptics else {
            return
        }

        switch kind {
        case .lightImpact:
            let generator = UIImpactFeedbackGenerator(style: .light)
            generator.prepare()
            generator.impactOccurred()
        case .success:
            let generator = UINotificationFeedbackGenerator()
            generator.prepare()
            generator.notificationOccurred(.success)
        case .warning:
            let generator = UINotificationFeedbackGenerator()
            generator.prepare()
            generator.notificationOccurred(.warning)
        case .selection:
            let generator = UISelectionFeedbackGenerator()
            generator.prepare()
            generator.selectionChanged()
        }
        #endif
    }

    #if canImport(UIKit)
    private static var shouldPlayHaptics: Bool {
        ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] != "1"
    }
    #endif
}

private struct SynaraHapticFeedbackModifier: ViewModifier {
    let kind: SynaraHaptics.Kind
    let trigger: Int

    func body(content: Content) -> some View {
        if #available(iOS 17.0, *) {
            content
                .sensoryFeedback(sensoryFeedback, trigger: trigger)
        } else {
            content
                .onChange(of: trigger) { _ in
                    SynaraHaptics.trigger(kind)
                }
        }
    }

    @available(iOS 17.0, *)
    private var sensoryFeedback: SensoryFeedback {
        switch kind {
        case .lightImpact:
            return .impact(weight: .light)
        case .success:
            return .success
        case .warning:
            return .warning
        case .selection:
            return .selection
        }
    }
}

extension View {
    func synaraHapticFeedback(_ kind: SynaraHaptics.Kind, trigger: Int) -> some View {
        modifier(SynaraHapticFeedbackModifier(kind: kind, trigger: trigger))
    }
}