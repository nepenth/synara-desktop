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
| Rooms/channels list | `01-mock-room-list.png` from deterministic mock smoke; `01-live-room-list.png` from gated live visual smoke | Partial | Mock fixture provides stable rooms, but app still uses Rooms/Direct messages grouping instead of the supplied Favorites/Other grouping; row height/icon treatment is close but not pixel-close; bottom tab has current app style rather than mockup exact styling. |
| Room timeline | `02-mock-room-timeline.png` from deterministic mock smoke; `02-live-room-timeline.png` from gated live visual smoke | Partial | Mock fixture now gives stable Product-room content; text scale/line wrapping still differ from mockup; Load Older pill appears at top; unread divider is not yet represented in the deterministic fixture. |
| Composer typing | `03-mock-composer-typing.png` from deterministic mock smoke; `03-live-composer-typing.png` from gated live visual smoke | Partial | Composer shape is closer, but vertical keyboard transition and timeline occlusion need tuning; mockup uses compact row with consistent bottom spacing. |
| Attachment sheet | `04-mock-attachment-sheet.png` from deterministic mock smoke; `04-live-attachment-sheet.png` from gated live visual smoke | Partial | Sheet labels and grid now fit, but sheet height, top offset, corner radius, and background dimming still need pixel tuning against the attachment mockup. |
| Thread view | `05-mock-thread.png`, `06-mock-thread-typing.png` from gated mock visual smoke | Partial | Thread data is reply-backed fixture, not real `m.thread`; row spacing, avatar style, reactions, and keyboard composition differ from mockup. |
| Agent approval | Existing agent visual smoke/manual screenshot path | Partial | Dark agent room direction exists, but exact mockup card density, status badge, action layout, and preview panel need a screenshot baseline. |
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

## Phase 6.7 Remediation Order

1. Build deterministic visual fixtures for the exact Rooms, Product timeline,
   attachment sheet, and Thread mockups instead of relying on live room content.
2. Tune layout metrics: margins, row heights, section spacing, avatar size,
   icon scale, composer height, sheet detent, and tab bar shape.
3. Add screenshot assertions that all required visible labels/controls exist.
4. Add manual visual review notes with before/after screenshots.
5. Promote screens from `Partial` to `Complete` only after the delta is small
   enough for product acceptance.
