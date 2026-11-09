import Foundation
import SwiftUI

/// Where the composer is in a slash command. Core parses and runs the
/// command; Swift only decides when to ask it.
enum ComposerCommandInput {
    /// Whether a send goes to Core as a command first. Everything else keeps
    /// the plain send path, with no extra round trip.
    static func isCommand(_ text: String) -> Bool {
        text.trimmingCharacters(in: .whitespacesAndNewlines).hasPrefix("/")
    }

    /// The command name being typed after "/", while the composer holds only
    /// that first word and the cursor is in it. `nil` once a space is typed,
    /// for `//text` (a message starting with a slash), and for a selection.
    static func suggestionQuery(in text: String, cursor: Int, selectionLength: Int = 0) -> String? {
        let ns = text as NSString
        guard selectionLength == 0,
              text.hasPrefix("/"),
              text.hasPrefix("//") == false,
              cursor >= 1,
              cursor <= ns.length,
              text.rangeOfCharacter(from: .whitespacesAndNewlines) == nil
        else {
            return nil
        }
        return String(text.dropFirst())
    }

    /// Replaces the typed "/prefix" with "/name " and puts the cursor after
    /// the space, ready for the arguments.
    static func insert(_ suggestion: SynaraCommandSuggestion, in text: String) -> (text: String, cursor: Int) {
        let ns = text as NSString
        let wordEnd = text.rangeOfCharacter(from: .whitespacesAndNewlines)
            .map { NSRange($0, in: text).location } ?? ns.length
        let rest = String(ns.substring(from: wordEnd).drop(while: { $0.isWhitespace }))
        let replacement = "/" + suggestion.name + " "
        return (replacement + rest, (replacement as NSString).length)
    }
}

/// A destructive command Core wants confirmed before it runs again.
struct ComposerCommandConfirmation: Identifiable, Equatable {
    let id = UUID()
    let title: String
    let message: String
    let confirmLabel: String
    /// The command as typed; Confirm runs it again with `confirmed: true`.
    let text: String

    static func == (lhs: Self, rhs: Self) -> Bool {
        lhs.title == rhs.title && lhs.message == rhs.message
            && lhs.confirmLabel == rhs.confirmLabel && lhs.text == rhs.text
    }
}

/// The commands /help lists, presented as a sheet.
struct ComposerCommandHelp: Identifiable, Equatable {
    let id = UUID()
    let commands: [SynaraCommandSuggestion]

    static func == (lhs: Self, rhs: Self) -> Bool {
        lhs.commands == rhs.commands
    }
}

/// What the composer does with Core's answer to a command.
enum ComposerCommandAction: Equatable {
    /// A short confirmation shown above the composer.
    case notice(String)
    case confirm(ComposerCommandConfirmation)
    /// Sent by the screen with the composer's reply or thread relation.
    case sendMessage(SynaraCommandMessage)
    case sendPoll(SynaraCommandPoll)
    case openGifPicker(query: String)
    /// Shown in the composer's error line; the text stays.
    case error(String)
    case navigate(roomID: String, eventID: String?)
    case openProfile(userID: String)
    case help([SynaraCommandSuggestion])

    /// Whether the command text leaves the composer now. A send drops it
    /// once the send is accepted, as an ordinary message does; a
    /// confirmation or an error keeps it.
    var clearsComposerNow: Bool {
        switch self {
        case .notice, .openGifPicker, .navigate, .openProfile, .help:
            return true
        case .confirm, .sendMessage, .sendPoll, .error:
            return false
        }
    }
}

enum ComposerCommandFlow {
    /// `gifUnavailableMessage` is nil when the GIF picker can open.
    static func action(
        for outcome: SynaraCommandOutcome,
        text: String,
        gifUnavailableMessage: String?
    ) -> ComposerCommandAction {
        switch outcome {
        case let .done(message):
            return .notice(message)
        case let .needsConfirmation(title, message, confirmLabel):
            return .confirm(
                ComposerCommandConfirmation(title: title, message: message, confirmLabel: confirmLabel, text: text)
            )
        case let .sendMessage(message):
            return .sendMessage(message)
        case let .sendPoll(poll):
            return .sendPoll(poll)
        case let .openGifPicker(query):
            if let gifUnavailableMessage {
                return .error(gifUnavailableMessage)
            }
            return .openGifPicker(query: query)
        case let .navigate(roomID, eventID):
            return .navigate(roomID: roomID, eventID: eventID)
        case let .openProfile(userID):
            return .openProfile(userID: userID)
        case let .help(commands):
            return .help(commands)
        }
    }
}

