import Foundation

enum SynaraContractError: Error, Equatable {
    case invalidRoute
    case invalidRouteSegment(String)
    case invalidNotificationSummary
    case invalidLaterContent
    case invalidLaterItemKind(String)
    case invalidTextField(String)
    case invalidURL
    case missingAgentActionPayload
    case invalidAgentCardField(String)
}

enum SynaraContractURLPolicy {
    static func isSafeHTTPS(_ value: String) -> Bool {
        guard let parsed = URL(string: value),
              let scheme = parsed.scheme?.lowercased(),
              scheme == "https",
              let host = parsed.host else {
            return false
        }

        if host.contains("localhost") ||
            host.contains("127.") ||
            host.hasSuffix(".local") ||
            host.hasSuffix(".lan") ||
            host.hasSuffix(".internal") {
            return false
        }

        return true
    }
}

struct SynaraNotificationSummary: Codable, Equatable {
    let appBadgeCount: Int
    let inboxBadgeCount: Int
    let laterActiveCount: Int
    let inviteCount: Int
    let agentApprovalCount: Int
    let highlightCount: Int
    let unreadCount: Int

    init(
        appBadgeCount: Int,
        inboxBadgeCount: Int,
        laterActiveCount: Int,
        inviteCount: Int,
        agentApprovalCount: Int,
        highlightCount: Int,
        unreadCount: Int
    ) throws {
        guard [appBadgeCount, inboxBadgeCount, laterActiveCount, inviteCount, agentApprovalCount, highlightCount, unreadCount].allSatisfy(
            { $0 >= 0 }
        ) else {
            throw SynaraContractError.invalidNotificationSummary
        }

        self.appBadgeCount = appBadgeCount
        self.inboxBadgeCount = inboxBadgeCount
        self.laterActiveCount = laterActiveCount
        self.inviteCount = inviteCount
        self.agentApprovalCount = agentApprovalCount
        self.highlightCount = highlightCount
        self.unreadCount = unreadCount
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            appBadgeCount: container.decode(Int.self, forKey: .appBadgeCount),
            inboxBadgeCount: container.decode(Int.self, forKey: .inboxBadgeCount),
            laterActiveCount: container.decode(Int.self, forKey: .laterActiveCount),
            inviteCount: container.decode(Int.self, forKey: .inviteCount),
            agentApprovalCount: container.decode(Int.self, forKey: .agentApprovalCount),
            highlightCount: container.decode(Int.self, forKey: .highlightCount),
            unreadCount: container.decode(Int.self, forKey: .unreadCount)
        )
    }

    private enum CodingKeys: String, CodingKey {
        case appBadgeCount
        case inboxBadgeCount
        case laterActiveCount
        case inviteCount
        case agentApprovalCount
        case highlightCount
        case unreadCount
    }
}

struct SynaraLaterContent: Codable, Equatable {
    let version: Int
    let items: [String: SynaraLaterItem]

    init(version: Int, items: [String: SynaraLaterItem]) throws {
        guard version == 1 else {
            throw SynaraContractError.invalidLaterContent
        }

        self.version = version
        self.items = items
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let version = try container.decode(Int.self, forKey: .version)
        let items = try container.decode([String: SynaraLaterItem].self, forKey: .items)
        try self.init(version: version, items: items)
    }

    private enum CodingKeys: String, CodingKey {
        case version
        case items
    }
}

struct SynaraLaterItem: Codable, Equatable {
    let id: String
    let kind: Kind
    let roomId: String
    let eventId: String
    let createdAt: Int
    let dueTs: Int?
    let remindedAt: Int?
    let completedAt: Int?

