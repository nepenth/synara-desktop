import Foundation
import UserNotifications

/// Owns every mutable piece of notification-extension delivery state.
///
/// State transitions are serialized, while cancellation and Apple's handler
/// are deliberately invoked after leaving the queue to avoid re-entrancy and
/// lock inversion with framework code.
final class NotificationDeliveryCoordinator: @unchecked Sendable {
    typealias ContentHandler = (UNNotificationContent) -> Void

    private struct RequestState {
        let id: UUID
        let fallback: UNNotificationContent
        var handler: ContentHandler?
        var task: Task<Void, Never>?
        var cancelCore: (() -> Void)?
    }

    private struct Completion {
        let handler: ContentHandler?
        let content: UNNotificationContent
        let task: Task<Void, Never>?
        let cancelCore: (() -> Void)?
    }

    private let queue = DispatchQueue(
        label: "com.whylandcreative.synara.notification-service.delivery"
    )
    private var state: RequestState?

    @discardableResult
    func begin(content: UNNotificationContent, handler: @escaping ContentHandler) -> UUID {
        let requestID = UUID()
        let fallback = (content.copy() as? UNNotificationContent) ?? content
        let displaced = queue.sync { () -> Completion? in
            let previous = state.map(Self.completion)
            state = RequestState(
                id: requestID,
                fallback: fallback,
                handler: handler,
                task: nil,
                cancelCore: nil
            )
            return previous
        }
        complete(displaced, cancelWork: true)
        return requestID
    }

    func install(task: Task<Void, Never>, requestID: UUID) {
        let cancel = queue.sync { () -> Bool in
            guard var current = state,
                  current.id == requestID,
                  current.handler != nil else {
                return true
            }
            current.task = task
            state = current
            return false
        }
        if cancel { task.cancel() }
    }

    func installCoreCancellation(_ cancelCore: @escaping () -> Void, requestID: UUID) {
        let cancel = queue.sync { () -> Bool in
            guard var current = state,
                  current.id == requestID,
                  current.handler != nil else {
                return true
            }
            current.cancelCore = cancelCore
            state = current
            return false
        }
        if cancel { cancelCore() }
    }

    func deliver(_ content: UNNotificationContent, requestID: UUID) {
        let delivery = queue.sync { () -> Completion? in
            guard let current = state,
                  current.id == requestID,
                  current.handler != nil else {
                return nil
            }
            state = nil
            return Completion(
                handler: current.handler,
                content: (content.copy() as? UNNotificationContent) ?? content,
                task: nil,
                cancelCore: nil
            )
        }
        complete(delivery, cancelWork: false)
    }

    func expireCurrent() {
        let delivery = queue.sync { () -> Completion? in
            guard let current = state else { return nil }
            state = nil
            return Self.completion(current)
        }
        complete(delivery, cancelWork: true)
    }

    private static func completion(_ state: RequestState) -> Completion {
        Completion(
            handler: state.handler,
            content: state.fallback,
            task: state.task,
            cancelCore: state.cancelCore
        )
    }

    private func complete(_ completion: Completion?, cancelWork: Bool) {
        guard let completion else { return }
        if cancelWork {
            completion.task?.cancel()
            completion.cancelCore?()
        }
        completion.handler?(completion.content)
    }
}
