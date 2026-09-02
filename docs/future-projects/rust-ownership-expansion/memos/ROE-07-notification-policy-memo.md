# ROE-07 Research Memo: Notification Eligibility and Privacy Policy

Status: draft research; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                                                                                 |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-07 (notifications and agent policy)                                                                                                                                                               |
| Research owner     | ROE-07 residual-census researcher                                                                                                                                                                     |
| Reviewers          | Unassigned                                                                                                                                                                                            |
| Source census      | 2026-09-01; worktree `eb994ec4695a7f33282647ab3402941e72f45412` (feature-branch program docs on `main` `011cf39a`). Product paths re-read on this commit. [CENSUS.md](../program/CENSUS.md) is snapshot only. |
| ADR baseline       | ADR 0003, 0004, 0005 last reviewed 2026-09-01 against `011cf39a` (ADR index same date). Goal-graph and playbook §5 read on this census commit.                                                         |

## Observable problem

Users set mute, mentions-and-keywords, and keyword rules once and expect the
same rooms to stay quiet or high-signal on desktop and iOS. OS banners, tray
icons, APNs payloads, and lock-screen chrome can differ. Whether a room is
muted, whether only mentions should count, and whether ciphertext may become
preview text cannot.

This memo asks only which muted-room, mention, foreground-suppression,
preview-privacy, encrypted-content, replacement/dedup, expiry, or clock-skew
decisions are still competing *authority* rather than OS capability handling.

Critical agent-approval classification and action resolution are already
decided by accepted [ROE-08](ROE-08-agent-approval-memo.md) and human-gate
D10. This memo does not reopen those detectors, propose deleting them, or
design a new approval engine.

This memo does not authorize product work, a new Core surface, or a
shared-Core phase change.

## Current ownership census

`program/CENSUS.md` (snapshot of `main` `011cf39a`) correctly names Core push
rules and room notification modes, desktop OS delivery in
`desktop_notifications.rs`, and iOS APNs/NSE as platform-owned. Source on
this commit agrees those files still exist. Source also shows cutovers the
snapshot does not state:

- Settings writes already go through Core (`matrix_push_rules_*`,
  `matrix_room_notification_*` / UniFFI `room_notification_*`).
- Room-list DTOs already project `notification_mode`.
- Desktop unread badges already skip muted rooms via that Core field.
- `NotificationIndex` is a P7.1 harness, not a live product owner.
- Leftover UniFFI `set_notification_mode` stays fail-closed; product iOS uses
  `roomNotificationSet`.
- Desktop `getNotificationType` / `getRoomPushRule` are leftover readers that
  fail closed to `Default` on the native facade.

Where snapshot and source disagree, source wins.

Two product contracts must not be collapsed:

1. **Homeserver push-rule and per-room mode settings** (mute / mentions /
   all / keywords). This is the Core `push_rules` / `room_notification`
   domain. iOS APNs eligibility is the homeserver applying those rules.
2. **OS delivery and local privacy chrome** (APNs, NSE, tray, banners,
   badges, lock-screen preview toggle, focus suppression). Platform
   integrations.

