import XCTest
@testable import Synara

final class PerformanceFixtureTests: XCTestCase {
    private let roomSortThresholdSeconds = 0.05
    private let timelineFixtureThresholdSeconds = 0.10
    private let replyCountThresholdSeconds = 0.05

    func testLargeRoomFixtureSortPerformance() {
        let rooms = RoomListFixtures.large(count: 1_000)

        measure {
            XCTAssertEqual(RoomListFixtures.sorted(rooms).count, 1_000)
        }

        assertCompletesWithin(roomSortThresholdSeconds) {
            _ = RoomListFixtures.sorted(rooms)
        }
    }

    func testLargeTimelineFixtureCreationPerformance() {
        measure {
            XCTAssertEqual(TimelineFixtures.largeTimeline(count: 10_000).count, 10_000)
        }

        assertCompletesWithin(timelineFixtureThresholdSeconds) {
            _ = TimelineFixtures.largeTimeline(count: 10_000)
        }
    }

    func testTimelineReplyCountPerformance() {
        let items = TimelineFixtures.largeTimeline(count: 10_000)

        measure {
            let counts = TimelineReplyCounter.replyCounts(for: items)
            XCTAssertTrue(counts.isEmpty)
        }

        assertCompletesWithin(replyCountThresholdSeconds) {
            _ = TimelineReplyCounter.replyCounts(for: items)
        }
    }

    private func assertCompletesWithin(_ threshold: TimeInterval, _ block: () -> Void) {
        let start = Date()
        block()
        XCTAssertLessThan(Date().timeIntervalSince(start), threshold)
    }
}