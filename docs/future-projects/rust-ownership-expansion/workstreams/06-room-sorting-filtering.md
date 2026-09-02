# ROE-06: Room Sorting and Filtering Rules

Hypothesis: shared semantic ordering and filter predicates may prevent desktop
and iOS drift, while UI sections and platform interaction remain local.

Investigate:

- favorites, spaces, direct chats, invites, unread/mentions, agents, archived
  or left rooms, low-priority rooms, and account-specific preferences;
- stable tie-breaking, locale/collation, timestamp uncertainty, missing names,
  incremental updates, and large room sets;
- which categories are Matrix semantics, Synara product policy, or purely
  platform navigation;
- persistence and sync of user-defined order, including note-like reordering;
- whether Core should return ranked rows, sort keys, or reusable predicates.

Minimum proof: deterministic/property tests, locale and missing-data fixtures,
large-list performance benchmarks, incremental-diff tests, and desktop/iOS
parity snapshots without forcing identical layouts.
