# Reconciled program state

Last updated: 2026-09-02.
Research run: completed 2026-09-01 on
`feature/rust-ownership-residual-census`, based on `main` `011cf39a`.
Remediation pass: explicitly human-authorized 2026-09-02 on the same branch.

## Headline

All scheduled ownership memos were researched and independently reviewed.
Their principal boundary conclusion holds: Synara should not create a second
Core, move native rendering into Rust, or treat adapters as competing engines.

The original completion statement was too broad. It closed entire workstreams
when only the duplicate-ownership question had been answered. The promotion
review reopened confirmed defects, parity gaps, product decisions, and missing
proof in [ACTIONS.md](ACTIONS.md). A later, explicit human instruction
authorized remediation; the register now separates implemented corrections
from live-evidence gates that still cannot honestly be closed.

## Ownership outcomes

- ROE-01, ROE-02, ROE-03, ROE-09, ROE-10, and ROE-11 found an existing Core
  owner or an intentional Core/platform split.
- ROE-04/12 keeps output-context rendering and sanitization platform-side,
  with shared semantic/security fixtures required before stronger claims.
- ROE-05 keeps visibility observation platform-side and receipt authority in
  Core.
- ROE-06 keeps locale sorting, navigation topology, and filter chrome native.
- ROE-07 keeps push-rule/pusher operations in Core and OS delivery native.
- ROE-08 found genuine duplicated approval eligibility and decision policy;
  its original recognition-only recommendation is reopened.

## Remediation outcome

- A1 has one Core-backed reply owner across desktop text, attachment, poll,
  GIF, and reply-in-thread sends. Send completion and manual cancellation
  atomically compare the displayed Core-issued draft revision before clearing
  it, including repeated or classic/threaded selections of the same event. A
  final-review finding in the standalone upload handler was repaired by
  snapshotting that revision in the same lexical scope as its relation; focused
  executable tests and the full desktop typecheck passed. iOS text, media,
  queue, and retry paths retain both the replied-to child and authoritative
  thread root.
- A2 now has one exact-event, serialized Core decision route across desktop,
  iOS, in-app, and background entry points. Hermes still does not emit signed
  bot identity or configured-expiry metadata, so those protocol questions and
  live readback remain open.
- A3's current-device harness now defaults to the authoritative own-identity
  route and uses encrypted, private, fail-closed proof storage. The live
  verification result is still unproven without the external test authority.
- A4 requires an exact observed live tail for automatic reads, cancels stale
  iOS observations, and gives explicit Mark Read one cross-client privacy
  contract. The retired iOS HTTP writer that could emit public `m.read` was
  removed, leaving the Core private-receipt route as the production writer.
  Two-client live evidence remains open.
- A5 rejects unknown/oversized account data before destructive writeback,
  serializes fresh-server read-modify-write operations, and protects a
  successful local result from stale SDK projection. Non-matching delayed
  pre-PUT events cannot gain a second overlay lifetime, and every mutation
  target uses the same room/item bounds before network I/O. The documented v1
  policy remains whole-document last-write-wins at the server; only the local
  read-your-write projection is bounded to 30 seconds. No merge protocol is
  invented.
- A6 added one shared semantic/security corpus for the desktop and iOS
  harnesses. Core now rejects, rather than truncates, combined plain/formatted
  text and caption payloads above 65,536 UTF-8 bytes and bounds raw mention
  cardinality and identifiers before parsing; the action remains partial until
  the complete adversarial matrix is present.
- A7 transports Core-owned sender, reply, thread, poll, reaction, and
  capability presentation without dropping non-message event metadata.
  Desktop message, poll, and sticker rows share the typed reply/thread/reaction
  presenter. The existing reply/edit/redact/react/pin UI consumes its
  applicable gates;
  poll voting and report/forward/decline-call UI remain explicit follow-on
  product scope.
- A8 is decided and tested: marked-unread alone is not a mention.
- A11 removes the retired direct Matrix Swift package edge and makes CI lock
  behavior derive semantically from whether the reachable generated package
  graph actually has remote packages. The Core Swift/XCFramework pair was
  regenerated across device, simulator-arm64, simulator-x86_64, and macOS.
  After the final relation and receipt cleanup, the exact promoted head's iOS
  unit target executed 658 tests: 655 passed and 3 were intentionally skipped.
  Its iOS UI target executed 73 tests: 59 passed and 14 were intentionally
  skipped.

The final deterministic validation also passed Core's 836-test unit target
(833 passed, 3 environment-gated tests ignored), every Core integration binary
and doc test, desktop modernization 910/910 after typecheck and production
build, and the complete Tauri compile. The offline cold-restart proof was
repaired at the Matrix SDK store-lifecycle boundary: the first client now
awaits `Client::pause()` before the same persistent root is reopened, replacing
the nondeterministic `drop` plus guessed delay.

Notification-delivery reliability (A9), media measurement/cache policy (A10),
the remaining A2/A3/A4/A5 live proofs, and the explicit A6/A7 follow-ons remain
open. Deterministic tests must not be cited as those live proofs.

## Promotion status

The memos and original run remain docs-only research artifacts. The 2026-09-02
product changes were separately authorized and implemented in the remediation
change set. Independent implementation review completed and its concrete P1
reply-revision finding was remediated. The final Grok 4.6 High review then
identified an attachment-send revision-scope defect; that path was corrected,
passed the focused executable suite and full desktop typecheck, and a targeted
Grok re-review returned no P0-P2 findings. Consolidated deterministic
validation is green. PR `#1092` promoted exact head `ce77bdcc` through 13
applicable successful checks with zero failures; four scope-inapplicable checks
were skipped and are not counted as evidence. The PR merged as `e9b5016e`, and
both local `main` and authoritative `origin/main` resolved to that merge commit
on 2026-09-02. Neither the research verdict nor compilation alone is proof that
an external operating path works.

## Historical provenance

Worker PRs `#1081`–`#1091` were merged into the research branch. Their review
verdicts are recorded in [TRACKING.md](TRACKING.md) and memo headers. The
historical coordination protocol is archived in [OPERATING.md](OPERATING.md).
