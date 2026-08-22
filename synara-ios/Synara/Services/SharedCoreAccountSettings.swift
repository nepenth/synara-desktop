import Foundation
import SynaraCore

enum SharedCoreAccountSettings {
    static func ignoredUsersSnapshot(core: SharedCore) async throws -> IgnoredUsersSnapshotDto {
        try await core.ignoredUsersSnapshot()
    }

    static func ignoredUsersIgnore(core: SharedCore, userId: String) async throws {
        _ = try await core.ignoredUsersIgnore(userId: userId)
    }

    static func ignoredUsersUnignore(core: SharedCore, userId: String) async throws {
        _ = try await core.ignoredUsersUnignore(userId: userId)
    }

    static func pushRulesSnapshot(core: SharedCore) async throws -> PushRulesSnapshotDto {
        try await core.pushRulesSnapshot()
    }

    static func pushRulesSetDefault(
        core: SharedCore,
        encrypted: Bool,
        oneToOne: Bool,
        mode: String
    ) async throws {
        _ = try await core.pushRulesSetDefault(encrypted: encrypted, oneToOne: oneToOne, mode: mode)
    }

    static func pushRulesSetMention(core: SharedCore, ruleId: String, enabled: Bool) async throws {
        _ = try await core.pushRulesSetMention(ruleId: ruleId, enabled: enabled)
    }

    static func pushRulesAddKeyword(core: SharedCore, keyword: String) async throws {
        _ = try await core.pushRulesAddKeyword(keyword: keyword)
    }

    static func pushRulesRemoveKeyword(core: SharedCore, keyword: String) async throws {
        _ = try await core.pushRulesRemoveKeyword(keyword: keyword)
    }

    static func threepidSnapshot(core: SharedCore) async throws -> ThreepidSnapshotDto {
        try await core.threepidSnapshot()
    }

    static func threepidDelete(core: SharedCore, address: String) async throws {
        _ = try await core.threepidDelete(address: address)
    }

    static func threepidRequestEmailToken(core: SharedCore, email: String) async throws -> ThreepidEmailTokenDto {
        try await core.threepidRequestEmailToken(email: email)
    }

    static func threepidAddEmail(core: SharedCore) async throws -> ThreepidAddDto {
        try await core.threepidAddEmail()
    }

    static func threepidAddEmailPassword(core: SharedCore, password: String) async throws -> ThreepidAddDto {
        try await core.threepidAddEmailPassword(password: password)
    }
}