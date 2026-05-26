import SwiftUI

@main
struct SynaraApp: App {
    private let environment = AppEnvironment.live()

    var body: some Scene {
        WindowGroup {
            RootShellView(environment: environment)
        }
    }
}
