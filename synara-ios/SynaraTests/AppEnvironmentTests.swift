import XCTest
@testable import Synara

final class AppEnvironmentTests: XCTestCase {
    func testMockEnvironmentInstallsExpectedServices() {
        let router = AppRouter()
        let environment = AppEnvironment.mock(router: router)

        XCTAssertTrue(environment.session.currentState == .signedOut)
        XCTAssertEqual(environment.matrix.syncStatusDescription, "Not connected")
        XCTAssertFalse(environment.push.isRegistrationAvailable)
        XCTAssertTrue(environment.router === router)
        XCTAssertTrue(environment.auth is MockAuthService)
    }

    func testSettingsStorePersistsBooleansInMemory() {
        let settings = InMemorySettingsStore()

        XCTAssertFalse(settings.bool(for: "largeText"))

        settings.set(true, for: "largeText")

        XCTAssertTrue(settings.bool(for: "largeText"))
    }
}
