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

## Behaviour tradeoffs

- Sound and highlight now come from the account's rules. The renderer still
  plays the local sound for every shown notification when the user setting is
  on, so there is no audible change in this PR; a follow-up may switch the
  renderer to honour the `sound` echo.
- Room-mode changes made in another client apply to desktop notifications on
  the next event, exactly as the SDK sees them, without a 30-second snapshot
  poll.
- A decision for an event not yet in the SDK event cache makes one bounded
  `/event` request. The renderer already treats a failed decision as transient
  and resubmits on its next scan; no event is remembered as decided unless Core
  returned `show`, `duplicate-event`, or `own-event`.

## Still open (unchanged by this PR)

- Live macOS/Linux tray delivery readback and two-client interoperability.
- A spontaneous Core→renderer push stream; decide remains request/response.
- Delivery still swallows OS errors and acknowledges the candidate; a
  receipt/ack design should be tested before adding automatic retries.
