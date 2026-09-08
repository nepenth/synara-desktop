# Desktop notification policy: SDK push rules as the single owner

Status: implemented on `cursor/notification-core-push-rules-c25c`; deterministic
Core and desktop proofs pass locally. Live tray delivery on macOS/Linux and
two-client interoperability remain open gates, as before.

## Why this workstream

The 2026-09-04 weekly review ([`2026-09-04-past-week-astra.md`](2026-09-04-past-week-astra.md))
accepted the Core-owned desktop decision stream from PR #1097 but listed one
follow-on first under "Recommended follow-on work":

> Notification owner completion: consume SDK-evaluated push actions through Core
> instead of reconstructing mention/keyword/default rules in TypeScript. Room
> power-level checks for room mentions and full rule ordering should have one
> owner.

Notification policy is also the most-touched bug class of the last ten days
(v2.1.23, 2.1.26, 2.1.27, 2.1.28, 2.1.29 all changed read/notify behaviour),
and the 2026-09-04 review itself had to patch a TypeScript matcher defect
(`m.mentions: {}` still highlighting on legacy name/`@room` text). Every such
fix was made to a reconstruction of Matrix push rules rather than to the rules
themselves. This change removes the reconstruction.

## Intended operating path

- Goal: a desktop user receives an OS notification for a new message exactly
  when their account's Matrix push rules say `notify`, with highlight and sound
  following the same rules; muted rooms, mentions-only rooms, keywords, room
  mentions, and suppressed edits behave identically to every other Matrix
  client on the account.
- Actor: the signed-in desktop user with the room not focused.
- Starting state: an authenticated session whose `SyncService` is running; the
  SDK event cache is subscribed by the room-list service.
- First action: a new room event arrives through sync.
- Owner route: renderer observation pump → `matrix_notification_decide` with
  `(roomId, eventId, title, body, route)` → Tauri bridge → Core
  `NativeNotificationDecisionOwner::decide_observed` → SDK
  `Room::load_or_fetch_event` → SDK push actions → closed decision table →
  focus/dedup index → typed readback → `desktop_notifications.rs` OS mapping.
- Transitions: observed event → Core-resolved sender and SDK push evaluation →
  `show` (with `highlight` / `sound` echoes) or `suppress` with a static
  reason → OS delivery → candidate dismissed.
- Authority boundary: the renderer supplies identity and privacy-filtered
  product strings only. The wire request rejects `roomMode`, `highlight`,
  `isOwnEvent`, and `isEncrypted`; Core derives each from the SDK.
- Completion: the readback `decision`, `reason`, `highlight`, and `sound` agree
  with the account's real `m.push_rules` for that exact event.
- Disqualifiers: any TypeScript branch on mute/mentions/keywords, any body text
  or display name crossing to Core for matching, a decision made without the
  SDK event, or a notification shown when the SDK produced no `notify` action.

## Diagnosis of the prior design

`nativeNotificationDecision.ts` and `MessageNotifications` resolved the room
mode from three snapshots (`matrix_room_notification_snapshot`,
`matrix_push_rules_snapshot`, the room-list mode), then computed `highlight` by
re-implementing `.m.rule.is_user_mention`, `.m.rule.is_room_mention`,
`.m.rule.contains_display_name`, `.m.rule.contains_user_name`, `.m.rule.roomnotif`,
and keyword content rules over the plaintext body. Core then applied a small
mode×highlight table over those two booleans.

That duplicated policy had known gaps that the SDK ruleset does not have:

- `.m.rule.roomnotif` requires `notifications.room` power level for the sender;
  the TypeScript matcher never checked power levels.
- Rule ordering (override → content → room → sender → underride) and per-rule
  `enabled` flags were approximated by a mode string plus a highlight flag.
- The account-wide suppress-edits override installed by
  `edit_policy.rs` was invisible to the desktop matcher.
- Three snapshots polled every 30 seconds could disagree with each other and
  with the ruleset the SDK had already applied at sync time.
- Mentions inside encrypted events were unreachable because the matcher looked
  at the ciphertext envelope; the SDK recomputes actions after decryption.

## Change

Core (`crates/synara-core/src/app/notifications/decision.rs`):

- `NativeNotificationDecisionOwner` now retains the bound `Client` (same
  template as `NativeHttpPusherOwner`).
- New `decide_observed(NativeNotificationDecideRequest)`: parses the closed
  kind; for `message` it loads the exact event via
  `Room::load_or_fetch_event` (event cache first, one bounded authenticated
  `/event` attempt otherwise), compares the sender with the bound user, and
  reads `TimelineEvent::push_actions()`. `None` actions are recomputed once
  through `Room::event_push_actions`; a missing room context fails closed.
- `NotificationPushEvaluation { notify, highlight, sound }` folds the SDK
  actions. It replaces `room_mode` and `highlight` in the decision table.
