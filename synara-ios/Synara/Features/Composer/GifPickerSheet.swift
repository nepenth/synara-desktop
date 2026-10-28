import SwiftUI

/// What opens the GIF picker: the composer's GIF entry, or `/gif <search>`.
struct GifPickerRequest: Identifiable, Equatable {
    let id = UUID()
    var initialQuery = ""
}

/// Whether this build and the user's settings offer GIF search.
enum GifSearchAvailability: Equatable {
    case available(KlipyGifProvider)
    /// Built with a key, but the user turned GIF Search off.
    case turnedOff
    /// Built without a Klipy key (local, dev and CI test builds).
    case unconfigured

    static func resolve(settings: SettingsStoring, bundle: Bundle = .main) -> GifSearchAvailability {
        guard let provider = KlipyGifProvider.configured(bundle: bundle) else {
            return .unconfigured
        }
        return settings.bool(for: RichLinkSettings.gifSearchKey) ? .available(provider) : .turnedOff
    }

    var provider: KlipyGifProvider? {
        if case let .available(provider) = self { return provider }
        return nil
    }

    /// Why `/gif` did nothing, as desktop words it.
    var commandUnavailableMessage: String {
        switch self {
        case .available:
            return ""
        case .turnedOff:
            return String(localized: "Turn on GIF Search in Settings to use /gif.")
        case .unconfigured:
            return String(localized: "GIF search is not set up for this app.")
        }
    }
}

/// Klipy search: trending until the user types, three columns, more pages
/// as the grid scrolls, and the wording Klipy's terms require. Tapping a GIF
/// sends its link; the GIF itself is never re-uploaded.
struct GifPickerSheet: View {
    let provider: KlipyGifProvider
    let onSelect: (GifResult) -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var query: String
    @State private var results: [GifResult] = []
    @State private var nextPage: Int?
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var loadSequence = 0
    @FocusState private var isSearchFocused: Bool

    private static let searchDelayNanoseconds: UInt64 = 350_000_000
    private static let previewLimits = AnimatedImageDecoder.Limits(
        maxPixelSize: 240,
        maxFrames: 48,
        maxDecodedBytes: 4 * 1024 * 1024
    )
    private let columns = Array(repeating: GridItem(.flexible(), spacing: 6), count: 3)

    init(provider: KlipyGifProvider, initialQuery: String = "", onSelect: @escaping (GifResult) -> Void) {
        self.provider = provider
        self.onSelect = onSelect
        _query = State(initialValue: initialQuery)
    }

