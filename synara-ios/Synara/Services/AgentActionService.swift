import Foundation

enum SynaraAgentCardActionError: Error, Equatable {
    case unsupportedKind(String)
    case missingPayload
    case unsafeURL
    case encodingFailure
}

enum SynaraAgentCardActionExecution: Equatable {
    case openURL(URL)
    case copyText(String)
    case submitApproval(SynaraAgentApprovalDecision)
}

enum SynaraAgentApprovalDecision: String, Codable, Equatable {
    case approve
    case reject
}

enum SynaraAgentApprovalError: LocalizedError, Equatable {
    case signedOut
    case unsupportedAction
    case failed

    var errorDescription: String? {
        switch self {
        case .signedOut:
            return "Sign in to submit this agent action."
        case .unsupportedAction:
            return "This agent action cannot be submitted."
        case .failed:
            return "Agent action could not be submitted. Try again."
        }
    }
}

enum SynaraAgentApprovalNotificationActionID: String, Equatable {
    case approveOnce = "agent-approval.approve-once"
    case approveAlways = "agent-approval.approve-always"
    case deny = "agent-approval.deny"

    var reactionKey: String {
        switch self {
        case .approveOnce:
            return "✅"
        case .approveAlways:
            return "♾️"
        case .deny:
            return "❌"
        }
    }
}

enum SynaraAgentApprovalPromptReaction: String, CaseIterable, Equatable, Identifiable {
    case approveOnce
    case approveAlways
    case deny

    var id: String { rawValue }

    var reactionKey: String {
        switch self {
        case .approveOnce:
            return "✅"
        case .approveAlways:
            return "♾️"
        case .deny:
            return "❌"
        }
    }

    var title: String {
        switch self {
        case .approveOnce:
            return "Approve once"
        case .approveAlways:
            return "Always"
        case .deny:
            return "Deny"
        }
    }

    var accessibilityIdentifierSuffix: String {
        switch self {
        case .approveOnce:
            return "approveOnce"
        case .approveAlways:
            return "approveAlways"
        case .deny:
            return "deny"
        }
    }
}

struct SynaraAgentApprovalPrompt: Equatable {
    let title: String
    let body: String
    let command: String?
    let commandPreview: String?
}

enum SynaraAgentApprovalPromptDetector {
    private static let maxBodyCharacters = 100_000
    private static let maxCommandPreviewCharacters = 180
    private static let maxCommandCharacters = 8_000
    private static let commandFencePattern = #"```(?:[a-z0-9_-]+)?\s*\n([\s\S]*?)```"#
    private static let codeBlockLabelPattern = #"\bCode\s+(?:Copy\s*)?([\s\S]*?)(?=\n+Reason:|\n+Reply\s+[!/](?:approve|deny)\b|$)"#
    private static let approvalHeadings = [
        "approval required: dangerous command",
        "dangerous command requires approval"
    ]

    static func detect(in item: TimelineItem) -> SynaraAgentApprovalPrompt? {
        switch item.kind {
        case .text(let body):
            return detect(body: body)
        case .formattedText(let body, let html):
            let markdown = MatrixHTMLRenderer.sanitizedMarkdown(body: body, html: html)
            return [body, markdown]
                .removingAdjacentDuplicates()
                .compactMap(detect(body:))
                .first
        default:
            return nil
        }
    }

