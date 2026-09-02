# ROE-05: Read-Marker and Unread Calculations

Hypothesis: unread truth, receipts, marked-unread state, and read-marker
advancement should be calculated once in Rust; viewport visibility remains a
platform observation supplied to that owner.

Investigate:

- public/private receipts, fully-read markers, notification/highlight counts,
  threads, invites, encrypted/late-decrypted events, and local echoes;
- the contract by which each UI reports genuine visibility and user intent;
- races among sync, pagination, room changes, backgrounding, and receipt send;
- offline persistence, retry/idempotency, and multi-device behavior;
- separation between authoritative Matrix state and badge presentation.

Minimum proof: ordering/property tests, focus/visibility contract tests,
two-client receipt Synapse proof, offline/reconnect cases, room-switch stress,
desktop viewport integration, and iOS foreground/background tests.
