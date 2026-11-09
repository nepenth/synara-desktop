import Foundation
import SynaraCore

/// One slash command the composer offers while "/…" is typed, or that /help
/// lists. Core filters by the user's power levels and the room kind, and
/// writes the usage and description; Swift shows them verbatim.
struct SynaraCommandSuggestion: Equatable, Identifiable {
    /// The primary name, without the slash.
    let name: String
    let aliases: [String]
    /// The alias the typed prefix matched, when it matched an alias only.
    let matchedAlias: String?
    /// Such as `/kick <@user:server or server> [-r reason]`.
    let usage: String
    let summary: String
    /// Ban, delete and room upgrade ask before they run.
    let destructive: Bool

    var id: String { name }

    init(
        name: String,
        aliases: [String] = [],
        matchedAlias: String? = nil,
        usage: String,
        summary: String,
        destructive: Bool = false
    ) {
        self.name = name
        self.aliases = aliases
        self.matchedAlias = matchedAlias
        self.usage = usage
        self.summary = summary
        self.destructive = destructive
    }

    init(dto: CommandSuggestion) {
        self.init(
            name: dto.name,
            aliases: dto.aliases,
            matchedAlias: dto.matchedAlias,
            usage: dto.usage,
            summary: dto.description,
            destructive: dto.destructive
        )
    }
}

/// A message a command asks the composer to send (/me, /notice, /shrug,
/// /spoiler, /plain, /html, or `//text`).
struct SynaraCommandMessage: Equatable {
    enum MessageType: Equatable {
        case text
        case emote
        case notice

        /// The `msgtype` Core's text send takes; nil is `m.text`.
        var matrixMsgType: String? {
            switch self {
            case .text: return nil
            case .emote: return "m.emote"
            case .notice: return "m.notice"
            }
        }
    }

    let msgType: MessageType
    /// The user's text without the command, to format.
    let text: String
    /// Goes before the text, never formatted (such as ¯\_(ツ)_/¯).
    let prefix: String?
    /// The plain body: prefix, a space and the text.
    let body: String
    /// Core-built HTML (/spoiler, /html), sent as is.
    let formattedBody: String?
    /// False for /plain, /spoiler and /html: the text is not Markdown.
    let allowFormatting: Bool

    init(
        msgType: MessageType = .text,
        text: String,
        prefix: String? = nil,
        body: String,
        formattedBody: String? = nil,
        allowFormatting: Bool = true
    ) {
        self.msgType = msgType
        self.text = text
        self.prefix = prefix
        self.body = body
        self.formattedBody = formattedBody
        self.allowFormatting = allowFormatting
    }

    init(dto: ComposerMessage) {
        let type: MessageType
        switch dto.msgtype {
        case .text: type = .text
        case .emote: type = .emote
        case .notice: type = .notice
        }
        self.init(
            msgType: type,
            text: dto.text,
            prefix: dto.prefix,
            body: dto.body,
            formattedBody: dto.formattedBody,
            allowFormatting: dto.allowFormatting
        )
    }
}

/// A poll /poll asks the composer to send.
struct SynaraCommandPoll: Equatable {
    let question: String
    let answers: [String]
    let maxSelections: UInt32
}

/// What Core decided a composer command does. Copy is Core's, shown as is.
enum SynaraCommandOutcome: Equatable {
    case done(String)
    case needsConfirmation(title: String, message: String, confirmLabel: String)
    case sendMessage(SynaraCommandMessage)
    case sendPoll(SynaraCommandPoll)
    case openGifPicker(query: String)
    case navigate(roomID: String, eventID: String?)
    case openProfile(userID: String)
    case help([SynaraCommandSuggestion])

    init(dto: CommandOutcome) {
        switch dto {
        case let .done(message):
            self = .done(message)
        case let .needsConfirmation(title, message, confirmLabel):
            self = .needsConfirmation(title: title, message: message, confirmLabel: confirmLabel)
        case let .sendMessage(message):
            self = .sendMessage(SynaraCommandMessage(dto: message))
        case let .sendPoll(poll):
            self = .sendPoll(
                SynaraCommandPoll(question: poll.question, answers: poll.answers, maxSelections: poll.maxSelections)
            )
        case let .openGifPicker(query):
            self = .openGifPicker(query: query)
        case let .navigate(roomId, eventId):
            self = .navigate(roomID: roomId, eventID: eventId)
        case let .openProfile(userId):
            self = .openProfile(userID: userId)
        case let .help(commands):
            self = .help(commands.map(SynaraCommandSuggestion.init(dto:)))
        }
    }
}

/// "@Name" typed as text that Core resolved to exactly one member.
struct SynaraTypedMention: Equatable {
    /// UTF-16 offsets into the text, covering "@" and the name.
    let start: Int
    let end: Int
    let userID: String
    /// The covered text as typed.
    let text: String

    init(start: Int, end: Int, userID: String, text: String) {
        self.start = start
        self.end = end
        self.userID = userID
        self.text = text
    }

