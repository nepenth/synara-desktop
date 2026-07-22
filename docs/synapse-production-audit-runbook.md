# Synapse Production Read-Only Audit Runbook

Reviewed: 2026-07-22

The repository harness is disposable and is not production guidance. This
runbook records the evidence required before changing a real homeserver. The
audit is read-only; any deployment mutation requires a separate reviewed plan.

Review the current official Synapse [installation](https://element-hq.github.io/synapse/latest/setup/installation.html),
[worker](https://element-hq.github.io/synapse/latest/workers.html), and
[metrics](https://element-hq.github.io/synapse/latest/metrics-howto.html)
documentation for the deployed version during every audit.

## Current public evidence: converser.eu

The repository lists `converser.eu` as its first homeserver, but new desktop
sessions currently default to list index `1` (`matrix.org`). Treat changing the
default to `converser.eu` as a product decision, not an operational server fix.

Read-only checks on 2026-07-22 established:

| Surface          | Observed state                                                                                                 | Assessment                                                                                                                                      |
| ---------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Synapse version  | `1.155.0`                                                                                                      | Upgrade required: upstream stable is [`1.157.0`](https://github.com/element-hq/synapse/releases/tag/v1.157.0).                                  |
| Federation       | Matrix Federation Tester passed IPv4 and IPv6 with valid Ed25519 keys and TLS 1.3                              | Healthy public federation path.                                                                                                                 |
| Discovery        | Client and server well-known endpoints return HTTPS homeserver/federation targets                              | Healthy; client CORS is present.                                                                                                                |
| Client APIs      | Through Matrix Client-Server `v1.12`; Simplified Sliding Sync and initial-sync topological ordering advertised | Sliding Sync is available, but the server is behind current Synapse and Matrix authentication discovery.                                        |
| Authentication   | MAS issuer at `auth.converser.eu`; password, SSO, and token compatibility flows advertised                     | Functional; discovery still uses the pre-stable `org.matrix.msc2965.authentication` key. Recheck after upgrading.                               |
| Admin API        | Public `/_synapse/admin/...` returns `403` from nginx                                                          | Correctly blocked at the reverse proxy.                                                                                                         |
| Metrics          | Public `/_synapse/metrics` returns `404`                                                                       | Correctly absent from the public route; this does not prove internal monitoring exists.                                                         |
| Identity service | Well-known delegates to `vector.im`                                                                            | Confirm this external service and its privacy implications are intentional.                                                                     |
| Content origin   | `converser.eu` serves both Matrix content APIs and a public website                                            | Acceptable only if the website holds no sensitive state or broadly scoped cookies; Synapse recommends origin isolation for user-supplied media. |

Unauthenticated endpoint latency from the audit location was roughly 0.46-0.70
seconds for `/_matrix/client/versions` and 0.56-0.93 seconds for a rejected
`/sync`. Network and TLS setup dominate these samples, so they are not evidence
of large-room database or worker performance.

Run the repeatable public audit with:

```bash
node scripts/audit-matrix-public.mjs \
  converser.eu \
  --minimum-synapse 1.157.0 \
  --json
```

The command exits nonzero for version, discovery, or public operational-endpoint
failures. It never authenticates and never sends a Matrix access token.

## Required production configuration evidence

No production `homeserver.yaml`, worker configuration, reverse-proxy
configuration, deployment manifest, metrics dashboard, or backup report exists
in this workspace. SSH on the public host's standard port is refused. Therefore
the public checks above cannot certify the database engine, worker/Redis routing,
TURN, URL-preview SSRF controls, background updates, backups, or capacity.

On a host checkout, audit every configuration file passed to Synapse:

```bash
python3 scripts/audit-synapse-config.py \
  /etc/matrix-synapse/homeserver.yaml \
  /etc/matrix-synapse/conf.d/*.yaml
```

For the official container, stream the auditor into the container without
copying it or changing server state:

```bash
docker exec -i synapse python3 - /data/homeserver.yaml \
  < scripts/audit-synapse-config.py
```

The output includes only public identifiers, booleans, counts, listener routes,
and findings. Passwords, tokens, shared secrets, private keys, and database
connection values are never emitted. Review the output before sharing it anyway.

Before upgrading from 1.155, read both the
[`1.156`](https://element-hq.github.io/synapse/v1.156/upgrade.html) and
[`1.157`](https://element-hq.github.io/synapse/v1.157/upgrade.html) notes. The
1.157 release is particularly relevant to Synara's symptoms: it fixes a Sliding
Sync lazy-member deadlock and adds support for moving fully-read markers
backwards. Confirm the deployment uses stable `matrix_authentication_service`
configuration rather than the removed experimental MSC3861 delegation.

## Version and client endpoints

- Confirm the deployed version is a supported stable Synapse release and record
  the image/package digest, not only a floating tag.
- Verify `/_matrix/client/versions`, classic `/sync`, native Sliding Sync routing,
  federation, media, and well-known discovery through the production proxy.
- Confirm proxy timeouts/body limits support long sync responses and that the
  Synapse replication listener is not publicly reachable.

## Data and worker topology

- Confirm PostgreSQL is the production database, backups have a restore test,
  autovacuum/statistics are healthy, and connection pools remain within database
  capacity across every worker.
- Record worker roles, instance maps, Redis/replication configuration, sticky
  routing requirements, background task ownership, and graceful restart order.
- Compare the routing table to current Synapse worker documentation. Treat an
  unmapped endpoint, duplicate singleton task, or exposed replication port as a
  release blocker.

## Metrics and large-room evidence

- Scrape Prometheus metrics from the main process and every worker. Record sync
  request latency, event persistence, database pool wait/query latency, Redis and
  replication lag, federation queues, CPU, memory, file descriptors, and errors.
- Correlate client tests for 1/100/5,000 unread events with server request IDs and
  latency, without collecting room IDs, event content, tokens, or key material.
- Verify room-list bump data, limited sync gaps, account-data propagation, public
  and private receipts, and `/read_markers` responses with two test accounts.
- Establish normal and alert thresholds from a representative soak; do not copy
  the client UI budgets directly into server paging alerts.

The next authenticated, server-local audit should additionally record:

- `GET /_synapse/admin/v1/background_updates/status`, with no long-paused or
  stalled updates.
- Main-process and every-worker Prometheus targets on private listeners.
- PostgreSQL slow-query, pool-wait, autovacuum, table/index bloat, disk-latency,
  and restore-test evidence.
- Redis/replication lag, `/sync` and Sliding Sync p50/p95/p99 latency, outbound
  federation queues, and per-worker CPU/RSS/file-descriptor saturation.
- Reverse-proxy routing for the current worker map, long-poll timeouts, request
  body limits, encoded-path handling, client IP forwarding, and graceful drain.
- A two-client production-safe canary room proving `/read_markers`, public and
  private receipts, account-data convergence, room-list bump ordering, limited
  sync gaps, and reconnection without using a real large room as a fixture.

## Audit output

The report contains topology, configuration source and revision, sanitized proxy
routes, dashboard links, observed percentiles, backup/restore evidence, risks, and
recommended changes. Secrets and raw homeserver configuration never enter the
repository. Production changes are separately approved, staged, monitored, and
reversible.
