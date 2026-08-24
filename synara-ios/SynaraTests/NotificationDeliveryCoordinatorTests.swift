import XCTest
import UserNotifications
@testable import Synara

final class NotificationDeliveryCoordinatorTests: XCTestCase {
    func testExpirationDeliversFallbackExactlyOnceAndDoesNotObserveLaterMutation() {
        let coordinator = NotificationDeliveryCoordinator()
        let original = UNMutableNotificationContent()
        original.body = "private fallback"
        var delivered: [String] = []
        let requestID = coordinator.begin(content: original) { content in
            delivered.append(content.body)
        }

        original.body = "mutated after begin"
        coordinator.expireCurrent()
        coordinator.deliver(original, requestID: requestID)

        XCTAssertEqual(delivered, ["private fallback"])
    }

    func testStaleCompletionCannotDeliverTheNewestRequest() {
        let coordinator = NotificationDeliveryCoordinator()
        let first = UNMutableNotificationContent()
        first.body = "first fallback"
        let second = UNMutableNotificationContent()
        second.body = "second fallback"
        var firstDeliveries: [String] = []
        var secondDeliveries: [String] = []

        let firstID = coordinator.begin(content: first) { firstDeliveries.append($0.body) }
        let secondID = coordinator.begin(content: second) { secondDeliveries.append($0.body) }
        let staleResult = UNMutableNotificationContent()
        staleResult.body = "stale result"
        coordinator.deliver(staleResult, requestID: firstID)
        let currentResult = UNMutableNotificationContent()
        currentResult.body = "current result"
        coordinator.deliver(currentResult, requestID: secondID)

        XCTAssertEqual(firstDeliveries, ["first fallback"])
        XCTAssertEqual(secondDeliveries, ["current result"])
    }

    func testExpirationCancelsSwiftAndCoreWorkBeforeCallingHandler() {
        let coordinator = NotificationDeliveryCoordinator()
        let content = UNMutableNotificationContent()
        var events: [String] = []
        let requestID = coordinator.begin(content: content) { _ in events.append("handler") }
        let task = Task<Void, Never> {
            try? await Task.sleep(nanoseconds: 30_000_000_000)
        }
        coordinator.install(task: task, requestID: requestID)
        coordinator.installCoreCancellation({ events.append("core-cancelled") }, requestID: requestID)

        coordinator.expireCurrent()
        coordinator.expireCurrent()

        XCTAssertTrue(task.isCancelled)
        XCTAssertEqual(events.filter { $0 == "core-cancelled" }.count, 1)
        XCTAssertEqual(events.filter { $0 == "handler" }.count, 1)
        XCTAssertLessThan(
            events.firstIndex(of: "core-cancelled")!,
            events.firstIndex(of: "handler")!
        )
    }
}
