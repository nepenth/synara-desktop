import Foundation
import SynaraNseCore
import XCTest

final class SynaraNseCoreBindingsTests: XCTestCase {
    func testCancelledRequestCrossesGeneratedBindingAndReadsNoSecrets() async {
        let vault = RecordingNseVault()
        let request = NsePreviewRequest(
            store: vault,
            userId: "@alice:example.org",
            homeserverUrl: "https://matrix.example.org",
            storeRoot: "/tmp/synara-nse-cancelled-binding-test",
            roomId: "!room:example.org",
            eventId: "$event:example.org"
        )

        request.cancel()

        do {
            _ = try await request.resolve()
            XCTFail("A cancelled NSE request must fail closed")
        } catch let error as NseCoreError {
            guard case let .Failed(code, description) = error else {
                return XCTFail("Unexpected NSE error: \(error)")
            }
            XCTAssertEqual(code, "nse-preview-request-cancelled")
            XCTAssertEqual(description, "The notification request was cancelled.")
        } catch {
            XCTFail("Unexpected generated binding error: \(error)")
        }

        XCTAssertEqual(vault.readKeys, [])
    }

    func testCancelIsSafeWhileGeneratedBindingIsResolving() async {
        let vault = BlockingNseVault()
        let request = NsePreviewRequest(
            store: vault,
            userId: "@alice:example.org",
            homeserverUrl: "https://matrix.example.org",
            storeRoot: "/tmp/synara-nse-concurrent-cancel-binding-test",
            roomId: "!room:example.org",
            eventId: "$event:example.org"
        )

        let resolution = Task { try await request.resolve() }
        XCTAssertTrue(vault.waitUntilReadStarts(timeout: 2))

        request.cancel()
        vault.allowReadToFinish()

        do {
            _ = try await resolution.value
            XCTFail("A concurrently cancelled NSE request must fail closed")
        } catch let error as NseCoreError {
            guard case let .Failed(code, _) = error else {
                return XCTFail("Unexpected NSE error: \(error)")
            }
            XCTAssertEqual(code, "nse-preview-request-cancelled")
        } catch {
            XCTFail("Unexpected generated binding error: \(error)")
        }
    }
}

private final class RecordingNseVault: NseSecretVault, @unchecked Sendable {
    private let lock = NSLock()
    private var keys: [String] = []

    var readKeys: [String] {
        lock.withLock { keys }
    }

    func get(key: String) throws -> Data? {
        lock.withLock {
            keys.append(key)
        }
        return nil
    }
}

private final class BlockingNseVault: NseSecretVault, @unchecked Sendable {
    private let readStarted = DispatchSemaphore(value: 0)
    private let finishRead = DispatchSemaphore(value: 0)

    func waitUntilReadStarts(timeout: TimeInterval) -> Bool {
        readStarted.wait(timeout: .now() + timeout) == .success
    }

    func allowReadToFinish() {
        finishRead.signal()
    }

    func get(key _: String) throws -> Data? {
        readStarted.signal()
        _ = finishRead.wait(timeout: .now() + 2)
        return nil
    }
}
