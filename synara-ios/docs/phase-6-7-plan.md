# Phase 6.7: Functionality And Mockup Fidelity Validation

Reviewed: 2026-05-28

## Execution Status

Status: simulator-completable Phase 6.7 work is complete for local
device-readiness. Remaining items are product-review or future feature work:
true Matrix threads, production media/search depth, and final visual tuning on a
physical iPhone.

Completed:

- Functionality matrix covers current iOS surfaces and desktop-parity gaps.
- Visual fidelity matrix covers rooms, timeline, composer, attachment sheet,
  thread, and agent approval screenshots.
- Deterministic screenshot tests cover rooms/timeline/composer/attachment,
  thread, and agent approval.
- Live visual smoke remains available for real Matrix data.
- Room list now uses Favorites/Other grouping for mockup parity.
- Timeline pagination is visually subdued.
- Non-photo attachment actions now show an honest unavailable state instead of
  fake upload behavior.

Release-prep accepted:

- Generated mockup chrome is not treated as a source-of-truth for exact system
  status bar, hardware frame, or keyboard rendering.
- Thread UI is reply-backed until true Matrix `m.thread` implementation is
  scoped.
- Settings/login/later/notifications retain their current modernized Synara UI
  until product review asks for dedicated mockup tuning.

## Goal

Make iOS validation honest and repeatable. Phase 6.7 must prove two things for
each supported surface:

1. The function exists and works in the app.
2. The live or fixture UI matches the approved mockups closely enough to ship.

## Scope

Phase 6.7 covers:

- Rooms/channels list.
- Room timeline.
- Composer typing state.
- Attachment sheet.
- Thread view and thread typing.
- Agent approval room/card.
- Login, settings, Later, notifications, and room-management screens as
  secondary fidelity targets.

## Implementation Tasks

### 6.7.1 Functionality Matrix

Requirements:

- Maintain `ios-functionality-matrix.md`.
- Track each capability by status, validation mode, and next required evidence.
- Do not mark a feature complete when it only has a visible placeholder.

Acceptance criteria:

- Matrix includes all current iOS app surfaces.
- Matrix includes desktop-parity gaps.
- Each row has at least one current or required validation path.

### 6.7.2 Visual Fidelity Matrix

Requirements:

- Maintain `ios-visual-fidelity-matrix.md`.
- Map each mockup to named simulator screenshots.
- Keep explicit mismatch notes.

Acceptance criteria:

- Rooms/channels, timeline, composer, attachment sheet, and thread screens have
  named screenshot outputs.
- Gaps are concrete enough to drive implementation tasks.

### 6.7.3 Screenshot Automation

Requirements:

- Keep deterministic screenshot tests for mock fixture screens.
- Keep gated live screenshot tests for live Matrix data.
- Never commit credentials or generated screenshots unless intentionally
  adding curated reference images.

Acceptance criteria:

- `testLiveVisualMockupScreenshotsWhenConfigured` captures live room list,
  timeline, composer typing, and attachment sheet.
- `testMockRoomsVisualScreenshotsWhenConfigured` captures deterministic room
  list, timeline, composer typing, and attachment sheet screenshots.
- `testMockThreadVisualScreenshotWhenConfigured` captures thread view and
  thread typing.
- Screenshot tests skip by default unless gated environment variables are set.

### 6.7.4 Pixel-Fidelity Remediation

Requirements:

- Build fixture data matching the supplied mockups.
- Tune SwiftUI components against target screenshots.
- Avoid claiming pixel-perfect while live data diverges from mockup data.

Acceptance criteria:

- Deterministic screenshots resemble the supplied mockups without relying on a
  particular homeserver state.
- Live screenshots prove the same components survive real Matrix content.
- Product-reviewed delta notes are recorded after each pass.

### 6.7.5 Missing Functionality Remediation

Requirements:

- Promote placeholder affordances to real implementations or disable them.
- Prioritize high-frequency daily messaging workflows before platform extras.

Acceptance criteria:

- Attachment actions either work or communicate unavailable state.
- Thread UI either uses true Matrix threads or is documented as reply-backed.
- Media/search/composer gaps are ordered for Phase 7+ parity work.

## Desktop Parity Gap Backlog

Priority order:

1. Production E2EE completion: recovery, verification, cross-signing, key
   backup, encrypted media.
2. Message search and global search.
3. Rich composer: mentions, emoji/custom emoji/sticker/GIF, formatting.
4. True Matrix threads.
5. Media: thumbnails, image/video/audio viewer, file share/download, upload
   progress/retry/cancel.
6. Room/member management depth: member list, kick/ban, power-level editing,
   avatars, aliases, spaces/lobby depth.
7. Real APNs device push validation and notification center data.
8. User profile, device/session list, verification UX.
9. iPad layout, share extension, App Intents/Shortcuts.

## Exit Criteria

Phase 6.7 is complete when:

- The functionality matrix and visual matrix are current.
- Gated live visual smoke passes.
- Deterministic visual smoke passes.
- Known visual mismatches are either fixed or accepted as follow-up items.
- No feature is represented as complete without validation evidence.
