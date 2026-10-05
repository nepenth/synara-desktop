import Foundation
import UserNotifications

/// Shareable, device-local evidence. Deliberately excludes payloads, store
/// paths, account/Matrix identifiers, gateway addresses and raw error messages.
enum SynaraNotificationDiagnosticReport {
    struct Context: Encodable {
        let appVersion: String?
        let appBuild: String?
        let messagePreviewsEnabled: Bool
        let urgentApprovalsEnabled: Bool
        let appGroupAvailable: Bool
        let sharedStoreReady: Bool
        let pushRegistrationAvailable: Bool
        let pushRegistered: Bool
        let pushGatewayConfigured: Bool
        let notificationAuthorization: String
        let alertSetting: String
        let soundSetting: String
        let lockScreenSetting: String
        let notificationCenterSetting: String
        let criticalAlertSetting: String
        let timeSensitiveSetting: String
        let showPreviewsSetting: String
        let criticalAlertsBuildEnabled: Bool
        let notificationFilteringBuildEnabled: Bool

        static func capture(
            settings: UNNotificationSettings,
            pushRegistrationAvailable: Bool,
            pushRegistered: Bool,
            pushGatewayConfigured: Bool,
            bundle: Bundle = .main,
            fileManager: FileManager = .default
        ) -> Context {
            let root = SynaraSharedConstants.sharedCoreStoreRoot(fileManager: fileManager)
            return Context(
                appVersion: publicVersion(bundle.object(forInfoDictionaryKey: "CFBundleShortVersionString")),
                appBuild: publicVersion(bundle.object(forInfoDictionaryKey: "CFBundleVersion")),
                messagePreviewsEnabled: SynaraNotificationPreviewPreference.isEnabled(),
                urgentApprovalsEnabled: SynaraTimeSensitiveAgentApprovalPreference.isEnabled(),
                appGroupAvailable: root != nil,
                sharedStoreReady: root.map { SynaraSharedConstants.sharedCoreStoreIsReady(at: $0, fileManager: fileManager) } ?? false,
                pushRegistrationAvailable: pushRegistrationAvailable,
                pushRegistered: pushRegistered,
                pushGatewayConfigured: pushGatewayConfigured,
                notificationAuthorization: authorizationName(settings.authorizationStatus),
                alertSetting: settingName(settings.alertSetting),
                soundSetting: settingName(settings.soundSetting),
                lockScreenSetting: settingName(settings.lockScreenSetting),
                notificationCenterSetting: settingName(settings.notificationCenterSetting),
                criticalAlertSetting: settingName(settings.criticalAlertSetting),
                timeSensitiveSetting: settingName(settings.timeSensitiveSetting),
                showPreviewsSetting: previewSettingName(settings.showPreviewsSetting),
                criticalAlertsBuildEnabled: SynaraNotificationCapabilities.criticalAlertsEnabled(bundle: bundle),
                notificationFilteringBuildEnabled: SynaraNotificationCapabilities.filteringEnabled(bundle: bundle)
            )
        }
    }

    private struct Report: Encodable {
        let schemaVersion = 1
        let generatedAt: Date
        let context: Context
        let extensionReceiptCount: Int
        let records: [Record]
    }

    private struct Record: Encodable {
        let timestamp: Date
        let runID: UUID?
        let stage: String
    }

    static func text(
        context: Context,
        entries: [SynaraNotificationDiagnosticEntry],
        now: Date = Date()
    ) -> String {
        let bounded = entries.suffix(SynaraNotificationDiagnostics.maximumEntries)
        // Unknown codes never pass through as arbitrary diagnostic text.
        let records = bounded.compactMap { entry -> Record? in
            guard let stage = SynaraNotificationDiagnostics.Stage(rawValue: entry.stage) else { return nil }
            return Record(timestamp: entry.timestamp, runID: entry.runID, stage: stage.rawValue)
        }
        let report = Report(
            generatedAt: now,
            context: context,
            extensionReceiptCount: records.filter { $0.stage == "received" }.count,
            records: records
        )
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? encoder.encode(report) else {
            return "Notification diagnostic report unavailable."
        }
        return String(decoding: data, as: UTF8.self)
    }

    static func publicVersion(_ value: Any?) -> String? {
        guard let value = value as? String, value.isEmpty == false, value.count <= 40,
              value.unicodeScalars.allSatisfy({
                  CharacterSet(charactersIn: "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-_").contains($0)
              }) else { return nil }
        return value
    }

    private static func settingName(_ setting: UNNotificationSetting) -> String {
        switch setting {
        case .notSupported: return "not-supported"
        case .disabled: return "disabled"
        case .enabled: return "enabled"
        @unknown default: return "unknown"
        }
    }

    private static func authorizationName(_ status: UNAuthorizationStatus) -> String {
        switch status {
        case .notDetermined: return "not-determined"
        case .denied: return "denied"
        case .authorized: return "authorized"
        case .provisional: return "provisional"
        case .ephemeral: return "ephemeral"
        @unknown default: return "unknown"
        }
    }

    private static func previewSettingName(_ setting: UNShowPreviewsSetting) -> String {
        switch setting {
        case .always: return "always"
        case .whenAuthenticated: return "when-authenticated"
        case .never: return "never"
        @unknown default: return "unknown"
        }
    }
}
