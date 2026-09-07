# Desktop window movement and client depth

## Final implementation

macOS again uses its native titlebar and traffic lights. Window movement belongs
to the operating system and no longer depends on a WebView drag handler. The
obsolete sidebar traffic-light spacer is removed. Linux retains its persistent
custom strip and explicit minimize/maximize/close controls. Its main-webview
capability now grants `core:window:allow-start-dragging`, which is absent from
Tauri's default window permissions. Tauri owns strip double-click handling;
React no longer duplicates that operation.

The Linux maximize/restore label and icon now follow the native window state.
A resize-event subscription is installed before the initial `isMaximized` read,
so Tauri-owned double-clicks and window-manager maximize changes refresh the
control. Button completion also requests current state instead of applying a
possibly stale command result. Newer queries supersede older results, and
unmount disposes both active and asynchronously registered listeners.

The Linux strip remains available while client configuration loads or fails.
SplashScreen consumes the remaining flex height, with a zero minimum and
vertical overflow for oversized content, so the strip cannot clip its footer
or recovery controls. macOS does not render a second custom strip.

Settings sequence cards use the shared restrained tonal fold and inner edge;
navigation uses the existing hover/selected treatment. Increased Contrast
removes decorative shadows/gradients and supplies a visible boundary. iOS room
rows use a separate collection depth role: no drop shadow, faint edges, and the
existing strong semantic boundary in Increased Contrast. Other surfaces retain
their existing depth hierarchy.

## Native movement operating path

- Goal: move the macOS desktop window using its visible native titlebar.
- Actor: a user operating a non-fullscreen Synara window.
- Start: an isolated QA window, with no account or session loaded.
- First action: press the primary pointer button in the native titlebar, move, release.
- Owner route: native titlebar → macOS window manager.
- Transitions: resting window → pointer press → native drag → relocated window.
- Side effects: window position changes; no Matrix/account/settings writes.
- Authority: macOS owns pointer handling and window movement; no programmatic position setter is a substitute for this route.
- Completion: native position changes while dimensions remain constant.
- Readback: native window geometry through Tauri's read-only window API and the operating system's window list.
- Acceptance: one direct drag relocates the window; ordinary content controls remain clickable.
- Disqualifiers: resizing instead of moving, using a position setter, manipulating a personal account, or bypassing input permissions.

The original implementation replaced native macOS chrome with an overlay and
relied on header drag regions without granting the required drag command.
Restoring the native owner removes that dependency. The Linux capability repair
is independently required for its borderless window.

## Validation and limits

The isolated native QA shell compiled successfully with the shipping window
configuration/capability, changing only its app identifier and development URL.
The fixture rendered production titlebar/Settings components without starting an
account session. macOS displayed its native title and traffic lights, with no
extra custom strip. A Settings switch changed from off to on through a normal
click. Native zoom/restore changed geometry from (400,92,2560,1800) to
(0,60,3360,1924), then back to the original values (physical pixels), confirming
live native control and geometry readback.

**Native drag verdict: Not confirmed (input automation boundary).** CUA's drag
sequence did not relocate the QA window, including with the restored native
bar. A separate bounded HID diagnostic checked `CGPreflightPostEventAccess`,
which returned false; it exited before posting any input. No permission prompt,
bypass, or programmatic movement was used. This evidence does not prove a
remaining product drag defect, and does not establish successful movement.
Physical-pointer confirmation on macOS and native Linux drag testing remain.
The user's originally reported path is Failed; do not relabel it based on source
or visual inspection alone.

A Chromium component run at 1280 × 720 exercised the Linux strip plus production
configuration loading/error components and a recovery fixture using the shared
SplashScreen. Each splash measured y=40, height=680, bottom=720; each footer
measured y=658, height=62, bottom=720. Retry, Continue, and recovery controls were
clickable. This confirms layout and controls, not a live account recovery.

Light/dark Settings screenshots were inspected after the theme changed and
140 ms transitions settled (250 ms capture delay). Dark selected navigation
foreground was rgb(177,178,179) on rgb(39,40,42), with independent switch
interaction. Increased Contrast rendering was also inspected.

- Frontend modernization typecheck and ESLint passed for changed source.
- Nine existing titlebar/depth checks passed with the final native macOS contract.
- Signed iOS theme tests: 27 passed, zero failures, including collection and accessibility metrics.
- Existing signed simulator mock-room visual smoke passed in Light and Dark Modes; both room-list screenshots were inspected and shows restrained row edges without repeated drop shadows.
- Documentation hygiene and diff whitespace checks passed.
- Follow-up maximize-state repair: full frontend TypeScript check and focused
  ESLint passed. Four behavioral observer tests passed for external maximize/
  restore, out-of-order readbacks, unmount during registration, and failed-query
  recovery; four existing titlebar checks also passed. These use a mocked native
  window interface and establish subscription/state behavior, not native drag
  or Linux window-manager execution.

The iOS visual fixture is mock data in the dedicated test simulator. It does
not establish live Matrix behavior, encryption, or the user's physical-device
appearance preference. No personal desktop account or unrelated simulator was
used.
