import Foundation

/// Owns signed-in Matrix + push startup so login, restore, and re-login share one path.
enum SessionCoordinator {
    static func startSignedInSession(environment: AppEnvironment, session: AuthenticatedSession) async {
        await environment.matrix.start(session: session)
        environment.push.configure(with: session)
        await environment.matrix.warmSync(session: session)
    }
}