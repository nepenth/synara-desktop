# Desktop render efficiency plan

Status, 2026-11-17: stages 0–5 are implemented on `perf/linux-render-efficiency`, not yet
released or tried on hardware beyond a manual Balanced run on NVIDIA/Hyprland:

- Stage 0: `scripts/linux-thread-load.sh` and typing latency in the Performance diagnostics.
- Stage 1: Balanced mode, now the Arch package default (Off and Full remain), with an automatic
  fallback to Off when the web process crashes or a start dies before drawing, and a capped page
  reload when the web process ends without one (see `docs/linux.md`).
- Stage 2: one motion policy (`utils/motion.ts`): infinite animations pause in an unfocused window
  and step a few times a second in Off mode; the highlight pulse is finite.
- Stage 3: composer drafts are written after a 400 ms pause (2 s max wait) and at once on send,
  room switch, blur, hide or quit, never per keystroke.
- Stage 4: an inline thread no longer streams while the side pane shows it; read receipts survive a
  view closing mid-write and report accurate error codes.
- Stage 5: measured; parsing a 40-item message is ~1.5 ms. The real cost was every visible row
  re-rendering on each edit (unstable callbacks); now only the edited row does.

Goal: make Synara cheaper to draw and smoother to type in, above all on Linux, without
removing any feature. Every change below keeps the same behaviour or replaces an effect
with an equivalent one that costs less; the few visible changes are listed under
"Decisions" and need a yes first.

## What we measured

Synara 3.0.1 (`synara-desktop-bin`), Arch/Hyprland, RTX 4080 SUPER, 3440x1440 at 120 Hz,
package default "safe" rendering (`WEBKIT_DISABLE_DMABUF_RENDERER=1`,
`WEBKIT_DISABLE_COMPOSITING_MODE=1`). Sampled from `/proc` per thread.

| State                                                                                         | UI process main thread | WebKit web process main thread | Main-loop wakeups/s |
| --------------------------------------------------------------------------------------------- | ---------------------- | ------------------------------ | ------------------- |
| "Forge is typing…" shown, bot editing a long message every few seconds, window focused or not | 30–35% of a core       | 7–16%                          | ~290                |
| Same, later stretch (window state unknown)                                                    | 12%                    | 4–8%                           | ~290                |
| Typing indicator gone, no edits                                                               | 0%                     | 0–1%                           | ~40–60              |

The wakeups move only a few bytes each (eventfd/frame clock), so the load was repainting,
not data. Main-thread I/O, sockets and the Matrix sync were not involved. The main thread
burning that CPU is the same GTK thread that handles key events, so keystrokes queue
behind frames.

What drives it:

1. **Safe mode turns off two different things.** Disabling DMABUF is the usual NVIDIA
   fix. Disabling compositing is the expensive one: each animation frame is painted on the
   CPU and copied on the UI process main thread.
2. **Infinite CSS animations run at the display rate.** The typing dots
   (`TypingIndicator.css.ts`, `0.6s infinite alternate`) kept the window repainting at
   120 Hz for as long as an agent was "typing", which for an agent is its whole run.
   Others: `ConnectionBeam.css.ts` (sweep), the message highlight pulse in
   `components/message/layout/layout.css.ts`, and folds `Spinner` (~140 uses).
3. **Matrix typing notices are room-wide.** An agent working in a thread shows as typing
   in the main room the whole time.
4. **Agent edits re-render one long formatted message** (html-react-parser projection
   of the whole body) on every edit. Rows are memoised and deltas are already coalesced
   per frame (`nativeTimelineView.ts`), so this is a per-edit spike, not a loop.

Found by reading code (not measured):

5. **An inline-expanded thread stays live after the side pane closes.** "Reply in thread"
   is reached from an expanded inline thread (`NativeInlineThread.tsx`), whose expansion
   lives in the room presenter until collapse or a room switch. Its own thread timeline
   view keeps streaming and the row keeps growing in the main virtualised list while the
   same thread is open in the pane. This is the best code-level match for "stutter began
   after a thread session"; it is a suspect, not proven.
6. **Every keystroke in the room composer** writes the full draft to localStorage
   synchronously, serialises markdown and plain text, and sends `matrix_drafts_save` over
   IPC with the full Slate JSON, with no debounce (`RoomInput.tsx` `handleEditorChange`,
   `useSyncedRoomDraft.ts`). A fixed cost per key, but it is paid on the main threads that
   are already busy.
7. **Log noise:** `matrix_timeline_set_read_state … v-timeline-view-not-open` comes from a
   receipt in flight when the view closes (`crates/synara-core/src/app/timeline/live.rs`
   `set_read_state` re-looks-up the stream after the network call). Also, the bridge maps
   every `SdkInvariant` to that code (`src-tauri/src/bridge/timeline_set_read_state.rs`),
   so the label can be wrong. Harmless, but it hides real failures.

## Plan

Each stage is its own PR and is measured before and after with the harness from stage 0.

### Stage 0 — a repeatable measurement harness (first, small)

- Script `scripts/perf/linux-thread-load.sh <pid>`: per-thread CPU for the UI and web
  process main threads plus main-loop wakeups/s, every N seconds (what was used above).
