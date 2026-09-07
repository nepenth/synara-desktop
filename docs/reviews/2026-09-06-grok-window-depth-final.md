# Grok4.6 High source review

I’ll review the maximize-state repair at this head: observer, titlebar wiring, tests, and whether `getCurrentWindow` / ACL are safe.**ACCEPT**

Head `9e69a7d8db9cd54e6bd4b58805d3aae09b4dcd08` keeps the previously accepted window/depth contract and correctly repairs the Linux maximize chrome-state gap. No P1 or new P2 in this delta.

## What the delta does

`observeNativeMaximizedState` is the single authority path for the Restore/Maximize label and icon:

- Subscribe to `onResized` first, then read `isMaximized`, so a native resize during init is not lost.
- Monotonic `revision` drops stale readbacks.
- `dispose` blocks `onChange`, stops an already-attached listener, and `stop()`s a listener that arrives after unmount.
- Failed `isMaximized` calls keep the last confirmed value; a later resize can recover.
- The button still calls `desktop_window_toggle_maximize`, then `refresh()` instead of trusting the command’s boolean.

That matches Tauri 2.11.2: `is_maximized` and event listen/unlisten are in `core:default`; no extra ACL is required. `getCurrentWindow()` runs only when the Linux strip is visible (`isSynaraDesktop() && isLinuxOS()`). Static `@tauri-apps/api` import matches existing desktop modules. macOS native titlebar is unchanged.

The four observer tests cover external maximize/restore, out-of-order readbacks, dispose during registration, and failed-query recovery. They prove subscription/state logic with a mock window, not live WM or drag.

## Findings

None.

## Proof limitations (not code defects)

- Native macOS titlebar **drag** is still **not confirmed**. That remains the documented input-automation boundary (`CGPreflightPostEventAccess` false / CUA did not move the window). Do not treat this source accept as drag proof, and do not treat it as a remaining product-drag failure.
- Observer tests do not execute Linux window-manager maximize or Tauri `WINDOW_RESIZED` on a real window.
- CI status is unchanged from the prior packet unless separately reported.
