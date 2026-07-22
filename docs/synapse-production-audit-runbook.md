# Synapse Production Read-Only Audit Runbook

This runbook defines the evidence required before changing a production
homeserver. It is intentionally environment-neutral: hostnames, addresses,
credentials, raw configuration, account identifiers, and observed production
weaknesses belong in an operator-private system, not this repository.

Review the official Synapse [installation](https://element-hq.github.io/synapse/latest/setup/installation.html),
[worker](https://element-hq.github.io/synapse/latest/workers.html), and
[metrics](https://element-hq.github.io/synapse/latest/metrics-howto.html)
documentation for the deployed version during every audit.

## Version and client endpoints

- Record the supported stable Synapse version and immutable package or image
  digest.
- Verify `/_matrix/client/versions`, classic `/sync`, native Sliding Sync,
  media, federation, and well-known discovery through the production proxy.
- Confirm proxy timeouts and body limits support long sync responses.
- Confirm Admin API, metrics, replication, and database listeners are not
  publicly reachable.

## Data and worker topology

- Confirm PostgreSQL is the production database and that an
  application-consistent backup has passed a restore test.
- Review autovacuum, statistics freshness, connection-pool capacity, query
  latency, and disk-growth trends.
- Record worker roles, instance maps, Redis and replication configuration,
  sticky-routing requirements, background-task ownership, and graceful restart
  order in private operational records.
- Compare proxy routing with the current worker documentation. Treat an
  unmapped endpoint, duplicate singleton task, or exposed replication port as a
  release blocker.

## Security and transport

- Review file ownership and modes for live configuration, signing keys,
  database credentials, logs, media, and backups.
- Verify forwarded-client-IP trust is restricted to the known reverse proxy and
  cannot be spoofed by direct clients.
- Review service sandboxing, privilege boundaries, listener bind addresses,
  container and host firewall policy, and IPv6 parity.
- Verify secret rotation procedures without copying secret values into audit
  output.
- Validate TURN configuration and relay reachability when voice or video is
  supported.

## Metrics and large-room evidence

- Scrape metrics from the main process and every worker. Record sync latency,
  event persistence, database pool wait and query latency, Redis and replication
  lag, federation queues, CPU, memory, file descriptors, and errors.
- Correlate client tests for small, medium, and very large unread ranges with
  server request IDs and latency without collecting room IDs, event content,
  access tokens, device IDs, or encryption material.
- Verify room-list bump data, limited-sync gaps, account-data propagation,
  public and private receipts, and `/read_markers` behavior with disposable test
  accounts.
- Establish alert thresholds from a representative soak rather than copying UI
  paging budgets into server alerts.

## Audit output

Store production-specific results in an access-controlled operator system. A
public report may contain only generalized findings, sanitized proxy shapes,
non-sensitive performance percentiles, and recommended classes of change.
Secrets, raw homeserver configuration, internal topology, stable user or device
identifiers, and unresolved production weaknesses must never enter this
repository.

Production changes require a separately reviewed, staged, monitored, and
reversible change plan.
