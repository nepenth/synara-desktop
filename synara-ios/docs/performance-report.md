# iOS Performance Report

Reviewed: 2026-05-27

Status: initial Phase 6 fixture pass complete. Instruments traces remain a release-candidate gate.

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
- Timeline rows use `LazyVStack`; room rows use native `List`.
- Media upload and timeline pagination remain bounded by explicit user actions.

## Performance Backlog

- Capture Instruments Time Profiler traces for cold launch, warm resume, room-list first render, timeline first render, timeline scroll, message send, media upload, and sync.
- Add measured launch and warm-resume baselines once release signing and device targets are available.
- Add memory graph snapshots for large timeline scrolling and media viewer presentation.
- Track any P0/P1 regressions before TestFlight; no current fixture-blocking issue is known.
