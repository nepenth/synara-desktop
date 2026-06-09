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
                guard let navigationController = Self.findNavigationController(from: self) else {
                    return
                }
                navigationController.interactivePopGestureRecognizer?.isEnabled = true
                navigationController.interactivePopGestureRecognizer?.delegate = nil
            }
        }

        private static func findNavigationController(from viewController: UIViewController) -> UINavigationController? {
            if let navigationController = viewController.navigationController {
                return navigationController
            }

            var current: UIViewController? = viewController
            while let parent = current?.parent {
                if let navigationController = parent as? UINavigationController {
                    return navigationController
                }
                if let navigationController = parent.navigationController {
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
    }
}
#endif

extension View {
    /// Re-enables the native edge swipe-back gesture when the navigation bar is hidden.
    func synaraInteractiveSwipeBack() -> some View {
        #if canImport(UIKit)
        background(NavigationInteractivePopGestureEnabler())
        #else
        self
        #endif
    }
}