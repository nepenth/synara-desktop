import Foundation

/// Owns signed-in Matrix + push startup so login, restore, and re-login share one path.
enum SessionCoordinator {
    static func startSignedInSession(environment: AppEnvironment, session: AuthenticatedSession) async {
        await environment.matrix.start(session: session)
        environment.push.configure(with: session)
        // Room list streaming starts sync after the first interactive load. Starting
        // warm sync here raced with initial room loading and could free the Matrix
        // client while room summaries were still being read.
    }
}