    init(dto: TypedMention) {
        self.init(start: Int(dto.start), end: Int(dto.end), userID: dto.userId, text: dto.text)
    }
}

/// A command Core refused or could not finish. `message` is Core's copy
/// (usage, unknown command, permission...) and is shown as is.
struct SynaraCommandError: LocalizedError, Equatable {
    enum Reason: Equatable {
        case usage
        case unknownCommand
        case permission
        case notFound
        case partial
        case failed
        case notSignedIn
    }

    let reason: Reason
    let message: String

    var errorDescription: String? { message }

    static var signedOut: SynaraCommandError {
        SynaraCommandError(reason: .notSignedIn, message: String(localized: "Sign in to use commands."))
    }

    static var failed: SynaraCommandError {
        SynaraCommandError(reason: .failed, message: String(localized: "The command could not be run. Try again."))
    }

    static var pollFailed: SynaraCommandError {
        SynaraCommandError(reason: .failed, message: String(localized: "The poll could not be sent. Try again."))
    }
}

extension SynaraCommandError.Reason {
    init(_ kind: ComposerErrorKind) {
        switch kind {
        case .usage: self = .usage
        case .unknownCommand: self = .unknownCommand
        case .permission: self = .permission
        case .notFound: self = .notFound
        case .partial: self = .partial
        case .failed: self = .failed
        case .notSignedIn: self = .notSignedIn
        }
    }
}

/// Slash commands and typed mentions, parsed and run by Core.
protocol ComposerCommandServicing {
    /// Commands for "/prefix" (without the slash); empty when none apply.
    func suggestions(roomID: String, prefix: String) async -> [SynaraCommandSuggestion]
    /// Runs the composer text. Text that is not a command comes back as a
    /// message to send; an unknown command throws.
    func run(roomID: String, text: String, confirmed: Bool) async throws -> SynaraCommandOutcome
    /// "@Name" typed as text that names exactly one joined or invited member.
    func resolveMentions(text: String, members: [RoomMemberSummary], ownUserID: String) -> [SynaraTypedMention]
    /// Sends a /poll with the composer's relation.
    func sendPoll(
        _ poll: SynaraCommandPoll,
        roomID: String,
        threadRootEventID: String?,
        replyToEventID: String?
    ) async throws
}

/// Core-backed commands for a signed-in session.
final class SharedCoreComposerCommandService: ComposerCommandServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    private var isSignedIn: Bool {
        if case .signedIn = host.sessionStore.currentState {
            return true
        }
        return false
    }

    func suggestions(roomID: String, prefix: String) async -> [SynaraCommandSuggestion] {
        guard isSignedIn else { return [] }
        do {
            return try await host.core.composerCommandSuggestions(roomId: roomID, prefix: prefix)
                .map(SynaraCommandSuggestion.init(dto:))
        } catch {
            return []
        }
    }

    func run(roomID: String, text: String, confirmed: Bool) async throws -> SynaraCommandOutcome {
        guard isSignedIn else {
            throw SynaraCommandError.signedOut
        }
        do {
            let outcome = try await host.core.composerRunCommand(roomId: roomID, text: text, confirmed: confirmed)
            return SynaraCommandOutcome(dto: outcome)
        } catch let error as ComposerCommandRunError {
            switch error {
            case let .Failed(_, reason, _, description):
                throw SynaraCommandError(
                    reason: SynaraCommandError.Reason(reason),
                    message: description.isEmpty ? SynaraCommandError.failed.message : description
                )
            }
        } catch {
            throw SynaraCommandError.failed
        }
    }

    func resolveMentions(text: String, members: [RoomMemberSummary], ownUserID: String) -> [SynaraTypedMention] {
        let candidates = ComposerTypedMentions.candidates(from: members).map {
            MentionCandidate(userId: $0.userID, displayName: $0.displayName)
        }
        guard candidates.isEmpty == false, text.contains("@") else { return [] }
        return host.core.composerResolveMentions(text: text, members: candidates, ownUserId: ownUserID)
            .map(SynaraTypedMention.init(dto:))
    }

    func sendPoll(
        _ poll: SynaraCommandPoll,
        roomID: String,
        threadRootEventID: String?,
        replyToEventID: String?
    ) async throws {
        guard isSignedIn else {
            throw SynaraCommandError.signedOut
        }
        do {
            _ = try await SharedCoreSendPoll.sendPoll(
                core: host.core,
                roomId: roomID,
                question: poll.question,
                answers: poll.answers,
                maxSelections: poll.maxSelections,
                threadRoot: threadRootEventID,
                replyTo: replyToEventID
            )
        } catch {
            throw SynaraCommandError.pollFailed
        }
    }
}

/// Previews and UI tests. Without a fixture it sends every text as typed,
/// as Core does for text that is not a command; the UI-test fixture knows a
/// few commands and answers them the way Core would.
final class MockComposerCommandService: ComposerCommandServicing {
    var commands: [SynaraCommandSuggestion]
    var knowsCommands: Bool
    private(set) var runs: [(text: String, confirmed: Bool)] = []
    private(set) var sentPolls: [SynaraCommandPoll] = []

