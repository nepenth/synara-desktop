# Grok 4.6 High source review: edit-policy

Read-only review; runtime evidence is separate. Session `01a0795c-0a5c-7d41-9b3e-42bb1740f404`.

I’ll pick up from the Ruma insert/`set_pushrule` lookup and finish the remaining owner, send-path, and leftover-pusher checks before giving a verdict.Checking one remaining product-path timeout and whether leftover pusher registration can skip the policy.**Verdict: ACCEPT**

Commit `96ccf6a9a2bce36edbc602105fe0b5d8b7e7fabb` vs `0695da32c77bc1a56ca22c9ec383f8fd006e6a24`. No P1/P2 code defects. The account-wide override is the right enforcement point for `event_id_only` APNs without an NSE filtering entitlement.

## Actionable findings

None.

## Why the policy is correct

The default `.m.rule.suppress_edits` is below `.m.rule.is_user_mention` in Ruma 0.19 `Ruleset::server_default`. Mentioning edits therefore still notify. This branch installs `com.whylandcreative.synara.suppress_edits` as a **user-defined override** with the same condition as the spec default: `event_property_is` on `content.m\.relates_to.rel_type` == `m.replace`, empty actions.

Exact matching holds:

- Condition key/value are identical to `ConditionalPushRule::suppress_edits()` in ruma-common 0.19.
- Tests reject `m.replacement` and `m.thread`; only `m.replace` clears actions.
- Mentions on a replacement do not leak to later mention rules when the relation is visible: first matching override wins, and empty actions mean do not notify.

Ordering vs custom/default rules:

- `set_pushrule::v3::Request::new` sends `before: None, after: None`.
- Ruma `Ruleset::insert` documents that a new rule with neither parameter becomes highest priority of its kind; override insertion is capped at index 1 so `.m.rule.master` stays first. `insert_and_move_rule` does **not** move an existing rule unless `before`/`after` is set, which matches the comment that PUT preserves order.
- User-defined overrides are evaluated before default mention rules, so a new install sits above `.m.rule.is_user_mention`.
- `precedes_notifying_overrides` fails closed if any **enabled notifying** override is ahead of the owned rule. Non-notifying rules (master, suppress_notices, mute overrides) are allowed in front.
- Shadow repair deletes **only** the owned rule, then PUTs it again so insertion returns it to the front of user-defined overrides. Unrelated rules are not rewritten.

Enabled state, idempotence, lifecycle:

- Confirmation requires `!default`, `enabled`, empty actions, exact single condition, and unshadowed order.
- Disabled-in-place: PUT then `set_pushrule_enabled(true)` (PUT keeps enabled; new rules default on).
- Already-confirmed: one GET, no writes (route test).
- Policy is installed from `register_http_pusher` before `pusher().set`. iOS product path is `HttpPusherOwner` → `NativeHttpPusherOwner::register` → that function. Leftover `pusher_set` is fail-closed (`p4-s10-leftover-unavailable`), so it cannot skip policy.
- Account-wide by Matrix design, including other clients on the account. Documented.
- Existing server pusher is not deleted on policy failure. Next APNs registration (process restart / token refresh) is the migration path; `append` is false so a successful run replaces the device pusher after confirmation.

Failure / privacy:

- Authoritative `GET /_matrix/client/v3/pushrules/` via `client.send`, not `NotificationSettings` cache.
- GET/PUT/DELETE failures → `v-pusher.edit-policy-failed`; unconfirmed readback → `v-pusher.edit-policy-unconfirmed`; pusher write is skipped.
- Errors are static; pusher data remains `event_id_only` with URL + format only.

Agent tool-call edits: Synara replacements go through `make_replacement`, which sets `m.relates_to.rel_type == m.replace`. The override is type-agnostic and matches that relation on any event type, including repeated tool-call replacements, as long as the relation is in cleartext content.

Tests exercise real write operations, not a stub policy helper: `edit_policy_route_tests.rs` calls production `register_http_pusher` against a scripted homeserver (install → readback → pusher, repeat read-only, 403 PUT, unconfirmed GET, disabled enable, shadowed DELETE+PUT, failed GET). Ruma evaluation tests cover mentions, threads, exact rel_type, and cleartext relation on `m.room.encrypted`. `p4_s9_http_pusher.rs` now serves an already-confirmed rule so the bound-owner fixture still reaches `pushers/set`.

