# New-message notification policy

## Intended operating path

- Goal: a Synara iOS notification registration suppresses replacement events before APNs delivery, including replacements containing mentions, while ordinary new messages retain the account's notification rules.
- Actor: the signed-in user enabling or restoring notifications in Synara.
- Starting state: the account-bound Core pusher capability owns the active Matrix session and iOS supplies a valid APNs token and gateway configuration.
- First action: normal application push registration after session/APNs attachment.
- Owner route: Swift PushService → account-bound NativeHttpPusherOwner → Core edit policy → SDK authenticated push-rule API → homeserver → SDK pusher registration. No Matrix writes move to Swift or the gateway.
- Transitions: validate registration arguments; inspect authoritative rules; install/repair only Synara's replacement override when needed; independently read back ordering, condition, actions and enablement; register the pusher.
- Side effects: one named account-level custom push rule can be created/repaired, followed by the existing device pusher write. Account rules are shared across devices. Other rules and their relative order remain unchanged.
- Authority boundary: the retained authenticated Matrix client owns every write; no current-account lookup occurs after capability binding. No secrets enter generic commands or diagnostics.
- Completion: the homeserver confirms a correctly ordered replacement-suppression rule and accepts the pusher.
- Authoritative readback: SDK GET pushrules, with exactly one `m.replace` condition, no actions, enabled, before every notifying override except the non-notifying master rule.
- Acceptance: new messages preserve their original matching behavior; replacements never reach lower mention/room rules when their relation is visible to the server; a second registration performs no policy writes; policy failures prevent a new registration success claim.
- Disqualifiers: rewriting the ruleset, changing unrelated rules, treating local cache as server confirmation, omitting readback, claiming APNs suppression from empty NSE content, or exposing encrypted message text to the gateway.

## Diagnosis

The Matrix default `.m.rule.suppress_edits` follows mention overrides. Both dedicated test accounts have that rule enabled already; merely enabling it cannot implement the requested no-edit-notification contract. Read-only API probes found healthy key queries on those accounts and do not establish a general connectivity or encryption cause.

Synara registers `event_id_only` pushers. The gateway cannot classify a replacement from that sparse payload. Suppression belongs before pusher delivery. The NSE currently lacks Apple's notification-filtering entitlement, so an empty notification is not a supported replacement for server policy.

The iOS generic room title has a separate cause: notification routes omit a title and the timeline never resolves it from the existing room-list owner. Focused Core timelines request 25 context events; one-event history is not the intended focus contract. These are separate workstreams.

## Repair and migration

The named rule is `com.whylandcreative.synara.suppress_edits`. Its single `event_property_is` condition is identical to the server's current default rule: `content.m\.relates_to.rel_type` equals `m.replace`, with no actions. Insertion puts this custom override ahead of default mention rules. If another notifying override shadows it, Core recreates only this owned rule; all unrelated rule values and their relative order are preserved. Re-enabling a disabled Synara-owned rule is deliberate under the requested no-edit-alert contract.

This is **account-wide Matrix push policy**, including other Matrix clients logged into the account. It is not an iOS-only preference. A process restart supplies a fresh in-memory pusher binding, so existing installations establish the policy during normal APNs registration without logout. Already-confirmed policy causes one read and no policy writes. An existing server pusher is not deleted when policy installation fails, and no new registration success is claimed.

## Limits

The homeserver can match the replacement rule only when `m.relates_to.rel_type` is available in the event's cleartext content. An encrypted sender that conceals the relation cannot be classified by this rule; preview decryption does not grant the gateway access to encrypted content. Platform preview preferences, NSE key availability and physical APNs delivery require their own evidence. This branch does not claim that those paths are repaired.

## Sources

- [Matrix push rules and edit suppression](https://spec.matrix.org/latest/client-server-api/#mrulesuppress_edits)
- [Apple notification filtering entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.usernotifications.filtering)
- [ADR 0002](../adr/0002-ios-architecture.md) and [ADR 0004](../adr/0004-rust-language-boundaries.md)

## Validation

- `git diff --check`, Rust formatting of new files, and Matrix boundary checks passed.
- Added real pinned-Ruma rule evaluation tests for new messages, edits with mentions, threaded replies, exact relation matching, and exposed encrypted relations; unrelated action sets remain unchanged.
- Added production Core pusher-route HTTP tests for install → readback → registration, repeated registration without policy writes, disabled-rule repair, priority repair, authoritative-read failures, policy-write rejection and unconfirmed readback.
- Updated the existing account-bound pusher rotation fixture to serve an already-established policy.
- Rust execution is pending remote CI: the orchestrator paused local native builds because disk space is critically low. No live account push rules were modified by the diagnostic API probe.
- Physical APNs delivery: **Not confirmed**. The server probes establish rule availability and healthy key-query responses, not actual notification delivery.

