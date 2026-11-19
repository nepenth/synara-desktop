import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

enum WhatsNewCopy {
    static func title(version: String) -> String {
        String(localized: "What's New in \(version)")
    }
}

/// True while something that needs attention first is on screen (a sheet,
/// the verification prompts, the log-out confirmation); the card waits.
private struct WhatsNewBlockedKey: EnvironmentKey {
    static let defaultValue = false
}

extension EnvironmentValues {
    var synaraWhatsNewBlocked: Bool {
        get { self[WhatsNewBlockedKey.self] }
        set { self[WhatsNewBlockedKey.self] = newValue }
    }
}

/// The headline and the iOS items; the card and Settings › About › What's
/// New share it.
struct WhatsNewNotesContent: View {
    let notes: WhatsNewNotes

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            Text(notes.headline)
                .font(SynaraTypography.supporting.weight(.semibold))
                .foregroundStyle(SynaraColor.primaryText)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("WhatsNewHeadline")
            ForEach(Array(notes.items.enumerated()), id: \.offset) { index, item in
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                    Text(verbatim: "•")
                        .accessibilityHidden(true)
                    Text(item)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("WhatsNewItem-\(index)")
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

/// A compact card at the top of the Rooms list after an update. It is part
/// of the list, so it scrolls away, and VoiceOver hears it as a queued
/// announcement without moving focus.
struct WhatsNewCard: View {
    let notes: WhatsNewNotes
    let onAppear: () -> Bool
    let onDismiss: () -> Void
    @Environment(\.openURL) private var openURL

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            HStack(alignment: .top, spacing: SynaraSpacing.small) {
                Label(WhatsNewCopy.title(version: notes.version), systemImage: "sparkles")
                    .labelStyle(.titleAndIcon)
                    .font(SynaraTypography.emphasis)
                    .foregroundStyle(SynaraColor.primaryText)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .accessibilityAddTraits(.isHeader)

                Button(action: onDismiss) {
                    Image(systemName: "xmark")
                        .font(SynaraTypography.chipLabel)
                        .foregroundStyle(SynaraColor.secondaryText)
                        // A 44-point target that lays out as 24 points.
                        .frame(width: 44, height: 44)
                        .contentShape(Rectangle())
                        .padding(-10)
                }
                .buttonStyle(.borderless)
                .accessibilityLabel(String(localized: "Close What's New"))
                .accessibilityIdentifier("WhatsNewDismissButton")
            }

            WhatsNewNotesContent(notes: notes)

            Button {
                openURL(notes.releaseURL)
            } label: {
                Text(String(localized: "Full Release Notes"))
                    .font(SynaraTypography.supporting.weight(.semibold))
            }
            .buttonStyle(.borderless)
            .accessibilityHint(String(localized: "Opens the release notes on GitHub"))
            .accessibilityIdentifier("WhatsNewReleaseNotesLink")
        }
        .padding(SynaraSpacing.medium)
        .synaraCard(fill: SynaraColor.accent.opacity(0.08), stroke: SynaraColor.accent.opacity(0.35))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("WhatsNewCard")
        .onAppear {
            guard onAppear() else {
                return
            }
            #if canImport(UIKit)
            let announcement = NSAttributedString(
                string: "\(WhatsNewCopy.title(version: notes.version)). \(notes.headline)",
                attributes: [.accessibilitySpeechQueueAnnouncement: true]
            )
            UIAccessibility.post(notification: .announcement, argument: announcement)
            #endif
        }
    }
}