    var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 0) {
                searchField
                    .padding(.horizontal, SynaraSpacing.medium)
                    .padding(.vertical, SynaraSpacing.small)

                if isShowingTrending {
                    Text("Trending")
                        .font(SynaraTypography.chipLabel)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .padding(.horizontal, SynaraSpacing.medium)
                        .padding(.bottom, SynaraSpacing.xSmall)
                        .accessibilityAddTraits(.isHeader)
                }

                ScrollView {
                    LazyVGrid(columns: columns, spacing: 6) {
                        ForEach(results) { gif in
                            cell(for: gif)
                                .onAppear {
                                    if gif.id == results.last?.id {
                                        loadMore()
                                    }
                                }
                        }
                    }
                    .padding(.horizontal, SynaraSpacing.medium)

                    statusView
                        .padding(SynaraSpacing.medium)
                }
                .scrollDismissesKeyboard(.immediately)
                .accessibilityIdentifier("GifPickerResults")

                Text(verbatim: KlipyGifProvider.attribution)
                    .font(SynaraTypography.fineMeta)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .frame(maxWidth: .infinity, alignment: .trailing)
                    .padding(.horizontal, SynaraSpacing.medium)
                    .padding(.vertical, SynaraSpacing.small)
                    .accessibilityIdentifier("GifPickerAttribution")
            }
            .background(SynaraColor.surface)
            .navigationTitle("GIFs")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                    .accessibilityIdentifier("GifPickerCancel")
                }
            }
        }
        .accessibilityIdentifier("GifPickerSheet")
        .task(id: query) {
            let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty == false {
                try? await Task.sleep(nanoseconds: Self.searchDelayNanoseconds)
            }
            guard Task.isCancelled == false else {
                return
            }
            await load(page: 1)
        }
    }

    private var searchField: some View {
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityHidden(true)
            TextField(
                "",
                text: $query,
                prompt: Text(verbatim: KlipyGifProvider.searchPlaceholder)
            )
            .focused($isSearchFocused)
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .submitLabel(.search)
            .onSubmit {
                Task { await load(page: 1) }
            }
            .accessibilityLabel(Text("Search GIFs"))
            .accessibilityIdentifier("GifPickerSearchField")

            if query.isEmpty == false {
                Button {
                    query = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundStyle(SynaraColor.tertiaryText)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(Text("Clear search"))
            }
        }
        .padding(.horizontal, SynaraSpacing.small)
        .frame(minHeight: SynaraHitTarget.minimum)
        .background(
            RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)
                .fill(SynaraColor.secondarySurface)
        )
    }

    private func cell(for gif: GifResult) -> some View {
        Button {
            onSelect(gif)
            dismiss()
        } label: {
            Color.clear
                .aspectRatio(1, contentMode: .fit)
                .overlay {
                    RemoteAnimatedImage(
                        url: URL(string: gif.previewURL ?? gif.url),
                        limits: Self.previewLimits,
                        releasesOnDisappear: true
                    ) {
                        SynaraColor.secondarySurface
                    }
                }
                .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(Text("Send \(gif.title)"))
        .accessibilityIdentifier("GifResult-\(gif.id)")
    }

    @ViewBuilder
    private var statusView: some View {
        if let errorMessage {
            Text(errorMessage)
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
                .frame(maxWidth: .infinity)
                .accessibilityIdentifier("GifPickerError")
        } else if isLoading {
            ProgressView()
                .frame(maxWidth: .infinity)
        } else if results.isEmpty, KlipyGifProvider.isSearchable(query) {
            Text("No GIFs found.")
                .font(SynaraTypography.supporting)
                .foregroundStyle(SynaraColor.secondaryText)
                .frame(maxWidth: .infinity)
                .accessibilityIdentifier("GifPickerNoResults")
        }
    }

    private var isShowingTrending: Bool {
        query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private func loadMore() {
        guard isLoading == false, let nextPage else {
            return
        }
        Task { await load(page: nextPage) }
    }

    /// Loads `page` of the feed for the current query: trending when it is
    /// empty. A newer load supersedes an older one still in flight.
    @MainActor
    private func load(page: Int) async {
        let term = query.trimmingCharacters(in: .whitespacesAndNewlines)
        loadSequence &+= 1
        let sequence = loadSequence
        let feed: GifFeed
        if term.isEmpty {
            feed = .trending
        } else if KlipyGifProvider.isSearchable(term) {
            feed = .search(term)
        } else {
            results = []
            nextPage = nil
            isLoading = false
            errorMessage = nil
            return
        }
        isLoading = true
        errorMessage = nil
        do {
            let loaded = try await provider.page(feed, page: page)
            guard sequence == loadSequence else { return }
            if page > 1 {
                let known = Set(results.map(\.id))
                results += loaded.results.filter { known.contains($0.id) == false }
            } else {
                results = loaded.results
            }
            nextPage = loaded.nextPage
        } catch {
            guard sequence == loadSequence, Task.isCancelled == false, (error is CancellationError) == false else {
                return
            }
            if (error as? URLError)?.code == .cancelled {
                return
            }
            errorMessage = error as? GifProviderError == .rateLimited
                ? String(localized: "Too many GIF searches. Wait a moment and try again.")
                : String(localized: "Synara could not search GIFs. Try again.")
        }
        if sequence == loadSequence {
            isLoading = false
        }
    }
}
