import Foundation
import SynaraCore

/// Hermes clarifying question ("question card") as Core projects it on a
/// timeline row: structured `org.hermes.clarify` content, or the plain-text
/// `❓` fallback. Core owns recognition and the answer route; Swift renders.
struct SynaraAgentQuestion: Equatable {
    enum Source: Equatable {
        case structured
        case fallback
    }

    enum State: Equatable {
        case pending
        case answered
        case skipped
        case expired
        case cancelled
    }

    struct Choice: Equatable, Identifiable {
        /// 1-based, as Hermes numbers it.
        let index: UInt32
        let label: String
        let recommended: Bool

        var id: UInt32 { index }
    }

    /// This account's own keycap or skip reaction on a structured card.
    enum OwnAnswer: Equatable {
        case choice(UInt32)
        case skipped
    }

    let source: Source
    let clarifyID: String?
    let question: String
    let choices: [Choice]
    let multiSelect: Bool
    let allowOther: Bool
    let allowSkip: Bool
    let requesterUserID: String?
    let expiresAt: Date?
    /// Hermes-settled state from the latest edit; fallback cards stay pending.
    let state: State
    let answer: String?
    /// False for another requester's card: render it read-only.
    let canAnswer: Bool
    let ownAnswer: OwnAnswer?
}

extension SynaraAgentQuestion {
    init(dto: TimelineViewAgentQuestionDto) {
        source = dto.source == .structured ? .structured : .fallback
        clarifyID = dto.clarifyId
        question = dto.question
        choices = dto.choices.map {
            Choice(index: $0.index, label: $0.label, recommended: $0.recommended)
        }
        multiSelect = dto.multiSelect
        allowOther = dto.allowOther
        allowSkip = dto.allowSkip
        requesterUserID = dto.requesterUserId
        expiresAt = dto.expiresAtMs.map { Date(timeIntervalSince1970: TimeInterval($0) / 1000) }
        switch dto.state {
        case .pending: state = .pending
        case .answered: state = .answered
        case .skipped: state = .skipped
        case .expired: state = .expired
        case .cancelled: state = .cancelled
        }
        answer = dto.answer
        canAnswer = dto.canAnswer
        if dto.ownSkipped {
            ownAnswer = .skipped
        } else if let index = dto.ownChoiceIndex {
            ownAnswer = .choice(index)
        } else {
            ownAnswer = nil
        }
    }

    func label(forChoice index: UInt32) -> String {
        choices.first { $0.index == index }?.label ?? String(index)
    }
}

/// The answer the card asks Core to send. Core chooses the route (keycap or
/// skip reaction on structured cards, otherwise a reply message).
enum SynaraAgentQuestionAnswer: Equatable {
    case choice(UInt32)
    case choices([UInt32])
    case other(String)
    case skip

    var dto: AgentQuestionAnswerDto {
        switch self {
        case let .choice(index): return .choice(index: index)
        case let .choices(indices): return .choices(indices: indices)
        case let .other(text): return .other(text: text)
        case .skip: return .skip
        }
    }
}

enum SynaraAgentQuestionSettlement: Equatable {
    case pending
    case answered(String?)
    case skipped
    case expired
    case cancelled

    /// The card's settled line; nil while the question waits for an answer.
    var title: String? {
        switch self {
        case .pending:
            return nil
        case let .answered(text?) where text.isEmpty == false:
            return String(localized: "Answered: \(text)")
        case .answered:
            return String(localized: "Answered")
        case .skipped:
            return String(localized: "Skipped")
        case .expired:
            return String(localized: "No answer (expired)")
        case .cancelled:
            return String(localized: "Cancelled")
        }
    }
}

enum SynaraAgentQuestionPresentation {
    /// Hermes' settled state wins; then this account's own answer (reaction,
    /// a send from this device, or its reply on the timeline); then expiry.
    static func settle(
        _ question: SynaraAgentQuestion,
        now: Date,
        ownReplyBody: String? = nil,
        localAnswer: SynaraAgentQuestionAnswer? = nil
    ) -> SynaraAgentQuestionSettlement {
        switch question.state {
        case .answered: return .answered(question.answer)
        case .skipped: return .skipped
        case .expired: return .expired
        case .cancelled: return .cancelled
        case .pending: break
        }
        switch question.ownAnswer {
        case .skipped: return .skipped
        case let .choice(index): return .answered(question.label(forChoice: index))
        case nil: break
        }
        if let localAnswer {
            return describe(localAnswer, for: question)
        }
        if let ownReplyBody {
            let reply = describeReply(ownReplyBody, for: question)
            if reply != .pending {
                return reply
            }
        }
        if let expiresAt = question.expiresAt, expiresAt <= now {
            return .expired
        }
        return .pending
    }

