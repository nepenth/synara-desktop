# Rust Ownership Expansion — Portfolio Triage

Status: research triage; not approved for implementation.

| Field | Value |
| --- | --- |
| Reviewed | 2026-09-01 |
| Source census | `main` at `7bc6e420` |
| Scope | Charter, twelve workstream briefs, agent guide, plan template, ADRs 0003–0005, shared-core playbook and goal graph, and current Rust / desktop / iOS source |
| Verdict | Keep the research container. Do not treat the twelve workstreams as an implementation program. |

This page is the first reading for the portfolio. It records an adversarial
review of the [charter](README.md). Current source, tests, and accepted ADRs
take precedence over this memo. This memo does not authorize product changes,
new Core routes, UniFFI surface growth, or an ADR edit.

## Governance precedence

When this memo and another document disagree about *what to do next*:

1. [ADR 0004](../../adr/0004-rust-language-boundaries.md) decides what may be
   written in Rust.
2. [ADR 0003](../../adr/0003-shared-native-rust-core.md) decides that desktop
   and iOS share one `synara-core`.
3. [ADR 0005](../../adr/0005-native-media-handle-channel.md) decides how media
   bytes move (dedicated handle channel, never `Core::command`).
4. The [implementer playbook](../../shared-native-core/11-implementer-playbook.md)
   §5 and the [language-boundary goal graph](../../shared-native-core/13-language-boundary-goal-graph.md)
   decide current engineering priority on `main`.
5. This directory is optional post-gate exploration. It is not a parallel
   SCOREBOARD, not P4-S38+, and not a shared-native-core slice ledger.

Playbook §5 step 4 and the goal graph currently stop at the **P4 engine-ready**
gate (blocked on paused iOS CI and live homeserver proof). They say do not
invent S38 and do not start P5. Research in this directory must not become a
way to stay busy on `main` while that gate is blocked, unless a human explicitly
charters a docs-only investigation.

## Executive recommendation

The architectural shape in the charter is **already the production
architecture**, not a future redesign:

```text
matrix-rust-sdk → crates/synara-core → typed Tauri DTOs/events → React
                                    → typed UniFFI models      → SwiftUI
```

Adopt the directory and the research rules. Do not adopt the twelve-lane table
as a backlog.

Most workstreams should close after a short census as **already correctly
owned** or **stay platform-side**. The live residual problems are narrower:

1. Agent-approval detection and notification planning are still duplicated
   across Rust, TypeScript, and Swift (ROE-08).
2. Inbound Matrix HTML policy is implemented twice: desktop sanitizes HTML
   into React; iOS already builds a typed semantic tree in
   `MatrixHTMLRenderer` (ROE-04 / ROE-12).
3. Receipt writes and unread counts are Core-owned; *when* to mark a room
   read is still presenter policy (ROE-05).
4. Room sort/filter helpers exist in Core and are unused by product UIs
   (ROE-06). That may be correct device-local ownership, not a migration.

A Core-owned semantic message tree (the portfolio-level ROE-04 hypothesis) is
the highest-risk idea and should stay **stay platform-side by default**. A
shared fixture corpus does not need an ADR. A Core AST does.

---

## 1. What this idea actually is

The charter asks which remaining cross-client product semantics should move
from presenters into `crates/synara-core`. That is a residual-duplication
question. It is not “should we have a Rust brain?” ADR 0003 already decided
that, and `main` already implements it.

Timeline rows, sync supervision, verification, notes, reply drafts, unread
counts, and agent-approval *decisions* already live in Core. Desktop React and
iOS SwiftUI are presenters over that owner. The remaining question is which
leftover *policies* still have two authors, and whether that duplication is a
problem or an intentional platform boundary.

The charter already says research may conclude **already correctly Rust-owned**,
**stay platform-side**, or **delete rather than migrate**, and that
implementation needs review and possibly a new ADR. Those rules are sound.
The risk is that twelve numbered “candidate workstreams” plus a 15-section
implementation template look like authorized work.

---

## 2. Is the rationale reasonable?

Yes, if this is a census of leftover dual owners. No, if this is twelve new
Rust-ownership programs.

**Reasonable.** Desktop and iOS should not independently invent
Matrix-correctness, crypto, approval eligibility, or unread truth. Shared
policy in Rust and native rendering on each platform is the modern client
pattern. The charter’s questions about presenter projections versus competing
business logic, and about FFI cost exceeding parity benefit, are the right
brakes.