    static func detect(body: String) -> SynaraAgentApprovalPrompt? {
        guard body.count <= maxBodyCharacters else {
            return nil
        }

        let normalized = normalizeWhitespace(body).lowercased()
        guard approvalHeadings.contains(where: { normalized.contains($0) }) else {
            return nil
        }

        let command = extractCommand(from: body)
        let reason = firstCapture(in: body, pattern: #"\bReason:\s*([^\n]+)"#)
            .map(normalizeWhitespace)
            .map { truncate($0, maxCharacters: 220) }

        return SynaraAgentApprovalPrompt(
            title: "Approval Required: Dangerous Command",
            body: reason ?? "A Hermes Agent command is waiting for approval.",
            command: command,
            commandPreview: command.flatMap { commandPreview(for: $0) }
        )
    }

    private static func extractCommand(from body: String) -> String? {
        let rawCommand = firstCapture(in: body, pattern: commandFencePattern)
            ?? firstCapture(in: body, pattern: codeBlockLabelPattern)
        guard let rawCommand else {
            return nil
        }

        let lines = rawCommand
            .components(separatedBy: .newlines)
            .enumerated()
            .compactMap { index, line -> String? in
                let isCopyLabel = index == 0
                    && line.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() == "copy"
                return isCopyLabel ? nil : trimmingTrailingWhitespace(from: line)
            }
        let command = lines
            .joined(separator: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        return command.isEmpty ? nil : truncate(command, maxCharacters: maxCommandCharacters)
    }

    private static func commandPreview(for command: String) -> String? {
        let firstUsefulLine = command
            .components(separatedBy: .newlines)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .first(where: { $0.isEmpty == false })
        guard let firstUsefulLine else {
            return nil
        }
        return truncate(normalizeWhitespace(firstUsefulLine), maxCharacters: maxCommandPreviewCharacters)
    }

    private static func normalizeWhitespace(_ value: String) -> String {
        value
            .components(separatedBy: .whitespacesAndNewlines)
            .filter { $0.isEmpty == false }
            .joined(separator: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func truncate(_ value: String, maxCharacters: Int) -> String {
        guard value.count > maxCharacters else {
            return value
        }
        let prefixCount = max(0, maxCharacters - 3)
        return "\(value.prefix(prefixCount))..."
    }

    private static func trimmingTrailingWhitespace(from value: String) -> String {
        var output = value
        while output.last?.isWhitespace == true {
            output.removeLast()
        }
        return output
    }

    private static func firstCapture(in value: String, pattern: String) -> String? {
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return nil
        }
        let range = NSRange(value.startIndex..<value.endIndex, in: value)
        guard let match = regex.firstMatch(in: value, range: range),
              match.numberOfRanges > 1,
              let captureRange = Range(match.range(at: 1), in: value) else {
            return nil
        }
        return String(value[captureRange])
    }
}

struct SynaraAgentApprovalRequest: Equatable {
    let roomID: String
    let sourceEventID: String?
    let action: SynaraAgentCardAction
    let decision: SynaraAgentApprovalDecision
}

struct SynaraAgentApprovalReactionRequest: Equatable {
    let roomID: String
    let sourceEventID: String
    let reactionKey: String
}

protocol AgentApprovalServicing {
    var supportsPendingApprovalInbox: Bool { get }
    func submit(_ request: SynaraAgentApprovalRequest) async throws
    func pendingApprovalCount() async -> Int
}

protocol AgentApprovalReactionServicing {
    func submitReaction(_ request: SynaraAgentApprovalReactionRequest) async throws
}

extension AgentApprovalServicing {
    var supportsPendingApprovalInbox: Bool { false }

    func pendingApprovalCount() async -> Int { 0 }
}

func encodeAgentApprovalMatrixEvent(
    _ request: SynaraAgentApprovalRequest,
    createdAt: Int = Int(Date().timeIntervalSince1970 * 1000),
    jsonEncoder: JSONEncoder = JSONEncoder()
) throws -> Data {
    try jsonEncoder.encode(makeAgentApprovalMatrixEvent(request, createdAt: createdAt))
}

func encodeAgentApprovalReactionMatrixEvent(
    _ request: SynaraAgentApprovalReactionRequest,
    jsonEncoder: JSONEncoder = JSONEncoder()
) throws -> Data {
    try jsonEncoder.encode(SynaraMatrixReactionEvent(
        relatesTo: SynaraMatrixReactionRelation(
            eventID: request.sourceEventID,
            key: request.reactionKey
        )
    ))
}

private func makeAgentApprovalMatrixEvent(
    _ request: SynaraAgentApprovalRequest,
    createdAt: Int
) -> SynaraAgentApprovalMatrixEvent {
    SynaraAgentApprovalMatrixEvent(
        body: "\(request.decision.displayName) agent action: \(request.action.title)",
        action: SynaraAgentApprovalContent(
            version: 1,
            actionID: request.action.id,
            actionTitle: request.action.title,
            decision: request.decision,
            sourceEventID: request.sourceEventID,
            createdAt: createdAt
        )
    )
}

struct SynaraAgentCardActionResolver {
    static let renderableKinds: Set<String> = SynaraAgentCardActionKind.renderableKinds

    static func shouldRender(_ action: SynaraAgentCardAction) -> Bool {
        guard let normalizedKind = SynaraAgentCardActionKind.resolved(from: action.kind) else {
            return action.kind == nil
        }

        return renderableKinds.contains(normalizedKind)
    }

    static func plan(for action: SynaraAgentCardAction) -> Result<SynaraAgentCardActionExecution, SynaraAgentCardActionError> {
        let kind = SynaraAgentCardActionKind.resolved(from: action.kind)
        let actionPayload: Result<SynaraAgentCardActionExecution, SynaraAgentCardActionError>

        switch kind {
        case SynaraAgentCardActionKind.open.rawValue,
             SynaraAgentCardActionKind.openURL.rawValue:
            guard let url = action.url.flatMap(URL.init) else {
                return .failure(.missingPayload)
            }
            guard SynaraContractURLPolicy.isSafeHTTPS(url.absoluteString) else {
                return .failure(.unsafeURL)
            }
            actionPayload = .success(.openURL(url))
        case SynaraAgentCardActionKind.copy.rawValue,
             SynaraAgentCardActionKind.prompt.rawValue,
             SynaraAgentCardActionKind.continueAction.rawValue,
             SynaraAgentCardActionKind.run.rawValue,
             SynaraAgentCardActionKind.agent.rawValue,
             SynaraAgentCardActionKind.export.rawValue:
            guard let prompt = action.prompt else {
                return .failure(.missingPayload)
            }
            actionPayload = .success(.copyText(prompt))
        case SynaraAgentCardActionKind.copyPrompt.rawValue:
            guard let prompt = action.prompt else {
                return .failure(.missingPayload)
            }
            actionPayload = .success(.copyText(prompt))
        case SynaraAgentCardActionKind.copyMarkdown.rawValue:
            guard let markdown = action.markdown else {
                return .failure(.missingPayload)
            }
            actionPayload = .success(.copyText(markdown))
        case SynaraAgentCardActionKind.copyJSON.rawValue:
            guard let json = encodeForClipboard(action) else {
                return .failure(.encodingFailure)
            }
            actionPayload = .success(.copyText(json))
        case SynaraAgentCardActionKind.approve.rawValue,
             SynaraAgentCardActionKind.reject.rawValue:
            let decision: SynaraAgentApprovalDecision = kind == SynaraAgentCardActionKind.approve.rawValue ? .approve : .reject
            actionPayload = .success(.submitApproval(decision))
        case nil where action.kind != nil:
            actionPayload = .failure(.unsupportedKind(action.kind ?? "unknown"))
        case nil:
            if let prompt = action.prompt {
                actionPayload = .success(.copyText(prompt))
            } else if let markdown = action.markdown {
                actionPayload = .success(.copyText(markdown))
            } else if let url = action.url.flatMap(URL.init), SynaraContractURLPolicy.isSafeHTTPS(url.absoluteString) {
                actionPayload = .success(.openURL(url))
            } else if let urlString = action.url, SynaraContractURLPolicy.isSafeHTTPS(urlString) {
                actionPayload = .failure(.unsupportedKind("unsupported no-payload action"))
            } else if action.url != nil {
                actionPayload = .failure(.unsafeURL)
            } else {
                actionPayload = .failure(.missingPayload)
            }
        case .some(let unsupported):
            actionPayload = .failure(.unsupportedKind(unsupported))
        }

        return actionPayload
    }

    private static func encodeForClipboard(_ action: SynaraAgentCardAction) -> String? {
        guard let data = try? JSONEncoder().encode(action) else {
            return nil
        }

        return String(data: data, encoding: .utf8)
    }
}

final class MatrixRustSDKAgentApprovalService: AgentApprovalServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore
    private let jsonEncoder: JSONEncoder

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore,
        jsonEncoder: JSONEncoder = JSONEncoder()
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
        self.jsonEncoder = jsonEncoder
    }

    func submit(_ request: SynaraAgentApprovalRequest) async throws {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw SynaraAgentApprovalError.signedOut
        }

        guard request.action.id.isEmpty == false else {
            throw SynaraAgentApprovalError.unsupportedAction
        }

        do {
            let data = try encodeAgentApprovalMatrixEvent(request, jsonEncoder: jsonEncoder)
            guard let content = String(data: data, encoding: .utf8) else {
                throw SynaraAgentApprovalError.failed
            }
            try await clientStore.sendRawRoomEvent(
                roomID: request.roomID,
                eventType: "m.room.message",
                content: content,
                session: session
            )
        } catch let error as SynaraAgentApprovalError {
            throw error
        } catch {
            throw SynaraAgentApprovalError.failed
        }
    }
}

