# Synara Performance Notes

This branch treats room-timeline smoothness as a first-class rendering concern.
The desktop app hosts the same web timeline in a Tauri webview, so scroll
fluidity starts with the web rendering pipeline.

## Timeline Strategy

- Use TanStack variable-height virtualization for the main room timeline.
- Keep timeline DOM bounded to the viewport plus overscan rather than rendering
  every loaded Matrix event.
- Preserve a visible event-ID anchor through pagination and dynamic height
  changes.
- Disable browser scroll anchoring inside the timeline where Synara owns scroll
  restoration.
- Batch scroll-anchor updates through animation frames.
- Use layout/style containment where safe. Do not use paint containment on
  interactive message rows because it can clip hover action UI.
- Avoid mounting expensive Hermes code bodies while their details section is
  collapsed.
- Defer Prism syntax highlighting until the browser is idle, and skip Prism for
  very large blocks.

## Performance Debugging

Set this in DevTools to enable focused timeline logs:

```js
localStorage.setItem('synara.performance.debug', 'true');
```

Then reload the app. The client will emit `[synara:perf]` console entries for:

- timeline range and virtual row changes
- slow per-event render passes
- virtual timeline scroll restoration
- dynamic resize compensation

The same flag enables a small in-app overlay with:

- requestAnimationFrame cadence
- long-task count
- rendered timeline row count
- JS heap usage where the browser exposes it

Disable it with:

```js
localStorage.removeItem('synara.performance.debug');
```

## Native Timeline Scrolling

The native presenter (`NativeTimelinePresenter`) owns scroll restoration, so
browser overflow anchoring is disabled on virtual rows. Smoothness work in this
layer is aimed at two hot paths: backward pagination in long rooms, and live
appends while the user is either following the tail or parked in history.

Current tactics:

- Kind-aware `estimateSize` plus a keyed measured-height cache (room, row key,
  and a size identity covering grouping, body, media, and reactions) so TanStack
  does not reuse a stale height after an edit or fall back to a constant 64px
  after `rows` array replacement.
- Intrinsic media boxes from Matrix `info.w/h` so image/video loads do not
  reflow already-painted rows.
- Same-frame coalescing of native view deltas in revision order (stale revisions
  skipped, true gaps fail closed while keeping a successful prefix), and
  metadata-only batches that keep the existing `rows` array identity.
- Follow-live sticks with `scrollTop = scrollHeight - clientHeight` in the same
  layout pass that grows the spacer, instead of a later `scrollToIndex(end)`.
  A small move away from an already-stuck tail during the programmatic lock
  (focus/`scrollIntoView`) releases follow-live; in-flight viewport saves are
  skipped so prepend anchoring is not overwritten.
- Bottom observation and live-read/follow-live attempts are rAF-batched; the
  presenter does not attach extra subtree MutationObservers while scrolling.

The Playwright native-timeline harness records dropped frames (>2 vsync) during
scripted wheel scrolling with mocked live appends (~300ms) and a backward
prepend. Run it with:

```sh
npm run test:browser:native-timeline
```

## Desktop Notes

Synara Desktop uses the platform webview. macOS WebKit already handles compositing
and GPU acceleration where applicable, so shell-level changes are secondary to
reducing DOM layout, paint, and JavaScript work in the web timeline. Native work
should focus on avoiding heavy transparent/vibrancy surfaces behind the webview,
keeping the visible window unthrottled, and profiling with Safari Web Inspector.
The modernization branch intentionally avoids private WebKit refresh-rate
toggles; smoothness work should remain measurable, cross-platform, and rooted in
bounded rendering.

## Automated Harness

Run the focused performance harness with:

```sh
npm run test:timeline-performance
```

It generates 10k and 50k synthetic timeline row sets and validates that row key
and index maps stay within the current budget. This is not a replacement for
Safari/Chrome profiling in a real room, but it catches regressions in the row
mapping layer that feeds virtualization.