Agent-approval prompt recognition, OS write planning, and in-app once/deny
remain ROE-08 / D10. They are named below only to keep them out of this
policy extract.

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Muted-room *settings* | **Core authority (hard invariant).** `snapshot_room_notification` / `set_room_notification` / `snapshot_room_notifications` map SDK `RoomNotificationMode` to wire `all` / `mentions` / `mute` / `default`. Room-list `notification_mode` is the same mapping (`room_list/live.rs`). Leftover UniFFI `set_notification_mode` returns `p4-s10-leftover-unavailable` and is not the product write. | **Presenter consumption of Core, not a second mute table.** Settings and room switcher write `matrix_room_notification_set`. `useRoomsNotificationPreferences` on a native session loads `nativeRoomNotificationsSnapshot`. Unread projection skips `notificationMode === 'mute'` (`roomToUnread.ts`). | **Presenter consumption of Core.** Room details picker writes `SharedCoreRoomNotification.set` → `roomNotificationSet`. List mute uses the same product owner. Leftover `SharedCoreLeftovers.setNotificationMode` is unused on the live path. | Core `room_notification.rs` tests; desktop `nativeRoomNotification.ts`; iOS `SharedCoreRoomNotification.swift`, `SharedCoreProductServices.swift`. |
| Muted-room *local OS emission* | **No live per-event mute evaluator.** `NotificationIndex` can drop focused-room candidates but is linked only as a P7.1 marker (`matrix_notifications_markers`). It is not registered on `Core::command` and is not consumed by either presenter. | **Leftover adapter, not competing mute authority.** `MessageNotifications` gates on `getNotificationType(...) === Mute`. Native `getRoomPushRule` always returns `undefined`; native `getAccountData` is a documented GAP and always `undefined`; so `getNotificationType` always returns `Default`. The native facade never emits `Room.timeline` (only `sync` / `session`), so this leftover listener is unwired on the live desktop client. | **OS / homeserver capability, not a second client mute engine.** NSE does not re-score mute. APNs arrives only if the homeserver applied Core-written push rules. | `nativeClientFacade.ts` (`getRoomPushRule`, `getAccountData`, emitter); `ClientNonUIFeatures.tsx`; `NotificationService.swift`. |
| Mention / keyword *settings* | **Core authority (hard invariant).** `snapshot_push_rules` / `set_default_room_mode` / `set_mention_enabled` / `add_keyword` / `remove_keyword` own DM/group defaults (including encrypted), `.m.rule.is_user_mention` and siblings, and a 64-keyword cap. | **Presenter consumption of Core.** Native settings editor calls `matrix_push_rules_*`. Leftover `useNotificationMode` / `pushRules.ts` action tables remain for the non-native branch that the desktop boot path no longer constructs. | **No second mention-rule editor.** Mention defaults are not re-implemented in Swift. Room mention-only mode is the Core wire mode. | Core `push_rules.rs` tests; desktop `nativePushRules.ts`, `Notifications.tsx`. |
| Mention *local OS emission* | No live per-event mention matcher for OS notify. Room-list `highlight_count` already uses SDK mention/unread counts. | Leftover `MessageNotifications` does not distinguish mentions-only from all-messages. Native `getUnreadNotificationCount` returns Core `unreadCount` only (no highlight argument). Because `Room.timeline` is unwired, this is not a live second mention engine. | NSE does not re-evaluate mentions. Homeserver push rules (written by Core) decide whether APNs fires for a mention-only room. | `room.ts` `getNotificationType`; `nativeClientFacade.ts` unread projection; NSE `NotificationService.swift`. |
| Foreground suppression | Unused harness field `suppress_if_focused_room` + `set_focused_room` on `NotificationIndex`. Not product-wired. | **Platform observation.** `document.hasFocus()` and selected-room / inbox-selected skip (`ClientNonUIFeatures.tsx`). Core cannot know genuine focus. | No local APNs mute when the app is foreground. Banner suppression is OS capability. Notification *actions* use `.foreground` so UIKit, not the extension, owns store writes (ROE-08 routing). | ADR 0004 observation examples; `PushService.swift` category options. |
| Preview privacy | No live shared privacy-level owner for OS notify. `NotificationCandidate` documents privacy-filtered title/body, but the index is unused. NSE preview text is bounded (240/64) after decrypt. | **Platform rendering / local preference.** Ordinary OS body is the generic string `New inbox notification from ${username}` — never the event body. `SystemNotificationRequest.privacy` exists (`standard` / `private`) but `showPlatformNotification` does not pass it to the shell. Local `showNotifications` is a device setting, not Matrix policy. | **Platform observation / OS lock-screen policy.** `lockScreenMessagePreviews` defaults false. NSE fills title/body only when that pref is on and Core resolved a preview. Diagnostics never store Matrix ids or bodies. | `systemNotification.ts`; `SynaraNotificationPreviewPreference`; `NotificationPreviewSupportTests.swift`. |
| Encrypted-content availability | **Core store authority; fail closed.** NSE `resolve_event_preview` restores one room, resolves one event, drops SDK owners. Missing session/store/event or non-`m.room.message` → unavailable. No media download (ADR 0005). | Never places ciphertext or decrypted body on the OS notification. Encrypted rooms are not a special emit branch. | `SynaraMatrixEventPreviewComposer` returns nil for `m.room.encrypted`. NSE keeps the generic APNs payload when resolution fails. | `nse_preview.rs`; `SynaraNotificationPreviewSupport.swift`. |
| Replacement / dedup | Unused harness: `(room_id, event_id)` `seen_events`, cap 128, generation retire. | **Platform observation / OS object coalescing.** `unreadNotificationCache` skips unchanged unread totals; the previous `window.Notification` is closed. `notifiedEventIdsCache` is used for approval emission (ROE-08). | Per-APNs request delivery. `NotificationResolutionGate` serializes one decrypt (NSE memory), not Matrix identity policy. Agent-approval action dedupe is ROE-08 local observation. | `notifications/index.rs` tests; `notificationCaches.ts`; `NotificationDeliveryCoordinator.swift`. |
| Expiry / clock skew (ordinary notify) | None for ordinary messages. NSE request/resolution timeouts are process bounds. Agent-approval TTL/skew is ROE-08 Core write authority. | None for ordinary messages. Later reminders use `dueTs` (different product). Approval `RECENT_AGENT_APPROVAL_MS` is ROE-08 emission, not this policy. | `serviceExtensionTimeWillExpire` is OS capability. Approval freshness gates time-sensitive category only (ROE-08 presentation). | `nse_preview.rs`; `NotificationService.swift`. |
| Tray / APNs / NSE / banners / badges / actions | Not Core. HTTP pusher set/delete exists as leftover I/O (push keys stay off `Core::command`). | **Accepted platform boundary.** `desktop_notifications.rs` sanitizes title/body/route/actions and posts tray notifications. Badge formula is desktop-local (`synara-notification-contract.md`). | **Accepted platform boundary.** APNs registration, category chrome, NSE lifecycle, and TestFlight proof stay Apple-owned. | ADR 0004 layer map; playbook leftover pusher/notification I/O. |

