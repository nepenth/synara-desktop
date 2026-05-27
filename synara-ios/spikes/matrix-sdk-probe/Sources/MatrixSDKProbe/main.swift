import Foundation
import MatrixRustSDK

private enum ProbeError: Error, CustomStringConvertible {
    case missingEnvironment([String])
    case roomNotFound(String)
    case encryptedRoomRequired(EncryptionState)

    var description: String {
        switch self {
        case let .missingEnvironment(names):
            return "Missing required environment values: \(names.joined(separator: ", "))"
        case let .roomNotFound(room):
            return "Unable to resolve, join, or find Matrix room: \(room)"
        case let .encryptedRoomRequired(state):
            return "Expected encrypted room, got encryption state: \(state)"
        }
    }
}

private final class UtdRecorder: UnableToDecryptDelegate, @unchecked Sendable {
    private let lock = NSLock()
    private var eventIds: [String] = []

    var count: Int {
        lock.lock()
        defer { lock.unlock() }
        return eventIds.count
    }

    func onUtd(info: UnableToDecryptInfo) {
        lock.lock()
        defer { lock.unlock() }
        eventIds.append(info.eventId)
    }
}

private final class TimelineRecorder: TimelineListener, @unchecked Sendable {
    private let lock = NSLock()
    private var updateCount = 0
    private var eventCount = 0
    private var unableToDecryptCount = 0

    var summary: String {
        lock.lock()
        defer { lock.unlock() }
        return "updates=\(updateCount), events=\(eventCount), unableToDecrypt=\(unableToDecryptCount)"
    }

    func onUpdate(diff: [TimelineDiff]) {
        lock.lock()
        defer { lock.unlock() }
        updateCount += 1
        for item in Self.items(from: diff) {
            guard let event = item.asEvent() else {
                continue
            }
            eventCount += 1
            if Self.isUnableToDecrypt(event.content) {
                unableToDecryptCount += 1
            }
        }
    }

    private static func items(from diffs: [TimelineDiff]) -> [TimelineItem] {
        diffs.flatMap { diff in
            switch diff {
            case let .append(values), let .reset(values):
                return values
            case let .pushFront(value), let .pushBack(value), let .insert(_, value), let .set(_, value):
                return [value]
            case .clear, .popFront, .popBack, .remove, .truncate:
                return []
            }
        }
    }

    private static func isUnableToDecrypt(_ content: TimelineItemContent) -> Bool {
        guard case let .msgLike(messageLike) = content else {
            return false
        }
        guard case .unableToDecrypt = messageLike.kind else {
            return false
        }
        return true
    }
}

@main
enum MatrixSDKProbe {
    static func main() async {
        do {
            if liveProbeRequested {
                try await runLiveE2EEProbe()
            } else {
                print(importProbeSummary)
            }
        } catch {
            fputs("MatrixSDKProbe failed: \(error)\n", stderr)
            Foundation.exit(1)
        }
    }

    private static var liveProbeRequested: Bool {
        let env = ProcessInfo.processInfo.environment
        return env["SYNARA_MATRIX_PROBE"] == "live-e2ee"
            || env["SYNARA_E2EE_HOMESERVER"] != nil
            || env["SYNARA_LIVE_HOMESERVER"] != nil
    }

    private static var importProbeSummary: String {
        """
        MatrixRustSDK import succeeded.

        Set SYNARA_MATRIX_PROBE=live-e2ee plus SYNARA_E2EE_HOMESERVER,
        SYNARA_E2EE_USERNAME, SYNARA_E2EE_PASSWORD, and SYNARA_E2EE_ROOM to run
        the disposable encrypted-room validation probe.
        """
    }

