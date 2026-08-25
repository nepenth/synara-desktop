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
    case review = "agent-approval.review"
    case approveOnce = "agent-approval.approve-once"
    case approveAlways = "agent-approval.approve-always"
    case deny = "agent-approval.deny"

    var reactionKey: String? {
        switch self {
        case .review:
            return nil
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
    /// Bounded original source body for operator context.
    let sourceContext: String?
    let replyInstructions: String?

    init(
        title: String,
        body: String,
        command: String?,
        commandPreview: String?,
        sourceContext: String? = nil,
        replyInstructions: String? = nil
    ) {
        self.title = title
        self.body = body
        self.command = command
        self.commandPreview = commandPreview
        self.sourceContext = sourceContext
        self.replyInstructions = replyInstructions
    }
}

enum SynaraAgentApprovalPromptDetector {
    private static let maxBodyCharacters = 100_000
    private static let maxCommandPreviewCharacters = 180
    private static let maxCommandCharacters = 8_000
    private static let maxSourceContextCharacters = 4_000
    private static let maxReplyInstructionsCharacters = 600
    private static let commandFencePattern = #"```(?:[a-z0-9_-]+)?\s*\n([\s\S]*?)```"#
    private static let codeBlockLabelPattern =
        #"\bCode\b(?:\s|\n)+(?:Copy\b(?:\s|\n)+)?([\s\S]*?)(?=\n+Reason:|\n+Reply\s+[!/](?:approve|deny)\b|$)"#
    private static let replyInstructionsPattern =
        #"(Reply\s+[!/](?:approve|deny)\b[\s\S]*?)(?=\n{3,}|$)"#
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
            let prompts = [body, markdown]
                .removingAdjacentDuplicates()
                .compactMap(detect(body:))
            return prompts.max(by: { score($0) < score($1) })
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
        let sourceContext = truncate(normalizeSourceBody(body), maxCharacters: maxSourceContextCharacters)
        let replyInstructions = extractReplyInstructions(from: body)

        return SynaraAgentApprovalPrompt(
            title: "Approval Required: Dangerous Command",
            body: reason ?? "A Hermes Agent command is waiting for approval.",
            command: command,
            commandPreview: command.flatMap { commandPreview(for: $0) },
            sourceContext: sourceContext.isEmpty ? nil : sourceContext,
            replyInstructions: replyInstructions
        )
    }

    private static func score(_ prompt: SynaraAgentApprovalPrompt) -> Int {
        var value = 0
        if let command = prompt.command { value += min(command.count, 2_000) }
        if let sourceContext = prompt.sourceContext { value += min(sourceContext.count, 1_000) }
        if prompt.replyInstructions != nil { value += 80 }
        if prompt.body.localizedCaseInsensitiveContains("waiting for approval") == false {
            value += 40
        }
        return value
    }

    private static func extractCommand(from body: String) -> String? {
        if let fenced = firstCapture(in: body, pattern: commandFencePattern),
           let cleaned = cleanCommand(fenced) {
            return cleaned
        }
        if let labeled = firstCapture(in: body, pattern: codeBlockLabelPattern),
           let cleaned = cleanCommand(labeled) {
            return cleaned
        }

        // Fallback: preserve multi-line / heredoc bodies between Code/Copy chrome and Reason/Reply.
        let lines = body.replacingOccurrences(of: "\r\n", with: "\n").components(separatedBy: "\n")
        var start = -1
        for (index, line) in lines.enumerated() {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            if trimmed == "code" || trimmed == "copy" {
                start = index + 1
                continue
            }
            if start >= 0, trimmed.isEmpty == false, trimmed != "code", trimmed != "copy" {
                start = index
                break
            }
        }
        guard start >= 0 else {
            return nil
        }

        var commandLines: [String] = []
        for index in start..<lines.count {
            let line = lines[index]
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.range(of: #"^Reason:"#, options: [.regularExpression, .caseInsensitive]) != nil
                || trimmed.range(
                    of: #"^Reply\s+[!/](?:approve|deny)\b"#,
                    options: [.regularExpression, .caseInsensitive]
                ) != nil {
                break
            }
            commandLines.append(trimmingTrailingWhitespace(from: line))
        }
        return cleanCommand(commandLines.joined(separator: "\n"))
    }

    private static func cleanCommand(_ value: String) -> String? {
        var lines = value
            .replacingOccurrences(of: "\r\n", with: "\n")
            .components(separatedBy: "\n")
            .map(trimmingTrailingWhitespace(from:))

        while let head = lines.first?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased(),
              head.isEmpty || head == "copy" || head == "code" {
            lines.removeFirst()
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

    private static func extractReplyInstructions(from body: String) -> String? {
        guard let section = firstCapture(in: body, pattern: replyInstructionsPattern) else {
            return nil
        }
        let normalized = normalizeSourceBody(section)
        return normalized.isEmpty
            ? nil
            : truncate(normalized, maxCharacters: maxReplyInstructionsCharacters)
    }

    private static func normalizeSourceBody(_ value: String) -> String {
        let lines = value
            .replacingOccurrences(of: "\r\n", with: "\n")
            .components(separatedBy: "\n")
            .map(trimmingTrailingWhitespace(from:))
        var collapsed: [String] = []
        var blankRun = 0
        for line in lines {
            if line.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                blankRun += 1
                if blankRun <= 1 {
                    collapsed.append("")
                }
            } else {
                blankRun = 0
                collapsed.append(line)
            }
        }
        return collapsed
            .joined(separator: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)
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

/// Pure helper for native/push approval action revalidation before reactions are sent.
enum SynaraAgentApprovalNativeActionValidator {
    struct Result: Equatable {
        let eventResolved: Bool
        let isApprovalPrompt: Bool
        let eventTimestamp: Date?
        let shouldSubmitReaction: Bool
        let reason: String
    }

    static func findTargetItem(in items: [TimelineItem], eventID: String) -> TimelineItem? {
        items.first { $0.eventID == eventID || $0.id == eventID }
    }

    /// Validates a resolved timeline window for a native approval action target.
    /// Fails closed when the event cannot be resolved or is not an approval prompt.
    static func validate(
        items: [TimelineItem],
        eventID: String,
        now: Date = Date(),
        ttl: TimeInterval = SynaraNotificationActionContract.nativeActionTTL
    ) -> Result {
        guard let item = findTargetItem(in: items, eventID: eventID) else {
            return Result(
                eventResolved: false,
                isApprovalPrompt: false,
                eventTimestamp: nil,
                shouldSubmitReaction: false,
                reason: "event-unresolved"
            )
        }

        let isApprovalPrompt = SynaraAgentApprovalPromptDetector.detect(in: item) != nil
        guard isApprovalPrompt else {
            return Result(
                eventResolved: true,
                isApprovalPrompt: false,
                eventTimestamp: item.timestamp,
                shouldSubmitReaction: false,
                reason: "not-approval-prompt"
            )
        }

        let eventTimestamp = item.timestamp
        if now.timeIntervalSince(eventTimestamp) > ttl {
            return Result(
                eventResolved: true,
                isApprovalPrompt: true,
                eventTimestamp: eventTimestamp,
                shouldSubmitReaction: false,
                reason: "expired-ttl"
            )
        }

        return Result(
            eventResolved: true,
            isApprovalPrompt: true,
            eventTimestamp: eventTimestamp,
            shouldSubmitReaction: true,
            reason: "validated"
        )
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
    func submitNativeDecision(
        roomID: String,
        eventID: String,
        actionIdentifier: String
    ) async throws
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

    func submitNativeDecision(
        roomID: String,
        eventID: String,
        actionIdentifier: String
    ) async throws {
        guard let action = SynaraAgentApprovalNotificationActionID(rawValue: actionIdentifier),
              action == .approveOnce || action == .deny,
              let reactionKey = action.reactionKey else {
            throw SynaraAgentApprovalError.unsupportedAction
        }
        try await submitReaction(
            SynaraAgentApprovalReactionRequest(
                roomID: roomID,
                sourceEventID: eventID,
                reactionKey: reactionKey
            )
        )
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
