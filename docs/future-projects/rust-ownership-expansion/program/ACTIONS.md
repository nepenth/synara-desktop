# Residual action register

Status: authoritative record of residual findings from the 2026-09-01
ownership census, reconciled and remediated where proven on 2026-09-02. The
original census was docs-only. A human explicitly authorized the subsequent
product remediation on `feature/rust-ownership-residual-census`; that later
authorization does not retroactively turn the research memos into implementation
plans.

## How to use this register

Each action keeps the accepted ownership boundary separate from the work still
needed. `proof-gate` means the architecture may be correct but the behavior is
not considered proven. `product-defect` means current source shows a user-
visible correctness failure. `parity` means an existing Core model is not fully
transported or consumed by a presenter. `product-decision` means source alone
cannot choose the desired behavior.

Priority is risk ordering for investigation, not release authorization.

## Completed by the original research run

- Current-source ownership censuses for ROE-01 through ROE-12.
- Docs-only worker PRs `#1081`–`#1091` with independent verdicts.
- Ownership taxonomy and stay-put alternatives for desktop, iOS, and Core.
- This reconciliation of accepted nits, transient run records, and residual
  actions.

That statement describes the original 33-commit research run. The later
human-authorized remediation is tracked independently in the `Status` column
below. A deterministic implementation is not promoted to live proof when the
required external environment or second client was unavailable.

| ID | ROE | Kind | Ownership verdict | Residual action | Priority | Status |
| --- | --- | --- | --- | --- | --- | --- |
| A1 | ROE-10 | product-defect | Core owns reply/thread target validation and metadata; composer UI stays platform-side | Repair desktop send-time consumption so a visible reply banner always produces the intended Matrix relation for text, attachments, polls, and GIFs | P0 | closed: every send and manual cancellation compares the exact Core-issued draft revision, including repeated or classic/threaded selections of the same event; the standalone upload route snapshots that revision in its own scope; iOS text/media/retry routes preserve child plus thread-root identity; focused executable tests and full desktop type validation passed |
| A2 | ROE-08 | security/authority | Core owns approval eligibility and decision policy; cards and OS delivery stay platform-side | Route in-app and OS decisions through exact-event Core validation; retain deliberate confirmation for permanent approval | P0 | partially closed: one serialized Core write authority is implemented; signed bot identity, Hermes-configured expiry metadata, and live Hermes readback remain open protocol/evidence work |
| A3 | ROE-02 | proof-gate | Core remains the sole verification/SAS/trust state machine | Prove current-device own-identity verification end to end and durable verified readback after relaunch | P0 | open proof gate: encrypted, fail-closed local diagnostics are ready; live credentials/authority proof still required |
| A4 | ROE-05 | product-defect/privacy | Core owns unread counts and receipt writes; platforms own genuine visibility observations | Prevent stale or inactive observations from marking messages read and reconcile explicit/automatic privacy semantics across clients | P1 | implementation complete; the retired iOS HTTP/public-receipt writer is removed; two-client live receipt proof remains open |
| A5 | ROE-09 | data-integrity | Core is the sole notes/account-data codec and writer; editors and gestures stay platform-side | Define and enforce version, size, sequential-write, local-projection, and cross-device conflict behavior | P1 | deterministic integrity remediation complete, including all mutation-target bounds and delayed pre-PUT projection expiry; concurrent two-device live proof remains open |
| A6 | ROE-04/12 | security/proof-gate | Output-context sanitization and rendering remain platform-side; Core owns shared protocol semantics and bounds | Run a shared Matrix/Hermes golden and adversarial fixture corpus through both presenter harnesses | P1 | partially closed: shared corpus, harness integrations, and bounded send/edit/caption/mention inputs are implemented; redaction, late-decryption, pagination, and full relation matrix expansion remain open |
| A7 | ROE-03 | parity | `TimelineViewRow` is the semantic row owner; presenters own native rendering | Close iOS DTO/adapter omissions for authoritative sender, thread/reply, poll, reaction, and capability fields | P1 | DTO and adapter parity implemented, including iOS thread-root send transport and shared desktop message/poll/sticker relation presentation; poll voting and report/forward/decline-call UI remain explicit follow-on product work |
| A8 | ROE-06 | product-decision | Core owns room facts; navigation topology, locale sort, and filter chrome stay platform-side | Decide whether iOS “Mentions” excludes manually marked-unread rooms and lock the choice with presenter tests | P2 | closed: Mentions requires an actual highlight; manually marked-unread rooms remain in Unread only |
| A9 | ROE-07 | evidence/product | Push-rule and pusher operations use Core; APNs/NSE/tray delivery and local privacy remain platform-side | Treat notification-preview reliability as a separate delivery investigation with live evidence; do not infer reliability from ownership | P2 | open |
| A10 | ROE-11 | evidence/future | ADR 0005’s opaque handle and dedicated byte-channel split remains correct | Measure original-byte fetch cost, caps, retry behavior, and cache need before projecting metadata or activating the unused cache harness | P2 | open |
| A11 | ROE-01 | hygiene | Matrix lifecycle, sync, and crypto ownership is correctly centralized | Remove stale direct Matrix Swift package linkage and keep the generated project/lock policy fail-closed | P3 | closed: all Apple Core slices regenerated transactionally; the all-local Xcode graph passed, and the final iOS unit target executed 657 tests (655 passed, 2 intentionally skipped) |

