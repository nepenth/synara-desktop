# Disposable Synapse integration harness

This local-only harness pins Synapse `1.157.0` and PostgreSQL `16.9`. It exists
for client integration and regression tests; it is not a production topology.
The HTTP listener binds to loopback, registration is intentionally open, and all
credentials and signing material are generated under ignored `runtime/` state.
The Synapse pin corresponds to the upstream
[`v1.157.0` release](https://github.com/element-hq/synapse/releases/tag/v1.157.0).
The 1.157 upgrade needs no template change: this harness does not configure the
removed experimental MSC3861 auth delegation. It does pick up the upstream
Sliding Sync lazy-member deadlock fix and the application-service ephemeral
event regression fix introduced in 1.156. See the official
[upgrade notes](https://element-hq.github.io/synapse/v1.157/upgrade.html#upgrading-to-v11570).

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

## Automated two-client gate

The noninteractive gate creates a reader on two independent device sessions and
a separate sender, then creates and joins a private room. For both public and
private receipts it sends 64 ordered events and proves all of the following:

- a limited initial sync ends at the actual latest event;
- SDK `/context` loading finds an older exact event and backwards pagination
  adds a preceding page without changing order;
- an explicit SDK latest-timeline load ends at the actual last event; and
- an exact `m.fully_read` marker plus the configured receipt converges from
  device A to device B.

Run it only against this disposable harness:

```bash
scripts/synapse-integration.sh up
npm run test:synapse-integration
scripts/synapse-integration.sh reset
```

`SYNARA_RECEIPT_MODE` accepts `public`, `private`, or `both` (the default).
`SYNARA_SYNAPSE_URL` can select another HTTP loopback port, but the runner
rejects non-loopback hosts, HTTPS, URL credentials, paths, queries, and
fragments before loading the SDK. Generated passwords and access tokens remain
in memory and the runner emits only phase/count diagnostics, never credentials
or message bodies. Every request, poll, CI job, and teardown is bounded; CI
always resets the database volume and generated runtime state.
