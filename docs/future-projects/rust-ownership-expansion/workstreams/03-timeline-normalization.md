# ROE-03: Timeline Normalization and Event Relationships

Prior: **extend `TimelineViewRow` only when a proven protocol-semantic field is
missing**.

`crates/synara-core/src/app/timeline` already normalizes messages, relations,
polls, media, encrypted placeholders, and other event rows into
`TimelineViewRow`. Do not introduce a second semantic-row layer.

## Bounded research question

Do edits, replies, threads, reactions, redactions, local echoes, pagination
overlap, late decryption, or relation-before-parent ordering produce a concrete
cross-client semantic divergence that the existing row model cannot express?
First determine what matrix-rust-sdk and current Core projection already
guarantee.

Prefer shared event-permutation and malformed-relation fixtures. If fixtures
prove a gap, propose the smallest versioned row/relationship field and measure
DTO diff/serialization cost.

## Keep closed

Scrolling, virtualization, gesture arbitration, visual grouping, typography,
selection, and invalidation strategy remain platform observation/rendering.
Never duplicate the normalized timeline in another Rust or platform model.
