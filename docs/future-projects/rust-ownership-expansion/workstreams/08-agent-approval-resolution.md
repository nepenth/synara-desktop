# ROE-08: Agent Approval Detection and Action Resolution

Prior: **highest-value demonstrated residual, sequenced behind the current
iOS-on-engine gate**.

Rust already has `app/agent_approvals.rs` classifiers and decision planning,
while desktop TypeScript and iOS notification code retain parallel
classification/planning behavior. This is credible competing policy, unlike
ordinary presenter projection.

## Bounded research question

Using the authoritative Hermes contract and real fixtures, map which clients
still independently decide prompt recognition, allowed actions, expiry,
authorization, response mapping, and idempotency. Separate structured events
from legacy formatted/plaintext fallback. Cover approve once, deny, approve
session, approve always, revocation, duplicates, room/sender trust, spoofing,
power/identity checks, response encoding, and redacted audit evidence.

The preferred direction is to make existing Rust policy authoritative and
remove competing planners, not create another approval subsystem. Sequence any
implementation behind the current shared-Core goal-graph stop gate; ADR 0004
defines the ownership boundary but does not define a historical phase queue.

Cards, sheets, notification buttons, haptics, emoji presentation, and OS
delivery remain platform rendering/integration. Required eventual proof
includes real-contract and adversarial fixtures, expiry/duplicate properties,
Hermes/Matrix end-to-end behavior, notification quick actions, and both-client
parity.
