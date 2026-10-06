# Session recovery investigation — 2026-10-06

Branch: `feature/2026-10-06-session-recovery`, based on main `92d90577`.
Dependency PR reviewed and incorporated: [#1180](https://github.com/nepenth/synara-desktop/pull/1180).

## Findings and evidence

The outage has two distinct client failure paths: stale refresh credentials after previous successful rotations, followed by an unbounded recovery loop that presents authentication failure as server unavailability.

The server history supplied by the operator records the following sequence for the failing source. Source addresses, account/device identifiers, tokens and event payloads are intentionally omitted.

| Event | UTC, 2026-10-05 | EDT | Evidence |
| --- | --- | --- | --- |
| Last successful refresh | 04:37:16 | 00:37:16 | Last of 80 refresh HTTP 200s, starting Oct 4 at 21:54:39 UTC, approximately five minutes apart |
| First rejected refresh | 10:49:47 | 06:49:47 | HTTP 401, missing refresh-token row; 102 failures in the first minute |
| Homeserver shutdown | 10:53:10 | 06:53:10 | After the failures began; 1,263 refresh 401s had already accumulated |

The supplied history reports no logout, device deletion, password change, account deactivation, new login, administrative token operation or expiry error from Oct 3 through this shutdown. Another source refreshed successfully without the predecessor-replay 403s that would indicate shared-token consumption.

[Synapse 1.162.0's authentication handler](https://github.com/element-hq/synapse/blob/v1.162.0/synapse/handlers/auth.py) distinguishes a missing refresh-token row (401) from a consumed predecessor or expired token (403). Its [refresh-token replacement implementation](https://github.com/element-hq/synapse/blob/v1.162.0/synapse/storage/databases/main/registration.py) deletes the predecessor and retains the two most recent chain entries. With the reported absence of other deletion paths, this sequence strongly supports restoration/reuse of a token older than the retained refresh chain. The original device chain was not independently inspected here; source address and user agent alone do not uniquely identify a session.

Local readback adds the following evidence:

- Both installed clients were release 2.1.45. `Synara-Desktop/0.1.0` comes from the Core crate version in the HTTP user agent and does not establish an obsolete desktop release.
- Anonymous client versions requests succeeded on both machines during diagnosis.
- The Linux native credential entry's modification time was Oct 2 at 14:06:04 UTC. Its stored access token was rejected by one read-only `whoami` request with `M_UNKNOWN_TOKEN`. No diagnostic refresh request consumed that credential.
- A bounded pause/resume of the Linux process stopped and restarted its continuous encrypted network traffic. HTTP statuses were not visible through TLS in this experiment. That verified process was then terminated to remove its contribution to the load; stored credentials and crypto data were left intact.
- The Mac credential entry had already been replaced, so its original persisted token could not be recovered for comparison. A current directory-fsync preflight succeeded; this does not prove the earlier filesystem/keychain was writable.
- Optional diagnostics did not retain the native rotation-save error. Historical keychain failure, locator failure, stale OS cache, multiple processes or an older callback implementation remain unproven explanations for the original stale credential.

## Earliest divergences

1. **Persistence boundary:** matrix-sdk 0.19.1 updates in-memory tokens before calling the product save callback. A callback error is logged by the SDK but refresh still returns success. A mock regression reproduces fresh in-memory tokens alongside old durable credentials. The previous product had no pending-save state or always-available evidence of that failure.
2. **Recovery boundary:** SDK offline mode hides the typed sync error and probes client versions with authenticated request configuration. A dead refresh token cannot make that probe succeed; the SDK's loop waits only 100 ms between failures. Repeating it cannot repair the session.

The first mechanism is reproduced, but its occurrence in the original incident is **not confirmed**. The second mechanism is confirmed from the pinned SDK implementation and matches the reported refresh/versions storm.

## Intended owner routes

For a rejected installed session: the native SDK produces a typed rejection, Core's sync owner latches a fixed authentication diagnostic for that generation, the desktop health watcher retires that same generation under the auth gate, and normal renderer session polling returns the user to sign-in with an expiry explanation. Retirement clears invalid session credentials and the active identity; it preserves the encrypted SQLite stores and their keys. It does not send another remote logout/refresh request using rejected credentials.

For refresh rotation: the SDK's synchronous callback saves current credentials through the existing persistence lease and durable identity locator. A failed preflight or save marks that lease pending. The health watcher attempts a local save of the latest in-memory credentials every five seconds, without another HTTP refresh. A failed save stays pending; a successful save clears it. Retirement fences both callback and catch-up writes so an old generation cannot overwrite a new session.

Actors are the installed native session owners; starting states are either an authenticated session encountering a terminal rejection or a successful SDK token rotation. Completion/readback is the closed native sync diagnostic plus logged-out snapshot for rejection, or successful vault write plus cleared pending flag for rotation. Only generation and save outcome cross to the renderer. Disqualifying deviations include retrying a rejected token, deleting crypto data, exporting secrets, or allowing a retired generation to save/log out a later one.

## Implementation

- Disable SDK offline mode. Native recovery retains typed errors and retries transient sync failures after five seconds, with explicit start/stop intent serialized against automatic recovery. No periodic authenticated versions probe remains.
- Classify refresh `M_UNKNOWN_TOKEN`, `M_FORBIDDEN` and missing refresh credentials as terminal; preserve transient network/server errors. An ordinary room permission denial is not classified as session rejection.
- Make the informational capability probe explicitly anonymous.
- Carry a second closed failure diagnostic through Core/platform/desktop status contracts without accepting arbitrary SDK text.
- Retire rejected desktop credentials through the existing logout coordinator with a generation check; preserve crypto storage. Existing voluntary logout behavior remains available.
- Track and catch up failed session saves. Surface a fixed warning while storage remains unavailable and an expiry explanation after session retirement.
- Always record rotation outcomes and allowlisted filesystem/keyring causes in `logs/matrix-session-lifecycle.log`, rotating at 64 KiB with one retained predecessor. No tokens, account IDs, paths or raw SDK errors are logged.

A process crash while native credential storage remains unwritable can still lose the latest in-memory token. The warning and pending-write recovery address the live case; a rejected restored token requires a fresh login. No insecure token copy or server token replay is used to bridge that limit.

## Validation

Regression coverage exercises the pinned SDK and real Core sync owner: rejected refreshes through room/encryption error wrappers; absence of anonymous-probe Authorization; no requests after the recovery interval or stop/start/wake of a rejected owner; delayed transient recovery and explicit-stop cancellation; successful rotation/restoration; and the SDK save-failure behavior. Native tests cover pending-save state, retirement fencing, bounded/private logs and existing logout/crypto continuity. Renderer coverage verifies expiry notice storage and unavailable-storage behavior.

Local validation passed 1,081 Core library tests (three existing ignored tests), 476 desktop library tests serially, and 1,209 renderer tests; renderer typechecks, lint and production build also passed. A parallel desktop run exposed an existing shared-state dropped-file grant collision; serial execution passed. Final Clippy and CI status are recorded in the feature PR. Live validation of the new signed Mac/Linux product and an OS credential-store lock/unlock is still required before claiming the original production path repaired end to end. Production clients were not replaced by this branch work.
