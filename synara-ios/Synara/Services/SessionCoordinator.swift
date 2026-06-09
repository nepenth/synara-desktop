import Foundation

/// Owns signed-in Matrix + push startup so login, restore, and re-login share one path.
enum SessionCoordinator {
    static func startSignedInSession(environment: AppEnvironment, session: AuthenticatedSession) async {
        await environment.sessionReadiness.beginPreparing(for: session)
        await environment.matrix.start(session: session)
        environment.push.configure(with: session)
        await environment.sessionReadiness.markPrepared(for: session)
        // Continuous sync starts when the room list begins streaming updates.
    }
}