/// "@Name" typed without picking a suggestion. Core resolves it against the
/// room's members; each hit joins the picked mentions under exactly the
/// typed text, so `ComposerMentions.resolve` links it and notifies them.
enum ComposerTypedMentions {
    struct Candidate: Equatable {
        let userID: String
        let displayName: String?
    }

    /// Joined and invited members, as the @ suggestions offer.
    static func candidates(from members: [RoomMemberSummary]) -> [Candidate] {
        members.compactMap { member in
            guard member.membership == .join || member.membership == .invite else {
                return nil
            }
            let name = member.displayName?.trimmingCharacters(in: .whitespacesAndNewlines)
            return Candidate(userID: member.userID, displayName: name?.isEmpty == false ? name : nil)
        }
    }

    static func mentions(from typed: [SynaraTypedMention]) -> [ComposerMention] {
        typed.compactMap { mention in
            let text = mention.text.trimmingCharacters(in: .whitespacesAndNewlines)
            guard text.count > 1, text.hasPrefix("@") else { return nil }
            return ComposerMention(userID: mention.userID, displayName: text)
        }
    }

    /// The picked mentions, then each typed one not already among them.
    static func merge(_ typed: [SynaraTypedMention], into picked: [ComposerMention]) -> [ComposerMention] {
        var merged = picked
        for mention in mentions(from: typed) where merged.contains(mention) == false {
            merged.append(mention)
        }
        return merged
    }
}

/// The text event a send produces: plain body, optional HTML, msgtype and
/// the people it mentions.
struct ComposerOutgoingText: Equatable {
    let body: String
    let formattedBody: String?
    /// nil is `m.text`.
    let msgType: String?
    let mentionUserIDs: [String]?

    /// An ordinary message: Markdown with mention links.
    static func message(
        _ draftBody: String,
        mentions: [ComposerMention],
        currentUserID: String
    ) -> ComposerOutgoingText {
        let mentioned = ComposerMentions.resolve(draftBody, mentions: mentions, currentUserID: currentUserID)
        let formattedBody = ComposerMatrixFormatting.formattedBody(for: mentioned.markdown)
        return ComposerOutgoingText(
            body: ComposerMatrixFormatting.plainBody(for: draftBody, formattedBody: formattedBody),
            formattedBody: formattedBody,
            msgType: nil,
            mentionUserIDs: mentioned.userIDs.isEmpty ? nil : mentioned.userIDs
        )
    }

    /// A command's message. Core-built HTML goes out as is; otherwise the
    /// text is formatted (unless the command says not to) and the prefix
    /// goes before it unformatted, in both the body and the HTML.
    static func command(
        _ message: SynaraCommandMessage,
        mentions: [ComposerMention],
        currentUserID: String
    ) -> ComposerOutgoingText {
        let msgType = message.msgType.matrixMsgType
        if let formattedBody = message.formattedBody {
            return ComposerOutgoingText(
                body: message.body,
                formattedBody: formattedBody,
                msgType: msgType,
                mentionUserIDs: nil
            )
        }
        let text = message.text.trimmingCharacters(in: .whitespacesAndNewlines)
        let mentioned = ComposerMentions.resolve(text, mentions: mentions, currentUserID: currentUserID)
        let mentionUserIDs = mentioned.userIDs.isEmpty ? nil : mentioned.userIDs
        guard message.allowFormatting, text.isEmpty == false else {
            return ComposerOutgoingText(
                body: message.body,
                formattedBody: nil,
                msgType: msgType,
                mentionUserIDs: mentionUserIDs
            )
        }
        let textHTML = ComposerMatrixFormatting.formattedBody(for: mentioned.markdown)
        let textPlain = ComposerMatrixFormatting.plainBody(for: text, formattedBody: textHTML)
        let prefix = message.prefix ?? ""
        return ComposerOutgoingText(
            body: prefix.isEmpty ? textPlain : prefix + " " + textPlain,
            formattedBody: textHTML.map { prefix.isEmpty ? $0 : escapeHTML(prefix) + " " + $0 },
            msgType: msgType,
            mentionUserIDs: mentionUserIDs
        )
    }

