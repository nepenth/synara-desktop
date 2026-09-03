import XCTest
import UserNotifications
@testable import Synara

final class NotificationDeliveryCoordinatorTests: XCTestCase {
    func testResolutionGateSerializesMatrixWork() async {
        let gate = NotificationResolutionGate()
        let firstAcquired = await gate.acquire()
        XCTAssertTrue(firstAcquired)
        let probe = ResolutionGateProbe()
        let second = Task {
            let acquired = await gate.acquire()
            if acquired { await probe.markAcquired() }
            return acquired
        }

        try? await Task.sleep(nanoseconds: 50_000_000)
        let acquiredWhileHeld = await probe.wasAcquired
        XCTAssertFalse(acquiredWhileHeld)
        await gate.release()
        let didAcquireSecond = await second.value
        XCTAssertTrue(didAcquireSecond)
        let acquiredAfterRelease = await probe.wasAcquired
        XCTAssertTrue(acquiredAfterRelease)
        await gate.release()
    }

    func testResolutionGateRemovesCancelledWaiter() async {
        let gate = NotificationResolutionGate()
        let firstAcquired = await gate.acquire()
        XCTAssertTrue(firstAcquired)
        let waiter = Task { await gate.acquire() }
        waiter.cancel()

        let cancelledWaiterAcquired = await waiter.value
        XCTAssertFalse(cancelledWaiterAcquired)
        await gate.release()
        let reacquired = await gate.acquire()
        XCTAssertTrue(reacquired)
        await gate.release()
    }

    func testExpirationDeliversFallbackExactlyOnceAndDoesNotObserveLaterMutation() {
        let coordinator = NotificationDeliveryCoordinator()
        let original = UNMutableNotificationContent()
        original.body = "private fallback"
        var delivered: [String] = []
        let requestID = coordinator.begin(content: original) { content in
            delivered.append(content.body)
        }

        original.body = "mutated after begin"
        let expiredRequestIDs = coordinator.expireAll()
        let lateResolverWon = coordinator.deliver(original, requestID: requestID)

        XCTAssertEqual(expiredRequestIDs, [requestID])
        XCTAssertFalse(lateResolverWon)
        XCTAssertEqual(delivered, ["private fallback"])
    }

    func testConcurrentRequestsCompleteIndependently() {
        let coordinator = NotificationDeliveryCoordinator()
        let first = UNMutableNotificationContent()
        first.body = "first fallback"
        let second = UNMutableNotificationContent()
        second.body = "second fallback"
        var firstDeliveries: [String] = []
        var secondDeliveries: [String] = []

        let firstID = coordinator.begin(content: first) { firstDeliveries.append($0.body) }
        let secondID = coordinator.begin(content: second) { secondDeliveries.append($0.body) }
        let firstResult = UNMutableNotificationContent()
        firstResult.body = "first result"
        let firstWon = coordinator.deliver(firstResult, requestID: firstID)
        let currentResult = UNMutableNotificationContent()
        currentResult.body = "current result"
        let secondWon = coordinator.deliver(currentResult, requestID: secondID)

        XCTAssertTrue(firstWon)
        XCTAssertTrue(secondWon)
        XCTAssertFalse(coordinator.deliver(currentResult, requestID: secondID))
        XCTAssertEqual(firstDeliveries, ["first result"])
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

        let firstExpiredRequestIDs = coordinator.expireAll()
        let secondExpiredRequestIDs = coordinator.expireAll()

        XCTAssertEqual(firstExpiredRequestIDs, [requestID])
        XCTAssertTrue(secondExpiredRequestIDs.isEmpty)
        XCTAssertTrue(task.isCancelled)
        XCTAssertEqual(events.filter { $0 == "core-cancelled" }.count, 1)
        XCTAssertEqual(events.filter { $0 == "handler" }.count, 1)
        XCTAssertLessThan(
            events.firstIndex(of: "core-cancelled")!,
            events.firstIndex(of: "handler")!
        )
    }

    func testExpirationReturnsWinningIDsOnlyAfterFallbackDelivery() {
        let coordinator = NotificationDeliveryCoordinator()
        let content = UNMutableNotificationContent()
        var delivered = false
        _ = coordinator.begin(content: content) { _ in delivered = true }

        let expiredRequestIDs = coordinator.expireAll()

        XCTAssertTrue(delivered)
        XCTAssertEqual(expiredRequestIDs.count, 1)
    }
}

private actor ResolutionGateProbe {
    private(set) var wasAcquired = false

    func markAcquired() {
        wasAcquired = true
    }
}