final class MatrixRustSDKAgentApprovalReactionService: AgentApprovalReactionServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore
    private let jsonEncoder: JSONEncoder

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore,
        jsonEncoder: JSONEncoder = JSONEncoder()
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
        self.jsonEncoder = jsonEncoder
    }

    func submitReaction(_ request: SynaraAgentApprovalReactionRequest) async throws {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw SynaraAgentApprovalError.signedOut
        }

        guard request.roomID.isEmpty == false,
              request.sourceEventID.isEmpty == false,
              request.reactionKey.isEmpty == false else {
            throw SynaraAgentApprovalError.unsupportedAction
        }

        do {
            let data = try encodeAgentApprovalReactionMatrixEvent(request, jsonEncoder: jsonEncoder)
            guard let content = String(data: data, encoding: .utf8) else {
                throw SynaraAgentApprovalError.failed
            }
            try await clientStore.sendRawRoomEvent(
                roomID: request.roomID,
                eventType: "m.reaction",
                content: content,
                session: session
            )
        } catch let error as SynaraAgentApprovalError {
            throw error
        } catch {
            throw SynaraAgentApprovalError.failed
        }
    }
}

final class MockAgentApprovalService: AgentApprovalServicing {
    private(set) var submitted: [SynaraAgentApprovalRequest] = []
    var error: SynaraAgentApprovalError?
    var pendingCount = 0
    var supportsPendingApprovalInbox = false

