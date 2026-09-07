# Desktop window movement and client depth

## Intended operating path

- Goal: move the macOS desktop window by dragging its visible title strip.
- Actor: a user operating a non-fullscreen Synara window.
- Start: a fresh isolated QA window, with no account or session loaded.
- First action: press the primary mouse button in the top strip, move, release.
- Owner route: desktop React title strip → Tauri drag-region handler → main-window capability → native window manager.
- Transitions: resting window → pointer press → native drag → relocated resting window.
- Side effects: window position changes; no Matrix messages, account access, or settings writes.
- Authority: main privileged webview is explicitly allowed to start its native drag; native traffic lights remain owned by macOS.
- Completion: the window visibly moves and its native position changes.
- Readback: native accessibility window position and dimensions, plus before/after screenshots.
- Acceptance: one direct drag changes position without changing dimensions; normal content controls remain clickable; a visible strip remains across routes.
- Disqualifiers: resizing instead of moving, using an API to set position, dragging another surface to compensate, or touching a personal account.

## Diagnosis and repair

The existing macOS window used overlay native chrome while the persistent React
strip was Linux-only. Individual header backgrounds and a small sidebar spacer
were the remaining affordances. More significantly, the main capability granted
`core:default`, whose pinned Tauri window default permits window readback and
internal double-click maximize but does **not** permit `start_dragging`. Tauri's
own drag-region script invokes that command for a primary drag.

The earliest divergence is the missing authorization at the desktop capability
boundary, compounded by a missing persistent macOS affordance. The repair grants
only `core:window:allow-start-dragging`, extends the strip to macOS with native
traffic-light clearance, and removes the obsolete sidebar spacer. Tauri already
owns double-click handling; the strip no longer duplicates that operation with a
React handler. Linux retains its explicit minimize/maximize/close buttons.

Settings sequence cards now use the shared restrained tonal fold and inner edge,
and navigation uses the existing hover/selected treatment. Increased Contrast
removes decorative shadows/gradients and supplies a visible boundary. iOS room
rows get a separate collection depth role: no drop shadows, faint edges, and the
existing strong semantic boundary in Increased Contrast. Other surfaces retain
their current depth hierarchy.

## Validation

The component smoke rendered the changed `DesktopTitleBar`, shared Settings
sequence-card class, navigation class, and switch through Vite in Chromium.
The macOS strip measured 1280 × 40 at origin (0, 0). The switch changed from
unchecked to checked after a normal click. Light, dark, and Increased Contrast
screenshots were inspected; no account/session initialization was included.
This confirms rendered affordances and independent control interaction, not
native window movement or full authenticated Settings navigation.

- Frontend modernization typecheck passed.
- Nine existing desktop titlebar/depth checks passed after updating the old
  Linux-only contract.
- ESLint passed for changed TypeScript files.
- Swift syntax parsing passed; native theme tests are updated but not executed.
- `git diff --check` passed.

The native QA build used shipping titlebar implementation and capability with
only an isolated app identifier and dev URL. It was stopped with SIGINT when
available disk reached 1.3 GiB, before completing the app/link. No compiler error
was reported before cancellation. No personal desktop session was opened.
The iOS simulator build/visual run was deferred for the same capacity limit.

**Runtime window movement verdict: Not confirmed (environment capacity).**
The source defect is established, and the reported user route is Failed, but
native position/dimension readback remains required after a successful build.
No claim of macOS/Linux native drag success or iOS visual acceptance is made
from the browser render or source checks. The separate collection role preserves
the existing increased-contrast boundary, but device appearance remains for
visual review.
