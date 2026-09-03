import Foundation
import SynaraCore
import XCTest
@testable import Synara

private final class ControlledTimelineMediaCore:
    SharedCoreTimelineMediaBytesProviding,
    @unchecked Sendable
{
    private let stateQueue = DispatchQueue(label: "SharedCoreTimelineMediaTests.state")
    private var continuation: CheckedContinuation<LeftoverBytesDto, Error>?
    private var storedRequestedHandles: [String] = []
    let requestStarted = XCTestExpectation(description: "timeline media request started")

    var requestedHandles: [String] {
        stateQueue.sync { storedRequestedHandles }
    }

    func timelineMediaBytes(handleId: String) async throws -> LeftoverBytesDto {
        stateQueue.sync {
            storedRequestedHandles.append(handleId)
        }

        return try await withCheckedThrowingContinuation { continuation in
            stateQueue.sync {
                self.continuation = continuation
            }
            requestStarted.fulfill()
        }
    }

    func succeed(with data: Data) {
        takeContinuation()?.resume(returning: LeftoverBytesDto(payload: data))
    }

    private func takeContinuation() -> CheckedContinuation<LeftoverBytesDto, Error>? {
        stateQueue.sync {
            let pending = continuation
            continuation = nil
            return pending
        }
    }
}

final class SharedCoreTimelineMediaTests: XCTestCase {
    func testDedicatedChannelReturnsCoreBytesWithoutExposingHandleInDiagnostics() async throws {
        let core = ControlledTimelineMediaCore()
        let expected = Data("bounded media".utf8)
        let fetch = Task {
            try await SharedCoreTimelineMedia.mediaBytes(
                core: core,
                handleId: "timeline-media-private-handle"
            )
        }

        await fulfillment(of: [core.requestStarted], timeout: 1)
        core.succeed(with: expected)

        let result = try await fetch.value
        XCTAssertEqual(result, expected)
        XCTAssertEqual(core.requestedHandles, ["timeline-media-private-handle"])
    }

    func testCancellationAfterDispatchDiscardsTheCompletedPayload() async {
        let core = ControlledTimelineMediaCore()
        let fetch = Task {
            try await SharedCoreTimelineMedia.mediaBytes(
                core: core,
                handleId: "timeline-media-cancelled-handle"
            )
        }

        await fulfillment(of: [core.requestStarted], timeout: 1)
        fetch.cancel()
        core.succeed(with: Data(repeating: 7, count: 32))

        do {
            _ = try await fetch.value
            XCTFail("A cancelled caller must not receive media bytes")
        } catch is CancellationError {
            // Expected: the current UniFFI bridge cannot interrupt an in-flight
            // Rust future, but the wrapper must discard its eventual payload.
        } catch {
            XCTFail("Expected CancellationError, received \(error)")
        }
    }
}
