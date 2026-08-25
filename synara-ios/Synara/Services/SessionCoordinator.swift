import Foundation

/// Owns signed-in Matrix + push startup so login, restore, and re-login share one path.
enum SessionCoordinator {
    /// Prepares only the Matrix owner graph needed for exact notification
    /// actions. This deliberately excludes push registration and permission UI
    /// so an OS action never waits on unrelated shell startup.
    static func prepareMatrixOwner(environment: AppEnvironment, session: AuthenticatedSession) async -> Bool {
        guard await environment.sessionReadiness.claimPreparation(for: session) else {
            return await environment.sessionReadiness.waitUntilPrepared(for: session)
        }
        return await withTaskCancellationHandler {
            await environment.matrix.start(session: session)
            guard Task.isCancelled == false else {
                await environment.sessionReadiness.cancelPreparation(for: session)
                return false
            }
            await MainActor.run {
                environment.connectionStatus.update(environment.matrix.syncStatus)
            }
            return await environment.sessionReadiness.markPrepared(for: session)
        } onCancel: {
            Task {
                await environment.sessionReadiness.cancelPreparation(for: session)
            }
        }
    }

    static func startSignedInSession(environment: AppEnvironment, session: AuthenticatedSession) async {
        guard await prepareMatrixOwner(environment: environment, session: session) else { return }
        environment.push.configure(with: session)
        await NotificationPermissionCoordinator.promptOnFirstSignInIfNeeded(environment: environment)
    }
}
