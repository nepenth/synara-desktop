# ROE-09: Notes and Account-Data Synchronization

Hypothesis: note/ToDo schemas, ordering, conflict handling, and Matrix
account-data synchronization should be Rust-owned; editors and drag/reorder UI
remain platform-owned.

Investigate:

- current account-data schema, versioning, migration, limits, and ownership;
- create/edit/delete/reorder/complete operations and stable identifiers;
- concurrent edits from desktop/iOS, conflict policy, tombstones, offline
  queues, and clock independence;
- message anchors when source events are redacted, unavailable, or moved;
- export, privacy, retention, and malformed remote data.

Minimum proof: schema compatibility/golden tests, operation-sequence property
tests, two-client concurrent Synapse proof, offline merge tests, migration and
downgrade fixtures, and native reorder/editor acceptance on both clients.
