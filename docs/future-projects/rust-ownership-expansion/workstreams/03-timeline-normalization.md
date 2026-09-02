# ROE-03: Timeline Normalization and Event Relationships

Hypothesis: protocol event normalization and relationships should be shared,
while viewport math and visual grouping remain presenter-owned.

Investigate:

- edits, replies, threads, reactions, redactions, polls, state events, media,
  encrypted placeholders, local echoes, and aggregation ordering;
- stable event identity, pagination overlap, deduplication, late decryption,
  relation arrival before parent, and replacement precedence;
- which normalization matrix-rust-sdk already guarantees;
- the smallest versioned row/relationship model that supports both clients;
- backpressure and diff granularity needed to avoid UI invalidation storms.

Do not move scrolling, virtualization, gestures, typography, or message-cell
grouping into Core.

Minimum proof: event permutation/property tests, pagination and local-echo
fixtures, malformed relation tests, multi-client Synapse histories, DTO
contract tests, and desktop/iOS performance budgets for large rooms.
