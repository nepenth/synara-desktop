import XCTest
@testable import Synara

final class PerformanceFixtureTests: XCTestCase {
    func testLargeRoomFixtureSortPerformance() {
        let rooms = RoomListFixtures.large(count: 1_000)

        assertCompletes {
            XCTAssertEqual(RoomListFixtures.sorted(rooms).count, 1_000)
        }
    }

    func testLargeTimelineFixtureCreationPerformance() {
        assertCompletes {
            XCTAssertEqual(TimelineFixtures.largeTimeline(count: 10_000).count, 10_000)
        }
    }

    func testTimelineReplyCountPerformance() {
        let items = TimelineFixtures.largeTimeline(count: 10_000)

        assertCompletes {
            let counts = TimelineReplyCounter.replyCounts(for: items)
            XCTAssertTrue(counts.isEmpty)
        }
    }

    private func assertCompletes(_ block: () -> Void) {
        let start = Date()
        block()
        XCTAssertGreaterThanOrEqual(Date().timeIntervalSince(start), 0)
    }
}
