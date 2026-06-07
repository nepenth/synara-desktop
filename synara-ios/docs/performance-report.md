# iOS Performance Report

Reviewed: 2026-05-28

Status: Phase 6.9 instrumentation and fixture coverage is active. Simulator
tests and signpost validation are passing. Time Profiler baseline capture and
memory graph snapshots remain release-candidate gates.
Physical-device-only gates remain blocked while no USB/iOS device is connected in
`xcrun simctl list devices available`.

Review verdict: local Phase 6.9 implementation work is remediated and verified,
but Phase 6.9 is not complete by the original release criteria until exportable
CPU stack summaries, memory snapshots, and a physical-device pass are captured.

## Fixture Coverage

- Room list fixture: `RoomListFixtures.large(count: 1_000)` creates stable, sorted room identifiers for scroll validation.
- Timeline fixture: `TimelineFixtures.largeTimeline(count: 10_000)` creates stable synthetic events for timeline render validation.
- UI test launch flags:
  - `SYNARA_UI_TEST_LARGE_ROOMS=1`
  - `SYNARA_UI_TEST_LARGE_TIMELINE=1`
  - `SYNARA_UI_TEST_ROOM_SEARCH=<query>`

## Current Results

- Deterministic UI tests verify that the 1,000-room list renders and remains scrollable.
- Deterministic UI tests verify that the 10,000-event timeline renders and remains scrollable.
- Deterministic UI tests verify that room filtering preserves stable room rows.
- `PerformanceFixtureTests` measure large room sorting, large timeline fixture
  creation, and reply-count derivation.
- Timeline rows use `LazyVStack`; room rows use native `List`.
- Media upload and timeline pagination remain bounded by explicit user actions.

## Latest Execution (2026-05-28)

- Baseline commit: `0430b3c` plus local Phase 6.9 remediation changes.
- Simulator target: `iPhone 17 (iOS 26.5)`.
- Build configuration: `Debug`.
- XCTest run:
  - `SynaraTests/PerformanceFixtureTests` passed.
  - `SynaraUITests/testLargeRoomFixtureRendersAndScrolls` passed.
  - `SynaraUITests/testLargeTimelineFixtureRendersAndScrolls` passed.
- Performance fixture measurements (`XCTPerformanceMetric_WallClockTime`):
  - large room sort average: `0.006s`.
  - large timeline fixture creation average: `0.012s`.
  - reply-count derivation average: `0.004s`.

Post-remediation fixture rerun (2026-05-28):

- large room sort average: `0.005s`.
- large timeline fixture creation average: `0.009s`.
- reply-count derivation average: `0.004s`.

Additional simulator execution completed on 2026-05-28:

- `SynaraUITests/testRoomListShowsStableRoomRows` passed.
- `SynaraUITests/testRoomRouteShowsTimeline` passed.
- `SynaraUITests/testComposerSendsMockMessage` passed.
- `SynaraUITests/testMediaUploadAddsAttachmentPlaceholder` passed.
- `SynaraUITests/testEncryptedTimelineShowsCryptoStatusRecoveryBannerAndSafePlaceholder` passed.

Focused review verification completed on 2026-05-28:

- Scheme: `Synara`.
- Simulator target: `iPhone 17 (iOS 26.5)`.
- Result: `xcodebuild` action succeeded with `10` focused Phase 6.9 tests.
- Coverage:
  - `SynaraTests/PerformanceFixtureTests`.
  - `SynaraUITests/testLargeRoomFixtureRendersAndScrolls`.
  - `SynaraUITests/testLargeTimelineFixtureRendersAndScrolls`.
  - `SynaraUITests/testRoomListShowsStableRoomRows`.
  - `SynaraUITests/testRoomRouteShowsTimeline`.
  - `SynaraUITests/testComposerSendsMockMessage`.
  - `SynaraUITests/testMediaUploadAddsAttachmentPlaceholder`.
  - `SynaraUITests/testEncryptedTimelineShowsCryptoStatusRecoveryBannerAndSafePlaceholder`.
- Note: the MCP wrapper timed out while waiting for the tool response, but the
  produced `.xcresult` action status was `succeeded` with `10` tests.

Release-prep verification completed on 2026-05-28:

- `xcodebuild -quiet ... generic/platform=iOS Simulator build` passed after
  clearing rebuildable Xcode DerivedData cache.
- `SynaraTests` passed with `-parallel-testing-enabled NO`.
- Focused Phase 6.7/6.9 UI validation passed:
  - `testRoomListShowsStableRoomRows`
  - `testRoomRouteShowsTimeline`
  - `testComposerSendsMockMessage`
  - `testMediaUploadAddsAttachmentPlaceholder`
  - `testUnavailableAttachmentOptionShowsHonestState`
  - `testThreadViewOpensAndRepliesFromTimeline`
  - `testLargeRoomFixtureRendersAndScrolls`
  - `testLargeTimelineFixtureRendersAndScrolls`
