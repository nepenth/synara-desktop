# ROE-08 Research Memo: Agent Approval Classifiers and Planners

Status: census accepted; implementation recommendation reopened for Hermes contract and action authority; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                                                                                 |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-08 (notifications and agent policy)                                                                                                                                                               |
| Research owner     | ROE-08 residual-census researcher                                                                                                                                                                     |
| Reviewers          | Independent feature-branch review; original `ACCEPT` on PR `#1082` at `cd1c655b3a8ff02452cbb4b061658b1be06e844e`; later promotion review reopened the recommendation                                  |
| Source census      | 2026-09-01; worktree `0b6c4297989746a95df21d4a6d286480ee61d570` (feature-branch program docs on `main` `011cf39a`). Product paths re-read on this commit. [CENSUS.md](../program/CENSUS.md) is snapshot only. |
| ADR baseline       | ADR 0003, 0004, 0005 last reviewed 2026-09-01 against `011cf39a` (ADR index same date). Goal-graph and playbook §5 read on this census commit.                                                         |

## Observable problem

Hermes dangerous-command prompts must be recognized, expired, and decided the
same way on desktop and iOS. An OS notification must not approve or deny a
prompt that the in-app contract would refuse. Cards, notification buttons, and
haptics can differ by platform; whether a body *is* an approval prompt, whether
a background action is allowed, and whether the account has already decided
cannot.

Two clients still independently decide “this is an approval prompt” with a
looser body-substring rule than Core. OS notification *writes* already call the
existing Core decide owner. The remaining question is whether the leftover
TypeScript and Swift classifiers and planners are still a second authority or
only presentation and routing.

This memo does not authorize product work, a new Core surface, or a shared-Core
phase change.

## Current ownership census

`program/CENSUS.md` (snapshot of `main` `011cf39a`) correctly names
`crates/synara-core/src/app/agent_approvals.rs`, desktop
`synara/src/app/utils/agentApprovals.ts`, and iOS notification/PushService
planners as the residual. Source on this commit agrees those files still exist.
Source also shows a cutover the snapshot does not state: native approve-once
and deny writes already go through `matrix_agent_approval_decide` /
UniFFI `agent_approval_decide`. Where snapshot and source disagree, source
wins.

Two product contracts must not be collapsed:

1. **Hermes dangerous-command prompts** (plaintext / `formatted_body` headings,
   decided by reactions). This is the `agent_approvals.rs` domain.
2. **Structured agent-card decisions** (`in.synara.agent.action` approve/reject
   notices via `send_agent_approval`). Separate write path; it does not compete
   with `is_agent_approval_prompt` / `plan_agent_approval`.

