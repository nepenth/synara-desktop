# iOS Performance Report

Reviewed: 2026-05-28

Status: Phase 6.9 instrumentation and fixture performance coverage started.
Simulator/device trace baselines remain a release-candidate gate.

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

## Instrumentation

The app now emits `OSSignpost` markers under subsystem `app.synara.ios`,
category `performance`.

Current stable signposts:

- App lifecycle: `AppInit`, `AppEnvironmentCreate`, `RootShellAppear`,
  `SceneActive`, `SceneInactive`, `SceneBackground`.
- Session startup: `SignedInSessionStart`.
- Room list: `RoomListLoad`.
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

## Performance Backlog

- Capture Instruments Time Profiler traces for cold launch, warm resume,
  room-list first render, timeline first render, timeline scroll, message send,
  media upload, encrypted room open, and sync.
- Add measured launch and warm-resume baselines once release signing and device targets are available.
- Add memory graph snapshots for large timeline scrolling and media viewer presentation.
- Track any P0/P1 regressions before TestFlight; no current fixture-blocking issue is known.
