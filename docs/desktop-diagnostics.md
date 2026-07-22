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

## Privacy and retention

The structured writer accepts only predefined event categories, event-name namespaces, and typed fields. Reports exclude message bodies, tokens, Matrix user/room/event identifiers, homeserver URLs, attachment contents and names, and exception messages. Room and event correlation uses temporary per-run aliases.

Diagnostic files are owner-readable and owner-writable on Unix platforms (`0600`), expire after seven days, and are bounded to a 5 MiB current file plus one 5 MiB rotation. Export reads only the structured diagnostic store; the general application log is not included.

Performance capture uses frame callbacks and the browser long-task observer even when the visual overlay is hidden. Room-activity records are burst-limited and include coalescing counts so troubleshooting does not create unbounded native writes during large syncs.