There is no `TimelineViewRow` approval flag. Presenters classify prompt bodies
themselves. The companion Hermes Matrix adapter implements typed
`!approve session` through its slash-command handler, while its reaction map
has no session reaction. Synara exposes neither an equivalent session action
nor an explicit typed-command route for that contract.

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Prompt recognition (eligibility) | **Core authority (hard invariant).** `is_agent_approval_prompt` matches the first non-empty line after whitespace collapse, lowercase, and heading-chrome trim (`⚠`, `️`, `*`, trailing `:`). Headings: `approval required: dangerous command`, `dangerous command requires approval`. Rejects bodies over `AGENT_APPROVAL_MAX_BODY_CHARS` (100_000). NSE preview uses this on `m.room.message` plaintext `body` (`nse_preview.rs` / `preview_from_status`). | **Competing eligibility authority, not rendering.** `detectAgentApprovalPrompt` / `detectAgentApprovalPromptBody` accept if the normalized *whole body* `includes` either heading, and may score `formatted_body` HTML-to-text when it is richer. Used to choose `AgentApprovalCard` (`RenderMessageContent.tsx`), emit dedicated OS notifications, and suppress ordinary room notifications (`ClientNonUIFeatures.tsx`). Inbox and search only pass `agentApprovalTarget` into that renderer (`Notifications.tsx`, `SearchResultGroup.tsx`). | **Competing eligibility authority, not rendering.** `SynaraAgentApprovalPromptDetector.detect` / `detect(body:)` use the same `contains` rule, including `formattedText` via `MatrixHTMLRenderer.sanitizedMarkdown`. Used to choose `AgentApprovalPromptTimelineCard` (`RoomTimelineView.swift` `approvalPrompt`). NSE does *not* re-classify: it consumes Core `is_agent_approval`. | Core: `crates/synara-core/src/app/agent_approvals.rs` tests `classifier_accepts_both_contract_headings` (rejects a later-in-body quoted heading). Desktop: `synara/src/app/utils/__tests__/agentApprovals.test.ts`. iOS: `AgentApprovalServiceTests.swift`. Proven split: Core false / platforms true on a heading that is not the first non-empty line. |
| OS / background action resolution (writes) | **Core authority (hard invariant).** `plan_agent_approval` allows only `agent-approval.approve-once` and `agent-approval.deny`; rejects `approve-always`; enforces timestamp sanity (`origin == 0` or `origin > now + 60s`); expires at `now - origin >= AGENT_APPROVAL_TTL_MS` (5 minutes); treats current-account terminal reactions including `♾` and `❎` as already decided; ignores Hermes bot-owned seed reactions. `NativeTimelineOwner::decide_agent_approval` resolves the exact event, plans, then sends one `m.annotation` under a per-event lock (`live.rs`). Exposed as `matrix_agent_approval_decide` (`core.rs`) and UniFFI `agent_approval_decide` (`shared_core_ffi.rs`). | **Platform observation / routing, not write authority.** `planAgentApprovalNativeNotificationAction` fast-rejects kind/ids, routes Review and approve-always to open-room, and can apply a local TTL. Live handler `AgentApprovalNotifications.handleNativeNotificationAction` then calls `decideAgentApprovalWithNativeOwner` → Tauri `matrix_agent_approval_decide` (`nativeReactionOwner.ts`, `timeline_reactions.rs`). The live call omits `eventTsMs`, so the TypeScript TTL branch is not the write gate. | **Platform observation / routing, not write authority.** `SynaraNotificationActionContract.planAgentApprovalNotificationAction` parses room/event ids, routes Review and approve-always to open-room, and honors local `alreadyActed`. Comment and tests say payload clocks defer to Core. Live `SynaraApp.handleAgentApprovalNotificationAction` then `submitNativeDecision` → `SharedCore.agentApprovalDecide`. `SynaraAgentApprovalNativeActionValidator` still encodes prompt+TTL policy but is not on the live write path (tests only). | Core unit tests in `agent_approvals.rs`; desktop `agentApprovals.test.ts` and `nativeReactionOwner.ts`; iOS `PushServiceTests.swift`, `AgentApprovalServiceTests.swift`. Production smoke `docs/production-smoke-checklist.md` MAC-DESK-007a / IOS-005. |
| In-app once / deny / always writes | **Core does not plan in-app always.** `plan_agent_approval` refuses `agent-approval.approve-always` so an OS action cannot permanently trust a pattern. Reaction ensure/toggle remain generic Matrix writes. | **Accepted platform boundary for always confirmation; residual second write path for once/deny.** `AgentApprovalCard.sendReaction` uses `ensureReactionWithNativeOwner` → `matrix_reaction_ensure`, after a second-click confirm for `♾️`. That path does not call `plan_agent_approval` (no Core expiry / first-line prompt / already-decided lock). | Same split. `RoomTimelineView.submitAgentApprovalReaction` → `SharedCoreAgentApprovalReactionService.submitReaction` → `reactionEnsure`. Haptics after send (`SynaraHaptics`) stay platform rendering. | `AgentApprovalCard.tsx`; `RoomTimelineView.swift`; `SharedCoreProductServices.swift`. |
| Expiry / freshness | **Core authority for writes, but currently hardcoded.** Synara uses 5-minute event age, 60s future skew, `>=` TTL. Hermes defaults to 300 seconds but permits `MATRIX_APPROVAL_TIMEOUT_SECONDS`, so Synara can disagree with the actual server-side prompt lifetime. | Duplicate constants: `AGENT_APPROVAL_NATIVE_ACTION_TTL_MS`, `isAgentApprovalNativeActionExpired` (uses `>` and optional notification-created clock). Used for recent-event notification *emission* (`RECENT_AGENT_APPROVAL_MS`) and planner tests; not the live OS write gate. | `SynaraNotificationActionContract.nativeActionTTL` is unused in the live planner (`now _:`). `SynaraAgentApprovalFreshness.isFresh` (5 min, 60s future, `origin > 0`) gates NSE time-sensitive category / interruption only. | Core planner tests; desktop `isAgentApprovalNativeActionExpired` tests; iOS `NotificationPreviewSupportTests.swift`; Hermes Matrix adapter timeout configuration. |
| Idempotency / duplicates | **Core authority.** Per-event async lock, completed-decision registry, current-account terminal reactions. | **Platform observation.** `createAgentApprovalNativeActionDedupeStore` (localStorage, room+event key) prevents a second native callback from this desktop process. `hasLocalAgentApprovalReactionFromSenders` exists but is test-only and omits `♾` / `❎`. | **Platform observation.** `SynaraAgentApprovalNotificationActionDedupeStore` (UserDefaults, room+event key). | `ApprovalDecisionRegistry` in `live.rs`; desktop/iOS dedupe helpers and tests. |
| Allowed background actions | **Core authority.** Once and deny only. | Action-id tables and `getAgentApprovalReactionForNotificationAction` still map always → `♾️` for a send plan that the live path must not take; native OS buttons omit always (`AGENT_APPROVAL_NATIVE_NOTIFICATION_ACTIONS`). | Category registration omits always (`agentApprovalActions`). Planner still names the always id so a stale payload opens the room. | `desktop_notifications.rs` (button ids / time-sensitive kind); `PushService.swift` `registerCategories`. |
| Structured agent-card approve/reject | **Core write authority for that contract.** `send_agent_approval` validates `approve`/`reject`, ids, and envelope size, then sends `m.notice` + `in.synara.agent.action`. Not the Hermes prompt planner. | Desktop Hermes cards parse structured payloads (`parseHermesAgentPayload`) and render `HermesAgentCard`. Not a second `agent_approvals.rs`. | `SynaraAgentCardActionResolver.plan` maps card kinds to open-URL / copy / `submitApproval`. Live submit uses `SharedCoreAgentApprovals.send`. `encodeAgentApprovalMatrixEvent` remains a test-only encoder of the same notice shape. | Fixture `synara/docs/contracts/fixtures/synara-agent-approval-action.json`; `p4_s10_leftovers.rs`; `AgentApprovalServiceTests.swift`. |
| Room / sender / power / spoofing | Exact event resolve and current-account terminal-reaction checks; no additional prompt-sender authorization is encoded in this planner. First-line heading is its spoofing control. Hermes independently binds prompt resolution to its requesting/authorized sender and expiry, so the client contract must verify whether Core needs equivalent prompt-origin evidence. | Notification emit skips the current user. Card send honors `canSendReaction` (room power observation). Classifier `includes` is the weaker spoofing rule. | Same: detector `contains`; NSE restores category only after Core classification (`docs/agent-approval-notification-proxy-spec.md`). Proxy category/kind cannot grant controls. | Core heading-negative tests; NSE `NotificationService.swift`; companion Hermes Matrix adapter reaction handler. |
| Revocation / redacted audit / response encoding | Decision is an `m.annotation` reaction from `decide_agent_approval`. Generic `matrix_reaction_redact` exists; no approval-specific revocation or redacted-audit policy in this module. | In-app ensure uses the same reaction encoding. No approval-specific redact planner. | Same. `encodeAgentApprovalReactionMatrixEvent` is test-only. | `timeline_reactions.rs`; iOS reaction tests. |
| Cards, sheets, buttons, haptics, OS delivery | Not Core. | **Platform rendering / OS integration.** `AgentApprovalCard` layout and confirm UI; `desktop_notifications.rs` tray/urgency; command/reason/source extraction for display (`extractCommand`, `extractReplyInstructions`). | **Platform rendering / OS integration.** `AgentApprovalPromptTimelineCard`, `AgentApprovalButtonStyle`, `AgentApprovalDetails`; APNs/NSE delivery; `SynaraHaptics`; time-sensitive preference. | ADR 0004 layer map; workstream 08 closed boundary. |

