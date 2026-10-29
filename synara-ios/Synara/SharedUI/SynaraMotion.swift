import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

struct SynaraSendSlideInModifier: ViewModifier {
    let isEnabled: Bool
    let fromTrailing: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var didAnimate = false

    func body(content: Content) -> some View {
        content
            .offset(x: offsetX)
            .opacity(isEnabled && didAnimate == false ? 0 : 1)
            .onAppear {
                guard isEnabled, didAnimate == false, reduceMotion == false else {
                    didAnimate = true
                    return
                }
                withAnimation(.easeOut(duration: 0.22)) {
                    didAnimate = true
                }
            }
    }

    private var offsetX: CGFloat {
        guard isEnabled, didAnimate == false else {
            return 0
        }
        return fromTrailing ? 16 : -16
    }
}

struct SynaraReactionPopModifier: ViewModifier {
    let animationIndex: Int
    let animationKey: String
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var isVisible = false

    func body(content: Content) -> some View {
        content
            .scaleEffect(isVisible || reduceMotion ? 1 : 0.85)
            .opacity(isVisible ? 1 : 0)
            .onAppear {
                guard isVisible == false else {
                    return
                }
                pop(delay: Double(animationIndex) * 0.04)
            }
            .onChange(of: animationKey) { _ in
                // A changed count (for example your own reaction joining an
                // existing chip) pops the same chip again. `onAppear` does not
                // re-run for it, so it must become visible here.
                var reset = Transaction()
                reset.disablesAnimations = true
                withTransaction(reset) {
                    isVisible = false
                }
                DispatchQueue.main.async {
                    pop(delay: 0)
                }
            }
    }

    /// A short settle with no overshoot; under Reduce Motion the chip
    /// fades in place.
    private func pop(delay: Double) {
        let animation: Animation = reduceMotion
            ? .easeOut(duration: 0.15)
            : .spring(response: 0.28, dampingFraction: 1).delay(delay)
        withAnimation(animation) {
            isVisible = true
        }
    }
}

#if canImport(UIKit)
private struct SynaraKeyboardAdaptiveInsetModifier: ViewModifier {
    @State private var keyboardOverlap: CGFloat = 0

    func body(content: Content) -> some View {
        content
            .padding(.bottom, keyboardOverlap)
            .onReceive(NotificationCenter.default.publisher(for: UIResponder.keyboardWillChangeFrameNotification)) { notification in
                applyKeyboardFrame(from: notification)
            }
    }

    private func applyKeyboardFrame(from notification: Notification) {
        guard let frame = notification.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect else {
            return
        }

        let duration = (notification.userInfo?[UIResponder.keyboardAnimationDurationUserInfoKey] as? Double) ?? 0.25
        let overlap = keyboardOverlapAmount(for: frame)

        withAnimation(.easeOut(duration: duration)) {
            keyboardOverlap = overlap
        }
    }

    private func keyboardOverlapAmount(for keyboardFrame: CGRect) -> CGFloat {
        guard let window = UIApplication.shared.connectedScenes
            .compactMap({ $0 as? UIWindowScene })
            .flatMap(\.windows)
            .first(where: \.isKeyWindow) else {
            return 0
        }

        let keyboardFrameInWindow = window.convert(keyboardFrame, from: nil)
        let intersection = window.bounds.intersection(keyboardFrameInWindow)
        guard intersection.height > 0 else {
            return 0
        }

        // The system already reserves the home-indicator safe area; subtract it so we do not double-lift.
        return max(0, intersection.height - window.safeAreaInsets.bottom)
    }
}
#endif

extension View {
    func synaraSendSlideIn(isEnabled: Bool, fromTrailing: Bool) -> some View {
        modifier(SynaraSendSlideInModifier(isEnabled: isEnabled, fromTrailing: fromTrailing))
    }

    func synaraReactionPop(animationIndex: Int = 0, animationKey: String = "") -> some View {
        modifier(SynaraReactionPopModifier(animationIndex: animationIndex, animationKey: animationKey))
    }

    func synaraKeyboardAdaptiveInset() -> some View {
        #if canImport(UIKit)
        modifier(SynaraKeyboardAdaptiveInsetModifier())
        #else
        self
        #endif
    }
}