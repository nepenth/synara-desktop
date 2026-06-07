# Phase 6.9: Performance Review And Remediation

Reviewed: 2026-05-28

## Goal

Make iOS performance measurable before we continue widening the feature surface.
Phase 6.9 covers cold launch, warm resume, room-list loading, timeline loading,
timeline scrolling, message send, media upload, encrypted room open, and
large-fixture behavior.

## Scope

Phase 6.9 applies to the native iOS app on Simulator first, then physical
devices before external TestFlight. Simulator results are useful for regression
detection, but release decisions require device traces.

## Execution Status (2026-05-28)

- Review verdict: Phase 6.9 is locally remediated and re-verified on Simulator,
  but it is not 100% complete by the original release criteria until exportable
  Time Profiler CPU summaries, memory graph snapshots, and a physical-device
  pass are captured.
- `6.9.1 Signpost Instrumentation`: Completed
  - Completed: lifecycle/session/room/timeline/send/upload signposts in app code, including `RoomOpen` and `EncryptedRoomOpen`.
  - Completed: simulator signpost verification captured from runtime logs.
  - Completed: release-candidate evidence captured in `performance-report.md` via `simulator-signposts.log`.
  - Completed: async interval signposts now use scoped cleanup so normal early exits still close the interval.
- `6.9.2 Fixture Performance Tests`: Completed on Simulator
  - Completed: `PerformanceFixtureTests` passes.
  - Completed: large-room and large-timeline fixture UI tests pass.
  - Completed: focused Phase 6.9 verification rerun passed on 2026-05-28 (`10` tests, `Synara` scheme, iPhone 17 Simulator iOS 26.5).
  - Completed: release-prep rerun passed with serial `SynaraTests`, focused
    Phase 6.7/6.9 UI tests, and deterministic screenshot tests.
- `6.9.3 Trace Baselines`: Blocked on simulator trace export reliability
  - Completed: external trace artifact directory created under `/private/tmp/synara-ios-phase6-9-traces/2026-05-28`.
  - Completed: scenario-driven simulator flows executed and documented.
  - Completed: blocked attempts were captured for trace artifact handoff:
    - `xctrace record --template 'Time Profiler' --device <booted-sim-udid> --all-processes --time-limit 8s`
    - `xctrace record --template 'CPU Profiler' --device <booted-sim-udid> --all-processes --time-limit 5s`
    - `xctrace record --template 'Time Profiler' --device <booted-sim-udid> --all-processes --output trace-test-timeprof-all.trace --time-limit 8s`
    - `xctrace record --template 'Time Profiler' --device <booted-sim-udid> --attach <name/pid>` and launch variants.
    - All runs either hang and must be terminated, or export returns `Document Missing Template Error`.
  - Pending: physical-device baseline and memory pass (no iOS device currently attached on this build machine).
- `6.9.4 Memory And Leak Review`: In progress
  - Completed: code-path memory hygiene pass for picker/media/session flows.
  - Pending: Instruments memory graph snapshots for scroll/media/encrypted-room scenarios.
  - Blocked: `xcrun xctrace` simulator trace/memory capture stability remains broken.
- `6.9.5 Performance Remediation Order`: In progress
  - Completed: broad room-list invalidation/load-loop remediation applied before row-level micro-optimization.
  - Completed: signpost interval cleanup remediation applied during review.
  - Pending: further P0/P1 remediation from Time Profiler + memory evidence.
  - Blocked: evidence-driven prioritization is on hold until baselines are exportable.

## Implementation Tasks

### 6.9.1 Signpost Instrumentation

Requirements:

- Emit `OSSignpost` events for app init, environment creation, root-shell
  appearance, scene active/background transitions, and signed-in session start.
- Emit interval signposts for room-list load, timeline initial load, timeline
  pagination, thread load, message send, thread reply send, and media upload.
- Keep signpost names stable so traces can be compared across builds.
- Do not log Matrix access tokens, APNs tokens, room IDs, event IDs, passwords,
  or media URLs in performance instrumentation.

Acceptance criteria:

- Build succeeds with performance instrumentation enabled.
- Instruments can filter signposts under subsystem `com.whylandcreative.synara` and
  category `performance`.
- No sensitive values appear in signpost names or payloads.

### 6.9.2 Fixture Performance Tests

Requirements:

- Keep deterministic XCTest `measure` coverage for large room sorting,
  large-timeline creation, and reply-count derivation.
- Add UI coverage for 1,000-room and 10,000-event fixtures.
- Use performance tests as regression signals, not final release proofs.

Acceptance criteria:

- `PerformanceFixtureTests` passes.
- Large room and large timeline UI tests remain green.
- Any regression that makes fixture rendering visibly unusable becomes a P1
  before adding more UI surface area.

### 6.9.3 Trace Baselines

Requirements:

- Capture Time Profiler traces for:
  - cold launch to signed-out/signed-in shell
  - signed-in session start
  - room-list first render
  - timeline first render
  - timeline scroll
  - message send
  - media upload
  - encrypted room open
- Save trace summaries outside the repository by default.
- Record hardware, simulator/device, OS version, build configuration, and app
  commit in `performance-report.md`.

Acceptance criteria:

- Baseline trace summary exists for Simulator.
- Device trace summary exists before external TestFlight.
- Top CPU stacks and main-thread stalls are listed with owner/fix decision.

### 6.9.4 Memory And Leak Review

Requirements:

- Capture memory graph snapshots for large timeline scroll, media viewer, and
  encrypted room open.
- Look for retained timeline rows, media buffers, `PhotosPickerItem` payloads,
  and duplicate Matrix client/session objects.
- Treat unbounded memory growth during scroll or media viewing as a release
  blocker.

Acceptance criteria:

- Memory snapshot notes are added to `performance-report.md`.
- No obvious retain cycle or duplicate live Matrix session remains unresolved.
- Media upload/viewer paths release large temporary data after completion.

### 6.9.5 Performance Remediation Order

Requirements:

- Fix broad SwiftUI invalidation before micro-optimizing row bodies.
- Keep stable identities for room and timeline rows.
- Avoid doing expensive sorting, parsing, media decoding, or Matrix mapping on
  the main thread where the SDK/API allows background work.
- Keep pagination and media loading explicitly user-bounded until measured
  automatic prefetch is justified.

Acceptance criteria:

- P0/P1 performance findings are fixed before Phase 7+ feature expansion.
- Remaining P2/P3 findings have explicit owners and target phases.
- Performance report is updated after each remediation pass.

## Current Instrumented Signposts

- `AppInit`
- `AppEnvironmentCreate`
- `RootShellAppear`
- `SceneActive`
- `SceneInactive`
- `SceneBackground`
- `SignedInSessionStart`
- `RoomOpen`
- `EncryptedRoomOpen`
- `RoomListLoad`
- `TimelineInitialLoad`
- `TimelineLoadOlder`
- `ThreadTimelineLoad`
- `MessageSend`
- `ThreadMessageSend`
- `MediaUpload`
- `PhotoPickerUpload`
- `ThreadMediaUpload`

## Exit Criteria

Phase 6.9 is complete when:

- Simulator build and test suite pass with instrumentation enabled.
- Fixture performance tests pass.
- At least one Simulator trace pass is recorded in `performance-report.md`.
- Device trace and memory pass are scheduled as release-blocking work while no physical
  device is attached to the machine at present.
- P0/P1 performance issues are fixed or explicitly block the next release gate.
