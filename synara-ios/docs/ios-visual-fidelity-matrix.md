# iOS Visual Fidelity Matrix

Reviewed: 2026-05-28

Purpose: make mockup matching explicit and testable. Phase 6.6 established the
native visual direction. Phase 6.7 changes the bar: each target screen needs a
captured simulator screenshot, an accepted delta, and follow-up implementation
items for visible mismatches.

## Target Environment

- Simulator: iPhone 17 Pro, iOS 26.5 unless otherwise noted.
- Orientation: portrait.
- Text size: default Dynamic Type.
- Appearance: light mode for rooms/chat/attachment/thread mockups; dark mode for
  agent approval mockup.
- Live account: disposable test account only.
- Screenshot output: `/private/tmp/synara-ui-validation`.

## Current Mockup Targets

| Mockup | Current screenshot source | Current status | Visible gaps |
| --- | --- | --- | --- |
| Rooms/channels list | `01-mock-room-list.png` from deterministic mock smoke; `01-live-room-list.png` from gated live visual smoke | Release-prep accepted | Mock fixture now uses Favorites/Other grouping; row height/icon treatment and bottom tab glass still intentionally follow current Synara components rather than exact generated-device chrome. |
| Room timeline | `02-mock-room-timeline.png` from deterministic mock smoke; `02-live-room-timeline.png` from gated live visual smoke | Release-prep accepted | Mock fixture gives stable Product-room content; pagination is now visually subdued. Remaining differences are sender/avatar styling, no unread divider fixture, and live-data variance. |
| Composer typing | `03-mock-composer-typing.png` from deterministic mock smoke; `03-live-composer-typing.png` from gated live visual smoke | Release-prep accepted | Composer shape and controls match the direction; exact keyboard transition spacing remains a manual device-review item. |
| Attachment sheet | `04-mock-attachment-sheet.png` from deterministic mock smoke; `04-live-attachment-sheet.png` from gated live visual smoke | Release-prep accepted | Sheet layout matches the supplied grid direction. Non-photo options now show an honest unavailable state instead of fake uploads. |
| Thread view | `05-mock-thread.png`, `06-mock-thread-typing.png` from gated mock visual smoke | Release-prep accepted | Thread UI is reply-backed, not true Matrix `m.thread`; exact sender spacing/reactions remain future Matrix-thread work. |
| Agent approval | `07-mock-agent-approval.png` from deterministic mock smoke; live agent approval smoke path | Release-prep accepted | Dark agent approval baseline exists. Card density and generated mockup chrome are close enough for device testing; exact final polish follows product review. |
| Settings/login/later/notifications | Earlier generated mockups, no canonical fidelity matrix yet | Partial | These screens have modernized UI but lack strict target screenshots and reviewed deltas. |

## Phase 6.7 Acceptance Criteria

For each target screen:

1. A deterministic fixture screenshot exists for pixel comparison.
2. A live screenshot exists when live Matrix data changes the UI meaningfully.
3. The screenshot is named and stored outside the repo by default.
4. The mismatch list is updated after every tuning pass.
5. Any nonfunctional mockup affordance is either implemented or visibly disabled
   with a product decision recorded.

## Test Commands

Live visual smoke:

```sh
TEST_RUNNER_SYNARA_LIVE_VISUAL_SMOKE=1 \
TEST_RUNNER_SYNARA_LIVE_HOMESERVER=<homeserver> \
TEST_RUNNER_SYNARA_LIVE_USERNAME=<username> \
TEST_RUNNER_SYNARA_LIVE_PASSWORD=<password> \
TEST_RUNNER_SYNARA_LIVE_ROOM_ID=<room-id> \
TEST_RUNNER_SYNARA_SCREENSHOT_DIR=/private/tmp/synara-ui-validation \
xcodebuild -project Synara.xcodeproj -scheme Synara \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  -only-testing:SynaraUITests/SynaraUITests/testLiveVisualMockupScreenshotsWhenConfigured test
```

Thread visual smoke:

```sh
TEST_RUNNER_SYNARA_MOCK_THREAD_VISUAL_SMOKE=1 \
TEST_RUNNER_SYNARA_SCREENSHOT_DIR=/private/tmp/synara-ui-validation \
xcodebuild -project Synara.xcodeproj -scheme Synara \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  -only-testing:SynaraUITests/SynaraUITests/testMockThreadVisualScreenshotWhenConfigured test
```

Rooms visual smoke:

```sh
TEST_RUNNER_SYNARA_MOCK_ROOMS_VISUAL_SMOKE=1 \
TEST_RUNNER_SYNARA_SCREENSHOT_DIR=/private/tmp/synara-ui-validation \
xcodebuild -project Synara.xcodeproj -scheme Synara \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  -only-testing:SynaraUITests/SynaraUITests/testMockRoomsVisualScreenshotsWhenConfigured test
```

Agent visual smoke:

```sh
TEST_RUNNER_SYNARA_MOCK_AGENT_VISUAL_SMOKE=1 \
TEST_RUNNER_SYNARA_SCREENSHOT_DIR=/private/tmp/synara-ui-validation \
xcodebuild -project Synara.xcodeproj -scheme Synara \
  -destination 'platform=iOS Simulator,name=iPhone 17 Pro' \
  -only-testing:SynaraUITests/SynaraUITests/testMockAgentVisualScreenshotWhenConfigured test
```

## Phase 6.7 Remediation Order

1. Build deterministic visual fixtures for the exact Rooms, Product timeline,
   attachment sheet, and Thread mockups instead of relying on live room content.
2. Tune layout metrics: margins, row heights, section spacing, avatar size,
   icon scale, composer height, sheet detent, and tab bar shape.
3. Add screenshot assertions that all required visible labels/controls exist.
4. Add manual visual review notes with before/after screenshots.
5. Promote screens from `Partial` to `Release-prep accepted` only after the
   delta is small enough for local device testing; promote to `Complete` only
   after physical-device product review.
