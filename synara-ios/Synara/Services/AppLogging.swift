import Foundation
import OSLog

enum LogCategory: String, CaseIterable {
    case app
    case auth
    case matrix
    case push
    case routing
    case settings
    case sync
    case timeline
}

protocol LoggingServicing: Sendable {
    func debug(_ message: String, category: LogCategory)
    func info(_ message: String, category: LogCategory)
    func error(_ message: String, category: LogCategory)
}

struct AppLogger: LoggingServicing {
    private let subsystem: String

    init(subsystem: String = "com.whylandcreative.synara") {
        self.subsystem = subsystem
    }

    func debug(_ message: String, category: LogCategory) {
        #if DEBUG
        logger(for: category).debug("\(LogRedactor.redact(message), privacy: .public)")
        #endif
    }

    func info(_ message: String, category: LogCategory) {
        logger(for: category).info("\(LogRedactor.redact(message), privacy: .public)")
    }

    func error(_ message: String, category: LogCategory) {
        logger(for: category).error("\(LogRedactor.redact(message), privacy: .public)")
    }

    private func logger(for category: LogCategory) -> Logger {
        Logger(subsystem: subsystem, category: category.rawValue)
    }
}

final class MockLoggingService: LoggingServicing, @unchecked Sendable {
    private(set) var entries: [String] = []

    func debug(_ message: String, category: LogCategory) {
        append(message, category: category)
    }

    func info(_ message: String, category: LogCategory) {
        append(message, category: category)
    }

    func error(_ message: String, category: LogCategory) {
        append(message, category: category)
    }

    private func append(_ message: String, category: LogCategory) {
        entries.append("[\(category.rawValue)] \(LogRedactor.redact(message))")
    }
}

enum LogRedactor {
    private static let rules: [(pattern: String, replacement: String)] = [
        ("(?i)Bearer\\s+[A-Za-z0-9._~+/=-]+", "Bearer <redacted:token>"),
        ("(?i)(access_token|refresh_token|token)=([^\\s&]+)", "$1=<redacted:token>"),
        ("(?i)\"(access_token|refresh_token|token)\"\\s*:\\s*\"[^\"]+\"", "\"$1\":\"<redacted:token>\""),
        ("\\b[a-fA-F0-9]{64}\\b", "<redacted:apns-token>"),
        ("https?://[^\\s)]+", "<redacted:url>"),
        ("[@!][A-Za-z0-9._=/-]+:[A-Za-z0-9.-]+", "<redacted:matrix-id>"),
        ("\\$[A-Za-z0-9._=/-]+:[A-Za-z0-9.-]+", "<redacted:event-id>")
    ]

    static func redact(_ value: String) -> String {
        rules.reduce(value) { partial, rule in
            partial.replacingOccurrences(
                of: rule.pattern,
                with: rule.replacement,
                options: .regularExpression
            )
        }
    }
}