    init(
        error: SynaraAgentApprovalError? = nil,
        pendingCount: Int = 0,
        supportsPendingApprovalInbox: Bool = false
    ) {
        self.error = error
        self.pendingCount = pendingCount
        self.supportsPendingApprovalInbox = supportsPendingApprovalInbox
    }

    func pendingApprovalCount() async -> Int {
        pendingCount
    }

    func submit(_ request: SynaraAgentApprovalRequest) async throws {
        if let error {
            throw error
        }
        submitted.append(request)
    }
}

final class MockAgentApprovalReactionService: AgentApprovalReactionServicing {
    private(set) var submitted: [SynaraAgentApprovalReactionRequest] = []
    var error: SynaraAgentApprovalError?

    init(error: SynaraAgentApprovalError? = nil) {
        self.error = error
    }

    func submitReaction(_ request: SynaraAgentApprovalReactionRequest) async throws {
        if let error {
            throw error
        }
        submitted.append(request)
    }
}

private struct SynaraAgentApprovalMatrixEvent: Encodable {
    let msgtype = "m.notice"
    let body: String
    let action: SynaraAgentApprovalContent

    enum CodingKeys: String, CodingKey {
        case msgtype
        case body
        case action = "in.synara.agent.action"
    }
}

private struct SynaraAgentApprovalContent: Encodable {
    let version: Int
    let actionID: String
    let actionTitle: String
    let decision: SynaraAgentApprovalDecision
    let sourceEventID: String?
    let createdAt: Int

    enum CodingKeys: String, CodingKey {
        case version
        case actionID = "action_id"
        case actionTitle = "action_title"
        case decision
        case sourceEventID = "source_event_id"
        case createdAt = "created_at"
    }
}

private struct SynaraMatrixReactionEvent: Encodable {
    let relatesTo: SynaraMatrixReactionRelation

    enum CodingKeys: String, CodingKey {
        case relatesTo = "m.relates_to"
    }
}

private struct SynaraMatrixReactionRelation: Encodable {
    let relType = "m.annotation"
    let eventID: String
    let key: String

    enum CodingKeys: String, CodingKey {
        case relType = "rel_type"
        case eventID = "event_id"
        case key
    }
}

private extension SynaraAgentApprovalDecision {
    var displayName: String {
        switch self {
        case .approve:
            return "Approved"
        case .reject:
            return "Rejected"
        }
    }
}

private extension Array where Element: Equatable {
    func removingAdjacentDuplicates() -> [Element] {
        reduce(into: []) { result, element in
            if result.last != element {
                result.append(element)
            }
        }
    }
}