    static func describe(
        _ answer: SynaraAgentQuestionAnswer,
        for question: SynaraAgentQuestion
    ) -> SynaraAgentQuestionSettlement {
        switch answer {
        case let .choice(index):
            return .answered(question.label(forChoice: index))
        case let .choices(indices):
            return .answered(indices.sorted().map { question.label(forChoice: $0) }.joined(separator: ", "))
        case let .other(text):
            return .answered(text.trimmingCharacters(in: .whitespacesAndNewlines))
        case .skip:
            return .skipped
        }
    }

    /// Read this account's reply as Hermes does: "skip", choice numbers
    /// ("1, 3"), or free text.
    static func describeReply(
        _ body: String,
        for question: SynaraAgentQuestion
    ) -> SynaraAgentQuestionSettlement {
        let text = body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard text.isEmpty == false else { return .pending }
        if question.allowSkip, text.lowercased() == "skip" {
            return .skipped
        }
        let parts = text.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
        let indices = parts.compactMap { UInt32($0) }
        if question.choices.isEmpty == false, indices.count == parts.count, indices.isEmpty == false {
            return .answered(indices.map { question.label(forChoice: $0) }.joined(separator: ", "))
        }
        return .answered(text)
    }

    static func toggle(_ index: UInt32, in selection: [UInt32]) -> [UInt32] {
        selection.contains(index)
            ? selection.filter { $0 != index }
            : (selection + [index]).sorted()
    }

    /// This account's latest reply to each card in the loaded window, keyed
    /// by the card's event id.
    static func ownReplies(in items: [TimelineItem], currentUserID: String) -> [String: String] {
        // Ordinary rooms have no cards; skip the scan entirely.
        guard currentUserID.isEmpty == false,
              items.contains(where: { $0.agentQuestion != nil })
        else { return [:] }
        var replies: [String: String] = [:]
        for item in items where item.senderID == currentUserID {
            guard let target = item.replyToEventID else { continue }
            switch item.kind {
            case let .text(body), let .formattedText(body, _):
                replies[target] = body
            default:
                continue
            }
        }
        return replies
    }
}

enum SynaraAgentQuestionError: LocalizedError, Equatable {
    case signedOut
    case failed

    var errorDescription: String? {
        switch self {
        case .signedOut:
            return String(localized: "Sign in to answer this question.")
        case .failed:
            return String(localized: "Your answer could not be sent. Try again.")
        }
    }
}

enum SynaraAgentQuestionAnswerOutcome: Equatable {
    case sent
    case alreadyAnswered
}

protocol AgentQuestionAnswering {
    func answer(
        _ answer: SynaraAgentQuestionAnswer,
        roomID: String,
        eventID: String
    ) async throws -> SynaraAgentQuestionAnswerOutcome
}

/// Default for environments without a Core session (previews, older mocks).
struct UnavailableAgentQuestionService: AgentQuestionAnswering {
    func answer(
        _ answer: SynaraAgentQuestionAnswer,
        roomID: String,
        eventID: String
    ) async throws -> SynaraAgentQuestionAnswerOutcome {
        throw SynaraAgentQuestionError.failed
    }
}

final class MockAgentQuestionService: AgentQuestionAnswering {
    private(set) var submitted: [(answer: SynaraAgentQuestionAnswer, roomID: String, eventID: String)] = []
    var error: SynaraAgentQuestionError?

    init(error: SynaraAgentQuestionError? = nil) {
        self.error = error
    }

    func answer(
        _ answer: SynaraAgentQuestionAnswer,
        roomID: String,
        eventID: String
    ) async throws -> SynaraAgentQuestionAnswerOutcome {
        if let error {
            throw error
        }
        submitted.append((answer, roomID, eventID))
        return .sent
    }
}

/// Typed native question answer. Core resolves the exact card, enforces the
/// requester and pending state, and sends the reaction or reply itself.
final class SharedCoreAgentQuestionService: AgentQuestionAnswering {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func answer(
        _ answer: SynaraAgentQuestionAnswer,
        roomID: String,
        eventID: String
    ) async throws -> SynaraAgentQuestionAnswerOutcome {
        guard case .signedIn = host.sessionStore.currentState else {
            throw SynaraAgentQuestionError.signedOut
        }
        do {
            let result = try await host.core.agentQuestionAnswer(
                roomId: roomID,
                eventId: eventID,
                answer: answer.dto
            )
            switch result.status {
            case .sent:
                return .sent
            case .alreadyAnswered:
                return .alreadyAnswered
            }
        } catch {
            throw SynaraAgentQuestionError.failed
        }
    }
}
