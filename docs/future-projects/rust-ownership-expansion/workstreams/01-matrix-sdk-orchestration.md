# ROE-01: Matrix SDK Orchestration, Sync, and Encryption

Hypothesis: lifecycle, sync supervision, crypto state, and Matrix SDK ownership
should have one Rust owner consumed by both clients.

Current caution: the repository already describes `synara-core` as the shared
Matrix engine. Begin by proving which behavior remains outside Core; do not
invent a migration from stale plans or count thin presenter adapters as a
second engine.

Investigate:

- client construction, restore, sync start/stop, backoff, cancellation, and
  store lifecycle on desktop and iOS;
- encryption, UTD recovery, key backup, cross-signing, and crypto diagnostics;
- task supervision, event delivery, reconnect ordering, and destructive
  account lifecycle;
- credentials or recovery material that must remain in platform vaults;
- whether any TypeScript still makes Matrix policy decisions rather than
  presenting typed state.

Minimum proof: Rust lifecycle/concurrency tests, store recovery and destructive
failure injection, two-client encrypted Synapse proof, iOS simulator binding
proof, desktop restart proof, and diagnostics showing one live owner.
