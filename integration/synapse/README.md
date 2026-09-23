# Disposable Synapse integration harness

This local-only harness pins Synapse `1.161.0` and PostgreSQL `16.9`. It exists
for client integration and regression tests; it is not a production topology.
The HTTP listener binds to loopback, registration is intentionally open, and all
credentials and signing material are generated under ignored `runtime/` state.
The Synapse pin corresponds to the upstream
[`v1.161.0` release](https://github.com/element-hq/synapse/releases/tag/v1.161.0).
This stable baseline exercises the recent Sliding Sync, profile, media, and
room-state behavior before a separate room-version-12 pass on Synapse 1.162.

## Start and use

```bash
scripts/synapse-integration.sh up
scripts/synapse-integration.sh create-user
```

Create two accounts for read-marker propagation tests, then configure independent
desktop/iOS test clients with `http://127.0.0.1:8008`. Useful endpoints include:

- `/_matrix/client/versions` for readiness and supported API discovery.
- `/_matrix/client/v3/sync` for classic sync assertions.
- `/_matrix/client/unstable/org.matrix.simplified_msc3575/sync` for Synapse's
  native Simplified Sliding Sync coverage when supported by the client SDK.

The target integration matrix covers public and encrypted rooms, 1/100/5,000+
unread events, limited sync gaps, reconnect, room bump ordering, local echo,
`/read_markers`, and convergence of two clients on the same `m.fully_read` event.
Never point destructive fixture generation at a production homeserver.

## Lifecycle

```bash
scripts/synapse-integration.sh status
scripts/synapse-integration.sh logs
scripts/synapse-integration.sh down
scripts/synapse-integration.sh reset
```

`down` retains the disposable PostgreSQL volume. `reset` deletes that volume and
everything generated below `runtime/` except the tracked `.gitkeep`, including
media and stale PID state. `status` and `logs` never initialize a clean harness;
run `up` first. If port 8008 is occupied, set `SYNARA_PORT` before the first
`up`; the chosen port is retained in `runtime/.env`.

Run `npm run check:synapse-harness` without Docker to validate image pins,
loopback binding, runtime secret generation, and ignored state.

## Automated native client proofs

The CI workflow starts this disposable harness for the native reaction,
attachment, poll, rich-message, thread, and two-client receipt proofs in
`src-tauri/src/matrix/`. Each job resets the generated state after its test.
The two-client proof covers ordered delivery, exact-event read markers, and
receipt convergence. See the `synapse-native-*` jobs in
`.github/workflows/ci.yml` for the exact test commands and opt-in variables.

Synapse 1.162 room-version-12 behavior requires a separate disposable 1.162
server once stable. In that environment, create both a server-default room and
an explicit version-12 room, then exercise invite/join, encrypted send and
readback, redaction, and room settings from two Synara clients. Record the room
version in `m.room.create` and repeat restricted space hierarchy, cross-device
profile updates, and RTC transport discovery with the relevant server feature
flags. The 1.161 harness does not claim those 1.162 results.

`node scripts/synapse-v12-smoke.mjs` automates the HTTP portion against a
loopback-only disposable server. Supply `SYNARA_V12_BASE_URL` and two freshly
created user sessions via `SYNARA_V12_ALICE_TOKEN`, `SYNARA_V12_BOB_TOKEN`, and
`SYNARA_V12_BOB_USER_ID`. The script creates a default room, a version-12
event room, and a version-12 encrypted room; it checks invite/join, room
settings, event readback, `/relations`, and redaction from both accounts. It
prints only room IDs and the default room version. Open the encrypted room in
both Synara clients to finish the encrypted send/readback check. Run
`node --test scripts/__tests__/synapse-v12-smoke.test.mjs` to check the script
without a server.