    init(commands: [SynaraCommandSuggestion] = [], knowsCommands: Bool = false) {
        self.commands = commands
        self.knowsCommands = knowsCommands
    }

    static func uiTestFixture() -> MockComposerCommandService {
        MockComposerCommandService(
            commands: [
                SynaraCommandSuggestion(
                    name: "ban",
                    usage: "/ban <@user:server> [reason]",
                    summary: "Bans a user from the room.",
                    destructive: true
                ),
                SynaraCommandSuggestion(name: "help", usage: "/help", summary: "Lists the commands you can use here."),
                SynaraCommandSuggestion(name: "me", usage: "/me <message>", summary: "Sends an action."),
                SynaraCommandSuggestion(name: "tableflip", usage: "/tableflip [message]", summary: "Prepends (╯°□°)╯︵ ┻━┻."),
                SynaraCommandSuggestion(name: "topic", usage: "/topic [topic]", summary: "Shows or sets the room topic."),
                SynaraCommandSuggestion(name: "whois", usage: "/whois <@user:server>", summary: "Shows a user's profile."),
            ],
            knowsCommands: true
        )
    }

    func suggestions(roomID: String, prefix: String) async -> [SynaraCommandSuggestion] {
        let needle = prefix.hasPrefix("/") ? String(prefix.dropFirst()) : prefix
        return commands.filter { $0.name.hasPrefix(needle.lowercased()) }
    }

    func run(roomID: String, text: String, confirmed: Bool) async throws -> SynaraCommandOutcome {
        runs.append((text, confirmed))
        guard knowsCommands,
              text.hasPrefix("/"),
              text.hasPrefix("//") == false
        else {
            let body = text.hasPrefix("//") ? String(text.dropFirst()) : text
            return .sendMessage(SynaraCommandMessage(text: body, body: body))
        }
        let parts = text.dropFirst().split(separator: " ", maxSplits: 1, omittingEmptySubsequences: true)
        let name = parts.first.map(String.init) ?? ""
        let argument = parts.count > 1 ? String(parts[1]).trimmingCharacters(in: .whitespaces) : ""
        switch name {
        case "topic":
            return .done(argument.isEmpty ? "No topic is set." : "Topic changed.")
        case "ban":
            guard argument.isEmpty == false else {
                throw SynaraCommandError(reason: .usage, message: "Usage: /ban <@user:server> [reason]")
            }
            let target = argument.split(separator: " ").first.map(String.init) ?? argument
            return confirmed
                ? .done("Banned \(target).")
                : .needsConfirmation(title: "Ban \(target)?", message: "They will be removed and cannot rejoin.", confirmLabel: "Ban")
        case "me":
            return .sendMessage(SynaraCommandMessage(msgType: .emote, text: argument, body: argument))
        case "tableflip":
            let prefix = "(╯°□°)╯︵ ┻━┻"
            return .sendMessage(
                SynaraCommandMessage(
                    text: argument,
                    prefix: prefix,
                    body: argument.isEmpty ? prefix : prefix + " " + argument
                )
            )
        case "whois":
            return .openProfile(userID: argument)
        case "help":
            return .help(commands)
        default:
            throw SynaraCommandError(reason: .unknownCommand, message: "Synara does not know the command /\(name).")
        }
    }

    /// Exact "@Display Name" or "@user:server" matches, enough for previews.
    func resolveMentions(text: String, members: [RoomMemberSummary], ownUserID: String) -> [SynaraTypedMention] {
        let ns = text as NSString
        var found: [SynaraTypedMention] = []
        for candidate in ComposerTypedMentions.candidates(from: members) where candidate.userID != ownUserID {
            for label in ["@" + (candidate.displayName ?? ""), candidate.userID] where label.count > 1 {
                var searchRange = NSRange(location: 0, length: ns.length)
                while true {
                    let range = ns.range(of: label, options: [], range: searchRange)
                    guard range.location != NSNotFound else { break }
                    let startsWord = range.location == 0
                        || ns.substring(with: NSRange(location: range.location - 1, length: 1)) == " "
                    let end = range.location + range.length
                    let endsWord = end == ns.length
                        || (ns.substring(with: NSRange(location: end, length: 1)).rangeOfCharacter(from: .alphanumerics) == nil)
                    if startsWord, endsWord, found.contains(where: { $0.start == range.location }) == false {
                        found.append(
                            SynaraTypedMention(start: range.location, end: end, userID: candidate.userID, text: label)
                        )
                    }
                    searchRange = NSRange(location: end, length: ns.length - end)
                }
            }
        }
        return found.sorted { $0.start < $1.start }
    }

    func sendPoll(
        _ poll: SynaraCommandPoll,
        roomID: String,
        threadRootEventID: String?,
        replyToEventID: String?
    ) async throws {
        sentPolls.append(poll)
    }
}