## Required acceptance evidence

### A1 — reply relation integrity

- One authoritative reply target is consumed by every desktop send path.
- Cancelling and sending compare the displayed Core-issued draft revision, so a
  delayed completion cannot clear a later selection even when its event and
  relation fields are identical.
- Tests assert the emitted Matrix relation for text, attachment, poll, and GIF
  sends, plus reply-in-thread behavior.
- A visible reply banner can never silently degrade to an ordinary message.

### A2 — Hermes approval contract and authority

- Re-read the current Hermes Matrix adapter and slash-command implementation;
  fixtures cover the exact heading, sender/room/event binding, expiry, already-
  decided reactions, and spoofed or quoted prompt text.
- Core owns prompt eligibility and decision validation for both OS and in-app
  actions. Generic reaction writes are not an approval-policy bypass.
- Once, deny, always, and session form an explicit action matrix. Hermes
  currently has typed `!approve session` behavior but no equivalent session
  reaction; do not emulate it without a coordinated protocol decision.
- Synara does not hardcode five minutes if Hermes is configured to another
  timeout. Expiry and clock-skew behavior are deterministic and tested.
- “Always” retains deliberate in-app confirmation and all actions remain
  idempotent and bound to the exact prompt.

### A3 — current-device verification

- Start the own-identity route with `device_id = nil` from an unverified
  current device against an already cross-signed authority device.
- Reach `KeysExchanged`, prove `can_be_presented()`, and display identical emoji
  or decimal SAS on both devices.
- Confirm on both sides, complete the flow, and read
  `Encryption::verification_state() == Verified`.
- Relaunch the app and prove the verified state remains durable. Capture
  privacy-safe diagnostics for any earliest divergence.

### A4 — read/privacy contract

- Auto-read requires active application/scene, selected room, live bottom,
  stable tail, and no incompatible interaction.
- Cancellation clears both pending and last-candidate state; leaving a room or
  scrolling away cannot later flush a stale read.
- Explicit Mark Read versus automatic receipts have one documented
  `hideActivity` policy on desktop and iOS.
- Transition-race tests and a two-client Synapse proof verify the actual
  receipt and unread outcomes.

### A5 — notes integrity

- A future unknown version is never normalized and written back as v1 with
  unknown data silently removed.
- Two-device concurrent add/edit/delete and reorder cases have a documented
  merge, conflict, or explicit last-write-wins contract.
- Deletion/tombstone, idempotency, retry/offline behavior, item and room bounds,
  and maximum global payload are covered by contract and live tests.

### A6/A7 — presentation fixtures and adapter parity

- Shared legitimate and adversarial Matrix/Hermes fixtures run through both
  desktop and iOS presenter test harnesses.
- Expected equivalence is semantic/security behavior, not pixel-identical UI.
- The corpus covers nested edits/replies/threads, reactions, polls, redactions,
  malformed relations, late decryption, pagination, mentions, spoilers, links,
  lists, inline/preformatted code, tables, and formatted reply fallback.
- `formatted_body` is never documented as universally pre-sanitized unless
  source actually establishes that guarantee.

## Promotion rule

The research record and the later remediation may be promoted only after the
state documents distinguish deterministic implementation from live evidence,
tests proportionate to the actions pass, independent review is recorded, and
any changed boundary has the required ADR amendment. No boundary in this pass
required an ADR change: the work removed duplicate authority or completed an
already accepted Core/platform transport.

The final deterministic validation record is: Core's 836-test unit target
passed 833 with 3 environment-gated tests ignored, every Core integration
binary and doc test passed, desktop modernization passed 910/910 after a
production renderer build and typecheck, the Tauri application compiled with
the revised Core boundary, and the exact promoted head's iOS targets passed
655 with 3 skips out of 658 unit tests and 59 with 14 skips out of 73 UI tests
against the regenerated four-slice Apple package. The offline
cold-restart proof now closes the first SDK client through `Client::pause()`
before rebuilding from the same store; it no longer relies on a guessed sleep
to release persistent-store ownership.

The final Grok 4.6 High review found one attachment-send revision-scope defect.
After that route was corrected and validated by its focused executable suite
and the full desktop typecheck, targeted Grok re-review returned no P0-P2
findings. This review evidence does not close any live Matrix proof gate.
