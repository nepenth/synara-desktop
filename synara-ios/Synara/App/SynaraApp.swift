import SwiftUI

@main
struct SynaraApp: App {
    private let environment: AppEnvironment = {
        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1" {
            return .mock()
        }
        return .live()
    }()

    var body: some Scene {
        WindowGroup {
            RootShellView(environment: environment)
        }
    }
}
