# ROE-01: Matrix SDK Orchestration, Sync, and Encryption

Prior: **already correctly owned; census and close**.

The shared-Core cutover already places Matrix client construction, sync
supervision, timeline ownership, crypto state, UTD recovery, and Matrix writes
in `crates/synara-core`. Thin Tauri and UniFFI adapters are expected platform
boundaries, not evidence of competing engines.

## Bounded research question

Does current shipped source contain any desktop or iOS code that independently
owns lifecycle, retry/backoff, crypto transitions, store identity, or a Matrix
write rather than reporting platform lifecycle or presenting typed Core state?

The census must cover construction/restore, sync start/stop, cancellation,
reconnect ordering, destructive account lifecycle, key backup, cross-signing,
and platform vault handoffs. Name the exact duplicate authority or confirm none.

## Keep closed

- No new Matrix engine, Core crate, command route, or migration ledger.
- Credentials and recovery material stay in platform credential boundaries.
- Do not count UI lifecycle observations or DTO projection as orchestration.

Completion is a source-linked memo confirming one live owner and existing
regression/live proof, or isolating one concrete remainder. Do not write an
implementation plan merely to re-prove the accepted architecture.
