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
    private let resolutionGate = NotificationResolutionGate()

    override func didReceive(
        _ request: UNNotificationRequest,
        withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void
    ) {
        let requestID = coordinator.begin(content: request.content, handler: contentHandler)
        SynaraNotificationDiagnostics.record(.received, runID: requestID)

        guard let content = request.content.mutableCopy() as? UNMutableNotificationContent else {
            logger.error("preview stage=content-copy-failed")
            SynaraNotificationDiagnostics.record(.contentCopyFailed, runID: requestID)
            deliver(request.content, requestID: requestID)
            return
        }

        let showPreview = SynaraNotificationPreviewPreference.isEnabled()
        let timeSensitiveApprovals = SynaraTimeSensitiveAgentApprovalPreference.isEnabled()
        // Gateway metadata can request extension execution, but it cannot grant
        // reaction controls. Remove any proxy-provided approval presentation
        // before local decryption and let the shared classifier add it back.
        if content.categoryIdentifier == "synara.agent-approval" {
            content.categoryIdentifier = ""
            content.interruptionLevel = .active
        }
        guard let payload = SynaraNotificationPreviewPayloadParser.payload(from: request.content.userInfo) else {
            logger.info("preview stage=payload-invalid")
            SynaraNotificationDiagnostics.record(.payloadInvalid, runID: requestID)
            deliver(content, requestID: requestID)
            return
        }
        guard SynaraSharedConstants.appGroupDefaults() != nil else {
            logger.error("preview stage=app-group-unavailable")
            SynaraNotificationDiagnostics.record(.appGroupUnavailable, runID: requestID)
            deliver(content, requestID: requestID)
            return
        }
        guard showPreview || timeSensitiveApprovals else {
            logger.info("preview stage=preferences-disabled")
            SynaraNotificationDiagnostics.record(.preferencesDisabled, runID: requestID)
            deliver(content, requestID: requestID)
            return
        }

        logger.info("preview stage=resolution-started")
        SynaraNotificationDiagnostics.record(.resolutionQueued, runID: requestID)
        let resolver = MatrixNotificationPreviewResolver()
        let logger = self.logger
        let enrichmentTask = Task { [coordinator, resolutionGate] in
            guard await resolutionGate.acquire() else {
                SynaraNotificationDiagnostics.record(.resolutionCancelled, runID: requestID)
                coordinator.deliver(content, requestID: requestID) {
                    SynaraNotificationDiagnostics.record(.delivered, runID: requestID)
                }
                return
            }
            if Task.isCancelled {
                await resolutionGate.release()
                SynaraNotificationDiagnostics.record(.resolutionCancelled, runID: requestID)
                coordinator.deliver(content, requestID: requestID) {
                    SynaraNotificationDiagnostics.record(.delivered, runID: requestID)
                }
                return
            }
            if let resolved = await resolver.resolve(
                for: payload,
                onRequest: { request in
                    coordinator.installCoreCancellation(
                        { request.cancel() },
                        requestID: requestID
                    )
                },
                recordStage: { stage in
                    SynaraNotificationDiagnostics.record(stage, runID: requestID)
                }
            ) {
                var diagnosticStage = SynaraNotificationDiagnostics.Stage.resolvedWithoutPreview
                if showPreview, let preview = resolved.preview {
                    content.title = preview.title
                    content.body = preview.body
                    diagnosticStage = .resolvedPreview
                }
                if timeSensitiveApprovals,
                   resolved.isAgentApproval,
                   SynaraAgentApprovalFreshness.isFresh(
                       originServerTimestampMS: resolved.originServerTimestampMS
                   )
                {
                    if showPreview == false {
                        content.title = "Agent approval needed"
                        content.body = "Review a time-sensitive request in Synara."
                    }
                    content.categoryIdentifier = "synara.agent-approval"
                    content.interruptionLevel = .timeSensitive
                    content.sound = .default
                    diagnosticStage = .resolvedApproval
                }
                logger.info("preview stage=resolved")
                SynaraNotificationDiagnostics.record(diagnosticStage, runID: requestID)
            } else {
                logger.error("preview stage=resolution-failed")
            }
            await resolutionGate.release()
            coordinator.deliver(content, requestID: requestID) {
                SynaraNotificationDiagnostics.record(.delivered, runID: requestID)
            }
        }
        coordinator.install(task: enrichmentTask, requestID: requestID)
    }

    override func serviceExtensionTimeWillExpire() {
        logger.error("preview stage=system-deadline")
        let expiredRequestIDs = coordinator.expireAll()
        SynaraNotificationDiagnostics.recordDeadlineDeliveries(for: expiredRequestIDs)
    }

    private func deliver(_ content: UNNotificationContent, requestID: UUID) {
        coordinator.deliver(content, requestID: requestID) {
            SynaraNotificationDiagnostics.record(.delivered, runID: requestID)
        }
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

    func resolve(
        for payload: SynaraNotificationPreviewPayload,
        onRequest: (NsePreviewRequest) -> Void,
        recordStage: (SynaraNotificationDiagnostics.Stage) -> Void
    ) async -> ResolvedNotificationEvent? {
        let fileManager = FileManager.default
        guard Task.isCancelled == false else {
            logger.info("preview stage=cancelled-before-restore")
            return nil
        }
        guard let session = sessionStore.load() else {
            logger.error("preview stage=shared-session-missing")
            recordStage(.sharedSessionMissing)
            return nil
        }
        guard let storeRoot = SynaraSharedConstants.sharedCoreStoreRoot(fileManager: fileManager),
              SynaraSharedConstants.sharedCoreStoreIsReady(at: storeRoot, fileManager: fileManager) else {
            logger.error("preview stage=shared-store-not-ready")
            recordStage(.sharedStoreNotReady)
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
            return ResolvedNotificationEvent(
                preview: SynaraMatrixEventPreviewComposer.preview(from: SynaraMatrixEventPreviewInput(
                    eventType: event.eventType,
                    senderID: event.senderId,
                    body: event.body,
                    messageType: event.messageType
                )),
                isAgentApproval: event.isAgentApproval,
                originServerTimestampMS: event.originServerTs
            )
        } catch {
            await memorySampler.capture()
            let peakKB = await memorySampler.peakFootprintKB
            logger.error("preview memory failed_peak_footprint_kb=\(peakKB, privacy: .public)")
            logger.error("preview stage=core-resolution-error")
            if let coreError = error as? NseCoreError,
               case let .Failed(code, _) = coreError {
                recordStage(SynaraNotificationDiagnostics.previewFailureStage(coreCode: code))
            } else {
                recordStage(.coreResolutionFailed)
            }
            return nil
        }
    }
}

private struct ResolvedNotificationEvent {
    let preview: SynaraNotificationPreview?
    let isAgentApproval: Bool
    let originServerTimestampMS: UInt64
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
