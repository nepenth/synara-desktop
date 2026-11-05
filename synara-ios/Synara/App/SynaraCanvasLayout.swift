import SwiftUI

/// Size-class canvas for iPhone and iPad, including resizable iPad windows.
enum SynaraCanvasLayout: Equatable {
    /// Compact width: a room list that pushes the conversation.
    case stacked
    /// Regular width: room list and conversation stay visible together.
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
/// any thread on top of it. Resizing an iPad window keeps the room selected.
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

/// Column widths shared by every split canvas (Rooms, Later, Notifications,
/// Settings) so the sidebar edge never moves between tabs.
enum SynaraSplitColumn {
    static let minimumWidth: CGFloat = 280
    static let idealWidth: CGFloat = 320
    static let maximumWidth: CGFloat = 400
}

struct SynaraConversationCanvas<List: View>: View {
    let tab: AppTab
    @ObservedObject private var router: AppRouter
    let list: List
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    /// Bound state, so the sidebar can be collapsed (system toggle or swipe)
    /// when a wide table or code block needs the width.
    @State private var columnVisibility: NavigationSplitViewVisibility = .all

    init(
        tab: AppTab,
        router: AppRouter,
        @ViewBuilder list: () -> List
    ) {
        self.tab = tab
        self.router = router
        self.list = list()
    }

    private var path: Binding<[AppRoute]> {
        router.binding(for: tab)
    }

    private var layout: SynaraCanvasLayout {
        SynaraCanvasLayoutPolicy.layout(horizontalSizeClass: horizontalSizeClass)
    }

    var body: some View {
        canvas
            .environment(\.synaraCanvasLayout, layout)
            .environment(
                \.synaraSelectedConversationID,
                SynaraConversationPath.selectedConversationID(in: path.wrappedValue)
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
        NavigationStack(path: path) {
            list
                .modifier(ConnectionChromeInset(enabled: tab.drawsOwnHeader == false))
                .navigationDestination(for: AppRoute.self) { route in
                    RoutePlaceholderView(route: route)
                }
        }
    }

    private var split: some View {
        NavigationSplitView(columnVisibility: $columnVisibility) {
            list
                .modifier(ConnectionChromeInset(enabled: tab.drawsOwnHeader == false))
                .navigationSplitViewColumnWidth(
                    min: SynaraSplitColumn.minimumWidth,
                    ideal: SynaraSplitColumn.idealWidth,
                    max: SynaraSplitColumn.maximumWidth
                )
                .accessibilityIdentifier("ConversationListColumn")
        } detail: {
            splitDetail
                .accessibilityIdentifier("ConversationDetailColumn")
        }
        .navigationSplitViewStyle(.balanced)
    }

    @ViewBuilder
    private var splitDetail: some View {
        if let root = SynaraConversationPath.splitRoot(in: path.wrappedValue) {
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
            get: { SynaraConversationPath.splitTail(in: path.wrappedValue) },
            set: { path.wrappedValue = SynaraConversationPath.applyingSplitTail($0, to: path.wrappedValue) }
        )
    }
}

struct SynaraConversationEmptyPane: View {
    let tab: AppTab

    var body: some View {
        SynaraEmptyState(
            title: tab.emptyConversationTitle,
            systemImage: tab.emptyConversationSystemImage,
            message: tab.emptyConversationMessage,
            showsBrandMark: tab == .rooms
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(SynaraChrome.chat)
        .accessibilityIdentifier("ConversationEmptyPane")
    }
}

/// Settings on both canvases. Compact width keeps the phone `NavigationStack`;
/// regular width shows the settings list as a sidebar with the selected page
/// (Account by default) in the detail column, like desktop's Settings.
struct SynaraSettingsCanvas: View {
    @ObservedObject var router: AppRouter
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    @State private var selection: SettingsPage? = .account
    @State private var columnVisibility: NavigationSplitViewVisibility = .all

    private var layout: SynaraCanvasLayout {
        SynaraCanvasLayoutPolicy.layout(horizontalSizeClass: horizontalSizeClass)
    }

    var body: some View {
        Group {
            switch layout {
            case .stacked:
                NavigationStack(path: router.binding(for: .settings)) {
                    SettingsView()
                        .synaraConnectionChrome()
                        .navigationDestination(for: AppRoute.self) { route in
                            RoutePlaceholderView(route: route)
                        }
                }
            case .split:
                NavigationSplitView(columnVisibility: $columnVisibility) {
                    SettingsView(selection: $selection)
                        .synaraConnectionChrome()
                        .navigationSplitViewColumnWidth(
                            min: SynaraSplitColumn.minimumWidth,
                            ideal: SynaraSplitColumn.idealWidth,
                            max: SynaraSplitColumn.maximumWidth
                        )
                        .accessibilityIdentifier("SettingsListColumn")
                } detail: {
                    NavigationStack {
                        SettingsPageView(page: selection ?? .account)
                    }
                    .accessibilityIdentifier("SettingsDetailColumn")
                }
                .navigationSplitViewStyle(.balanced)
            }
        }
        .environment(\.synaraCanvasLayout, layout)
    }
}

/// Connection chrome for a tab root that keeps the system navigation bar.
/// Rooms, room timelines and threads draw their own header and place the
/// chrome under it themselves.
private struct ConnectionChromeInset: ViewModifier {
    let enabled: Bool

    @ViewBuilder
    func body(content: Content) -> some View {
        if enabled {
            content.synaraConnectionChrome()
        } else {
            content
        }
    }
}

extension AppTab {
    /// The root hides the system navigation bar and draws its own header.
    var drawsOwnHeader: Bool {
        self == .rooms
    }
}