## Proof limitations (not code defects)

These do **not** justify REQUEST_CHANGES or INCOMPLETE:

- **No APNs / live delivery proof.** This review is source-only. CI `34071122750` is pending; tests were not executed here.
- **Homeserver insertion is confirmed by readback, not by a live Synapse.** Production PUT omits `before`/`after`. If a server placed a new user-defined override after mentioning defaults, `policy_is_confirmed` would return `POLICY_UNCONFIRMED` and skip pusher success. Compliant servers (Ruma/Synapse model: new user-defined override at highest priority after master) succeed.
- **Encrypted events that hide `m.relates_to` cannot be classified.** Documented. Do not put bodies on APNs or weaken encryption. Preview decryption does not help the push gateway.
- **Reposition is delete-then-create.** If DELETE succeeds and PUT fails, the owned rule is briefly absent while an existing pusher remains. Next registration reinstalls. Uncommon (only when another notifying override already shadows).
- **iOS FFI timeout is pre-existing.** `HttpPusherOwner::register_http_pusher` still wraps the whole register in 5s (`shared_core_ffi.rs:9555`). First install now does extra GETs/PUTs inside that budget. Retry on the next APNs registration heals a timeout; not a policy-logic bug.
- **Route tests call `register_http_pusher` directly**, which is what `NativeHttpPusherOwner::register` and Core both call. They do not wrap the UniFFI 5s timeout.

## Checklist

| Check | Result |
| --- | --- |
| Exact condition matching | Pass — same key/value as spec default `suppress_edits`; `m.replacement` does not match |
| Ordering vs custom/default | Pass — user-defined override above mentions; shadow repair only moves owned rule |
| Enabled state | Pass — confirmation requires enabled; disabled is repaired |
| Idempotence | Pass — confirmed policy is GET-only |
| Account/session lifecycle | Pass — bound authenticated client; account-wide rule; leftover `pusher_set` cannot bypass |
| Existing pusher migration | Pass — policy on next register; pusher not deleted on policy failure |
| Failure behavior | Pass — fail closed before claiming registration success |
| Privacy | Pass — `event_id_only`, no bodies, static errors |
| Tests vs real owner ops | Pass — production `register_http_pusher` + bound-owner fixture |

No stylistic nits. Do not merge on the assumption of green CI or physical APNs; those are independent of this source verdict.

## Fixture correction review

I’ll re-evaluate head `32efeb4` by reading the HTTP fixture and checking that the bootstrap bypass is exact, bounded, and still fails on unexpected policy routes.**Verdict: ACCEPT** on head `32efeb4eea7255cf5e594315d3d228c89115a892`.

Production policy at `96ccf6a9` is unchanged (`ensure_edit_notification_policy` still gates `register_http_pusher`). This delta is test-fixture and evidence only. No P1/P2.

## Actionable findings

None.

## Fixture does not hide wrong policy traffic

The bypass is an exact request-line match after lowercasing:

`get /_matrix/client/v3/user/@reader:example.org/account_data/m.secret_storage.default_key http/1.1`

Anything else — other account-data types, encoded user ids, query strings, PUT/POST, pushrules, pushers — misses that `if` and hits the new exact `assert_eq!` on method+path (`edit_policy_route_tests.rs:102-106`). That is **stricter** than the previous `starts_with`, which would have accepted `?before=` / `?after=` on the PUT.

Bootstrap handling still requires:

- `authorization: bearer policy-proof-token` (`:92`)
- empty body (`content_len == 0`, `:93`)
- at most two such reads (`bootstrap_reads <= 2`, `:95`)

Those reads `continue` without pushing onto `bodies` and without advancing the scripted step, so policy body indices, counts, and order assertions are unchanged. A third default-key GET fails the test instead of being treated as a policy operation.

`wait_for_e2ee_initialization_tasks` is the SDK’s real setup-task join (`matrix-sdk` 0.18 `Encryption::wait_for_e2ee_initialization_tasks`), not a sleep or accept-any fallback. Unexpected owner routes during that wait still fail the exact path assertion.

## What this does not prove

Remote CI on this commit and physical APNs delivery remain unconfirmed. Local Core 882/3 and the six HTTP owner-route cases are evidence that the fixture matches the pinned SDK’s startup GETs; they are not APNs proof. The documented 5s UniFFI deadline is unchanged and still does not authorize a success claim.