    init(id: String, kind: Kind, roomId: String, eventId: String, createdAt: Int, dueTs: Int? = nil, remindedAt: Int? = nil, completedAt: Int? = nil) throws {
        let fields = [id, roomId, eventId]
        for field in fields {
            guard field.isEmpty == false else {
                throw SynaraContractError.invalidTextField("later-item")
            }
        }

        self.id = id
        self.kind = kind
        self.roomId = roomId
        self.eventId = eventId
        self.createdAt = createdAt
        self.dueTs = dueTs
        self.remindedAt = remindedAt
        self.completedAt = completedAt
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let id = try container.decode(String.self, forKey: .id)
        let kind = try container.decode(Kind.self, forKey: .kind)
        let roomId = try container.decode(String.self, forKey: .roomId)
        let eventId = try container.decode(String.self, forKey: .eventId)
        let createdAt = try container.decode(Int.self, forKey: .createdAt)

        try self.init(
            id: id,
            kind: kind,
            roomId: roomId,
            eventId: eventId,
            createdAt: createdAt,
            dueTs: try container.decodeIfPresent(Int.self, forKey: .dueTs),
            remindedAt: try container.decodeIfPresent(Int.self, forKey: .remindedAt),
            completedAt: try container.decodeIfPresent(Int.self, forKey: .completedAt)
        )
    }

    enum Kind: String, Codable {
        case saved
        case reminder
    }

    private enum CodingKeys: String, CodingKey {
        case id
        case kind
        case roomId
        case eventId
        case createdAt
        case dueTs
        case remindedAt
        case completedAt
    }
}

struct SynaraAgentAction: Codable, Equatable {
    let id: String
    let title: String
    let kind: Kind?
    let prompt: String?
    let url: String?
    let markdown: String?

    init(
        id: String,
        title: String,
        kind: Kind? = nil,
        prompt: String? = nil,
        url: String? = nil,
        markdown: String? = nil
    ) throws {
        let normalizedID = id.trimmingCharacters(in: .whitespacesAndNewlines)
        let normalizedTitle = title.trimmingCharacters(in: .whitespacesAndNewlines)

        guard normalizedID.isEmpty == false, normalizedTitle.isEmpty == false,
              normalizedID.utf8.count <= 1024, normalizedTitle.utf8.count <= 1024 else {
            throw SynaraContractError.invalidTextField("agent-action")
        }

        guard prompt != nil || url != nil || markdown != nil else {
            throw SynaraContractError.missingAgentActionPayload
        }

        if let url {
            guard SynaraContractURLPolicy.isSafeHTTPS(url) else {
                throw SynaraContractError.invalidURL
            }
        }

        if let normalizedPrompt = prompt, normalizedPrompt.isEmpty {
            throw SynaraContractError.invalidTextField("agent-action-prompt")
        }

        if let normalizedMarkdown = markdown, normalizedMarkdown.isEmpty {
            throw SynaraContractError.invalidTextField("agent-action-markdown")
        }

        let normalizedKind = kind

        self.id = normalizedID
        self.title = normalizedTitle
        self.kind = normalizedKind
        self.prompt = prompt?.trimmingCharacters(in: .whitespacesAndNewlines)
        self.url = url
        self.markdown = markdown?.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let id = try container.decode(String.self, forKey: .id)
        let title = try container.decode(String.self, forKey: .title)
        let kind = try container.decodeIfPresent(Kind.self, forKey: .kind)
        let prompt = try container.decodeIfPresent(String.self, forKey: .prompt)
        let url = try container.decodeIfPresent(String.self, forKey: .url)
        let markdown = try container.decodeIfPresent(String.self, forKey: .markdown)

        try self.init(
            id: id,
            title: title,
            kind: kind,
            prompt: prompt,
            url: url,
            markdown: markdown
        )
    }

    enum Kind: String, Codable {
        case agent
        case copy
        case `continue`
        case export
        case prompt
        case regenerate
        case run
        case open
        case openURL = "open_url"
    }

    private enum CodingKeys: String, CodingKey {
        case id
        case title
        case kind
        case prompt
        case url
        case markdown
    }
}

struct SynaraAgentCardAction: Codable, Equatable {
    let id: String
    let title: String
    let kind: String?
    let prompt: String?
    let url: String?
    let markdown: String?

