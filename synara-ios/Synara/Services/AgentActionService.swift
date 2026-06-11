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

struct SynaraAgentApprovalRequest: Equatable {
    let roomID: String
    let sourceEventID: String?
    let action: SynaraAgentCardAction
    let decision: SynaraAgentApprovalDecision
}

protocol AgentApprovalServicing {
    var supportsPendingApprovalInbox: Bool { get }
    func submit(_ request: SynaraAgentApprovalRequest) async throws
    func pendingApprovalCount() async -> Int
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
