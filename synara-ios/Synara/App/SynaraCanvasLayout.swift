import SwiftUI

/// Size-class canvas for iPhone, iPhone Duo, and iPad.
///
/// Apple's iPhone Duo guidance is two layouts, not a layout per pose:
/// compact width on the outer 5.4-inch display, regular width on the inner
/// 7.6-inch display. The same regular-width split is what iPad has been
/// missing. Fold, tent, and tabletop poses then come from system bars,
/// safe areas, and (on iOS 27.1+) `ArrangementView` / reserved regions —
/// not from reading hinge angle.
enum SynaraCanvasLayout: Equatable {
    /// Outer display and iPhone: tab plus a stacked room list that pushes
    /// the conversation.
    case stacked
    /// Inner display and iPad: room list and conversation stay visible
    /// together. Hierarchy does not change when the device opens or closes.
    case split

    var hidesTabBarInConversation: Bool {
        self == .stacked
    }

    var showsConversationBackButton: Bool {
        self == .stacked
    }
}

enum SynaraCanvasLayoutPolicy {
    static func layout(horizontalSizeClass: UserInterfaceSizeClass?) -> SynaraCanvasLayout {
        horizontalSizeClass == .regular ? .split : .stacked
    }
}

/// Interprets a tab's navigation path as Mail-style columns.
///
/// Stacked compact width keeps the full path on one `NavigationStack`.
/// Split regular width shows the first conversation as the detail root and
/// any thread on top of it. Opening or closing iPhone Duo must not rewrite
/// this path: an open room stays open on both canvases.
enum SynaraConversationPath {
    static func splitRoot(in path: [AppRoute]) -> AppRoute? {
        path.first(where: \.isConversation)
    }

    static func splitTail(in path: [AppRoute]) -> [AppRoute] {
        guard let index = path.firstIndex(where: \.isConversation) else {
            return []
        }
        return Array(path.suffix(from: path.index(after: index)))
    }

    static func applyingSplitTail(_ tail: [AppRoute], to path: [AppRoute]) -> [AppRoute] {
        guard let root = splitRoot(in: path) else {
            return tail
        }
        return [root] + tail
    }

    static func selectedConversationID(in path: [AppRoute]) -> String? {
        path.lazy.compactMap(\.conversationRoomID).first
    }
}

private struct SynaraCanvasLayoutKey: EnvironmentKey {
    static let defaultValue: SynaraCanvasLayout = .stacked
}

private struct SynaraSelectedConversationIDKey: EnvironmentKey {
    static let defaultValue: String? = nil
}

private struct SynaraAllowsConversationSwipeBackKey: EnvironmentKey {
    static let defaultValue = true
}

extension EnvironmentValues {
    var synaraCanvasLayout: SynaraCanvasLayout {
        get { self[SynaraCanvasLayoutKey.self] }
        set { self[SynaraCanvasLayoutKey.self] = newValue }
    }

    var synaraSelectedConversationID: String? {
        get { self[SynaraSelectedConversationIDKey.self] }
        set { self[SynaraSelectedConversationIDKey.self] = newValue }
    }

    var synaraAllowsConversationSwipeBack: Bool {
        get { self[SynaraAllowsConversationSwipeBackKey.self] }
        set { self[SynaraAllowsConversationSwipeBackKey.self] = newValue }
    }
}

struct SynaraConversationCanvas<List: View>: View {
    let tab: AppTab
    @Binding var path: [AppRoute]
    let list: List
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass

    init(
        tab: AppTab,
        path: Binding<[AppRoute]>,
        @ViewBuilder list: () -> List
    ) {
        self.tab = tab
        self._path = path
        self.list = list()
    }

    private var layout: SynaraCanvasLayout {
        SynaraCanvasLayoutPolicy.layout(horizontalSizeClass: horizontalSizeClass)
    }

    var body: some View {
        canvas
            .environment(\.synaraCanvasLayout, layout)
            .environment(
                \.synaraSelectedConversationID,
                SynaraConversationPath.selectedConversationID(in: path)
            )
    }

    @ViewBuilder
    private var canvas: some View {
        switch layout {
        case .stacked:
            stacked
        case .split:
            split
        }
    }

    private var stacked: some View {
        NavigationStack(path: $path) {
            list
                .navigationDestination(for: AppRoute.self) { route in
                    RoutePlaceholderView(route: route)
                }
        }
    }

    private var split: some View {
        NavigationSplitView(columnVisibility: .constant(.all)) {
            list
                .navigationSplitViewColumnWidth(min: 280, ideal: 320, max: 400)
                .accessibilityIdentifier("ConversationListColumn")
        } detail: {
            splitDetail
                .accessibilityIdentifier("ConversationDetailColumn")
        }
        .navigationSplitViewStyle(.balanced)
    }

    @ViewBuilder
    private var splitDetail: some View {
        if let root = SynaraConversationPath.splitRoot(in: path) {
            NavigationStack(path: splitTail) {
                RoutePlaceholderView(route: root)
                    .navigationDestination(for: AppRoute.self) { route in
                        RoutePlaceholderView(route: route)
                    }
            }
        } else {
            SynaraConversationEmptyPane(tab: tab)
        }
    }

    private var splitTail: Binding<[AppRoute]> {
        Binding(
            get: { SynaraConversationPath.splitTail(in: path) },
            set: { path = SynaraConversationPath.applyingSplitTail($0, to: path) }
        )
    }
}

struct SynaraConversationEmptyPane: View {
    let tab: AppTab

    var body: some View {
        SynaraEmptyState(
            title: tab.emptyConversationTitle,
            systemImage: tab.emptyConversationSystemImage,
            message: tab.emptyConversationMessage
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(SynaraChrome.chat)
        .accessibilityIdentifier("ConversationEmptyPane")
    }
}