- Suppress vocabulary is now `own-event`, `push-rules-no-notify`,
  `focused-room`, `duplicate-event`. `muted-room` and
  `mentions-only-without-highlight` are retired because they are now outcomes
  of the SDK rules, not Core branches.
- `NativeNotificationDecideRequest` drops `roomMode`, `highlight`,
  `isOwnEvent`, `isEncrypted`; `deny_unknown_fields` rejects them.
- `NotificationDecisionReadback` gains `highlight` and `sound` echoes for a
  shown message. Kinds without a timeline event (invite, agent approval, Later
  reminder) still surface through the same focus/dedup index.
- `NotificationRoomMode`, `AccountNotificationDefaults`, and
  `effective_room_mode` are removed from the decision module; the settings
  snapshot code in `push_rules.rs` / `room_notification.rs` is unchanged.

Desktop (`synara/`):

- `nativeNotificationDecision.ts` keeps only the three Core commands and
  readback validation. The mode resolver, mention/keyword/display-name matcher,
  and snapshot map helpers are deleted.
- `MessageNotifications` no longer loads or subscribes to the push-rule, room
  notification, or own-profile snapshots. It submits identity and presentation
  and delivers only on `show`. Sound behaviour is unchanged (user setting AND
  `show`); the `sound` echo is available for a later presentation decision.
- The source-guard test now locks both files against reintroducing a push-rule
  reading, mode resolution, mention matcher, or any `roomMode` / `highlight` /
  `isOwnEvent` field on the wire.

No Tauri command, ACL, permission TOML, schema, UDL, or iOS surface changed.
The command census is unchanged.

## Evidence

Deterministic, local, on this branch:

- `cargo test -p synara-core --lib notification`: 50 passed, including the
  rewritten decision-table tests and the `Core::command` round trip that now
  rejects the retired wire fields and fails closed without a bound client.
- `cargo test -p synara-core --test p4_s39_notification_push_rules`: 3 passed
  against a mock homeserver with real synced events and real `m.push_rules`
  account data:
  - server-default rules: plain group message → `show`, no highlight, no
    sound; `m.mentions` of the user → `show` + highlight + sound; own message →
    `own-event`; second observation → `duplicate-event`.
  - a room rule with no actions (mentions-only) → plain `push-rules-no-notify`
    while the mention still shows with highlight.
  - an override with no actions for the room (mute) → even a mention is
    `push-rules-no-notify`.
  - a room rule with `notify` and no tweaks → `show` without highlight/sound.
  - missing event id, malformed room id, unknown room → static diagnostics
    that never echo identifiers; an event absent from the cache is fetched once
    through `/event` and evaluated with the same rules; an unfetchable event →
    `event-unavailable`.
- Desktop modernization suite, TypeScript typecheck, ESLint, Prettier: see the
  pull request for exact counts.

## Second pass: delivery receipt, sound echo, encrypted and focus proof

The same branch closes three more items from the 2026-09-04 follow-on list
that do not need a macOS host to validate.

### Delivery receipt instead of a blind acknowledgement

Before: the renderer called the OS notification command fire-and-forget,
swallowed any error with `.catch(() => undefined)`, played the sound, and
acknowledged the candidate as if it had been delivered. Core could not tell a
delivered notification from a refused one.

Now: `matrix_notification_dismiss` accepts an optional closed `outcome`
(`delivered` | `failed`). The renderer awaits the OS answer, reports it with
the acknowledgement, and omits the outcome only when nothing was attempted
(system notifications off or no permission). Core keeps an identifier-free
`NotificationDeliveryLedger { delivered, failed, unreported }` per session
generation. It advances only when an acknowledgement releases a pending
candidate, so repeated or unknown acknowledgements cannot inflate it, and it is
echoed on `matrix_notification_dismiss` and
`matrix_notification_pending_snapshot` for diagnostics.

Deliberately not added: automatic retries. A `failed` receipt releases the
candidate and is counted; `(room_id, event_id)` dedup is retained, so a
flapping OS cannot re-notify the same message. The review asked for the
receipt/ack design to be tested before retries are considered; the ledger is
the observable that a future retry policy would have to justify itself
against.

### Sound follows the SDK tweak

The renderer now plays the local sound only when the local preference is on,
Core echoed the SDK `sound` tweak for the shown message, and the OS did not
refuse the delivery. Under server-default rules that means one-to-one rooms,
mentions, keywords, and any account rule with a sound tweak sound; plain group
messages notify silently, matching the rules the account's other clients
already follow. This is the behaviour change flagged as a follow-up in the
first pass.

### Additional SDK-backed proof

`p4_s39_notification_push_rules` grows from 3 to 5 tests:

- Encrypted room: an undecryptable `m.room.encrypted` event from another member
  in a three-member room → `show` from `.m.rule.encrypted` with no highlight or
  sound; `candidate.is_encrypted` is Core's own reading of the room encryption
  state; an undecryptable event from the session's own user → `own-event`
  without needing decryption; a second observation of the same id (the shape a
  late decryption takes) → `duplicate-event`, so late decryption can neither
  notify twice nor upgrade an already delivered notification.