    init(
        id: String,
        title: String,
        kind: String? = nil,
        prompt: String? = nil,
        url: String? = nil,
        markdown: String? = nil
    ) throws {
        let normalizedID = id.trimmingCharacters(in: .whitespacesAndNewlines)
        let normalizedTitle = title.trimmingCharacters(in: .whitespacesAndNewlines)

        guard normalizedID.isEmpty == false, normalizedID.utf8.count <= 200,
              normalizedTitle.isEmpty == false, normalizedTitle.utf8.count <= 80 else {
            throw SynaraContractError.invalidTextField("agent-card-action")
        }

        if let url {
            let normalized = url.trimmingCharacters(in: .whitespacesAndNewlines)
            if normalized.utf8.count <= 2048,
               normalized.isEmpty == false,
               SynaraContractURLPolicy.isSafeHTTPS(normalized) {
                self.url = normalized
            } else {
                throw SynaraContractError.invalidURL
            }
        } else {
            self.url = nil
        }

        if let normalizedKind = kind?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalizedKind.isEmpty == false {
            self.kind = normalizedKind
        } else {
            self.kind = nil
        }

        if let normalizedPrompt = prompt?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalizedPrompt.isEmpty == false {
            guard normalizedPrompt.utf8.count <= 5000 else {
                throw SynaraContractError.invalidTextField("agent-card-action-prompt")
            }
            self.prompt = normalizedPrompt
        } else {
            self.prompt = nil
        }

        if let normalizedMarkdown = markdown?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalizedMarkdown.isEmpty == false {
            guard normalizedMarkdown.utf8.count <= 5000 else {
                throw SynaraContractError.invalidTextField("agent-card-action-markdown")
            }
            self.markdown = normalizedMarkdown
        } else {
            self.markdown = nil
        }

        guard self.url != nil || self.prompt != nil || self.markdown != nil else {
            throw SynaraContractError.missingAgentActionPayload
        }

        self.id = normalizedID
        self.title = normalizedTitle
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let id = try container.decode(String.self, forKey: .id)
        let title = try container.decode(String.self, forKey: .title)
        let kind = try container.decodeIfPresent(String.self, forKey: .kind)
        let prompt = try container.decodeIfPresent(String.self, forKey: .prompt)
        let url = try container.decodeIfPresent(String.self, forKey: .url)
        let markdown = try container.decodeIfPresent(String.self, forKey: .markdown)

        try self.init(
            id: id,
            title: title,
            kind: kind,
            prompt: prompt,
            url: url,
            markdown: markdown
        )
    }

    private enum CodingKeys: String, CodingKey {
        case id
        case title
        case kind
        case prompt
        case url
        case markdown
    }
}

enum SynaraAgentCardActionKind: String {
    case open
    case openURL = "open_url"
    case copy
    case copyPrompt = "copy_prompt"
    case copyMarkdown = "copy_markdown"
    case copyJSON = "copy_json"
    case approve
    case reject
    case continueAction = "continue"
    case run
    case prompt
    case agent
    case export

    static let renderableKinds: Set<String> = Set([
        SynaraAgentCardActionKind.open.rawValue,
        SynaraAgentCardActionKind.openURL.rawValue,
        SynaraAgentCardActionKind.copy.rawValue,
        SynaraAgentCardActionKind.copyPrompt.rawValue,
        SynaraAgentCardActionKind.copyMarkdown.rawValue,
        SynaraAgentCardActionKind.copyJSON.rawValue,
        SynaraAgentCardActionKind.approve.rawValue,
        SynaraAgentCardActionKind.reject.rawValue,
        SynaraAgentCardActionKind.continueAction.rawValue,
        SynaraAgentCardActionKind.run.rawValue,
        SynaraAgentCardActionKind.prompt.rawValue,
        SynaraAgentCardActionKind.agent.rawValue,
        SynaraAgentCardActionKind.export.rawValue
    ])

    static func resolved(from rawKind: String?) -> String? {
        guard let rawKind else { return nil }
        let normalized = rawKind.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return SynaraAgentCardActionKind(rawValue: normalized)?.rawValue
    }
}

struct SynaraAgentCardArtifact: Codable, Equatable {
    let title: String
    let type: String?
    let url: String?
    let summary: String?