    static func escapeHTML(_ text: String) -> String {
        var escaped = ""
        for character in text {
            switch character {
            case "&": escaped += "&amp;"
            case "<": escaped += "&lt;"
            case ">": escaped += "&gt;"
            case "\"": escaped += "&quot;"
            default: escaped.append(character)
            }
        }
        return escaped
    }
}

/// The commands "/prefix" matches, modelled on the @ suggestions. Core's
/// usage and description copy is shown verbatim.
struct ComposerCommandSuggestions: View {
    let suggestions: [SynaraCommandSuggestion]
    let onPick: (SynaraCommandSuggestion) -> Void

    private static let visibleRows = 4

    var body: some View {
        Group {
            if suggestions.count > Self.visibleRows {
                ScrollView {
                    rows
                }
                .frame(height: 248)
            } else {
                rows
            }
        }
        .padding(.vertical, SynaraSpacing.xSmall)
        .synaraChromeGlass(in: RoundedRectangle(cornerRadius: SynaraRadius.composer, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("CommandSuggestions")
    }

    private var rows: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(suggestions) { suggestion in
                Button {
                    onPick(suggestion)
                } label: {
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(spacing: SynaraSpacing.small) {
                            Text(verbatim: "/" + suggestion.name)
                                .font(SynaraTypography.emphasis)
                                .foregroundStyle(
                                    suggestion.destructive ? SynaraColor.criticalText : SynaraColor.primaryText
                                )
                                .lineLimit(1)
                            if let alias = suggestion.matchedAlias {
                                Text("Alias /\(alias)")
                                    .font(SynaraTypography.messageMeta)
                                    .foregroundStyle(SynaraColor.secondaryText)
                                    .lineLimit(1)
                            }
                            Spacer(minLength: 0)
                        }
                        Text(verbatim: suggestion.usage)
                            .font(SynaraTypography.messageMeta.monospaced())
                            .foregroundStyle(SynaraColor.secondaryText)
                            .lineLimit(1)
                            .truncationMode(.tail)
                        if suggestion.summary.isEmpty == false {
                            Text(verbatim: suggestion.summary)
                                .font(SynaraTypography.messageTime)
                                .foregroundStyle(SynaraColor.secondaryText)
                                .lineLimit(2)
                        }
                    }
                    .padding(.horizontal, SynaraSpacing.medium)
                    .padding(.vertical, SynaraSpacing.xSmall)
                    .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel(Text(verbatim: "/" + suggestion.name))
                .accessibilityHint(Text(verbatim: suggestion.summary))
                .accessibilityIdentifier("CommandSuggestion-\(suggestion.name)")
            }
        }
    }
}

/// /help: every command available here, with its usage.
struct ComposerCommandHelpSheet: View {
    let commands: [SynaraCommandSuggestion]
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List(commands) { command in
                VStack(alignment: .leading, spacing: 2) {
                    Text(verbatim: command.usage)
                        .font(SynaraTypography.messageMeta.monospaced().weight(.semibold))
                        .foregroundStyle(SynaraColor.primaryText)
                    if command.summary.isEmpty == false {
                        Text(verbatim: command.summary)
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                    if command.aliases.isEmpty == false {
                        Text(verbatim: command.aliases.map { "/" + $0 }.joined(separator: ", "))
                            .font(SynaraTypography.messageMeta)
                            .foregroundStyle(SynaraColor.tertiaryText)
                    }
                }
                .padding(.vertical, 2)
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("CommandHelpRow-\(command.name)")
            }
            .listStyle(.plain)
            .navigationTitle("Commands")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
        }
        .accessibilityIdentifier("ComposerCommandHelp")
    }
}
