import Foundation
import UserNotifications

/// Bounds the extension to one Matrix restore/decrypt at a time while every
/// APNs request retains its own delivery handler.
actor NotificationResolutionGate {
    private var isHeld = false
    private var waiterOrder: [UUID] = []
    private var waiters: [UUID: CheckedContinuation<Bool, Never>] = [:]

    func acquire() async -> Bool {
        guard Task.isCancelled == false else { return false }
        if isHeld == false {
            isHeld = true
            return true
        }

        let id = UUID()
        return await withTaskCancellationHandler {
            await withCheckedContinuation { continuation in
                guard Task.isCancelled == false else {
                    continuation.resume(returning: false)
                    return
                }
                waiterOrder.append(id)
                waiters[id] = continuation
            }
        } onCancel: {
            Task { await self.cancelWaiter(id) }
        }
    }

    func release() {
        while waiterOrder.isEmpty == false {
            let id = waiterOrder.removeFirst()
            guard let continuation = waiters.removeValue(forKey: id) else {
                continue
            }
            continuation.resume(returning: true)
            return
        }
        isHeld = false
    }

    private func cancelWaiter(_ id: UUID) {
        waiters.removeValue(forKey: id)?.resume(returning: false)
    }
}

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
    private var requests: [UUID: RequestState] = [:]

    @discardableResult
    func begin(content: UNNotificationContent, handler: @escaping ContentHandler) -> UUID {
        let requestID = UUID()
        let fallback = (content.copy() as? UNNotificationContent) ?? content
        queue.sync {
            requests[requestID] = RequestState(
                id: requestID,
                fallback: fallback,
                handler: handler,
                task: nil,
                cancelCore: nil
            )
        }
        return requestID
    }

    func install(task: Task<Void, Never>, requestID: UUID) {
        let cancel = queue.sync { () -> Bool in
            guard var current = requests[requestID],
                  current.handler != nil else {
                return true
            }
            current.task = task
            requests[requestID] = current
            return false
        }
        if cancel { task.cancel() }
    }

    func installCoreCancellation(_ cancelCore: @escaping () -> Void, requestID: UUID) {
        let cancel = queue.sync { () -> Bool in
            guard var current = requests[requestID],
                  current.handler != nil else {
                return true
            }
            current.cancelCore = cancelCore
            requests[requestID] = current
            return false
        }
        if cancel { cancelCore() }
    }

    func deliver(_ content: UNNotificationContent, requestID: UUID) {
        let delivery = queue.sync { () -> Completion? in
            guard let current = requests.removeValue(forKey: requestID),
                  current.handler != nil else {
                return nil
            }
            return Completion(
                handler: current.handler,
                content: (content.copy() as? UNNotificationContent) ?? content,
                task: nil,
                cancelCore: nil
            )
        }
        complete(delivery, cancelWork: false)
    }

    func expireAll() {
        let deliveries = queue.sync { () -> [Completion] in
            let current = requests.values.map(Self.completion)
            requests.removeAll(keepingCapacity: true)
            return current
        }
        for delivery in deliveries {
            complete(delivery, cancelWork: true)
        }
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
