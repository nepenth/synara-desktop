# ROE-10: Draft Serialization and Reply Metadata

Prior: **split ownership**.

Core may own protocol reply/thread metadata and a wire-neutral durable draft
schema when a real persistence or cross-device requirement exists. Slate,
Swift editor/attributed state, typing, selection, composition, and ordinary
local composer bodies remain platform-owned.

## Bounded research question

Is there an accepted product requirement for crash-restored or cross-device
rich drafts, and can a minimal schema preserve plain/rich body, mentions,
reply/edit/thread identity, and attachment handles without serializing editor
implementation state? Census current reply metadata first; do not assume a
missing shared owner.

Evaluate local-only storage before Matrix account-data sync, including privacy,
autosave frequency, conflict behavior, stale targets, evolution, downgrade,
and typing-latency budgets. Attachments are metadata/handles only.

Moving Slate or Swift editor state—or all composer bodies—into Rust requires an
explicit product and boundary decision. A speculative parity benefit is not
enough.
