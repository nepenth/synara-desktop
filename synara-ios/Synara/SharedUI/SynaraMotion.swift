import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

struct SynaraSendSlideInModifier: ViewModifier {
    let isEnabled: Bool
    let fromTrailing: Bool
    @State private var didAnimate = false

    func body(content: Content) -> some View {
        content
            .offset(x: offsetX)
            .opacity(isEnabled && didAnimate == false ? 0 : 1)
            .onAppear {
                guard isEnabled, didAnimate == false else {
                    didAnimate = true
                    return
                }
                withAnimation(.spring(response: 0.36, dampingFraction: 0.82)) {
                    didAnimate = true
                }
            }
    }

    private var offsetX: CGFloat {
        guard isEnabled, didAnimate == false else {
            return 0
        }
        return fromTrailing ? 28 : -28
    }
}

struct SynaraReactionPopModifier: ViewModifier {
    let animationIndex: Int
    @State private var isVisible = false

    func body(content: Content) -> some View {
        content
            .scaleEffect(isVisible ? 1 : 0.55)
            .opacity(isVisible ? 1 : 0)
            .onAppear {
                let delay = Double(animationIndex) * 0.04
                withAnimation(.spring(response: 0.32, dampingFraction: 0.68).delay(delay)) {
                    isVisible = true
                }
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
        let overlap = max(0, frame.height - bottomSafeAreaInset)
        let isHidden = frame.origin.y >= UIScreen.main.bounds.height

        withAnimation(.easeOut(duration: duration)) {
            keyboardOverlap = isHidden ? 0 : overlap
        }
    }

    private var bottomSafeAreaInset: CGFloat {
        UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .flatMap(\.windows)
            .first { $0.isKeyWindow }?
            .safeAreaInsets.bottom ?? 0
    }
}
#endif

extension View {
    func synaraSendSlideIn(isEnabled: Bool, fromTrailing: Bool) -> some View {
        modifier(SynaraSendSlideInModifier(isEnabled: isEnabled, fromTrailing: fromTrailing))
    }

    func synaraReactionPop(animationIndex: Int = 0) -> some View {
        modifier(SynaraReactionPopModifier(animationIndex: animationIndex))
    }

    func synaraKeyboardAdaptiveInset() -> some View {
        #if canImport(UIKit)
        modifier(SynaraKeyboardAdaptiveInsetModifier())
        #else
        self
        #endif
    }
}