**Earliest actual divergence.** Not the OS write owner (already Core). The first
competing policy is prompt *eligibility*: Core first-line exact heading versus
desktop/iOS whole-body substring, including desktop `formatted_body` fallback.
That divergence decides which events become cards and which desktop events
get an approval-category OS notification. The second residual is in-app
once/deny via `reaction_ensure`, which skips Core plan checks that OS actions
cannot skip.

**Constraint classes.** Shared recognition, background action allowlist, expiry,
and terminal-decision idempotency are hard invariants (correctness / spoofing /
OS-bypass). Cards, sheets, buttons, haptics, APNs/NSE/tray, Review navigation,
and local dedupe stores are accepted platform boundaries. React, SwiftUI, and
the current notification stacks are technology preferences, not reasons to
move UI into Core.

## Boundary constraints

- ADR 0003: one Core Matrix/application owner; no second JS or Swift engine for
  Matrix writes. Presenter projections are not a second domain owner.
- ADR 0004 (2026-09-01): Core owns “recognition, eligibility, expiry and action
  resolution where shared.” Platforms own “cards, sheets, composer and
  notification UI.” Notification *delivery* (APNs, NSE, tray, banners, actions,
  haptics) stays platform. Agent-approval authorization policy is listed as
  Core-shaped authority; that assignment does not delete the TypeScript or
  Swift helpers tonight ([OPERATING.md](../program/OPERATING.md), D3/D9).
- ADR 0005: unused here; no media bytes or paths on `Core::command`.
- Playbook §5: after landed S12–S37 and later optional leftovers, the checklist
  stops. Docs-only PRs remain allowed. Do not invent leftover routes or S38.
  Do not start P5. Do not claim P4 engine-ready.