**Earliest actual divergence.** Not a second mute/mention *settings* engine
(already Core on both clients). Not a live second per-event eligibility
engine: desktop leftover `getNotificationType` cannot see Core modes and is
not fed `Room.timeline` on the native client; iOS does not re-score
mute/mentions in NSE. The remaining split is OS capability and local privacy
chrome (generic desktop body vs iOS lock-screen pref), plus an unused Core
harness (`NotificationIndex`) that must not be mistaken for a missing product
owner.

**Constraint classes.** Shared mute/mention/keyword *settings* and Matrix
writes are hard invariants (one Core; no second JS/Swift push-rule engine).
Focus, lock-screen preview preference, tray/APNs/NSE delivery, banner
coalescing, NSE memory/deadline, and badge chrome are accepted platform
boundaries. React, SwiftUI, and the current notification stacks are
technology preferences, not reasons to move OS delivery into Core.

## Boundary constraints

- ADR 0003: one Core Matrix/application owner; no second JS or Swift engine
  for Matrix writes. Presenter projections are not a second domain owner.
- ADR 0004 (2026-09-01): Core owns “push rules and shared
  eligibility/privacy/deduplication policy.” Platforms own “APNs/NSE/tray
  delivery, banners, actions, badges and haptics.” That assignment names
  *settings and shared policy*, not a mandate to productize the unused
  `NotificationIndex` or to put tray/APNs in Core. It does not authorize
  deleting leftover TypeScript helpers tonight (D3/D9/D10).
- ADR 0005: NSE must not download media. No media bytes or paths on
  `Core::command`. Unused for ordinary eligibility tables.
- Playbook §5: after landed S12–S37 and later optional leftovers, the
  checklist stops. Docs-only PRs remain allowed. Do not invent leftover
  routes or S38. Do not start P5. Do not claim P4 engine-ready. Leftover
  `set_notification_mode` / pusher I/O stay fail-closed without a live
  homeserver (decision 15).
- Goal-graph stop conditions
  ([13-language-boundary-goal-graph.md](../../../shared-native-core/13-language-boundary-goal-graph.md)):
  the next required node is the P4 engine-ready *gate* (pending; blocked on
  paused iOS CI and paused live-homeserver proof). Stop on that blocked gate.
  Do not invent S38. Do not start P5.
- NSE stays narrow: one-shot store preview; never start sync; never download
  media (ADR 0004 invariant 6, ADR 0005).
- D1–D10: this memo cannot open an implementation gate, amend an ADR, claim a
  shared-Core phase, or reopen ROE-08 detector deletion.

## Alternatives

1. **No ownership change (stay-put).** Keep Core as the sole mute/mention/
   keyword *settings* owner. Keep APNs/NSE/tray, focus, lock-screen preview
   preference, and OS coalescing on the platforms. Leave leftover
   `getNotificationType` / `NotificationIndex` unwired. Falsified if a live
   desktop or iOS path still *writes* push rules or room modes without Core,
   or if NSE/desktop independently decides mute/mentions for a room whose
   Core mode disagrees *and* that decision is what the user sees.

2. **Bounded extraction (live `should_notify` evaluator).** Productize
   `NotificationIndex` or a new Core eligibility function for muted-room,
   mention, privacy-level, encrypted-availability, and event-id dedup.
   Presenters would pass focus as an observation. Falsified if existing Core
   `notification_mode` / highlight counts plus homeserver push rules already
   produce the same semantic result *and* no live client consumes a second
   evaluator.

