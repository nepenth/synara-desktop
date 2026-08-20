# Timeline and Room-State Acceptance Budgets

These budgets are release gates for the modern timeline, read-state, Favorites,
and room-list sort paths. Measurements must use release-like builds, deterministic fixtures,
and privacy-safe diagnostics described in [timeline-diagnostics.md](timeline-diagnostics.md).

## Required budgets

| Area                            | Budget                                                                                        |
| ------------------------------- | --------------------------------------------------------------------------------------------- |
| Settled unintended anchor drift | At most 2 CSS px on desktop and 2 pt on iOS                                                   |
| Active input                    | No programmatic scroll write during drag or momentum                                          |
| Favorite tag write              | Native `m.favourite` write settles or surfaces failure without a local-only fake              |
| Room-list sort                  | One global recent/name sort; missing `last_activity_ts` sorts last                            |
| Jump Latest                     | Tail confirmed within 2 seconds in deterministic fixtures and 5 seconds against local Synapse |
| Live provider                   | At most 300 stable events plus unmatched local echoes                                         |
| Focused provider                | Initial context at most 50 events on each side of the anchor                                  |
| Memory                          | No sustained growth above 15% after 20 history/live cycles                                    |
| 5,000-room update               | Only the changed room remaps and publishes within 100ms on the reference device               |
| Cross-client read state         | Both clients converge on the same fully-read event after sync                                 |

## Acceptance scenarios

- Two accounts and two independent clients exercise public and encrypted rooms
  with 1, 100, and at least 5,000 unread events.
- Room opening covers no unread state, a current-session history viewport, an
  in-window marker, a marker outside the live window, and a purged marker.
- Layout coverage includes 1–200-line messages, delayed fonts/images, decryption,
  reply expansion, edits, redactions, Dynamic Type, and rotation.
- Input coverage includes wheel/touchpad momentum, iOS drag/deceleration, rapid
  room switching, pagination, live append, sync reset, offline/reconnect, and a
  failed/retried Jump to Latest.
- Room-list coverage includes favorite tag add/remove, encrypted rooms ordered
  by native latest-event timestamp, missing timestamps sorting last, and one
  global recent/name sort on Favorites and remaining rooms.
- Read-state coverage validates exact `/read_markers` payloads, public/private
  receipt choice, serialization, retry, late completion, and no pre-confirmation
  advancement of a historical or pre-jump tail.

## Evidence and diagnostic privacy

Every release candidate records the tested commit, build type, platform/device,
fixture seed and size class, result bundle/trace location, before/after memory,
latency percentiles, and pass/fail against every budget. Raw logs remain local;
only redacted summaries and approved artifacts attach to a PR.

Allowed diagnostic dimensions are provider mode, generation-relative counts,
navigation phase and duration, anchor correction magnitude, scroll-writer reason,
queued snapshot count, pagination direction, bottom confirmation, room-activity
update/expiry lag, read-marker outcome class, and process memory. Hashing a real
identifier is not sufficient anonymization; identifiers must be omitted.

Any accepted P0/P1, correctness/security P2, Matrix-spec violation, failed gate,
or unexplained budget regression blocks promotion. A rollback flag may mitigate a
released regression, but it does not convert a failing release candidate to pass.
