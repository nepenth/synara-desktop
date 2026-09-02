# ROE-06: Room Sorting and Filtering Rules

Prior: **split ownership; census existing Core policy before proposing more**.

Core already contains deterministic room-list predicates and sort helpers, and
ADR 0004 permits shared product-semantic ordering policy in Core. Desktop and
iOS also have intentional navigation, section, interaction, and preference
projection. Their layouts need not be identical.

## Bounded research question

Is shared Synara policy—favorites, spaces, direct chats, invites, unread,
mentions, agents, archived/left/low-priority state—being decided differently,
or are clients only presenting it differently? Determine whether existing Core
helpers are consumed, unnecessary, or need golden vectors before proposing a
new owner.

Stable protocol/product tie-breakers may belong in Core. UI sections, local
navigation state, incremental animation, and locale-aware display collation
remain platform-side. Do not centralize locale presentation in Rust merely to
force matching screenshots.

A census should first determine whether clients consume, bypass, or no longer
need the existing Core policy. A Core change requires a demonstrated
product-semantic divergence and large-list performance evidence; a platform
change may simply remove a competing semantic rule while retaining native
sections and collation.
