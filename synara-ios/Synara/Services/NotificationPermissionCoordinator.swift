import Foundation

enum NotificationPermissionSettingsKey {
    static let hasPromptedOnSignIn = "hasPromptedForNotificationPermissionOnSignIn"
}

enum NotificationPermissionCoordinator {
    static func promptOnFirstSignInIfNeeded(environment: AppEnvironment) async {
        guard environment.push.isRegistrationAvailable else {
            return
        }

        guard environment.settings.bool(for: NotificationPermissionSettingsKey.hasPromptedOnSignIn) == false else {
            await registerForPushIfAuthorized(environment: environment)
            return
        }

        let status = await environment.notificationPermission.currentStatus()
        let resolvedStatus: NotificationPermissionStatus

        if status == .notDetermined {
            resolvedStatus = await environment.notificationPermission.requestAuthorization()
            environment.settings.set(true, for: NotificationPermissionSettingsKey.hasPromptedOnSignIn)
            environment.logger.info(
                "Requested notification permission on first sign-in: \(resolvedStatus.displayName)",
                category: .push
            )
        } else {
            resolvedStatus = status
            environment.settings.set(true, for: NotificationPermissionSettingsKey.hasPromptedOnSignIn)
            environment.logger.info(
                "Skipped notification prompt on sign-in; status is \(resolvedStatus.displayName)",
                category: .push
            )
        }

        if resolvedStatus.allowsPushRegistration {
            await MainActor.run {
                environment.push.beginRegistration()
            }
        }
    }

    private static func registerForPushIfAuthorized(environment: AppEnvironment) async {
        let status = await environment.notificationPermission.currentStatus()
        guard status.allowsPushRegistration else {
            return
        }

        await MainActor.run {
            environment.push.beginRegistration()
        }
    }
}