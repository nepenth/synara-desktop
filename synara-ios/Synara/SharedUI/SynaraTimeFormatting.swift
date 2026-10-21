import Foundation

/// Cached formatters for message and room-list times. `DateFormatter` is
/// costly to build, and these run for every row on every body pass. The
/// formatters are rebuilt when the 24-hour clock setting, the locale or the
/// time zone changes.
enum SynaraTimeFormatting {
    struct Configuration: Equatable {
        let uses24HourClock: Bool
        let locale: Locale
        let timeZone: TimeZone

        static var current: Configuration {
            Configuration(
                uses24HourClock: SynaraSharedConstants.boolSetting(SynaraSharedConstants.hour24ClockKey),
                locale: .current,
                timeZone: .current
            )
        }
    }

    final class Formatters {
        let configuration: Configuration
        /// "14:05" or "2:05 PM".
        let clockTime: DateFormatter
        /// "Mon".
        let weekday: DateFormatter
        /// "Mar 4".
        let monthDay: DateFormatter
        /// "Mar 4, 2025".
        let monthDayYear: DateFormatter

        init(configuration: Configuration) {
            self.configuration = configuration
            clockTime = Self.formatter(configuration)
            clockTime.dateStyle = .none
            if configuration.uses24HourClock {
                clockTime.dateFormat = "HH:mm"
            } else {
                clockTime.timeStyle = .short
            }
            weekday = Self.formatter(configuration)
            weekday.dateFormat = "EEE"
            monthDay = Self.formatter(configuration)
            monthDay.dateFormat = "MMM d"
            monthDayYear = Self.formatter(configuration)
            monthDayYear.dateFormat = "MMM d, yyyy"
        }

        private static func formatter(_ configuration: Configuration) -> DateFormatter {
            let formatter = DateFormatter()
            formatter.locale = configuration.locale
            formatter.timeZone = configuration.timeZone
            return formatter
        }
    }

    private static let lock = NSLock()
    private static var cached: Formatters?

    /// The formatters for `configuration`, reused until it changes.
    static func formatters(for configuration: Configuration = .current) -> Formatters {
        lock.lock()
        defer { lock.unlock() }
        if let cached, cached.configuration == configuration {
            return cached
        }
        let formatters = Formatters(configuration: configuration)
        cached = formatters
        return formatters
    }

    static func clockTime(_ date: Date) -> String {
        formatters().clockTime.string(from: date)
    }
}
