# Desktop diagnostics

Synara's desktop diagnostics are an opt-in troubleshooting facility for macOS and Linux. They are disabled by default and store a bounded, privacy-filtered report locally. Nothing is uploaded automatically.

## Capture a reproduction

1. Open **Settings → Diagnostics**.
2. Enable **Diagnostic Capture**.
3. Enable the categories that match the problem:
   - **Performance** for frame cadence, long tasks, rendered timeline size, and slow operations.
   - **Session Persistence** for native credential-store, bootstrap, token refresh, Matrix store, crypto, and startup lifecycle evidence.
   - **Room State and Positioning** for room-open decisions, read markers, recent-room activity, pagination, anchoring, Jump to Latest, and unexpected scroll movement.
4. Optionally enable the performance overlay.
5. Reproduce the problem. Leave capture enabled across an app restart when investigating session restoration.
6. Return to **Settings → Diagnostics** and choose **Export report**.
7. Review the JSON report, then share it with the trusted developer or support contact investigating the issue.
8. Disable capture and use **Clear records** when the investigation is complete.

## Connection recovery

A signed-in desktop session is restored by `matrix_restore_session`; the native SyncService owns the connection. The renderer polls `matrix_sync_status` for readiness. Its optional `/versions` metadata request must not hold this path behind a separate “Connecting to server” screen or treat a metadata failure as proof that native sync is disconnected. Pre-login homeserver discovery still validates the server before authentication.

If startup reaches **Sync is taking longer than expected**, **Retry** invokes `matrix_sync_recover` through the existing native session owner, including idle, offline, failed and terminated states. The button remains disabled until the request settles. Recovery controls stay visible until native readiness is observed, even if the restart command has already returned. A rejected recovery stays visible and permits another explicit user attempt. Restore and startup errors remain in their error screen; retrying them does not sign out or delete the local encryption store.

The October 5, 2026 regressions in `synara/e2e/runtime-maturity.spec.ts` mount the production `ClientRoot` with a controlled native IPC boundary. They cover failed and hanging metadata requests, stopped sync recovery, repeated restore failures, startup hydration errors, and an established connection recovering after a network-return event. The original Retry path issued zero recovery commands, and the original metadata gate blocked a fixture whose native owner reported running; both failed the corresponding regression before repair. These tests confirm the renderer route and its readback, not authenticated SDK traffic or the cause of a historical incident. Capture **Session Persistence** across the failure and restart to distinguish those live cases.

## Privacy and retention

The structured writer accepts only predefined event categories, event-name namespaces, and typed fields. Reports exclude message bodies, tokens, Matrix user/room/event identifiers, homeserver URLs, attachment contents and names, and exception messages. Room and event correlation uses temporary per-run aliases.

Diagnostic files are owner-readable and owner-writable on Unix platforms (`0600`), expire after seven days, and are bounded to a 5 MiB current file plus one 5 MiB rotation. Export reads only the structured diagnostic store; the general application log is not included.

Performance capture uses frame callbacks and the browser long-task observer even when the visual overlay is hidden. Room-activity records are burst-limited and include coalescing counts so troubleshooting does not create unbounded native writes during large syncs.
