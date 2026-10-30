import SwiftUI

/// Shown above the composer when another Synara device saved a newer draft
/// while this one was being typed in. Nothing is overwritten until the user
/// chooses.
struct SyncedDraftBanner: View {
    let truncated: Bool
    let onUse: () -> Void
    let onKeep: () -> Void

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: "arrow.triangle.2.circlepath")
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityHidden(true)
            Text(truncated
                ? String(localized: "Draft updated on another device (too long to sync in full).")
                : String(localized: "Draft updated on another device."))
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
            Spacer(minLength: SynaraSpacing.small)
            Button(String(localized: "Use it"), action: onUse)
                .buttonStyle(.bordered)
                .controlSize(.small)
                .accessibilityIdentifier("SyncedDraftUse")
            Button(String(localized: "Keep mine"), action: onKeep)
                .buttonStyle(.bordered)
                .controlSize(.small)
                .accessibilityIdentifier("SyncedDraftKeep")
        }
        .padding(.horizontal, SynaraSpacing.medium)
        .padding(.vertical, SynaraSpacing.xSmall)
        .background(SynaraColor.surface)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("SyncedDraftBanner")
    }
}
