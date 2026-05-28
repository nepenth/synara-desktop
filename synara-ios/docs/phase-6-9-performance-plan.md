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
- Instruments can filter signposts under subsystem `app.synara.ios` and
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
- Device trace and memory pass are scheduled as release-blocking work if a
  physical device/signing setup is not yet available.
- P0/P1 performance issues are fixed or explicitly block the next release gate.
