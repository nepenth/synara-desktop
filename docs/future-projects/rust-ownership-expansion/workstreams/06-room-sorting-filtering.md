# ROE-06: Room Sorting and Filtering Rules

Prior: **stay platform-side by default**.

Core already contains deterministic room-list predicates and sort helpers.
Desktop and iOS also have intentional navigation, section, interaction, and
preference projection. Their layouts need not be identical.

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

A close/stay memo with deterministic fixture coverage is preferred. A Core
change requires a demonstrated product-semantic divergence and large-list
performance evidence.