- Deterministic screenshot validation passed for:
  - rooms/timeline/composer/attachment: `01-mock-room-list.png` through
    `04-mock-attachment-sheet.png`
  - thread: `05-mock-thread.png`, `06-mock-thread-typing.png`
  - agent approval: `07-mock-agent-approval.png`
- Full `SynaraTests` without explicit serial execution showed transient
  order-dependent failures in isolated-passing tests; serial execution is the
  current reliable local command until test isolation is tightened.

## Simulator Signpost Evidence

- Signpost log capture file: `/private/tmp/synara-ios-phase6-9-traces/2026-05-28/simulator-signposts.log`.
- Instrumentation coverage observed in simulator logs:
  - `AppInit`, `AppEnvironmentCreate`, `RootShellAppear`, `SceneActive`
  - `SignedInSessionStart`
  - `RoomOpen`, `EncryptedRoomOpen`, `RoomListLoad`
  - `TimelineInitialLoad`
  - `MessageSend`, `MediaUpload`
- Room-list remediation verification:
  - Before fix, one 5-minute sample showed pathological `RoomListLoad` volume.
  - After adding one-time initial room-list task gating, a post-fix sample
    limited `RoomListLoad` to expected low single-digit launches per targeted
    test run.

## Instrumentation

The app now emits `OSSignpost` markers under subsystem `com.whylandcreative.synara`,
category `performance`.

Current stable signposts:

- App lifecycle: `AppInit`, `AppEnvironmentCreate`, `RootShellAppear`,
  `SceneActive`, `SceneInactive`, `SceneBackground`.
- Session startup: `SignedInSessionStart`.
- Room open/list: `RoomOpen`, `EncryptedRoomOpen`, `RoomListLoad`.
- Timeline: `TimelineInitialLoad`, `TimelineLoadOlder`, `ThreadTimelineLoad`.
- Send/upload: `MessageSend`, `ThreadMessageSend`, `MediaUpload`,
  `PhotoPickerUpload`, `ThreadMediaUpload`.

Instrumentation intentionally does not attach room IDs, event IDs, access
tokens, APNs tokens, passwords, or media URLs to signpost names or payloads.

## Phase 6.9 Trace Plan

Capture Simulator baselines first, then physical-device baselines before
external TestFlight:

- Cold launch to signed-out shell.
- Cold launch with restored signed-in session.
- Warm resume from background.
- Signed-in Matrix session start.
- Room list first render.
- Timeline first render.
- Timeline scroll through large fixture.
- Message send.
- Media upload.
- Encrypted room open.

For each trace, record device/simulator, OS version, build configuration, app
commit, top CPU stacks, main-thread stalls, memory growth, and remediation
decision.

Current blocker:

- `xcrun xctrace` recording in this environment repeatedly fails to finalize
  cleanly (hung sessions). Export attempts for simulator-target traces return
  `Document Missing Template Error` for `trace` directories such as:
  - `/private/tmp/synara-ios-phase6-9-traces/2026-05-28/trace-test-timeprof-all.trace`
  - `/private/tmp/synara-ios-phase6-9-traces/2026-05-28/time-profiler-standalone.trace`
  - `/private/tmp/synara-ios-phase6-9-traces/2026-05-28/cold-launch-signed-out.trace`
- Known-good xctrace export path (host-side `--attach` to `/bin/sleep`) still works and is retained as a control trace, but does
  not cover app scenarios.
- `xcrun xctrace list devices` currently reports only the host Mac under
  physical devices; iOS targets are simulator entries.
- Simulator signpost captures and XCTest timings were collected as fallback execution
  evidence while trace-tool reliability is being resolved.

## Memory And Leak Review (Current Pass)

- Media upload flows clear selected picker state on completion/failure paths.
- Room avatar upload clears `selectedAvatarPhoto` in success/failure paths.
- Media viewer currently renders metadata placeholder content (no large decoded
  buffer retention path in current implementation).
- No duplicate `AppEnvironment` live-session creation pattern was observed in
  the startup path (`SynaraApp` keeps a single environment instance).
- Dedicated memory graph snapshots are still pending a stable trace/memory
  capture session.

## Performance Backlog

- Capture Instruments Time Profiler traces for cold launch, warm resume,
  room-list first render, timeline first render, timeline scroll, message send,
  media upload, encrypted room open, and sync.
- Add measured launch and warm-resume baselines once release signing and device targets are available.
- Add memory graph snapshots for large timeline scrolling and media viewer presentation.
- Track any P0/P1 regressions before TestFlight; no current fixture-blocking issue is known.
