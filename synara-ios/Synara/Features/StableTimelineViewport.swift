import SwiftUI
#if canImport(UIKit)
    import UIKit

    enum StableScrollAnchoringFeatureFlag {
        static let environmentKey = "SYNARA_STABLE_SCROLL_ANCHORING"
        static let userDefaultsKey = "stableScrollAnchoring"

        static var isEnabled: Bool {
            let environmentValue = ProcessInfo.processInfo.environment[environmentKey]
            let persistedValue = UserDefaults.standard.object(forKey: userDefaultsKey) as? Bool
            return resolve(environmentValue: environmentValue, persistedValue: persistedValue)
        }

        static func resolve(environmentValue: String?, persistedValue: Bool?) -> Bool {
            if let environmentValue {
                switch environmentValue.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
                case "0", "false", "no", "off":
                    return false
                case "1", "true", "yes", "on":
                    return true
                default:
                    break
                }
            }
            return persistedValue ?? true
        }
    }

    enum StableTimelineViewportPolicy {
        static func shouldDeferSnapshot(isDragging: Bool, isDecelerating: Bool) -> Bool {
            isDragging || isDecelerating
        }

        static func shouldFollowNewest(isLive: Bool, wasConfirmedPinned: Bool) -> Bool {
            isLive && wasConfirmedPinned
        }

    static func shouldRestoreAnchor(isDragging: Bool, isDecelerating: Bool, hasAnchor: Bool) -> Bool {
        hasAnchor && shouldDeferSnapshot(isDragging: isDragging, isDecelerating: isDecelerating) == false
    }

    static func restoredContentOffset(
        currentContentOffset: CGFloat,
        previousAnchorMinY: CGFloat,
        updatedAnchorMinY: CGFloat
    ) -> CGFloat {
        currentContentOffset - (updatedAnchorMinY - previousAnchorMinY)
    }

        static func shouldRequestPagination(
            contentOffset: CGFloat,
            contentHeight: CGFloat,
            viewportHeight: CGFloat,
            thresholdScreens: CGFloat = 1.5,
            hasUserInteracted: Bool,
            isPaginating: Bool,
            requestInFlight: Bool
        ) -> Bool {
            guard hasUserInteracted,
                  isPaginating == false,
                  requestInFlight == false,
                  contentHeight > viewportHeight
            else {
                return false
            }
            let threshold = max(viewportHeight, viewportHeight * thresholdScreens)
            return contentOffset + viewportHeight >= contentHeight - threshold
        }

        static func boundedVisibleCellUpperBound(
            viewportHeight: CGFloat,
            minimumEstimatedRowHeight: CGFloat,
            reuseOverscan: Int = 8
        ) -> Int {
            guard viewportHeight > 0, minimumEstimatedRowHeight > 0 else {
                return reuseOverscan
            }
            return Int(ceil(viewportHeight / minimumEstimatedRowHeight)) + reuseOverscan
        }
    }

    enum StableTimelineViewportItemID: Hashable {
        case event(String)
        case unreadDivider(String)
        case cryptoBanner
        case pagination
    }

    struct StableTimelineViewportEventRow: Equatable {
        let item: TimelineItem
        let isGroupedWithPrevious: Bool
        let animateSend: Bool
        let replyPreview: TimelineReplyPreview?
        let replyCount: Int
        let availability: EventActionAvailability
    }

    struct StableTimelineViewportRow: Equatable, Identifiable {
        enum Content: Equatable {
            case event(StableTimelineViewportEventRow)
            case unreadDivider
            case cryptoBanner(RoomCryptoStatus)
            case pagination
        }

        let id: StableTimelineViewportItemID
        let content: Content

        var eventID: String? {
            guard case let .event(eventRow) = content else {
                return nil
            }
            return eventRow.item.eventID
        }
    }

    struct StableTimelineViewportCommand: Equatable {
        enum Kind: Equatable {
            case latest(animated: Bool)
            case readMarker(eventID: String)
            case focused(eventID: String, animated: Bool)
        }

        let id: UInt64
        let routeID: String
        let sessionGeneration: UInt64
        let kind: Kind
    }

    struct StableTimelineViewport: UIViewControllerRepresentable {
        typealias UIViewControllerType = StableTimelineViewController

        let routeID: String
        let sessionGeneration: UInt64
        let rows: [StableTimelineViewportRow]
        let command: StableTimelineViewportCommand?
        let isLive: Bool
        let isPaginating: Bool
        let backgroundColor: UIColor
        let rowContent: (StableTimelineViewportRow) -> AnyView
        let onBottomPinnedChanged: (String, UInt64, Bool, String?) -> Void
        let onUserInteractionChanged: (String, UInt64, Bool) -> Void
        let onPaginationThresholdReached: (String, UInt64, String?) -> Bool
        let onCommandCompleted: (String, UInt64, StableTimelineViewportCommand, Bool, String?) -> Void

        func makeCoordinator() -> Coordinator {
            Coordinator(parent: self)
        }

        func makeUIViewController(context: Context) -> StableTimelineViewController {
            StableTimelineViewController(coordinator: context.coordinator)
        }

        func updateUIViewController(_ uiViewController: StableTimelineViewController, context: Context) {
            context.coordinator.update(parent: self)
            uiViewController.update(
                configuration: .init(
                    routeID: routeID,
                    sessionGeneration: sessionGeneration,
                    rows: rows,
                    command: command,
                    isLive: isLive,
                    isPaginating: isPaginating,
                    backgroundColor: backgroundColor
                )
            )
        }

        final class Coordinator {
            private(set) var parent: StableTimelineViewport

            init(parent: StableTimelineViewport) {
                self.parent = parent
            }

            func update(parent: StableTimelineViewport) {
                self.parent = parent
            }

            func content(for row: StableTimelineViewportRow) -> AnyView {
                parent.rowContent(row)
            }

            func bottomPinnedChanged(routeID: String, generation: UInt64, isPinned: Bool, newestEventID: String?) {
                parent.onBottomPinnedChanged(routeID, generation, isPinned, newestEventID)
            }

            func userInteractionChanged(routeID: String, generation: UInt64, isInteracting: Bool) {
                parent.onUserInteractionChanged(routeID, generation, isInteracting)
            }

            func paginationThresholdReached(routeID: String, generation: UInt64, anchorEventID: String?) -> Bool {
                parent.onPaginationThresholdReached(routeID, generation, anchorEventID)
            }

            func commandCompleted(
                routeID: String,
                generation: UInt64,
                command: StableTimelineViewportCommand,
                success: Bool,
                targetEventID: String?
            ) {
                parent.onCommandCompleted(routeID, generation, command, success, targetEventID)
            }
        }
    }

    final class StableTimelineViewController: UIViewController, UITableViewDelegate {
        struct Configuration {
            let routeID: String
            let sessionGeneration: UInt64
            let rows: [StableTimelineViewportRow]
            let command: StableTimelineViewportCommand?
            let isLive: Bool
            let isPaginating: Bool
            let backgroundColor: UIColor
        }

        private enum Section {
            case main
        }

        private struct LayoutAnchor {
            let id: StableTimelineViewportItemID
            let frame: CGRect
        }

        private final class Cell: UITableViewCell {
            var stableID: StableTimelineViewportItemID?

            override func prepareForReuse() {
                super.prepareForReuse()
                stableID = nil
                contentConfiguration = nil
            }
        }

        private let coordinator: StableTimelineViewport.Coordinator
        private let tableView = UITableView(frame: .zero, style: .plain)
        private var dataSource: UITableViewDiffableDataSource<Section, StableTimelineViewportItemID>!
        private var rowsByID: [StableTimelineViewportItemID: StableTimelineViewportRow] = [:]
        private var visualRows: [StableTimelineViewportRow] = []
        private var configuration: Configuration?
        private var pendingConfiguration: Configuration?
        private var updateSerial: UInt64 = 0
        private var lastExecutedCommandID: UInt64?
        private var pendingAnimatedCommand: (StableTimelineViewportCommand, String?)?
        private var isDragging = false
        private var isDecelerating = false
        private var hasUserInteracted = false
        private var paginationRequestInFlight = false
        private var lastReportedPinned: Bool?

        init(coordinator: StableTimelineViewport.Coordinator) {
            self.coordinator = coordinator
            super.init(nibName: nil, bundle: nil)
            configureTableView()
            configureDataSource()
        }

        @available(*, unavailable)
        required init?(coder _: NSCoder) {
            fatalError("init(coder:) is unavailable")
        }

        override func viewDidLoad() {
            super.viewDidLoad()
            view.addSubview(tableView)
        }

        override func viewDidLayoutSubviews() {
            super.viewDidLayoutSubviews()
            tableView.frame = view.bounds
            updateDiagnostics()
        }

        func update(configuration newConfiguration: Configuration) {
            if let configuration,
               configuration.routeID == newConfiguration.routeID,
               newConfiguration.sessionGeneration < configuration.sessionGeneration
            {
                return
            }

            tableView.backgroundColor = newConfiguration.backgroundColor
            view.backgroundColor = newConfiguration.backgroundColor

            let isNewRoute = configuration?.routeID != newConfiguration.routeID
            let isNewGeneration = isNewRoute || configuration?.sessionGeneration != newConfiguration.sessionGeneration
            if isNewGeneration {
                pendingConfiguration = nil
                pendingAnimatedCommand = nil
                lastExecutedCommandID = nil
                paginationRequestInFlight = false
                hasUserInteracted = false
                lastReportedPinned = nil
            }

            if configuration?.isPaginating == true, newConfiguration.isPaginating == false {
                paginationRequestInFlight = false
            }

            configuration = newConfiguration
            if StableTimelineViewportPolicy.shouldDeferSnapshot(
                isDragging: isDragging,
                isDecelerating: isDecelerating
            ) {
                pendingConfiguration = newConfiguration
                return
            }

            applySnapshot(
                configuration: newConfiguration,
                resetPosition: isNewRoute || (isNewGeneration && newConfiguration.command != nil)
            )
        }

        private func configureTableView() {
            tableView.register(Cell.self, forCellReuseIdentifier: "StableTimelineCell")
            tableView.separatorStyle = .none
            tableView.allowsSelection = false
            tableView.keyboardDismissMode = .interactive
            tableView.rowHeight = UITableView.automaticDimension
            tableView.estimatedRowHeight = 88
            tableView.delegate = self
            tableView.transform = CGAffineTransform(scaleX: 1, y: -1)
            tableView.accessibilityIdentifier = "TimelineList"
            tableView.accessibilityLabel = "Room timeline"
        }

        private func configureDataSource() {
            dataSource = UITableViewDiffableDataSource<Section, StableTimelineViewportItemID>(
                tableView: tableView
            ) { [weak self] tableView, indexPath, itemID in
                let cell = tableView.dequeueReusableCell(withIdentifier: "StableTimelineCell", for: indexPath)
                guard let self, let cell = cell as? Cell, let row = rowsByID[itemID] else {
                    return cell
                }

                cell.stableID = itemID
                cell.backgroundColor = .clear
                cell.contentView.backgroundColor = .clear
                cell.contentConfiguration = UIHostingConfiguration { [coordinator] in
                    coordinator.content(for: row)
                }
                .margins(.all, 0)
                .minSize(height: 1)
                .background(Color.clear)
                cell.contentView.transform = CGAffineTransform(scaleX: 1, y: -1)
                return cell
            }
            dataSource.defaultRowAnimation = UIAccessibility.isReduceMotionEnabled ? .none : .fade
        }

        private func applySnapshot(configuration: Configuration, resetPosition: Bool) {
            pendingConfiguration = nil
            let incomingIDs = Set(configuration.rows.map(\.id))
            let wasConfirmedPinned = isConfirmedPinned()
            let anchor = resetPosition ? nil : snapshotLayout(retaining: incomingIDs)
            let currentRows = rowsByID
            let previousVisualRows = visualRows

            visualRows = configuration.rows
            rowsByID = Dictionary(uniqueKeysWithValues: configuration.rows.map { ($0.id, $0) })

            var snapshot = NSDiffableDataSourceSnapshot<Section, StableTimelineViewportItemID>()
            snapshot.appendSections([.main])
            snapshot.appendItems(configuration.rows.reversed().map(\.id), toSection: .main)

            let currentIDs = Set(dataSource.snapshot().itemIdentifiers)
            let changedIDs = configuration.rows.compactMap { row -> StableTimelineViewportItemID? in
                guard currentIDs.contains(row.id), currentRows[row.id] != row else {
                    return nil
                }
                return row.id
            }
            if changedIDs.isEmpty == false {
                snapshot.reconfigureItems(changedIDs)
            }

            updateSerial &+= 1
            let serial = updateSerial
            let newestChanged = newestEventID(in: previousVisualRows) != newestEventID(in: configuration.rows)
            let shouldAnimate = configuration.isLive
                && wasConfirmedPinned
                && newestChanged
                && UIAccessibility.isReduceMotionEnabled == false

            dataSource.apply(snapshot, animatingDifferences: shouldAnimate) { [weak self] in
                guard let self,
                      serial == updateSerial,
                      self.configuration?.routeID == configuration.routeID,
                      self.configuration?.sessionGeneration == configuration.sessionGeneration
                else {
                    return
                }

                tableView.layoutIfNeeded()
                if executeCommandIfNeeded(configuration.command) {
                    updateDiagnostics()
                    return
                }

                if StableTimelineViewportPolicy.shouldFollowNewest(
                    isLive: configuration.isLive,
                    wasConfirmedPinned: wasConfirmedPinned
                ) {
                    scrollToNewest(animated: shouldAnimate)
                } else if StableTimelineViewportPolicy.shouldRestoreAnchor(
                    isDragging: isDragging,
                    isDecelerating: isDecelerating,
                    hasAnchor: anchor != nil
                ), let anchor {
                    restoreLayout(anchor)
                }

                reportBottomPinnedIfChanged(force: true)
                requestPaginationIfNeeded()
                updateDiagnostics()
            }
        }

        private func applyPendingConfigurationIfPossible() {
            guard StableTimelineViewportPolicy.shouldDeferSnapshot(
                isDragging: isDragging,
                isDecelerating: isDecelerating
            ) == false,
                let pendingConfiguration
            else {
                return
            }
            applySnapshot(configuration: pendingConfiguration, resetPosition: false)
        }

        @discardableResult
        private func executeCommandIfNeeded(_ command: StableTimelineViewportCommand?) -> Bool {
            guard let command,
                  lastExecutedCommandID != command.id,
                  command.routeID == configuration?.routeID,
                  command.sessionGeneration == configuration?.sessionGeneration
            else {
                return false
            }

            let target: (StableTimelineViewportItemID, String?, UITableView.ScrollPosition, Bool)?
            switch command.kind {
            case let .latest(animated):
                target = newestEventRow().map { ($0.id, $0.eventID, .top, animated) }
            case let .readMarker(markerEventID):
                target = rowAfterReadMarker(markerEventID).map { ($0.id, $0.eventID, .bottom, false) }
            case let .focused(eventID, animated):
                target = eventRow(eventID: eventID).map { ($0.id, $0.eventID, .bottom, animated) }
            }

            guard let target,
                  let indexPath = dataSource.indexPath(for: target.0)
            else {
                notifyCommandCompleted(command, success: false, targetEventID: nil)
                return false
            }

            lastExecutedCommandID = command.id
            tableView.scrollToRow(at: indexPath, at: target.2, animated: target.3)
            if target.3 {
                pendingAnimatedCommand = (command, target.1)
            } else {
                tableView.layoutIfNeeded()
                notifyCommandCompleted(command, success: isRowVisible(target.0), targetEventID: target.1)
                reportBottomPinnedIfChanged(force: true)
            }
            return true
        }

        private func notifyCommandCompleted(
            _ command: StableTimelineViewportCommand,
            success: Bool,
            targetEventID: String?
        ) {
            guard let configuration else {
                return
            }
            coordinator.commandCompleted(
                routeID: configuration.routeID,
                generation: configuration.sessionGeneration,
                command: command,
                success: success,
                targetEventID: targetEventID
            )
        }

        private func scrollToNewest(animated: Bool) {
            guard let newest = newestEventRow(), let indexPath = dataSource.indexPath(for: newest.id) else {
                return
            }
            tableView.scrollToRow(at: indexPath, at: .top, animated: animated)
            if animated == false {
                tableView.layoutIfNeeded()
                reportBottomPinnedIfChanged(force: true)
            }
        }

        private func newestEventRow() -> StableTimelineViewportRow? {
            visualRows.reversed().first { $0.eventID != nil }
        }

        private func newestEventID(in rows: [StableTimelineViewportRow]) -> String? {
            rows.reversed().compactMap(\.eventID).first
        }

        private func eventRow(eventID: String) -> StableTimelineViewportRow? {
            visualRows.first { row in
                guard case let .event(eventRow) = row.content else {
                    return false
                }
                return eventRow.item.eventID == eventID || eventRow.item.id == eventID
            }
        }

        private func rowAfterReadMarker(_ markerEventID: String) -> StableTimelineViewportRow? {
            guard let markerIndex = visualRows.firstIndex(where: { row in
                guard case let .event(eventRow) = row.content else {
                    return false
                }
                return eventRow.item.eventID == markerEventID || eventRow.item.id == markerEventID
            }) else {
                return nil
            }

            return visualRows.dropFirst(markerIndex + 1).first { $0.eventID != nil }
                ?? visualRows[markerIndex]
        }

        private func snapshotLayout(retaining incomingIDs: Set<StableTimelineViewportItemID>) -> LayoutAnchor? {
            let candidates = tableView.visibleCells.compactMap { cell -> (Cell, CGRect)? in
                guard let cell = cell as? Cell,
                      let stableID = cell.stableID,
                      incomingIDs.contains(stableID),
                      case .event = stableID
                else {
                    return nil
                }
                return (cell, tableView.convert(cell.frame, to: view))
            }
            guard let candidate = candidates.min(by: { $0.1.minY < $1.1.minY }),
                  let id = candidate.0.stableID
            else {
                return nil
            }
            return LayoutAnchor(id: id, frame: candidate.1)
        }

        private func restoreLayout(_ anchor: LayoutAnchor) {
            guard isDragging == false,
                  isDecelerating == false,
                  let indexPath = dataSource.indexPath(for: anchor.id)
            else {
                return
            }

            tableView.scrollToRow(at: indexPath, at: .bottom, animated: false)
            tableView.layoutIfNeeded()
            guard let newFrame = frame(for: anchor.id) else {
                return
            }
        let restoredOffset = StableTimelineViewportPolicy.restoredContentOffset(
            currentContentOffset: tableView.contentOffset.y,
            previousAnchorMinY: anchor.frame.minY,
            updatedAnchorMinY: newFrame.minY
        )
        if abs(restoredOffset - tableView.contentOffset.y) > .ulpOfOne {
            tableView.contentOffset.y = restoredOffset
                tableView.layoutIfNeeded()
            }
        }

        private func frame(for id: StableTimelineViewportItemID) -> CGRect? {
            tableView.visibleCells.compactMap { cell -> CGRect? in
                guard let cell = cell as? Cell, cell.stableID == id else {
                    return nil
                }
                return tableView.convert(cell.frame, to: view)
            }.first
        }

        private func isRowVisible(_ id: StableTimelineViewportItemID) -> Bool {
            guard let indexPath = dataSource.indexPath(for: id) else {
                return false
            }
            return tableView.indexPathsForVisibleRows?.contains(indexPath) == true
        }

        private func isConfirmedPinned() -> Bool {
            guard let newest = newestEventRow(),
                  isRowVisible(newest.id)
            else {
                return false
            }
            let bottomTolerance: CGFloat = 2
            return tableView.contentOffset.y <= -tableView.adjustedContentInset.top + bottomTolerance
        }

        private func reportBottomPinnedIfChanged(force: Bool = false) {
            guard let configuration else {
                return
            }
            let isPinned = isConfirmedPinned()
            guard force || lastReportedPinned != isPinned else {
                return
            }
            lastReportedPinned = isPinned
            coordinator.bottomPinnedChanged(
                routeID: configuration.routeID,
                generation: configuration.sessionGeneration,
                isPinned: isPinned,
                newestEventID: newestEventRow()?.eventID
            )
        }

        private func requestPaginationIfNeeded() {
            guard let configuration,
                  StableTimelineViewportPolicy.shouldRequestPagination(
                      contentOffset: tableView.contentOffset.y,
                      contentHeight: tableView.contentSize.height,
                      viewportHeight: tableView.bounds.height,
                      hasUserInteracted: hasUserInteracted,
                      isPaginating: configuration.isPaginating,
                      requestInFlight: paginationRequestInFlight
                  )
            else {
                return
            }

            let anchorEventID = visuallyTopmostVisibleEventID()
            paginationRequestInFlight = coordinator.paginationThresholdReached(
                routeID: configuration.routeID,
                generation: configuration.sessionGeneration,
                anchorEventID: anchorEventID
            )
        }

        private func visuallyTopmostVisibleEventID() -> String? {
            tableView.visibleCells.compactMap { cell -> (String, CGFloat)? in
                guard let cell = cell as? Cell,
                      let stableID = cell.stableID,
                      case let .event(eventID) = stableID
                else {
                    return nil
                }
                return (eventID, tableView.convert(cell.frame, to: view).minY)
            }.min(by: { $0.1 < $1.1 })?.0
        }

        private func updateDiagnostics() {
            guard ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1" else {
                return
            }
            let renderedEvents = visualRows.filter { $0.eventID != nil }.count
            let topEventID = visuallyTopmostVisibleEventID() ?? "none"
            tableView.accessibilityValue = "renderedEvents=\(renderedEvents);visibleCells=\(tableView.visibleCells.count);topEvent=\(topEventID);newestEvent=\(newestEventRow()?.eventID ?? "none");pinned=\(isConfirmedPinned())"
        }

        func scrollViewDidScroll(_: UIScrollView) {
            reportBottomPinnedIfChanged()
            requestPaginationIfNeeded()
            updateDiagnostics()
        }

        func scrollViewWillBeginDragging(_: UIScrollView) {
            isDragging = true
            isDecelerating = false
            hasUserInteracted = true
            if let configuration {
                coordinator.userInteractionChanged(
                    routeID: configuration.routeID,
                    generation: configuration.sessionGeneration,
                    isInteracting: true
                )
            }
        }

        func scrollViewDidEndDragging(_: UIScrollView, willDecelerate decelerate: Bool) {
            isDragging = false
            isDecelerating = decelerate
            if decelerate == false {
                finishUserScroll()
            }
        }

        func scrollViewDidEndDecelerating(_: UIScrollView) {
            isDecelerating = false
            finishUserScroll()
        }

        func scrollViewDidEndScrollingAnimation(_: UIScrollView) {
            if let pendingAnimatedCommand {
                self.pendingAnimatedCommand = nil
                notifyCommandCompleted(
                    pendingAnimatedCommand.0,
                    success: pendingAnimatedCommand.1.map { eventID in
                        eventRow(eventID: eventID).map { isRowVisible($0.id) } ?? false
                    } ?? false,
                    targetEventID: pendingAnimatedCommand.1
                )
            }
            reportBottomPinnedIfChanged(force: true)
            updateDiagnostics()
        }

        private func finishUserScroll() {
            if let configuration {
                coordinator.userInteractionChanged(
                    routeID: configuration.routeID,
                    generation: configuration.sessionGeneration,
                    isInteracting: false
                )
            }
            applyPendingConfigurationIfPossible()
            reportBottomPinnedIfChanged(force: true)
            requestPaginationIfNeeded()
            updateDiagnostics()
        }
    }
#endif