    private static func runLiveE2EEProbe() async throws {
        let config = try LiveProbeConfig.fromEnvironment()
        try config.prepareStore()

        print("MatrixRustSDK live E2EE probe starting.")
        print("Homeserver: \(config.safeHomeserverDescription)")
        print("Room: \(config.room)")
        print("Send probe message: \(config.sendProbeMessage ? "yes" : "no")")

        let client = try await ClientBuilder()
            .homeserverUrl(url: config.homeserver)
            .sessionPaths(dataPath: config.dataPath.path, cachePath: config.cachePath.path)
            .build()
        // The current macOS SwiftPM probe can panic when the Rust-backed client
        // tears down SQLite pools outside a Tokio reactor after an error path.
        // Leaking this short-lived probe object keeps failures reportable.
        _ = Unmanaged.passRetained(client)

        let utdRecorder = UtdRecorder()
        try await client.setUtdDelegate(utdDelegate: utdRecorder)

        try await client.login(
            username: config.username,
            password: config.password,
            initialDeviceName: "Synara Matrix SDK E2EE Probe",
            deviceId: nil
        )
        let session = try client.session()
        print("Login succeeded for \(session.userId); device=\(session.deviceId).")

        await client.encryption().waitForE2eeInitializationTasks()
        print("E2EE initialization tasks completed.")

        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: true))

        let room = try await resolveOrJoinRoom(config.room, client: client)
        if room.membership() == .invited || room.membership() == .left {
            try await room.join()
            _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: true))
        }

        let latestEncryptionState = try await room.latestEncryptionState()
        let currentEncryptionState = room.encryptionState()
        let isEncrypted = await room.isEncrypted()
        print("Room membership: \(room.membership()).")
        print("Room encryption state: current=\(currentEncryptionState), latest=\(latestEncryptionState), isEncrypted=\(isEncrypted).")

        guard latestEncryptionState == .encrypted || currentEncryptionState == .encrypted || isEncrypted else {
            throw ProbeError.encryptedRoomRequired(latestEncryptionState)
        }

        let timeline = try await room.timeline()
        let timelineRecorder = TimelineRecorder()
        let timelineTask = await timeline.addListener(listener: timelineRecorder)
        defer { timelineTask.cancel() }

        _ = try await timeline.paginateBackwards(numEvents: 20)

        if config.sendProbeMessage {
            let body = "Synara E2EE probe \(ISO8601DateFormatter().string(from: Date()))"
            let content = messageEventContentFromMarkdown(md: body)
            _ = try await timeline.send(msg: content)
            _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: false))
            print("Encrypted send path accepted a probe message.")
        }

        print("Timeline listener summary: \(timelineRecorder.summary).")
        print("Unable-to-decrypt delegate count: \(utdRecorder.count).")
        print("MatrixRustSDK live E2EE probe completed.")
    }

    private static func resolveOrJoinRoom(_ roomReference: String, client: Client) async throws -> Room {
        if roomReference.hasPrefix("!") {
            if let existingRoom = try client.getRoom(roomId: roomReference) {
                return existingRoom
            }
            return try await client.joinRoomById(roomId: roomReference)
        }

        if roomReference.hasPrefix("#") {
            if let existingRoom = findRoom(roomReference, in: client.rooms()) {
                return existingRoom
            }
            do {
                return try await client.joinRoomByIdOrAlias(roomIdOrAlias: roomReference, serverNames: [])
            } catch {
                printRoomDiagnostics(client.rooms())
                throw error
            }
        }

        if let existingRoom = findRoom(roomReference, in: client.rooms()) {
            return existingRoom
        }

        throw ProbeError.roomNotFound(roomReference)
    }

    private static func findRoom(_ roomReference: String, in rooms: [Room]) -> Room? {
        rooms.first { room in
            roomMatches(room, roomReference: roomReference)
        }
    }

    private static func roomMatches(_ room: Room, roomReference: String) -> Bool {
        let roomValues = [room.id(), room.displayName(), room.canonicalAlias()].compactMap(\.self)
            + room.alternativeAliases()
        let normalizedTargets = normalizedRoomValues(roomReference)

        return roomValues.contains(roomReference)
            || roomValues.map(normalizedRoomValue).contains { normalizedTargets.contains($0) }
    }

    private static func normalizedRoomValues(_ value: String) -> Set<String> {
        let normalized = normalizedRoomValue(value)
        var values = Set([normalized])
        if let separator = normalized.firstIndex(of: ":") {
            values.insert(String(normalized[..<separator]))
        }
        values.insert(normalized.trimmingCharacters(in: CharacterSet(charactersIn: "#!")))
        return values
    }

    private static func normalizedRoomValue(_ value: String) -> String {
        value
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .trimmingCharacters(in: CharacterSet(charactersIn: "#!"))
            .lowercased()
    }

    private static func printRoomDiagnostics(_ rooms: [Room]) {
        print("Discovered rooms: \(rooms.count).")
        for room in rooms.prefix(10) {
            let name = room.displayName() ?? "(no display name)"
            let alias = room.canonicalAlias() ?? "(no canonical alias)"
            print("Room candidate: id=\(room.id()), membership=\(room.membership()), name=\(name), alias=\(alias)")
        }
    }
}

private struct LiveProbeConfig {
    let homeserver: String
    let username: String
    let password: String
    let room: String
    let sendProbeMessage: Bool
    let dataPath: URL
    let cachePath: URL

    var safeHomeserverDescription: String {
        URL(string: homeserver)?.host ?? homeserver
    }

    static func fromEnvironment() throws -> LiveProbeConfig {
        let env = ProcessInfo.processInfo.environment
        let homeserver = env["SYNARA_E2EE_HOMESERVER"] ?? env["SYNARA_LIVE_HOMESERVER"]
        let username = env["SYNARA_E2EE_USERNAME"] ?? env["SYNARA_LIVE_USERNAME"]
        let password = env["SYNARA_E2EE_PASSWORD"] ?? env["SYNARA_LIVE_PASSWORD"]
        let room = env["SYNARA_E2EE_ROOM"]

        let required = [
            ("SYNARA_E2EE_HOMESERVER", homeserver),
            ("SYNARA_E2EE_USERNAME", username),
            ("SYNARA_E2EE_PASSWORD", password),
            ("SYNARA_E2EE_ROOM", room)
        ]
        let missing = required.compactMap { name, value in
            value?.isEmpty == false ? nil : name
        }
        guard missing.isEmpty, let homeserver, let username, let password, let room else {
            throw ProbeError.missingEnvironment(missing)
        }

        let basePath = URL(fileURLWithPath: env["SYNARA_E2EE_STORE_PATH"] ?? defaultStorePath())
        return LiveProbeConfig(
            homeserver: normalizedHomeserver(homeserver),
            username: username,
            password: password,
            room: room,
            sendProbeMessage: env["SYNARA_E2EE_SEND"] == "1",
            dataPath: basePath.appendingPathComponent("data", isDirectory: true),
            cachePath: basePath.appendingPathComponent("cache", isDirectory: true)
        )
    }

    func prepareStore() throws {
        try FileManager.default.createDirectory(at: dataPath, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: cachePath, withIntermediateDirectories: true)
    }

    private static func normalizedHomeserver(_ value: String) -> String {
        if value.hasPrefix("http://") || value.hasPrefix("https://") {
            return value
        }
        return "https://\(value)"
    }

    private static func defaultStorePath() -> String {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-matrix-sdk-probe", isDirectory: true)
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
            .path
    }
}
