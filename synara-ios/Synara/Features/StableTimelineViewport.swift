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
        enum AnimatedCommandSettlement {
            case noOp
            case animationEnded
            case userInterrupted
            case timeout
        }

        static let maximumMissingTargetAttempts = 3
        static let maximumReadMarkerSettlementAttempts = 6

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

        static func shouldScheduleCommandRetry(
            firedRetryCount: Int,
            hasScheduledRetry: Bool,
            maximumAttempts: Int = maximumMissingTargetAttempts
        ) -> Bool {
            hasScheduledRetry == false && firedRetryCount < maximumAttempts
        }

        static func nextFiredCommandRetryCount(_ firedRetryCount: Int) -> Int {
            firedRetryCount + 1
        }

        static func shouldReplaceScheduledCommandRetry(
            scheduledCommandID: UInt64?,
            currentCommandID: UInt64
        ) -> Bool {
            scheduledCommandID.map { $0 != currentCommandID } ?? false
        }

        static func shouldAbandonPendingNonAnimatedCommandRetry(
            hasScheduledRetry: Bool,
            scheduledCommandID: UInt64?,
            currentCommandID: UInt64?,
            currentCommandIsAnimated: Bool
        ) -> Bool {
            guard hasScheduledRetry,
                  currentCommandIsAnimated == false,
                  let currentCommandID
            else {
                return false
            }
            return scheduledCommandID == currentCommandID
        }

        static func commandRetryMayExecute(retryID: UUID, installedRetryID: UUID?) -> Bool {
            retryID == installedRetryID
        }

        static func animatedCommandSucceeded(
            settlement _: AnimatedCommandSettlement,
            targetsLatest: Bool,
            isTargetVisible: Bool,
            isConfirmedPinned: Bool
        ) -> Bool {
            commandSucceeded(
                targetsLatest: targetsLatest,
                isTargetVisible: isTargetVisible,
                isConfirmedPinned: isConfirmedPinned
            )
        }

        static func commandSucceeded(
            targetsLatest: Bool,
            isTargetVisible: Bool,
            isConfirmedPinned: Bool
        ) -> Bool {
            targetsLatest ? isConfirmedPinned : isTargetVisible
        }

        static func changedIdentifiers<ID: Hashable, Value: Equatable>(
            currentIDs: Set<ID>,
            currentValues: [ID: Value],
            incomingValues: [(id: ID, value: Value)]
        ) -> [ID] {
            incomingValues.compactMap { incoming in
                guard currentIDs.contains(incoming.id),
                      currentValues[incoming.id] != incoming.value
                else {
                    return nil
                }
                return incoming.id
            }
        }
    }

    enum TimelineTimestampRevealGesturePolicy {
        enum Intent: Equatable {
            case pending
            case reveal
            case reject
        }

        static let activationDistance: CGFloat = 12
        static let directionDominance: CGFloat = 1.15

        static func intent(
            translation: CGPoint,
            hasLockedHorizontalIntent: Bool = false,
            activationDistance: CGFloat = activationDistance,
            directionDominance: CGFloat = directionDominance
        ) -> Intent {
            if hasLockedHorizontalIntent {
                return .reveal
            }
            let horizontalDistance = abs(translation.x)
            let verticalDistance = abs(translation.y)
            guard max(horizontalDistance, verticalDistance) >= activationDistance else {
                return .pending
            }
            guard translation.x < 0 else {
                return .reject
            }
            return horizontalDistance > verticalDistance * directionDominance ? .reveal : .reject
        }

        static func ownsSingleTouch(activeTouchCount: Int, isTrackedTouch: Bool) -> Bool {
            activeTouchCount == 1 && isTrackedTouch
        }

        static func cancellationState(
            from state: UIGestureRecognizer.State
        ) -> UIGestureRecognizer.State? {
            switch state {
            case .possible:
                return .failed
            case .began, .changed:
                return .cancelled
            case .ended, .cancelled, .failed:
                return nil
            @unknown default:
                return nil
            }
        }

        static func sessionMatchesAppliedSnapshot(
            sessionRevision: UInt64,
            appliedRevision: UInt64?
        ) -> Bool {
            appliedRevision == sessionRevision
        }
    }

    /// A narrow gesture recognizer for the secondary timestamp affordance.
    ///
    /// Unlike SwiftUI's broad `DragGesture`, this recognizer remains possible until
    /// direction is known and fails as soon as the movement is vertical or rightward.
    /// The enclosing table's pan recognizer therefore retains ownership of ordinary
    /// timeline scrolling.
    private final class TimelineTimestampRevealGestureRecognizer: UIGestureRecognizer {
        private var initialLocation: CGPoint?
        private weak var trackedTouch: UITouch?
        var onRecognitionAborted: (() -> Void)?
        var onReset: (() -> Void)?

        var isTrackingTouch: Bool {
            trackedTouch != nil
        }

        var translation: CGPoint {
            guard let initialLocation, let view else { return .zero }
            let currentLocation = location(in: view)
            return CGPoint(
                x: currentLocation.x - initialLocation.x,
                y: currentLocation.y - initialLocation.y
            )
        }

        func initialLocation(in targetView: UIView) -> CGPoint? {
            guard let initialLocation, let view else { return nil }
            return view.convert(initialLocation, to: targetView)
        }

        override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent) {
            let activeTouchCount = event.allTouches?.reduce(into: 0) { count, touch in
                if touch.phase == .began || touch.phase == .moved || touch.phase == .stationary {
                    count += 1
                }
            } ?? touches.count
            guard trackedTouch == nil,
                  touches.count == 1,
                  activeTouchCount == 1,
                  let touch = touches.first,
                  let view
            else {
                cancelRecognition()
                return
            }
            trackedTouch = touch
            initialLocation = touch.location(in: view)
        }

        override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent) {
            let activeTouchCount = event.allTouches?.reduce(into: 0) { count, touch in
                if touch.phase == .began || touch.phase == .moved || touch.phase == .stationary {
                    count += 1
                }
            } ?? touches.count
            let movedTouch = touches.count == 1 ? touches.first : nil
            guard TimelineTimestampRevealGesturePolicy.ownsSingleTouch(
                activeTouchCount: activeTouchCount,
                isTrackedTouch: movedTouch === trackedTouch
            ) else {
                cancelRecognition()
                return
            }
            switch TimelineTimestampRevealGesturePolicy.intent(
                translation: translation,
                hasLockedHorizontalIntent: state == .began || state == .changed
            ) {
            case .pending:
                break
            case .reveal:
                state = state == .possible ? .began : .changed
            case .reject:
                cancelRecognition()
            }
        }

        override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent) {
            guard touches.count == 1, touches.first === trackedTouch else {
                cancelRecognition()
                return
            }
            switch state {
            case .began, .changed:
                state = .ended
            default:
                cancelRecognition()
            }
        }

        override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent) {
            cancelRecognition()
        }

        func cancelRecognition() {
            guard let cancellationState = TimelineTimestampRevealGesturePolicy.cancellationState(from: state) else {
                return
            }
            state = cancellationState
            onRecognitionAborted?()
        }

        override func reset() {
            super.reset()
            initialLocation = nil
            trackedTouch = nil
            onReset?()
        }
    }

    /// Serializes diffable-data-source mutations while retaining only the
    /// newest requested state. UIKit rejects reentrant snapshot application,
    /// which can otherwise occur when a cell's synchronous layout spins the
    /// main run loop before the current apply has completed.
    struct StableTimelineSnapshotApplyGate<Value> {
        struct Request {
            let value: Value
            let resetPosition: Bool
        }

        private(set) var isApplying = false
        private var pending: Request?

        mutating func schedule(value: Value, resetPosition: Bool) -> Request? {
            let request = Request(value: value, resetPosition: resetPosition)
            guard isApplying else {
                isApplying = true
                return request
            }

            pending = .init(
                value: value,
                resetPosition: resetPosition || (pending?.resetPosition ?? false)
            )
            return nil
        }

        mutating func complete() -> Request? {
            precondition(isApplying, "A timeline snapshot cannot complete without an active apply")
            isApplying = false
            defer { pending = nil }
            return pending
        }

        mutating func discardPending() {
            pending = nil
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
        let isTimestampRevealed: Bool
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

        var serverEventID: String? {
            guard case let .event(eventRow) = content else {
                return nil
            }
            return eventRow.item.serverEventID
        }
    }

    struct StableTimelineViewportCommand: Equatable {
        enum Kind: Equatable {
            case latest(animated: Bool)
            case readMarker(eventID: String)
            case focused(eventID: String, animated: Bool)

            var isAnimated: Bool {
                switch self {
                case let .latest(animated), let .focused(_, animated):
                    return animated
                case .readMarker:
                    return false
                }
            }
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
        let onTimestampRevealRequested: (String, UInt64, String) -> Void
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
                DispatchQueue.main.async { [weak self] in
                    self?.parent.onBottomPinnedChanged(routeID, generation, isPinned, newestEventID)
                }
            }

            func userInteractionChanged(routeID: String, generation: UInt64, isInteracting: Bool) {
                DispatchQueue.main.async { [weak self] in
                    self?.parent.onUserInteractionChanged(routeID, generation, isInteracting)
                }
            }

            func paginationThresholdReached(routeID: String, generation: UInt64, anchorEventID: String?) -> Bool {
                parent.onPaginationThresholdReached(routeID, generation, anchorEventID)
            }

            func timestampRevealRequested(routeID: String, generation: UInt64, eventID: String) {
                DispatchQueue.main.async { [weak self] in
                    self?.parent.onTimestampRevealRequested(routeID, generation, eventID)
                }
            }

            func commandCompleted(
                routeID: String,
                generation: UInt64,
                command: StableTimelineViewportCommand,
                success: Bool,
                targetEventID: String?
            ) {
                DispatchQueue.main.async { [weak self] in
                    self?.parent.onCommandCompleted(routeID, generation, command, success, targetEventID)
                }
            }
        }
    }

    final class StableTimelineViewController: UIViewController, UITableViewDelegate, UIGestureRecognizerDelegate {
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

        private struct PendingAnimatedCommand {
            let command: StableTimelineViewportCommand
            let targetID: StableTimelineViewportItemID
            let targetEventID: String?
            let timeout: DispatchWorkItem
        }

        private struct TimestampRevealGestureSession {
            let routeID: String
            let generation: UInt64
            let snapshotRevision: UInt64
            let itemID: StableTimelineViewportItemID
            let eventID: String
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
        private lazy var timestampRevealGesture: TimelineTimestampRevealGestureRecognizer = {
            let recognizer = TimelineTimestampRevealGestureRecognizer(
                target: self,
                action: #selector(handleTimestampRevealGesture(_:))
            )
            recognizer.onRecognitionAborted = { [weak self] in
                self?.timestampRevealGestureSession = nil
            }
            recognizer.onReset = { [weak self] in
                self?.timestampRevealGestureSession = nil
            }
            return recognizer
        }()
        private var timestampRevealGestureSession: TimestampRevealGestureSession?
        private var timestampRevealAppliedRouteID: String?
        private var timestampRevealAppliedGeneration: UInt64?
        private var timestampRevealAppliedSnapshotRevision: UInt64?
        private var dataSource: UITableViewDiffableDataSource<Section, StableTimelineViewportItemID>!
        private var rowsByID: [StableTimelineViewportItemID: StableTimelineViewportRow] = [:]
        private var visualRows: [StableTimelineViewportRow] = []
        private var configuration: Configuration?
        private var pendingConfiguration: Configuration?
        private var pendingConfigurationResetsPosition = false
        private var snapshotApplyGate = StableTimelineSnapshotApplyGate<Configuration>()
        private var updateSerial: UInt64 = 0
        private var lastExecutedCommandID: UInt64?
        private var pendingAnimatedCommand: PendingAnimatedCommand?
        private var pendingCommandRetry: DispatchWorkItem?
        private var pendingCommandRetryID: UUID?
        private var pendingCommandRetryCommandID: UInt64?
        private var pendingReadMarkerSettlementCommandID: UInt64?
        private var missingTargetAttempts: [UInt64: Int] = [:]
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
            if let command = configuration?.command,
               lastExecutedCommandID != command.id
            {
                _ = executeCommandIfNeeded(command)
            }
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
                timestampRevealGesture.cancelRecognition()
                timestampRevealGestureSession = nil
                timestampRevealAppliedRouteID = nil
                timestampRevealAppliedGeneration = nil
                timestampRevealAppliedSnapshotRevision = nil
                pendingConfiguration = nil
                pendingConfigurationResetsPosition = false
                snapshotApplyGate.discardPending()
                cancelPendingAnimatedCommand()
                pendingCommandRetry?.cancel()
                pendingCommandRetry = nil
                pendingCommandRetryID = nil
                pendingCommandRetryCommandID = nil
                pendingReadMarkerSettlementCommandID = nil
                missingTargetAttempts.removeAll()
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
                pendingConfigurationResetsPosition = pendingConfigurationResetsPosition
                    || isNewRoute
                    || (isNewGeneration && newConfiguration.command != nil)
                return
            }

            scheduleSnapshot(
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
            timestampRevealGesture.delegate = self
            timestampRevealGesture.cancelsTouchesInView = false
            timestampRevealGesture.delaysTouchesBegan = false
            timestampRevealGesture.delaysTouchesEnded = false
            tableView.addGestureRecognizer(timestampRevealGesture)
        }

        @objc private func handleTimestampRevealGesture(
            _ recognizer: TimelineTimestampRevealGestureRecognizer
        ) {
            if recognizer.state == .began {
                timestampRevealGestureSession = timestampRevealSession(at: recognizer.initialLocation(in: tableView))
                if timestampRevealGestureSession == nil {
                    recognizer.cancelRecognition()
                }
                return
            }
            let session = timestampRevealGestureSession
            if recognizer.state == .ended || recognizer.state == .cancelled || recognizer.state == .failed {
                timestampRevealGestureSession = nil
            }
            guard recognizer.state == .ended,
                  let session,
                  let configuration,
                  configuration.routeID == session.routeID,
                  configuration.sessionGeneration == session.generation,
                  TimelineTimestampRevealGesturePolicy.sessionMatchesAppliedSnapshot(
                      sessionRevision: session.snapshotRevision,
                      appliedRevision: timestampRevealAppliedSnapshotRevision
                  ),
                  let row = rowsByID[session.itemID],
                  case let .event(eventRow) = row.content,
                  eventRow.isGroupedWithPrevious,
                  eventRow.item.eventID == session.eventID
            else {
                return
            }
            coordinator.timestampRevealRequested(
                routeID: session.routeID,
                generation: session.generation,
                eventID: session.eventID
            )
        }

        private func timestampRevealSession(at origin: CGPoint?) -> TimestampRevealGestureSession? {
            guard let origin,
                  let configuration,
                  let snapshotRevision = timestampRevealAppliedSnapshotRevision,
                  timestampRevealAppliedRouteID == configuration.routeID,
                  timestampRevealAppliedGeneration == configuration.sessionGeneration,
                  let indexPath = tableView.indexPathForRow(at: origin),
                  let itemID = dataSource.itemIdentifier(for: indexPath),
                  let row = rowsByID[itemID],
                  case let .event(eventRow) = row.content,
                  eventRow.isGroupedWithPrevious
            else {
                return nil
            }
            return .init(
                routeID: configuration.routeID,
                generation: configuration.sessionGeneration,
                snapshotRevision: snapshotRevision,
                itemID: itemID,
                eventID: eventRow.item.eventID
            )
        }

        func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
            guard gestureRecognizer === timestampRevealGesture else {
                return true
            }
            if timestampRevealGesture.isTrackingTouch {
                // Receive additional touches so the recognizer rejects the
                // entire interaction instead of silently continuing one-finger recognition.
                return true
            }
            guard let configuration,
                  timestampRevealAppliedRouteID == configuration.routeID,
                  timestampRevealAppliedGeneration == configuration.sessionGeneration,
                  let indexPath = tableView.indexPathForRow(at: touch.location(in: tableView)),
                  let itemID = dataSource.itemIdentifier(for: indexPath),
                  let row = rowsByID[itemID],
                  case let .event(eventRow) = row.content,
                  eventRow.isGroupedWithPrevious
            else {
                return false
            }
            return true
        }

        func gestureRecognizer(
            _ gestureRecognizer: UIGestureRecognizer,
            shouldRecognizeSimultaneouslyWith otherGestureRecognizer: UIGestureRecognizer
        ) -> Bool {
            gestureRecognizer === timestampRevealGesture || otherGestureRecognizer === timestampRevealGesture
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

        private func scheduleSnapshot(configuration: Configuration, resetPosition: Bool) {
            guard let request = snapshotApplyGate.schedule(
                value: configuration,
                resetPosition: resetPosition
            ) else {
                return
            }
            applySnapshot(request)
        }

        private func applySnapshot(_ request: StableTimelineSnapshotApplyGate<Configuration>.Request) {
            let configuration = request.value
            let resetPosition = request.resetPosition
            pendingConfiguration = nil
            pendingConfigurationResetsPosition = false
            let incomingIDs = Set(configuration.rows.map(\.id))
            let wasConfirmedPinned = isConfirmedPinned()
            let anchor = resetPosition ? nil : snapshotLayout(retaining: incomingIDs)
            let currentRows = rowsByID
            let previousVisualRows = visualRows

            // A gesture may remain in `.possible` while SwiftUI publishes a
            // content-only update for this same room generation. Invalidate it
            // before replacing row identity so the eventual finger-up can never
            // act on a row captured from an earlier snapshot.
            if timestampRevealGesture.isTrackingTouch {
                timestampRevealGesture.cancelRecognition()
            }
            timestampRevealGestureSession = nil
            timestampRevealAppliedRouteID = nil
            timestampRevealAppliedGeneration = nil
            timestampRevealAppliedSnapshotRevision = nil

            visualRows = configuration.rows
            rowsByID = Dictionary(uniqueKeysWithValues: configuration.rows.map { ($0.id, $0) })

            var snapshot = NSDiffableDataSourceSnapshot<Section, StableTimelineViewportItemID>()
            snapshot.appendSections([.main])
            snapshot.appendItems(configuration.rows.reversed().map(\.id), toSection: .main)

            let currentIDs = Set(dataSource.snapshot().itemIdentifiers)
            let changedIDs = StableTimelineViewportPolicy.changedIdentifiers(
                currentIDs: currentIDs,
                currentValues: currentRows,
                incomingValues: configuration.rows.map { (id: $0.id, value: $0) }
            )
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
                guard let self else { return }
                let nextRequest = snapshotApplyGate.complete()

                if serial == updateSerial,
                   self.configuration?.routeID == configuration.routeID,
                   self.configuration?.sessionGeneration == configuration.sessionGeneration
                {
                    timestampRevealAppliedRouteID = configuration.routeID
                    timestampRevealAppliedGeneration = configuration.sessionGeneration
                    timestampRevealAppliedSnapshotRevision = serial
                    tableView.layoutIfNeeded()
                    if executeCommandIfNeeded(configuration.command) == false {
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
                    }
                    updateDiagnostics()
                }

                if let nextRequest {
                    if StableTimelineViewportPolicy.shouldDeferSnapshot(
                        isDragging: isDragging,
                        isDecelerating: isDecelerating
                    ) {
                        pendingConfiguration = nextRequest.value
                        pendingConfigurationResetsPosition = nextRequest.resetPosition
                    } else {
                        scheduleSnapshot(
                            configuration: nextRequest.value,
                            resetPosition: nextRequest.resetPosition
                        )
                    }
                }
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
            scheduleSnapshot(
                configuration: pendingConfiguration,
                resetPosition: pendingConfigurationResetsPosition
            )
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
            if StableTimelineViewportPolicy.shouldReplaceScheduledCommandRetry(
                scheduledCommandID: pendingCommandRetryCommandID,
                currentCommandID: command.id
            ) {
                cancelPendingCommandRetry()
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
                retryOrCompleteCommand(command, targetEventID: nil)
                return true
            }
            guard tableView.bounds.width > 0, tableView.bounds.height > 0 else {
                // SwiftUI may deliver the data snapshot before assigning this
                // representable a frame. Keep the command pending without
                // consuming its missing-target retry budget; the next layout
                // pass will execute it against real viewport dimensions.
                return true
            }

            let targetsReadMarker: Bool
            if case .readMarker = command.kind {
                targetsReadMarker = true
            } else {
                targetsReadMarker = false
            }
            let firedRetryCount = missingTargetAttempts[command.id] ?? 0
            if targetsReadMarker,
               firedRetryCount > 0,
               isRowAtVisualTop(target.0)
            {
                completeCommand(command, success: true, targetEventID: target.1)
                reportBottomPinnedIfChanged(force: true)
                return true
            }
            if targetsReadMarker,
               pendingReadMarkerSettlementCommandID == command.id,
               pendingCommandRetryCommandID == command.id
            {
                return true
            }

            if target.3 {
                cancelPendingCommandRetry()
                missingTargetAttempts.removeValue(forKey: command.id)
                lastExecutedCommandID = command.id
                beginAnimatedCommand(command, targetID: target.0, targetEventID: target.1)
                tableView.scrollToRow(at: indexPath, at: target.2, animated: true)
                DispatchQueue.main.async { [weak self] in
                    guard let self,
                          let pending = pendingAnimatedCommand,
                          pending.command.id == command.id
                    else {
                        return
                    }
                    let success = animatedCommandSucceeded(pending, settlement: .noOp)
                    if success {
                        finishPendingAnimatedCommand(success: true)
                    }
                }
            } else {
                tableView.scrollToRow(at: indexPath, at: target.2, animated: false)
                tableView.layoutIfNeeded()
                let targetsLatest: Bool
                if case .latest = command.kind {
                    targetsLatest = true
                } else {
                    targetsLatest = false
                }
                let succeeded = StableTimelineViewportPolicy.commandSucceeded(
                    targetsLatest: targetsLatest,
                    isTargetVisible: targetsReadMarker ? isRowAtVisualTop(target.0) : isRowVisible(target.0),
                    isConfirmedPinned: isConfirmedPinned()
                )
                if succeeded,
                   targetsReadMarker,
                   firedRetryCount < StableTimelineViewportPolicy.maximumReadMarkerSettlementAttempts
                {
                    // Self-sizing cells update their measured heights after
                    // the first scroll. The delayed retry must first observe
                    // that this placement stayed stable before completing it.
                    pendingReadMarkerSettlementCommandID = command.id
                    scheduleMissingTargetRetry(command)
                } else if succeeded {
                    completeCommand(command, success: true, targetEventID: target.1)
                } else {
                    retryOrCompleteCommand(command, targetEventID: target.1)
                }
                reportBottomPinnedIfChanged(force: true)
            }
            return true
        }

        private func retryOrCompleteCommand(
            _ command: StableTimelineViewportCommand,
            targetEventID: String?
        ) {
            let firedRetryCount = missingTargetAttempts[command.id] ?? 0
            if StableTimelineViewportPolicy.shouldScheduleCommandRetry(
                firedRetryCount: firedRetryCount,
                hasScheduledRetry: pendingCommandRetry != nil
                    && pendingCommandRetryCommandID == command.id
            ) {
                scheduleMissingTargetRetry(command)
            } else if firedRetryCount >= StableTimelineViewportPolicy.maximumMissingTargetAttempts {
                completeCommand(command, success: false, targetEventID: targetEventID)
            }
        }

        private func completeCommand(
            _ command: StableTimelineViewportCommand,
            success: Bool,
            targetEventID: String?
        ) {
            cancelPendingCommandRetry()
            missingTargetAttempts.removeValue(forKey: command.id)
            lastExecutedCommandID = command.id
            notifyCommandCompleted(command, success: success, targetEventID: targetEventID)
        }

        private func scheduleMissingTargetRetry(_ command: StableTimelineViewportCommand) {
            guard pendingCommandRetry == nil else {
                return
            }
            let retryID = UUID()
            let retry = DispatchWorkItem { [weak self] in
                guard let self,
                      StableTimelineViewportPolicy.commandRetryMayExecute(
                          retryID: retryID,
                          installedRetryID: pendingCommandRetryID
                      )
                else {
                    return
                }
                pendingCommandRetry = nil
                pendingCommandRetryID = nil
                pendingCommandRetryCommandID = nil
                pendingReadMarkerSettlementCommandID = nil
                guard configuration?.command?.id == command.id,
                      configuration?.routeID == command.routeID,
                      configuration?.sessionGeneration == command.sessionGeneration
                else {
                    missingTargetAttempts.removeValue(forKey: command.id)
                    return
                }
                missingTargetAttempts[command.id] = StableTimelineViewportPolicy.nextFiredCommandRetryCount(
                    missingTargetAttempts[command.id] ?? 0
                )
                _ = executeCommandIfNeeded(command)
            }
            pendingCommandRetry = retry
            pendingCommandRetryID = retryID
            pendingCommandRetryCommandID = command.id
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.25, execute: retry)
        }

        private func cancelPendingCommandRetry() {
            let abandonedCommandID = pendingCommandRetryCommandID
            pendingCommandRetryID = nil
            pendingCommandRetryCommandID = nil
            pendingReadMarkerSettlementCommandID = nil
            pendingCommandRetry?.cancel()
            pendingCommandRetry = nil
            if let abandonedCommandID {
                missingTargetAttempts.removeValue(forKey: abandonedCommandID)
            }
        }

        private func abandonPendingNonAnimatedCommandRetryForUserDrag() {
            guard let command = configuration?.command,
                  StableTimelineViewportPolicy.shouldAbandonPendingNonAnimatedCommandRetry(
                      hasScheduledRetry: pendingCommandRetry != nil,
                      scheduledCommandID: pendingCommandRetryCommandID,
                      currentCommandID: command.id,
                      currentCommandIsAnimated: command.kind.isAnimated
                  )
            else {
                return
            }

            cancelPendingCommandRetry()
            missingTargetAttempts.removeValue(forKey: command.id)
            lastExecutedCommandID = command.id
            notifyCommandCompleted(command, success: false, targetEventID: nil)
        }

        private func beginAnimatedCommand(
            _ command: StableTimelineViewportCommand,
            targetID: StableTimelineViewportItemID,
            targetEventID: String?
        ) {
            cancelPendingAnimatedCommand()
            let timeout = DispatchWorkItem { [weak self] in
                guard let self,
                      let pending = pendingAnimatedCommand,
                      pending.command.id == command.id
                else {
                    return
                }
                finishPendingAnimatedCommand(success: animatedCommandSucceeded(
                    pending,
                    settlement: .timeout
                ))
            }
            pendingAnimatedCommand = PendingAnimatedCommand(
                command: command,
                targetID: targetID,
                targetEventID: targetEventID,
                timeout: timeout
            )
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.25, execute: timeout)
        }

        private func finishPendingAnimatedCommand(success: Bool) {
            guard let pendingAnimatedCommand else {
                return
            }
            self.pendingAnimatedCommand = nil
            pendingAnimatedCommand.timeout.cancel()
            notifyCommandCompleted(
                pendingAnimatedCommand.command,
                success: success,
                targetEventID: pendingAnimatedCommand.targetEventID
            )
        }

        private func animatedCommandSucceeded(
            _ pending: PendingAnimatedCommand,
            settlement: StableTimelineViewportPolicy.AnimatedCommandSettlement
        ) -> Bool {
            let targetsLatest: Bool
            if case .latest = pending.command.kind {
                targetsLatest = true
            } else {
                targetsLatest = false
            }
            return StableTimelineViewportPolicy.animatedCommandSucceeded(
                settlement: settlement,
                targetsLatest: targetsLatest,
                isTargetVisible: isRowVisible(pending.targetID),
                isConfirmedPinned: isConfirmedPinned()
            )
        }

        private func cancelPendingAnimatedCommand() {
            pendingAnimatedCommand?.timeout.cancel()
            pendingAnimatedCommand = nil
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
                return (cell, cell.convert(cell.bounds, to: view))
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
                return cell.convert(cell.bounds, to: view)
            }.first
        }

        private func isRowVisible(_ id: StableTimelineViewportItemID) -> Bool {
            guard let indexPath = dataSource.indexPath(for: id) else {
                return false
            }
            return tableView.indexPathsForVisibleRows?.contains(indexPath) == true
        }

        private func isRowAtVisualTop(_ id: StableTimelineViewportItemID) -> Bool {
            guard let rowFrame = frame(for: id) else {
                return false
            }
            let viewportTop = tableView.convert(tableView.bounds, to: view).minY
            return abs(rowFrame.minY - viewportTop) <= 4
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
                newestEventID: visualRows.reversed().compactMap(\.serverEventID).first
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
                return (eventID, cell.convert(cell.bounds, to: view).minY)
            }.min(by: { $0.1 < $1.1 })?.0
        }

        private func updateDiagnostics() {
            guard ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1",
                  let configuration
            else {
                return
            }
            let renderedEvents = visualRows.filter { $0.eventID != nil }.count
            let topEventID = visuallyTopmostVisibleEventID() ?? "none"
            let revealedTimestampEventID = visualRows.compactMap { row -> String? in
                guard case let .event(eventRow) = row.content,
                      eventRow.isTimestampRevealed
                else {
                    return nil
                }
                return eventRow.item.eventID
            }.first ?? "none"
            tableView.accessibilityValue = "routeID=\(configuration.routeID);generation=\(configuration.sessionGeneration);renderedEvents=\(renderedEvents);visibleCells=\(tableView.visibleCells.count);topEvent=\(topEventID);newestEvent=\(newestEventRow()?.eventID ?? "none");pinned=\(isConfirmedPinned());revealedTimestamp=\(revealedTimestampEventID)"
        }

        func scrollViewDidScroll(_: UIScrollView) {
            reportBottomPinnedIfChanged()
            requestPaginationIfNeeded()
            updateDiagnostics()
        }

        func scrollViewWillBeginDragging(_: UIScrollView) {
            abandonPendingNonAnimatedCommandRetryForUserDrag()
            if let pendingAnimatedCommand {
                finishPendingAnimatedCommand(success: animatedCommandSucceeded(
                    pendingAnimatedCommand,
                    settlement: .userInterrupted
                ))
            }
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
                finishPendingAnimatedCommand(success: animatedCommandSucceeded(
                    pendingAnimatedCommand,
                    settlement: .animationEnded
                ))
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
