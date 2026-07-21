# Synapse Production Read-Only Audit Runbook

The repository harness is disposable and is not production guidance. This
runbook records the evidence required before changing a real homeserver. The
audit is read-only; any deployment mutation requires a separate reviewed plan.

Review the current official Synapse [installation](https://element-hq.github.io/synapse/latest/setup/installation.html),
[worker](https://element-hq.github.io/synapse/latest/workers.html), and
[metrics](https://element-hq.github.io/synapse/latest/metrics-howto.html)
documentation for the deployed version during every audit.

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

## Audit output

The report contains topology, configuration source and revision, sanitized proxy
routes, dashboard links, observed percentiles, backup/restore evidence, risks, and
recommended changes. Secrets and raw homeserver configuration never enter the
repository. Production changes are separately approved, staged, monitored, and
reversible.
