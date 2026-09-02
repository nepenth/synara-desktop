# ROE-10: Draft Serialization and Reply Metadata

Hypothesis: durable draft identity, room/thread association, reply references,
and cross-device serialization may be shared without moving the composer
editor into Rust.

Investigate:

- plain/rich draft representation, attachments as metadata/handles, mentions,
  replies, edits, threads, and per-room/per-thread identity;
- local-only versus Matrix account-data synchronization and privacy tradeoffs;
- autosave frequency, crash recovery, concurrent devices, stale reply targets,
  and schema evolution;
- safe boundaries for Slate state and Swift attributed/editor state;
- conversion between editor-specific models and a canonical wire-neutral
  draft without losing formatting.

Minimum proof: round-trip/property tests, schema migration, crash/restart and
offline cases, concurrent-device conflict tests, reply fallback tests, and
desktop/iOS composer fidelity and typing-latency budgets.