- Frontend probe behind the existing diagnostics switch (`desktopDiagnosticsPerformance`):
  keystroke-to-next-paint latency in the composer (input `timeStamp` → next rAF) as
  p50/p95, and long-task count. It goes into the existing `runtime.sample` record, so no new
  overlay is needed.
- Fixed scenarios, run on the Linux box against the throwaway Synapse (never on the user's
  live session without asking): idle room; typing notice held on; bot editing a 40-item
  message every 2 s; inline thread expanded plus pane open; typing 300 characters at a
  steady rate. Each one in safe, balanced (stage 1) and GPU modes.

Exit: numbers for every scenario that later stages can be held against.

### Stage 1 — a "balanced" Linux rendering mode

- Add `RenderingMode::Balanced` in `src-tauri/src/desktop_rendering.rs`: set only
  `WEBKIT_DISABLE_DMABUF_RENDERER=1`, so WebKit keeps GPU compositing (with NVIDIA, also
  `__NV_DISABLE_EXPLICIT_SYNC=1` if it turns out to be needed).
- Test on the NVIDIA/Hyprland box and on KDE Plasma Wayland (the reason safe exists).
  If it is stable, the Arch wrapper default becomes `balanced`; safe stays as the fallback.
- Settings → Appearance → Window → Hardware acceleration becomes a three-way choice. Docs:
  update `docs/linux.md`.
- Optional, if it is cheap: if the web process crashes during startup in balanced or GPU mode,
  record it and use safe on the next start, with a notice.

Exit: animation frames no longer cost UI-main-thread CPU on the box; no blanking or
crashes after a day of use.

### Stage 2 — a motion budget for continuous animations

One policy for every infinite animation, set as data attributes on `<html>` by a small
hook:

- `data-window-active="false"` when the Tauri window is unfocused, minimised or occluded
  (`document.hidden` does not cover a visible unfocused window). CSS then sets
  `animation-play-state: paused` for the infinite animations. They resume on focus, so
  nothing is lost.
- `data-render-mode="safe"`: infinite animations switch to a low-rate form. Example: the
  typing dots advance with a JS class toggle a few times a second (or `steps()` if WebKit
  is confirmed to repaint only on steps), and the spinner rotates in discrete steps. Each
  thing still shows that it is busy.
- Highlight pulse: a finite number of iterations instead of `infinite`.
- One shared stylesheet (`app/styles/motion.css.ts`) that overrides the folds spinner
  and the three app animations, so new components inherit it.
- Keep the existing `prefers-reduced-motion` handling as the strongest setting.

Exit: with "X is typing" shown in safe mode, UI main thread ≤ 3% (from ~32%); in balanced
or GPU mode it is not measurable; when the window is unfocused it is 0%.

### Stage 3 — keystroke path

- Debounce the synced draft save (`matrix_drafts_save`) and the localStorage
  `saveRoomDraft` to a trailing ~400 ms. Flush immediately on send, room switch, composer
  unmount, window blur and `beforeunload`, so a draft is never lost.
- Compute markdown/plain-text only when it is needed (draft save, send), not on every
  change.
- Typing notices: confirm they are already throttled (one notice per ~4 s, a stop on idle) and
  fix them if they are not.

Exit: composer keystroke-to-paint p95 unchanged or better in every scenario, with no IPC
per keystroke.

### Stage 4 — thread views

- When "Reply in thread" opens the side pane for a thread that is expanded inline, pause
  the inline thread's live view while the pane shows the same thread, and resume it when
  the pane closes. The expansion and its cached replies stay as they are. Alternative:
  share one thread view between the pane and the inline row instead of opening two.
- Check that collapsing or scrolling an inline thread out of view closes its stream
  (the code says it does; the harness should confirm it).
- Core: in `set_read_state`, hold the result of the first lookup or treat
  "closed during the call" as success, since the receipt was sent. Serialise the
  pane's mark-read calls. In the bridge, map `SdkInvariant` codes one-to-one instead
  of collapsing them into `v-timeline-view-not-open`.

Exit: the inline-plus-pane scenario costs the same as the pane alone; no
`v-timeline-view-not-open` lines when closing a thread.

### Stage 5 — edits of long messages

- Measure the per-edit cost (parse, React commit, layout) of the 40-item scenario first.
- If it is significant: cache the html projection by `(eventId, content hash)`, and make sure
  an edit that does not change a row's height does not remeasure the virtualiser beyond
  that row. When several edits of the same event arrive in one frame, keep only the last
  (check `flushCoalescedBatches` already does).

Exit: an edit costs one row's work; no long tasks over 50 ms per edit.

## Decisions needed before building

1. Stage 1: make "balanced" the Arch package default once tested? (Recommended.)
2. Stage 2: in safe mode, typing dots and spinners would move in visible steps instead of
   smoothly. Acceptable, or should safe mode only pause them when unfocused?
3. Stage 4: pause the inline copy, or share one view between inline and pane?

## Not in scope

Transport or React-virtualisation rewrites: the 2026-11 profiling showed those are not hot.
Thread-scoped typing notices: the Matrix protocol has no such notice.
