import XCTest
@testable import Synara

final class PerformanceFixtureTests: XCTestCase {
    func testLargeRoomFixtureSortPerformance() {
        let rooms = RoomListFixtures.large(count: 1_000)

        measure {
            XCTAssertEqual(RoomListFixtures.sorted(rooms).count, 1_000)
        }
    }

    func testLargeTimelineFixtureCreationPerformance() {
        measure {
            XCTAssertEqual(TimelineFixtures.largeTimeline(count: 10_000).count, 10_000)
        }
    }

    func testTimelineReplyCountPerformance() {
        let items = TimelineFixtures.largeTimeline(count: 10_000)

        measure {
            let counts = TimelineReplyCounter.replyCounts(for: items)
            XCTAssertTrue(counts.isEmpty)
        }
    }
}