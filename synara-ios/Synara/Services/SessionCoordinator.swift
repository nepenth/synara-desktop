import Foundation

/// Owns signed-in Matrix + push startup so login, restore, and re-login share one path.
enum SessionCoordinator {
    static func startSignedInSession(environment: AppEnvironment, session: AuthenticatedSession) async {
        await environment.sessionReadiness.beginPreparing(for: session)
        await environment.matrix.start(session: session)
        await MainActor.run {
            environment.connectionStatus.update(environment.matrix.syncStatus)
        }
        environment.push.configure(with: session)
        await NotificationPermissionCoordinator.promptOnFirstSignInIfNeeded(environment: environment)
        await environment.sessionReadiness.markPrepared(for: session)
    }
}