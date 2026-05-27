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
    case blocked(String)
}

struct SynaraAgentCardActionResolver {
    static let renderableKinds: Set<String> = SynaraAgentCardActionKind.renderableKinds

    static func shouldRender(_ action: SynaraAgentCardAction) -> Bool {
        guard let normalizedKind = SynaraAgentCardActionKind.resolved(from: action.kind) else {
            return true
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
            actionPayload = .success(.blocked("Approval actions require Matrix command routing and are not yet connected."))
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