    init(title: String, type: String? = nil, url: String? = nil, summary: String? = nil) throws {
        let normalizedTitle = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard normalizedTitle.isEmpty == false, normalizedTitle.utf8.count <= 200 else {
            throw SynaraContractError.invalidTextField("agent-card-artifact-title")
        }

        if let normalizedType = type?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalizedType.isEmpty == false {
            guard normalizedType.utf8.count <= 200 else {
                throw SynaraContractError.invalidTextField("agent-card-artifact-type")
            }
            self.type = normalizedType
        } else {
            self.type = nil
        }

        if let normalized = url?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalized.isEmpty == false {
            guard normalized.utf8.count <= 2048,
                  SynaraContractURLPolicy.isSafeHTTPS(normalized) else {
                throw SynaraContractError.invalidURL
            }
            self.url = normalized
        } else {
            self.url = nil
        }

        if let normalizedSummary = summary?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalizedSummary.isEmpty == false {
            guard normalizedSummary.utf8.count <= 5000 else {
                throw SynaraContractError.invalidTextField("agent-card-artifact-summary")
            }
            self.summary = normalizedSummary
        } else {
            self.summary = nil
        }

        self.title = normalizedTitle
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            title: try container.decode(String.self, forKey: .title),
            type: try container.decodeIfPresent(String.self, forKey: .type),
            url: try container.decodeIfPresent(String.self, forKey: .url),
            summary: try container.decodeIfPresent(String.self, forKey: .summary)
        )
    }

    private enum CodingKeys: String, CodingKey {
        case title
        case type
        case url
        case summary
    }
}

struct SynaraAgentCardCodeBlock: Codable, Equatable {
    let id: String
    let title: String?
    let language: String?
    let code: String

    init(
        id: String,
        title: String? = nil,
        language: String? = nil,
        code: String
    ) throws {
        let normalizedID = id.trimmingCharacters(in: .whitespacesAndNewlines)
        let normalizedCode = code.trimmingCharacters(in: .whitespacesAndNewlines)

        guard normalizedID.isEmpty == false, normalizedID.utf8.count <= 200,
              normalizedCode.isEmpty == false, normalizedCode.utf8.count <= 50_000 else {
            throw SynaraContractError.invalidTextField("agent-card-code")
        }

        if let normalizedTitle = title?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalizedTitle.isEmpty == false {
            guard normalizedTitle.utf8.count <= 200 else {
                throw SynaraContractError.invalidTextField("agent-card-code-title")
            }
            self.title = normalizedTitle
        } else {
            self.title = nil
        }

        if let normalizedLanguage = language?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalizedLanguage.isEmpty == false {
            guard normalizedLanguage.utf8.count <= 200 else {
                throw SynaraContractError.invalidTextField("agent-card-code-language")
            }
            self.language = normalizedLanguage
        } else {
            self.language = nil
        }

        self.id = normalizedID
        self.code = normalizedCode
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            id: try container.decode(String.self, forKey: .id),
            title: try container.decodeIfPresent(String.self, forKey: .title),
            language: try container.decodeIfPresent(String.self, forKey: .language),
            code: try container.decode(String.self, forKey: .code)
        )
    }

    private enum CodingKeys: String, CodingKey {
        case id
        case title
        case language
        case code
    }
}

struct SynaraAgentCard: Codable, Equatable {
    let title: String
    let status: String?
    let summary: String?
    let actions: [SynaraAgentCardAction]
    let artifacts: [SynaraAgentCardArtifact]
    let logs: [SynaraAgentCardCodeBlock]
    let code: [SynaraAgentCardCodeBlock]
    let diffs: [SynaraAgentCardCodeBlock]