3. **Broader Core model.** Move preview composition, lock-screen preference,
   foreground measurement, tray/APNs posting, or NSE lifecycle into Core.
   Falsified immediately by ADR 0004 (delivery and observation stay
   platform) and by NSE memory/deadline being Apple constraints.

Stay-put is the default unless harmful duplicated *authority* is proven. The
proven remainder is a leftover desktop reader that fails closed and is not
fed timeline events — not two mute tables, and not a missing Core settings
owner.

## Recommendation

**Already correctly owned** for mute, mention, and keyword *settings*;
**stay platform-side** for APNs/NSE/tray delivery, foreground suppression,
preview-privacy preferences, encrypted fail-closed handling, OS replacement,
and ordinary expiry/clock-skew.

Confidence: medium-high. Both clients already write and read room
notification modes and (on desktop) global push-rule defaults through Core.
Unread badges already honor Core mute. iOS APNs eligibility is the
homeserver applying those Core-written rules; NSE does not re-decide mute or
mentions. Encrypted preview fails closed in Core NSE and in the iOS
composer. Ordinary notifications have no competing expiry or clock-skew
policy.

None of the asked decisions is a live dual engine:

| Decision | Competing authority? |
| -------- | -------------------- |
| Muted rooms | No. Settings are Core. Leftover desktop mute check always returns `Default` and is unwired. iOS does not re-score mute. |
| Mentions / keywords | No. Settings are Core. Leftover desktop emit does not implement a second mention matcher. iOS uses homeserver rules. |
| Foreground suppression | No. Platform observation (desktop focus) / OS banner policy (iOS). Unused Core harness field is not a product owner. |
| Preview privacy | No. Device-local chrome (desktop generic body; iOS lock-screen pref). Not two Matrix privacy engines. |
| Encrypted-content availability | No. Fail-closed store/OS handling, not a second decrypt policy. |
| Replacement / dedup | No. Unused Core index vs local unread-cache / per-APNs delivery. Agent-approval action dedupe stays ROE-08 observation. |
| Expiry | No for ordinary notifications. Approval TTL is ROE-08. NSE deadline is OS. |
| Clock skew | No for ordinary notifications. Approval skew is ROE-08. |

Do not extract `NotificationIndex` or a new `should_notify` Core API. Do not
move APNs, NSE, tray, banners, badges, or haptics into Core. Do not treat
this memo as permission to delete leftover TypeScript notification helpers
or to reopen ROE-08 classifiers.

Strongest objection (extract anyway): ADR 0004 already lists shared
eligibility/privacy/dedup as Core-shaped, leftover `getNotificationType`
still *looks* like a second mute gate, and desktop OS notify may ignore mute
if `Room.timeline` were ever wired without consuming `notification_mode`.
That is a leftover-adapter / presenter-consumption defect, not evidence that
settings authority is missing. Wiring an unused harness would start product
work while the goal graph is stopped on the P4 engine-ready gate.

Unresolved questions:

- Whether a later, separately chartered presenter task should make desktop OS
  emission consume existing Core `notificationMode` / highlight counts if
  local notify is restored. That is not a Core extract and is not authorized
  here.
- Whether desktop should keep a permanently generic OS body or grow an iOS-
  like lock-screen preview preference. Product/UX, not competing Matrix
  authority.
- Shared golden fixtures for mute / mentions-only / encrypted-unavailable OS
  outcomes are not yet a portfolio artifact (design note only).

Regression proof to keep this close stable: room-mode and push-rule writes
only through Core; leftover `set_notification_mode` stays fail-closed;
product iOS continues to use `roomNotificationSet`; NSE restores category
only from Core `is_agent_approval` plus freshness observation (ROE-08);
NSE never starts sync or downloads media; desktop `desktop_notifications.rs`
remains delivery-only; `NotificationIndex` remains unregistered.

## Next gate

**Close the research item.** Settings are already correctly owned. Delivery
and observation stay platform-side. There is no extract/proceed
recommendation and therefore no implementation plan, Core API, UniFFI
change, or product edit.

A leftover desktop eligibility adapter remains as unused presentation code.
Retiring or rewiring it is a later human product decision, not an overnight
Core extract. The current goal graph does not permit a new residual
implementation slice: P4 engine-ready is pending/blocked; do not invent S38;
do not start P5. D10 (ROE-08) remains the only open human implementation
question in this cluster.
