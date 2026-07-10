# Timeline Diagnostics

Timeline diagnostics are designed for large-room scroll and blank-view failures.
They contain random per-open trace IDs, counts, ranges, state names, and timings.
They must not contain room IDs, event IDs, user IDs, message text, or server URLs.

## Desktop

Privacy-safe timeline records are always written through the native
`desktop_append_log` command in desktop builds. To mirror them to the web
inspector with additional generic performance records, enable performance debug
and reload:

```js
localStorage.setItem("synara.performance.debug", "true");
location.reload();
```

Resolve the current native log path from the console when developer tools are
available:

```js
window.__SYNARA_DESKTOP__?.invoke("desktop_log_path").then(console.info);
```

For a macOS foreground capture, stdout can also be retained alongside the native
file:

```bash
"/Applications/Synara.app/Contents/MacOS/synara" 2>&1 \
  | tee "$HOME/Desktop/synara-timeline-$(date +%Y%m%d-%H%M%S).log"
```

Expected records include:

- `room-timeline.open`
- `room-timeline.render-window`
- `room-timeline.pagination-start` and `pagination-complete`
- `room-timeline.anchor-restored` or `anchor-restore-cancelled`
- `room-timeline.pagination-suppressed`
- `room-timeline.first-stable-bottom`
- `room-timeline.live-reset`
- `room-timeline.jump-latest`

## iOS

Use the macOS Console app for a connected device, or Xcode's device console, and
filter for:

```text
subsystem: com.whylandcreative.synara
category: timeline
```

Expected records include snapshot counts, stream lifecycle, pagination state,
bottom visibility, and exactly one `scroll-executed` record for each accepted
`scroll-requested` record. A `scroll-cancelled` record is expected only when a
new higher-priority request or user interaction supersedes it.

## Reproduction Record

For each failure, retain:

1. Platform, app version, and whether the app was upgraded or freshly launched.
2. Approximate loaded-room size and whether the room was fully read.
3. Actions from room selection until the failure, including any manual scroll.
4. Screen recording or screenshots with wall-clock time visible when practical.
5. Desktop native log or iOS Console export covering 30 seconds before and after
   the failure.

Do not add captured logs to the repository. Review them locally and redact them
before sharing outside the development team.
