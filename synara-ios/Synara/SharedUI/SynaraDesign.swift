import SwiftUI

enum SynaraSpacing {
    static let xSmall: CGFloat = 4
    static let small: CGFloat = 8
    static let medium: CGFloat = 12
    static let large: CGFloat = 16
    static let xLarge: CGFloat = 24
}

enum SynaraColor {
    static let surface = Color(.systemBackground)
    static let secondarySurface = Color(.secondarySystemBackground)
    static let primaryText = Color(.label)
    static let secondaryText = Color(.secondaryLabel)
    static let accent = Color.accentColor
}

enum SynaraTypography {
    static let screenTitle = Font.title2.weight(.semibold)
    static let sectionTitle = Font.headline
    static let body = Font.body
    static let supporting = Font.callout
}

struct SynaraEmptyState: View {
    let title: String
    let systemImage: String
    let message: String?

    init(title: String, systemImage: String, message: String? = nil) {
        self.title = title
        self.systemImage = systemImage
        self.message = message
    }

    var body: some View {
        VStack(spacing: SynaraSpacing.large) {
            Image(systemName: systemImage)
                .font(.system(size: 42, weight: .semibold))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)

            VStack(spacing: SynaraSpacing.small) {
                Text(title)
                    .font(SynaraTypography.screenTitle)
                    .foregroundStyle(SynaraColor.primaryText)
                    .multilineTextAlignment(.center)
                    .lineLimit(nil)

                if let message {
                    Text(message)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .multilineTextAlignment(.center)
                        .lineLimit(nil)
                }
            }
        }
        .padding(SynaraSpacing.xLarge)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(SynaraColor.surface)
    }
}

struct SynaraLoadingState: View {
    let title: String

    var body: some View {
        VStack(spacing: SynaraSpacing.medium) {
            ProgressView()
            Text(title)
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.secondaryText)
                .multilineTextAlignment(.center)
        }
        .padding(SynaraSpacing.xLarge)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct SynaraErrorState: View {
    let title: String
    let message: String
    let retry: (() -> Void)?

    var body: some View {
        VStack(spacing: SynaraSpacing.large) {
            SynaraEmptyState(title: title, systemImage: "exclamationmark.triangle", message: message)

            if let retry {
                Button("Retry", action: retry)
                    .buttonStyle(.borderedProminent)
            }
        }
    }
}

struct SynaraToolbarIconButton: View {
    let systemImage: String
    let accessibilityLabel: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .frame(width: 28, height: 28)
        }
        .accessibilityLabel(Text(accessibilityLabel))
    }
}

struct SynaraDesignTokenGallery: View {
    var body: some View {
        NavigationStack {
            List {
                Section("States") {
                    SynaraEmptyState(title: "No Rooms", systemImage: "bubble.left.and.bubble.right", message: "Joined rooms will appear here.")
                        .frame(minHeight: 220)
                    SynaraLoadingState(title: "Syncing rooms")
                        .frame(minHeight: 120)
                }

                Section("Controls") {
                    HStack(spacing: SynaraSpacing.medium) {
                        SynaraToolbarIconButton(systemImage: "magnifyingglass", accessibilityLabel: "Search") {}
                        SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: "Account") {}
                    }
                }
            }
            .navigationTitle("Design Tokens")
        }
    }
}

struct SynaraDesignTokenGallery_Previews: PreviewProvider {
    static var previews: some View {
        Group {
            SynaraDesignTokenGallery()
                .preferredColorScheme(.light)
                .previewDisplayName("Light")

            SynaraDesignTokenGallery()
                .preferredColorScheme(.dark)
                .previewDisplayName("Dark")
        }
    }
}
