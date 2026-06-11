import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

#if canImport(UIKit)
struct NavigationInteractivePopGestureEnabler: UIViewControllerRepresentable {
    func makeUIViewController(context: Context) -> UIViewController {
        PopGestureViewController()
    }

    func updateUIViewController(_ uiViewController: UIViewController, context: Context) {
        guard let controller = uiViewController as? PopGestureViewController else {
            return
        }
        controller.enableInteractivePopGesture()
    }

    private final class PopGestureViewController: UIViewController {
        override func viewDidAppear(_ animated: Bool) {
            super.viewDidAppear(animated)
            enableInteractivePopGesture()
        }

        override func didMove(toParent parent: UIViewController?) {
            super.didMove(toParent: parent)
            enableInteractivePopGesture()
        }

        func enableInteractivePopGesture() {
            DispatchQueue.main.async { [weak self] in
                guard let self else {
                    return
                }

                if let navigationController = Self.findNavigationController(from: self)
                    ?? Self.findActiveNavigationController() {
                    navigationController.interactivePopGestureRecognizer?.isEnabled = true
                    navigationController.interactivePopGestureRecognizer?.delegate = nil
                }
            }
        }

        private static func findNavigationController(from viewController: UIViewController) -> UINavigationController? {
            if let navigationController = viewController.navigationController,
               navigationController.viewControllers.count > 1 {
                return navigationController
            }

            var current: UIViewController? = viewController
            while let parent = current?.parent {
                if let navigationController = parent as? UINavigationController,
                   navigationController.viewControllers.count > 1 {
                    return navigationController
                }
                if let navigationController = parent.navigationController,
                   navigationController.viewControllers.count > 1 {
                    return navigationController
                }
                current = parent
            }

            for child in viewController.children {
                if let navigationController = findNavigationController(from: child) {
                    return navigationController
                }
            }

            return nil
        }

        private static func findActiveNavigationController() -> UINavigationController? {
            guard let rootViewController = keyWindowRootViewController() else {
                return nil
            }

            var candidates: [UINavigationController] = []
            collectNavigationControllers(from: rootViewController, into: &candidates)
            return candidates.first(where: { $0.viewControllers.count > 1 }) ?? candidates.last
        }

        private static func keyWindowRootViewController() -> UIViewController? {
            let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
            for scene in scenes {
                if let window = scene.windows.first(where: \.isKeyWindow) ?? scene.windows.first {
                    return window.rootViewController
                }
            }
            return nil
        }

        private static func collectNavigationControllers(
            from viewController: UIViewController?,
            into candidates: inout [UINavigationController]
        ) {
            guard let viewController else {
                return
            }

            if let navigationController = viewController as? UINavigationController {
                candidates.append(navigationController)
            }

            if let navigationController = viewController.navigationController {
                candidates.append(navigationController)
            }

            for child in viewController.children {
                collectNavigationControllers(from: child, into: &candidates)
            }

            if let presentedViewController = viewController.presentedViewController {
                collectNavigationControllers(from: presentedViewController, into: &candidates)
            }

            if let tabBarController = viewController as? UITabBarController {
                collectNavigationControllers(from: tabBarController.selectedViewController, into: &candidates)
            }
        }
    }
}
#endif

private struct SynaraEdgeSwipeBackModifier: ViewModifier {
    @Environment(\.dismiss) private var dismiss

    private let edgeActivationWidth: CGFloat = 28
    private let commitTranslation: CGFloat = 96
    private let maxParallax: CGFloat = 72

    @State private var dragOffset: CGFloat = 0

    func body(content: Content) -> some View {
        content
            .offset(x: dragOffset)
            .overlay(alignment: .leading) {
                Color.clear
                    .frame(width: edgeActivationWidth)
                    .contentShape(Rectangle())
                    .gesture(edgeDragGesture)
                    .accessibilityIdentifier("SynaraEdgeSwipeBack")
            }
    }

    private var edgeDragGesture: some Gesture {
        DragGesture(minimumDistance: 12, coordinateSpace: .global)
            .onChanged { value in
                guard value.startLocation.x <= edgeActivationWidth,
                      value.translation.width > 0,
                      abs(value.translation.width) > abs(value.translation.height) else {
                    return
                }

                dragOffset = min(value.translation.width * 0.35, maxParallax)
            }
            .onEnded { value in
                defer {
                    withAnimation(.easeOut(duration: 0.18)) {
                        dragOffset = 0
                    }
                }

                guard value.startLocation.x <= edgeActivationWidth,
                      value.translation.width > 0,
                      abs(value.translation.width) > abs(value.translation.height) else {
                    return
                }

                if value.translation.width >= commitTranslation
                    || value.predictedEndTranslation.width >= commitTranslation * 1.4 {
                    dismiss()
                }
            }
    }
}

extension View {
    /// Re-enables native edge swipe-back when the navigation bar is hidden and adds a leading-edge fallback.
    func synaraInteractiveSwipeBack() -> some View {
        modifier(SynaraEdgeSwipeBackModifier())
            #if canImport(UIKit)
            .background(NavigationInteractivePopGestureEnabler())
            #endif
    }
}