- Focus changes through the real SDK path: a focused room suppresses even a
  mention and consumes no dedup; another room in focus does not shield it; a
  cleared focus shows the same mention with its highlight and sound; a `failed`
  receipt releases the candidate, is counted once, and leaves the event a
  duplicate; malformed focus is rejected and keeps the previous focus.

### Cheap iOS compile gate

`ci.yml` gains an `ios-compile` job that runs only when Swift/FFI/iOS paths
changed and the existing scheduling policy skipped the simulator lane
(unlabeled feature PRs into main). It runs the same scaffold and isolation
checks, generates SynaraCore/SynaraNseCore for the arm64 simulator slice, and
runs `xcodebuild build-for-testing` without booting a simulator or running a
test. Labeled, release, and main-push runs keep the full lane and skip it. The
quality gate aggregates it as success or skipped, `check-quality-gates.mjs`
requires it in the split layout, and `ci-scopes.test.mjs` executes the real
workflow shell for the new output. The first run of that job on this branch
is the macOS evidence for the gate itself; it is not evidence for any iOS
behaviour claim.

## Behaviour tradeoffs

- Sound and highlight now come from the account's rules. The renderer honours
  the `sound` echo: plain group messages notify without sound; one-to-one
  rooms, mentions, keywords, and account rules with a sound tweak still sound.
- Room-mode changes made in another client apply to desktop notifications on
  the next event, exactly as the SDK sees them, without a 30-second snapshot
  poll.
- A decision for an event not yet in the SDK event cache makes one bounded
  `/event` request. Since `466e41e` the renderer no longer scans, so a decision
  that fails transiently is dropped for that event (Core observed it once;
  there is no renderer retry and no TypeScript fallback); no event is
  remembered as decided unless Core returned `show`, `duplicate-event`, or
  `own-event`.

## Still open

- Linux tray delivery readback; macOS is now confirmed through the shipped
  path (below).
- The live `failed` receipt on macOS under a denied notification permission:
  the receipt code path is deterministic-tested and the rig is ready, but the
  System Settings toggle (Terminal identity in debug builds) has not been
  flipped in a recorded run.
- Automatic retry of a `failed` delivery; deliberately deferred.

Closed by `466e41e` (was open at `3ace2188`): the Core→renderer push stream
(`NativeNotificationObservationOwner` emitting `matrix-notification-observed`
per live message-like event; the renderer subscribes and no longer gates on
`SYNCING`, listens to `Room.timeline`, or scans every 30 s), and the macOS
route/action receipt (the send returns only after Notification Center reports
the record delivered, with a send error or 2.5 s silence reported as
`failed`).

## Live macOS result (2026-09-08, commit `3ace2188`)

Recorded in full in
[`A9-NOTIFICATION-DELIVERY.md`](../future-projects/rust-ownership-expansion/program/A9-NOTIFICATION-DELIVERY.md).
Two corrections to the claims above:

- The renderer observation pump never calls Core for a live message on the
  native client: both pumps in `MessageNotifications` return unless
  `getSyncState() === 'SYNCING'`, and the facade's highest state is
  `PREPARED`; `Room.timeline` has no production emitter and facade live
  timelines read as `[]`. Live: two real messages, zero `decide` calls. The
  SDK owner is correct but unreached; this predates the PR.
- "Delivery receipt instead of a blind acknowledgement" holds only for the
  Linux and no-route paths. On macOS with a `route` or actions,
  `desktop_notify` returns `Ok(true)` when the send task is spawned, before
  `notification.send()` runs, and the send error is dropped. Live: three
  `delivered` acks within ~1 s of the decision while the OS send had not yet
  returned minutes later. A denied OS delivery is counted as `delivered`, so
  the `failed` path is unreachable on macOS at this commit.

With the observation step injected past the dead gate, Core decided `show`
with the SDK sound tweak for a DM, a mention (`highlight true`), and a 2-member
room; `usernoted` delivered each record; the ledger advanced 1→2→3 with
`failed 0`; a focused room suppressed with no OS record.

## Live macOS rerun (2026-09-08, commit `466e41e`)

Same rig, same accounts, no injection. Each live message from the second
account reached Core through the shipped observation stream and produced one
`decide`: 2-member room `show`/`sound true`, DM `show`/`sound true`, mention
`show`/`highlight true`, focused room `suppress`/`focused-room` with nothing
attempted, DM while another room was focused `show`. The macOS receipt landed
52–64 ms after each send, after `usernoted` logged the delivered record; the
ledger advanced 1→2→3→4 with `failed 0`. Debug builds present as
`com.apple.Terminal` because `configure_macos_notification_application`
chooses that identity under `tauri::is_dev()`; the earlier "crate fallback"
wording was wrong. Full table and the pending deny-toggle note are in the A9
record.