    init(
        title: String,
        status: String? = nil,
        summary: String? = nil,
        actions: [SynaraAgentCardAction] = [],
        artifacts: [SynaraAgentCardArtifact] = [],
        logs: [SynaraAgentCardCodeBlock] = [],
        code: [SynaraAgentCardCodeBlock] = [],
        diffs: [SynaraAgentCardCodeBlock] = []
    ) throws {
        let normalizedTitle = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard normalizedTitle.isEmpty == false,
              normalizedTitle.utf8.count <= 200 else {
            throw SynaraContractError.invalidTextField("agent-card-title")
        }

        guard actions.count <= 12,
              artifacts.count <= 20,
              logs.count <= 20,
              code.count <= 20,
              diffs.count <= 20 else {
            throw SynaraContractError.invalidAgentCardField("agent-card-limits")
        }

        guard summary != nil || !actions.isEmpty || !artifacts.isEmpty || !logs.isEmpty || !code.isEmpty || !diffs.isEmpty else {
            throw SynaraContractError.invalidAgentCardField("agent-card-empty")
        }

        if let normalizedStatus = status?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalizedStatus.isEmpty == false {
            guard normalizedStatus.utf8.count <= 200 else {
                throw SynaraContractError.invalidTextField("agent-card-status")
            }
            self.status = normalizedStatus
        } else {
            self.status = nil
        }

        if let normalizedSummary = summary?.trimmingCharacters(in: .whitespacesAndNewlines),
           normalizedSummary.isEmpty == false {
            guard normalizedSummary.utf8.count <= 5000 else {
                throw SynaraContractError.invalidTextField("agent-card-summary")
            }
            self.summary = normalizedSummary
        } else {
            self.summary = nil
        }

        self.title = normalizedTitle
        self.actions = actions
        self.artifacts = artifacts
        self.logs = logs
        self.code = code
        self.diffs = diffs
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let title = try container.decode(String.self, forKey: .title)
        let status = try container.decodeIfPresent(String.self, forKey: .status)
        let summary = try container.decodeIfPresent(String.self, forKey: .summary)
        let actions = try container.decodeIfPresent([SynaraAgentCardAction].self, forKey: .actions) ?? []
        let artifacts = try container.decodeIfPresent([SynaraAgentCardArtifact].self, forKey: .artifacts) ?? []
        let logs = try container.decodeIfPresent([SynaraAgentCardCodeBlock].self, forKey: .logs) ?? []
        let code = try container.decodeIfPresent([SynaraAgentCardCodeBlock].self, forKey: .code) ?? []
        let diffs = try container.decodeIfPresent([SynaraAgentCardCodeBlock].self, forKey: .diffs) ?? []

        try self.init(
            title: title,
            status: status,
            summary: summary,
            actions: actions,
            artifacts: artifacts,
            logs: logs,
            code: code,
            diffs: diffs
        )
    }

    private enum CodingKeys: String, CodingKey {
        case title
        case status
        case summary
        case actions
        case artifacts
        case logs
        case code
        case diffs
    }
}

struct SynaraRoutePath: Codable, Equatable {
    let rawValue: String

    init(rawValue: String) throws {
        let normalized = rawValue.trimmingCharacters(in: .whitespacesAndNewlines)

        guard normalized.hasPrefix("/"),
              let percent = normalized.removingPercentEncoding,
              percent.isEmpty == false || normalized == "/" else {
            throw SynaraContractError.invalidRoute
        }

        guard let parsed = URL(string: "synara://host\(normalized)") else {
            throw SynaraContractError.invalidRoute
        }

        let segments = parsed.pathComponents
            .filter { $0 != "/" && $0 != "~" }

        if normalized != "/" {
            guard segments.isEmpty == false else {
                throw SynaraContractError.invalidRoute
            }
            guard percent.contains("//") == false else {
                throw SynaraContractError.invalidRouteSegment("empty segment")
            }
            guard Self.isAllowed(route: normalized, segments: segments) else {
                throw SynaraContractError.invalidRoute
            }
        }

        self.rawValue = normalized
    }

    var trimmed: String { rawValue }

    private static func isAllowed(route: String, segments: [String]) -> Bool {
        guard segments.allSatisfy({ $0.isEmpty == false }) else { return false }

        let banned = ["login", "register", "reset-password", "space-settings", "room-settings", "home", "direct", "create", "explore", "settings", "inbox", "room"]
        guard let first = segments.first else { return false }

        if banned.contains(first.lowercased()), !allowedReserved(first: first.lowercased(), segments: segments) {
            return false
        }

        if route.hasPrefix("/inbox/") {
            guard segments.count <= 2 else { return false }
            if segments.count == 1 { return true }
            return ["notifications", "invites", "later"].contains(segments[1].lowercased())
        }

        if first.lowercased() == "room" || first.lowercased() == "create" {
            return true
        }

        return true
    }

    private static func allowedReserved(first: String, segments: [String]) -> Bool {
        if first == "inbox", segments.count <= 2 { return true }
        if first == "room" { return true }
        if first == "create", segments.count <= 1 { return true }
        if first == "explore", segments.count <= 2 { return true }
        if first == "home" || first == "direct", segments.count <= 3 { return true }
        if first == "settings" { return segments.count == 1 }

        return false
    }
}