- Goal-graph stop conditions ([13-language-boundary-goal-graph.md](../../../shared-native-core/13-language-boundary-goal-graph.md)):
  the next required node is the P4 engine-ready *gate* (pending; blocked on
  paused iOS CI and paused live-homeserver proof). Stop on that blocked gate.
  Do not invent S38. Do not start P5.
- NSE stays narrow: classify via Core store preview; never start sync; never
  download media (ADR 0004 invariant 6, ADR 0005).
- Approve-always must remain in-app confirmation. Cards and OS buttons stay
  platform-owned even if recognition is later unified.
- D1–D9: this memo cannot open an implementation gate, amend an ADR, or claim
  a shared-Core phase.

## Alternatives

1. **No ownership change.** Keep platform detectors and leave in-app actions on
   generic `reaction_ensure`. Rejected: current source proves different prompt
   eligibility and proves that in-app once/deny/always skip Core prompt,
   expiry, already-decided, and per-event decision checks. False positives do
   not reliably fail closed when the same platform path writes the reaction.

2. **Bounded authority consolidation (recommended).** Make Core the sole owner
   of prompt eligibility and decision validation for OS and in-app actions.
   Presenters keep command/reason/source display, confirmation UI, haptics,
   routing, and local callback dedupe. Define the action matrix and Hermes TTL
   contract before changing APIs. Generic reactions remain available for
   ordinary reactions, not as an approval-policy bypass.

3. **Broader Core/UI ownership.** Move cards, parsed display strings, OS
   delivery, or confirmation UI into Core. Rejected by ADR 0004. A typed Core
   presentation signal may be considered only if needed to consume the shared
   eligibility result; it must not encode layout or OS behavior.

Stay-put is the default unless harmful duplicated *authority* is proven. Here
it is proven twice: eligibility drift changes which events become actionable,
and in-app generic reaction writes bypass the policy enforced on OS actions.

## Recommendation

**Reopen for bounded authority consolidation:** shared prompt eligibility and
decision validation, not cards or OS delivery.

Confidence: high that duplicate policy exists; medium on the eventual API
shape. Native OS once/deny writes already use Core. Platform classifiers still
compete with `agent_approvals.rs`, and in-app once/deny/always use
`reaction_ensure`, skipping Core prompt/expiry/already-decided checks. Routing,
confirmation UI, and local callback dedupe remain platform observation and
rendering.

Classifier duplication is competing authority for eligibility. The in-app
path is also competing decision authority because it can produce terminal
approval reactions without the approval planner. Structured
`send_agent_approval` is a different contract and is already Core-owned.

Do not proceed with broad Core ownership of cards, command extraction, haptics,
or notification chrome. Do not treat this memo as permission to delete
`agentApprovals.ts` or `AgentActionService.swift` in product code.

Strongest objection (stay-put): OS writes already fail closed in Core and the
remaining divergence could be treated as presentation. That objection fails
for in-app buttons because they write generic reactions directly. The need for
a presenter-visible signal affects API design, not the ownership conclusion.

Unresolved questions:

- Whether in-app once/deny after prompt expiry should be forbidden consistently
  with Hermes’s configured timeout.
- Whether desktop `formatted_body`-only prompts exist on the wire without a
  matching plaintext first line (would be Core-negative in NSE and decide).
- How Synara can represent Hermes’s implemented typed `!approve session`
  action. Hermes has no session reaction, so Synara must not infer one; this
  requires either a coordinated structured protocol extension or an explicit,
  correctly bound typed-command route.
- Shared golden/adversarial prompt fixtures for first-line versus substring
  cases are not yet a portfolio artifact (design note only: a small corpus
  under `docs/future-projects/**` if a later accepted memo asks for it).

Regression proof to keep any later decision stable: Core heading positives and
the quoted-later negative; OS once/deny only via `decide_agent_approval`;
approve-always absent from OS categories and rejected by `plan_agent_approval`;
Hermes seed reactions ignored; current-account `♾`/`❎` already decided;
platform cards still render command/reason locally; NSE category restored only
from Core `is_agent_approval` plus freshness observation.

## Next gate

The original recognition-only gate is superseded by
[A2](../program/ACTIONS.md#a2--hermes-approval-contract-and-authority). Re-read
the current Hermes prompt, reaction, typed-command, authorization, and timeout
contract; build shared spoofing/expiry/idempotency fixtures; define once, deny,
always, and session semantics; and then plan the smallest Core-owned eligibility
and decision route consumed by both OS and in-app actions. Cards, confirmation
UI, haptics, and delivery stay platform-side. No code follows directly from
this memo.
