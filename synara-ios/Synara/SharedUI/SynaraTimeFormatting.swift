import Foundation

/// Cached format styles for message, room-list and day-header times. They run
/// for every row on every body pass, so they are built once and rebuilt only
/// when the 24-hour clock setting, the locale or the time zone changes.
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
        let clockTime: Date.FormatStyle
        /// "Mon".
        let weekday: Date.FormatStyle
        /// "Mar 4".
        let monthDay: Date.FormatStyle
        /// "Mar 4, 2025".
        let monthDayYear: Date.FormatStyle
        /// "Mar 4, 2025 at 14:05" or "Mar 4, 2025 at 2:05 PM".
        let dateTime: Date.FormatStyle
        /// "Monday".
        let weekdayWide: Date.FormatStyle
        /// "Monday, March 4".
        let weekdayMonthDay: Date.FormatStyle
        /// "Monday, March 4, 2025".
        let weekdayMonthDayYear: Date.FormatStyle
        /// "Today", "Yesterday".
        private let relativeDay: RelativeDateTimeFormatter

        init(configuration: Configuration) {
            self.configuration = configuration
            let locale = configuration.locale
            var calendar = locale.calendar
            calendar.timeZone = configuration.timeZone
            let base = Date.FormatStyle(locale: locale, calendar: calendar, timeZone: configuration.timeZone)

            if configuration.uses24HourClock {
                // The setting forces a two-digit 24-hour clock whatever the
                // locale prefers; the locale still supplies the separators.
                var components = Locale.Components(locale: locale)
                components.hourCycle = .zeroToTwentyThree
                let locale24 = Locale(components: components)
                let base24 = Date.FormatStyle(locale: locale24, calendar: calendar, timeZone: configuration.timeZone)
                clockTime = base24.hour(.twoDigits(amPM: .omitted)).minute(.twoDigits)
                dateTime = base24.month(.abbreviated).day().year().hour(.twoDigits(amPM: .omitted)).minute(.twoDigits)
            } else {
                clockTime = Date.FormatStyle(
                    date: .omitted,
                    time: .shortened,
                    locale: locale,
                    calendar: calendar,
                    timeZone: configuration.timeZone
                )
                dateTime = Date.FormatStyle(
                    date: .abbreviated,
                    time: .shortened,
                    locale: locale,
                    calendar: calendar,
                    timeZone: configuration.timeZone
                )
            }
            weekday = base.weekday(.abbreviated)
            monthDay = base.month(.abbreviated).day()
            monthDayYear = base.month(.abbreviated).day().year()
            weekdayWide = base.weekday(.wide)
            weekdayMonthDay = base.weekday(.wide).month(.wide).day()
            weekdayMonthDayYear = base.weekday(.wide).month(.wide).day().year()

            relativeDay = RelativeDateTimeFormatter()
            relativeDay.locale = locale
            relativeDay.calendar = calendar
            relativeDay.dateTimeStyle = .named
            relativeDay.formattingContext = .beginningOfSentence
        }

        /// The named day `offset` days from today: 0 is "Today", -1 is
        /// "Yesterday".
        func relativeDayName(offset: Int) -> String {
            relativeDay.localizedString(from: DateComponents(day: offset))
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
        formatters().clockTime.format(date)
    }

    /// An abbreviated date with the time of day, on the app's clock setting.
    static func dateTime(_ date: Date) -> String {
        formatters().dateTime.format(date)
    }
}
