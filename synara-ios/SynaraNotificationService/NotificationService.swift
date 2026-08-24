import Foundation
import OSLog
import Security
import SynaraNseCore
import UserNotifications
import Darwin.Mach

final class NotificationService: UNNotificationServiceExtension {
    private let logger = Logger(
        subsystem: "com.whylandcreative.synara.notification-service",
        category: "preview"
    )
    private let coordinator = NotificationDeliveryCoordinator()

    override func didReceive(
        _ request: UNNotificationRequest,
        withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void
    ) {
        let requestID = coordinator.begin(content: request.content, handler: contentHandler)

        guard let content = request.content.mutableCopy() as? UNMutableNotificationContent else {
            logger.error("preview stage=content-copy-failed")
            deliver(request.content, requestID: requestID)
            return
        }

        guard SynaraNotificationPreviewPreference.isEnabled() else {
            logger.info("preview stage=preference-disabled")
            deliver(content, requestID: requestID)
            return
        }
        guard let payload = SynaraNotificationPreviewPayloadParser.payload(from: request.content.userInfo) else {
            logger.info("preview stage=payload-invalid")
            deliver(content, requestID: requestID)
            return
        }
        guard payload.isAgentApproval == false else {
            logger.info("preview stage=agent-approval-generic")
            deliver(content, requestID: requestID)
            return
        }

        logger.info("preview stage=resolution-started")
        let resolver = MatrixNotificationPreviewResolver()
        let logger = self.logger
        let enrichmentTask = Task { [coordinator] in
            if let preview = await resolver.preview(for: payload, onRequest: { request in
                coordinator.installCoreCancellation(
                    { request.cancel() },
                    requestID: requestID
                )
            }) {
                content.title = preview.title
                content.body = preview.body
                logger.info("preview stage=resolved")
            } else {
                logger.error("preview stage=resolution-failed")
            }
            coordinator.deliver(content, requestID: requestID)
        }
        coordinator.install(task: enrichmentTask, requestID: requestID)
    }

    override func serviceExtensionTimeWillExpire() {
        logger.error("preview stage=system-deadline")
        coordinator.expireCurrent()
    }

    private func deliver(_ content: UNNotificationContent, requestID: UUID) {
        coordinator.deliver(content, requestID: requestID)
    }
}

private struct StoredSynaraSession: Decodable {
    let userID: String
    let homeserverURL: URL
}

private struct StoredSynaraSessionEnvelope: Decodable {
    let version: Int
    let session: StoredSynaraSession
}

private struct NotificationSessionStore {
    private let service = "com.whylandcreative.synara.session"
    private let account = "current"

    func load() -> StoredSynaraSession? {
        guard let accessGroup = Self.sharedAccessGroup() else {
            return nil
        }

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrAccessGroup as String: accessGroup,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess, let data = item as? Data else {
            return nil
        }

        if let envelope = try? JSONDecoder().decode(StoredSynaraSessionEnvelope.self, from: data),
           envelope.version == 1 {
            return envelope.session
        }

        return try? JSONDecoder().decode(StoredSynaraSession.self, from: data)
    }

    static func sharedAccessGroup(bundle: Bundle = .main) -> String? {
        guard let value = bundle.object(forInfoDictionaryKey: SynaraSharedConstants.keychainAccessGroupInfoKey) as? String else {
            return nil
        }

        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false,
              trimmed.contains("$(") == false else {
            return nil
        }

        return trimmed
    }
}

private struct MatrixNotificationPreviewResolver {
    private let sessionStore: NotificationSessionStore
    private let logger = Logger(
        subsystem: "com.whylandcreative.synara.notification-service",
        category: "preview"
    )

    init(sessionStore: NotificationSessionStore = NotificationSessionStore()) {
        self.sessionStore = sessionStore
    }

    func preview(
        for payload: SynaraNotificationPreviewPayload,
        onRequest: (NsePreviewRequest) -> Void
    ) async -> SynaraNotificationPreview? {
        let fileManager = FileManager.default
        guard Task.isCancelled == false else {
            logger.info("preview stage=cancelled-before-restore")
            return nil
        }
        guard let session = sessionStore.load() else {
            logger.error("preview stage=shared-session-missing")
            return nil
        }
        guard let storeRoot = SynaraSharedConstants.sharedCoreStoreRoot(fileManager: fileManager),
              SynaraSharedConstants.sharedCoreStoreIsReady(at: storeRoot, fileManager: fileManager) else {
            logger.error("preview stage=shared-store-not-ready")
            return nil
        }

        let memorySampler = NotificationMemorySampler()
        await memorySampler.capture()
        let samplingTask = Task {
            while Task.isCancelled == false {
                await memorySampler.capture()
                try? await Task.sleep(nanoseconds: 100_000_000)
            }
        }
        defer { samplingTask.cancel() }
        do {
            let request = NsePreviewRequest(
                store: NotificationKeychainNseSecretVault(),
                userId: session.userID,
                homeserverUrl: session.homeserverURL.absoluteString,
                storeRoot: storeRoot.path,
                roomId: payload.roomID,
                eventId: payload.eventID
            )
            onRequest(request)
            guard Task.isCancelled == false else {
                request.cancel()
                return nil
            }
            let event = try await request.resolve()
            await memorySampler.capture()
            let peakKB = await memorySampler.peakFootprintKB
            logger.info("preview memory peak_footprint_kb=\(peakKB, privacy: .public)")
            guard Task.isCancelled == false else { return nil }
            return SynaraMatrixEventPreviewComposer.preview(
                from: SynaraMatrixEventPreviewInput(
                    eventType: event.eventType,
                    senderID: event.senderId,
                    body: event.body,
                    messageType: event.messageType
                )
            )
        } catch {
            await memorySampler.capture()
            let peakKB = await memorySampler.peakFootprintKB
            logger.error("preview memory failed_peak_footprint_kb=\(peakKB, privacy: .public)")
            logger.error("preview stage=core-resolution-error")
            return nil
        }
    }
}

private actor NotificationMemorySampler {
    private(set) var peakFootprintKB: UInt64 = 0

    func capture() {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(
            MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<natural_t>.size
        )
        let status = withUnsafeMutablePointer(to: &info) { pointer in
            pointer.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { rebound in
                task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), rebound, &count)
            }
        }
        guard status == KERN_SUCCESS else { return }
        peakFootprintKB = max(peakFootprintKB, info.phys_footprint / 1_024)
    }
}

private final class NotificationKeychainNseSecretVault: NseSecretVault, @unchecked Sendable {
    private let service = "com.whylandcreative.synara.core-vault"
    private let accessGroup: String?

    init(bundle: Bundle = .main) {
        self.accessGroup = NotificationSessionStore.sharedAccessGroup(bundle: bundle)
    }

    func get(key: String) throws -> Data? {
        guard let accessGroup else { throw Self.unavailable }
        var query = baseQuery(key: key, accessGroup: accessGroup)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw Self.unavailable }
        return item as? Data
    }

    private func baseQuery(key: String, accessGroup: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecAttrAccessGroup as String: accessGroup,
        ]
    }

    private static var unavailable: NseSecretVaultError {
        .Unavailable(
            code: "p4-s3-secret-vault-unavailable",
            description: "The secret store is unavailable."
        )
    }
}