**Overstated.** Several briefs are written as if ownership is still open when
source already answers them. See [§6](#6-workstream-priors).

**Lineage the charter omitted.** The UX audit
([`docs/ux-audit/desktop-cross-client-audit.md`](../../ux-audit/desktop-cross-client-audit.md)
item 11) already proposed a cross-client rich-content contract and put a
possible Core semantic tree **last**: build a parity fixture corpus, then
decide whether a tree is justified. ROE-04 jumped that queue and elevated the
tree to a portfolio-level hypothesis.

---

## 3. Is the high-level layout technically sound?

Yes. Do not change the topology.

What exists today is a fail-closed Matrix client split that matches accepted
ADRs and modern practice:

- One concurrency owner in Rust (`NativeTimelineOwner`,
  `NativeVerificationOwner`, sync service, and related owners).
- Versioned projections (`TIMELINE_VIEW_SCHEMA_VERSION`, sequenced
  snapshot/delta, 1 MiB envelope cap).
- Commands for mutation, snapshots for query.
- iOS wake-and-refetch versus desktop event streaming over the same owner
  (different latency profiles; both valid).
- Secrets, paths, and media bytes kept off `Core::command` (ADR 0004 / 0005).

A Rust UI toolkit, a second core crate, or putting HTML widgets in Core would
be a step backward. The charter correctly forbids those.

What to change in the *description*: stop presenting the diagram as a proposed
future shape. Call the project a **post-cutover residual census**. The brain
already exists.

### Ownership taxonomy

Force every leftover behavior into one bucket before proposing a move:

| Bucket | Owner | Examples |
| --- | --- | --- |
| Authority | Core | Protocol truth, state machines, eligibility, bounds, account-data schema, receipt writes |
| Observation | Platform | Viewport visibility, focus, notification banners, composer keystrokes, locale UI, OS delivery |
| Rendering | Platform | React / SwiftUI widgets, selection, accessibility, Prism, Dynamic Type |

Most “should this move?” confusion disappears once a behavior is forced into
one of those three buckets.

---

## 4. Process and document design

The research container is unusually careful. The *shape* still copies an
implementation program.

### What is good

- Explicit non-authorization language, linked from
  [`docs/README.md`](../../README.md).
- Census-before-conclusion.
- Valid negative outcomes, including delete-rather-than-migrate.
- Cross-client requirement (desktop macOS, desktop Linux, and iOS) even when
  a change touches one shared component.
- Adversarial review required, including an argument for staying
  platform-side.
- “Draft a new ADR; do not silently reinterpret ADR 0004.”
- Non-goals that correctly reject Rust UI frameworks, secret/byte envelopes,
  and speculative Core routes.

### Concerns

1. **Looks like a backlog.** Twelve numbered IDs, “candidate workstreams,” a
   planning-ready bar that demands Synapse proofs and delivery slices, and an
   agent guide that says copy the full template into `plans/`. Agents optimize
   for filled templates. “Stay platform-side” is one sentence; plan-template
   sections 8–12 are a migration.
2. **Recommendation comes after delivery design.** The investigation sequence
   designs slices before it asks for a go/no-go. That biases toward “proceed.”
3. **No binding to the playbook stop gate.** Nothing says this directory does
   not compete with P4-engine-ready / “do not invent S38.”
4. **ROE-04 is elevated too early.** A concrete node list in the charter
   reads as product direction, even though the text says evaluate-don’t-assume.
5. **ROE-08 omits ADR 0004 item 6.** Agent *policy* in Core is optional only
   after iOS-on-engine. A decide command already exists; further consolidation
   is still sequenced behind that gate if the ADR is taken literally.
6. **ROE-11 does not cite ADR 0005.** Research can accidentally reopen the
   closed byte-channel decision.
7. **`plans/` is referenced and does not exist.** No naming, review, or
   docs-only rule.
8. **Surrounding SNC inventory numbers are stale.** The playbook still cites
   older registered/leftover counts; source is larger. The portfolio’s “census
   source, not docs” rule is therefore necessary, not ornamental.
9. **Hypothesis phrasing points toward a move.** Every brief starts from
   “should have one Rust owner.” The default should be stay-put unless
   duplication is proven harmful.

### Recommended document changes (not done in this commit)

These remain optional follow-ups. This triage is the first control.

- Add the precedence block above to the parent
  [`future-projects/README.md`](../README.md) and the charter.
- Collapse twelve lanes to unordered research clusters (see [§8](#8-recommended-clusters)).
- Split a short research-memo template from the 15-section implementation plan.
  Use the full plan only after “proceed” plus any required ADR.
- Default recommendation: stay platform-side unless proven otherwise.
- Mark ADR gates on the cover of ROE-04 (Core tree), ROE-10 beyond reply
  metadata, ROE-11 byte/path questions, and ROE-08 (after iOS-on-engine).
- Operating rules: docs-only under `docs/future-projects/**`; no
  `crates/synara-core` edits from research; one deep lane at a time.
- Define `plans/` naming if research memos start landing.

---

## 5. Current-state census (source-linked)

Verified against `main` at `7bc6e420`. Historical playbook counts and dated
ADR inventories are not live inventory.

| Domain | Rust / Core today | Desktop today | iOS today |
| --- | --- | --- | --- |
| Sync, restore, crypto status, backup/cross-signing status | `app/lifecycle/`, `app/sync/`, `app/backup/`, `app/cross_signing/`, registered `matrix_*` status/setup-start commands | Thin Tauri bridges; password, keyring, recovery, and media-byte leftovers stay shell-side | `SharedCore*` session/sync wrappers; leftover recover/media/pusher fail-closed |
| Verification | `app/verification/live.rs` (`NativeVerificationOwner`), `matrix_verification_*` | Presentation helpers in `nativeVerification.ts` | `SharedCoreVerification*`; extra device-key continuity policy in `MatrixClientPolicies.swift` |
| Timeline rows / relations | `app/timeline/live.rs`, `view.rs` (`TimelineViewRow`) | Viewport + HTML render (`nativeTimelineViewportPolicy.ts`, `nativeTimelineRichText.ts`) | `SharedCoreTimeline*`; `MatrixHTMLRenderer` semantic segments |
| Formatted body | Outbound `markdown_to_html()`; inbound `formatted_body` string passthrough | Sanitize-html → React parse + Prism | Typed semantic tree already shipped (release 2.1.11) |
| Read / unread | Room-list counts, `matrix_timeline_set_read_state`, `matrix_room_set_read_state` | Auto-read / focus gating in viewport policy | Foreground/background + `SharedCoreReadMarkers` |
| Room sort / filter | `app/room_list/sort.rs` and `filters.rs` exist; **not used by product UIs** | Device-local sort in `homeRoomList.ts` | Swift reimplementation in `RoomListService.swift` |
| Notifications | Push rules and room notification modes in Core | OS delivery in `desktop_notifications.rs` | APNs / NSE platform-owned |
| Agent approval | `app/agent_approvals.rs` (`is_agent_approval_prompt`, `plan_agent_approval`); `matrix_agent_approval_decide` | Parallel planner in `agentApprovals.ts` | Parallel planner in `PushService.swift` / notification contract |
| Notes | `matrix_room_notes_*` on Core | Presenter panel only | `SharedCoreRoomNotes` + SwiftUI editor |
| Drafts | Reply/thread draft commands on the timeline owner | Slate / Jotai composer body is local-only | Core reply draft + local SwiftUI composer state |
| Media | Opaque `TimelineMediaHandle`; bytes off `Core::command` (ADR 0005) | Shell / `synara-media://` resolve | UniFFI byte channel by handle |
| Validation | Envelope `deny_unknown_fields`, IDs, notes limits, outbound HTML sanitize | Display HTML sanitizer + desktop route sanitize | `MatrixHTMLRenderer` allowlist |

`TimelineViewRow` is already a typed semantic **row** model (message, poll,
redacted, reply, thread, reactions, media handles, bounded `agent_card_json`).
It is not a formatted-body AST.

The `formatted_body` field comment in `crates/synara-core/src/app/timeline/view.rs`
calls the HTML “already-sanitized.” Presenters re-sanitize. That comment
overstates Core’s safety guarantee.

---

## 6. Workstream priors

Treat these as research priors, not implementation tickets. A later memo may
change a prior only with a source-linked census that contradicts this table.

| ID | Prior | Why |
| --- | --- | --- |
| ROE-01 | **Already owned** | Engine and sync supervisor are Core. Remaining leftovers are secrets/bytes on the desktop shell by design. Census, not a migration. |
| ROE-02 | **Already owned** | `NativeVerificationOwner` is the state machine. Screens stay platform. Optional leftover: iOS device-key continuity policy. |
| ROE-03 | **Extend existing rows** | Normalization is largely `TimelineViewRow`. Do not add a second layer. Do not move scroll or virtualization (ADR 0004). |
| ROE-04 | **Stay platform-side by default** | See [§7](#7-typed-semantic-message-model-roe-04). Shared fixtures need no ADR. A Core AST needs a replacement ADR and is probably the wrong layer. |
| ROE-05 | **Bounded remainder** | Core already writes receipts and counts. Formalize the visibility contract. Do not move viewport math. |
| ROE-06 | **Stay platform-side for UI** | Device-local sort and tabs are presenter work. Optional golden vectors if drift matters. Do not centralize locale in Rust (`to_lowercase` is not `localizedCaseInsensitiveCompare`). |
| ROE-07 | **Policy yes, delivery no** | Push-rule settings are already Core. APNs, NSE, and tray stay platform. Critical-approval overlaps ROE-08. |
| ROE-08 | **Highest real residual** | Core owns the decide path; classifiers and notification planners are still duplicated. Cards stay React/SwiftUI. Sequence further consolidation behind iOS-on-engine if ADR 0004 item 6 is honored. |
| ROE-09 | **Already owned** | Schema and CRUD are Core. Verify iOS is fully on SharedCore. Do not build a second notes engine. |
| ROE-10 | **Split** | Reply metadata is Core. Composer body stays local unless product requires cross-device rich drafts. Moving Slate/Swift editor state would violate ADR 0004. |
| ROE-11 | **Metadata only** | Handles and metadata are Core. Paths and bytes must not enter the generic envelope (ADR 0004 / 0005). |
| ROE-12 | **Spec, not a sanitizer crate** | Protocol validation and bounds already live in Core. Display sanitizers stay per renderer, with one shared allowlist spec and fixtures. Fix the “already-sanitized” DTO comment. |

If engineering value is ranked after P4-engine-ready: **ROE-08, then shared
fixtures (ROE-04 / ROE-12), then the read-visibility contract (ROE-05).**
Everything else is census-and-close.

---

## 7. Typed semantic message model (ROE-04)

This is the interesting idea and the one that should not be green-lit as
specified.

### Current split

- Core projects `formatted_body` as an HTML string.
- Outbound markdown → HTML is already Rust (`markdown_to_html` via Ruma).
  That is the right place for *send*.
- iOS already has a platform semantic tree
  (`MatrixHTMLRenderer.segments`) with a large conformance suite, shipped in
  2.1.11.
- Desktop still does `sanitize-html` → React parse, plus Prism.

So the real question is not “should messages be a tree?” iOS already answered
yes, locally. The question is “must Core emit that tree?”

### Why a Core-owned tree is a poor default

- It conflicts with ADR 0004 (“Markdown/HTML render stays TypeScript”) in
  practice, even if nodes are labeled UI-neutral.
- Received Matrix bodies are HTML, not Markdown. A Rust markdown parser does
  not solve inbound fidelity.
- A full tree on every timeline snapshot fights the 1 MiB envelope and UniFFI
  copy cost, and forces schema churn on both renderers.
- Selection, VoiceOver / Dynamic Type, spoiler reveal, copy/paste, and Prism
  still live in the presenter. Core would own a half-renderer.
- Presenter sanitizers would still be required for DOM and attributed-text
  output contexts. Threat models are not identical.

### Narrower alternative

1. **Shared golden corpus first** (no ADR, no Core API): one fixture set of
   Matrix/Hermes HTML, expected semantic structure, and adversarial/malformed
   cases, run against desktop and iOS. This is what the UX audit already asked
   for.
2. **Then**, only if the corpus proves material drift, add small structured
   fields on `TimelineViewRow` where HTML parsing is security-relevant:
   mention IDs, validated link URLs, spoiler reason, maybe a
   reply-fallback-stripped flag. That extends the row model that already
   exists.
3. **Do not** put a full AST in Core unless field additions cannot fix
   user-visible drift — and then write a replacement ADR *before* any API
   sketch or UniFFI work.

Shallow research (census, cost model, fixture design) does not need an ADR.
A Core semantic-tree type does.

---

## 8. Recommended clusters

The numbered table implies priority. It should not. If later research needs
structure, use unordered clusters rather than twelve parallel lanes:

| Cluster | Today’s IDs | Default outcome |
| --- | --- | --- |
| Residual engine census | ROE-01, ROE-02, parts of ROE-03 | Close as already owned |
| Message format and safety | ROE-03, ROE-04, ROE-12 | Shared fixtures; no Core AST |
| Read and list semantics | ROE-05, ROE-06 | Visibility contract; keep sort UI local |
| Notifications and agent policy | ROE-07, ROE-08 | Dedupe policy; keep OS delivery and cards local |
| Account-data and drafts | ROE-09, ROE-10 | Notes done; composer body stays local |
| Media metadata | ROE-11 | Subordinate to ADR 0005 |

Do not open a deep investigation in more than one cluster at a time.

---

## 9. Opportunities

- **Shared fixtures beat shared types.** A documented Matrix formatted-body
  corpus would reduce ROE-04 / ROE-12 risk immediately and does not touch
  ADR 0004.
- **`TimelineViewRow` is already the semantic model at the event/row layer.**
  ROE-04 should start there, not from a greenfield AST.
- **Delete is underused.** ADR 0004 already says stale WASM / IndexedDB paths
  are delete-not-rewrite. The portfolio allows that outcome but does not give
  it a home.
- **Playbook leftover commands are a closed set**, not research bait. Do not
  let ROE-01 re-propose putting passwords or attachment bytes on
  `Core::command`.
- **iOS-on-engine remains the highest-leverage accepted Rust outcome.** This
  portfolio should not displace that gate.
- **Triple-duplication is the only high-ROI consolidation:** wire desktop and
  iOS approval planners to the existing Rust policy (`agent_approvals.rs`)
  after the iOS-on-engine sequencing question is answered. Keep cards in
  React and SwiftUI.
- **Fix the misleading DTO comment** (`formatted_body` is not
  already-sanitized) as docs honesty, not as a reason to move rendering into
  Core.

---

## 10. Dangerous ideas to keep closed

| Idea | Why it stays closed |
| --- | --- |
| Full semantic HTML tree in Core | ADR 0004 conflict, FFI bloat, half-renderer |
| Cache file paths on the generic envelope | ADR 0004 / 0005; secret and path leakage |
| DOM / attributed-text sanitization as Core output | Does not remove presenter sanitizers; creates UI instructions in Core |
| Locale / collation centralized in Rust | Worse UX than platform collation; sort UI is presenter-owned |
| Live composer / Slate state in Rust | Keystroke FFI; ADR 0004 composer stay-put |
| Dual owners during cutover | Classic drift; any future slice must remove the old owner |
| Passwords, recovery material, or media bytes on `Core::command` | Explicit non-goal and playbook leftover rule |
| ROE-01 as a new orchestration program | Reinvents the completed shared-core playbook |
| Inventing S38 or starting P5 from this directory | Goal-graph stop conditions |

---

## 11. How to use this portfolio after triage

1. Read this page, the [charter](README.md), and the relevant ADR before any
   workstream brief.
2. Do not write a full [PLAN-TEMPLATE.md](PLAN-TEMPLATE.md) for a workstream
   whose prior in [§6](#6-workstream-priors) is **already owned** or **stay
   platform-side**, unless new source evidence overturns that prior.
3. If a prior is **bounded remainder** or **highest real residual**, write a
   short research memo first: census, ADR class, alternatives, recommendation,
   blockers. Use the full plan template only after a proceed recommendation
   and any required ADR.
4. Product code, new Core commands, and UniFFI changes are out of bounds for
   research PRs. Docs under `docs/future-projects/` only.
5. If a recommendation would change an accepted boundary, draft a new ADR as
   part of the proposal. Do not edit ADR 0004 in place.

Valid memo outcomes remain: already correctly owned, stay platform-side,
extract a bounded subset, proceed with Core ownership, or blocked on an ADR
or product decision.

---

## 12. What this review would adopt

- The directory, the non-authorization rules, and the valid-negative-outcomes
  rule.
- The no-Rust-UI / no-second-core / no-secret-envelope non-goals.
- The desktop-plus-iOS consequence rule.
- The adversarial-review requirement.
- The link from the documentation guide.

It would not adopt the twelve-lane program as written.

Single-sentence steer: this is a gated census of leftover dual owners on top
of an architecture that already exists; most lanes should close; the only
idea that needs an ADR to go further is a Core-owned message tree, and that
idea is probably wrong.
