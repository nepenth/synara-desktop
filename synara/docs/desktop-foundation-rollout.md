# Desktop foundation rollout controls

The read-marker, room-activity, bounded-context, and scroll-anchor changes are enabled by default. Each control is read at client startup. Change a control only as a temporary incident rollback, then reload the client; removing the override restores the refactored behavior.

| Feature                   | Build variable                                  | Runtime localStorage key                 | Disabled behavior                                                                                                                                           |
| ------------------------- | ----------------------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Exact read markers        | `VITE_SYNARA_FEATURE_EXACT_READ_MARKERS`        | `synara.feature.exactReadMarkers`        | Uses the SDK `sendReadReceipt` path. Cross-client fully-read marker consistency may regress.                                                                |
| Reactive room activity    | `VITE_SYNARA_FEATURE_REACTIVE_ROOM_ACTIVITY`    | `synara.feature.reactiveRoomActivity`    | Uses room summary timestamps captured when the room-list identity changes. Live Recent changes may require navigation or restart.                           |
| Bounded timeline contexts | `VITE_SYNARA_FEATURE_BOUNDED_TIMELINE_CONTEXTS` | `synara.feature.boundedTimelineContexts` | Stops trimming context as pagination expands. Large-room memory and render cost can grow.                                                                   |
| Stable scroll anchoring   | `VITE_SYNARA_FEATURE_STABLE_SCROLL_ANCHORING`   | `synara.feature.stableScrollAnchoring`   | Disables measured event/pixel anchor capture and restoration. Native/index scrolling remains, but pagination and late layout changes may move the viewport. |

Build values and runtime values accept only the literal strings `true` and `false`. A runtime value has precedence over a build value. Invalid or absent values are ignored, and the default is enabled.

For an emergency runtime disable, set the relevant key to `false` in the client developer console and reload. Remove the key and reload to re-enable the default. Build variables are intended for a controlled rollback build.

## Privacy-safe diagnostics

Timeline, read-marker, and room-activity lifecycle events are recorded in the existing 50-entry desktop diagnostic buffer. Events use a versioned JSON envelope capped at 220 characters. Room and event IDs are replaced by bounded, session-local tokens such as `room-3` and `event-7`; tokens are not persisted as an ID mapping. Only allowlisted numeric, boolean, enum-like, and range fields are accepted. Message bodies, arbitrary content fields, URLs, credentials, and raw Matrix identifiers are not admitted.

Use **Settings → General → Copy diagnostics** when collecting an incident report. Correlation tokens are meaningful only within the current client session and deliberately cannot identify a room or event outside it.
