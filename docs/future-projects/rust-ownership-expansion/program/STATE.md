# Reconciled program state

Last updated: 2026-09-03.
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
- A6's deterministic presentation/security slice is complete. Its 22-case
  shared corpus and required-coverage register run through Core, desktop, and
  iOS harnesses for formatted reply fallback, mentions, spoilers, links,
  nested lists, inline/preformatted code, tables, malformed/executable HTML,
  remote-resource rejection, plaintext fallback, and UTF-8 size boundaries.
  Core rejects, rather than truncates, combined plain/formatted text and
  caption payloads above 65,536 UTF-8 bytes and bounds raw mention cardinality
  and identifiers before parsing. The expanded corpus exposed and repaired
  iOS's loss of the historical Matrix `<strike>` alias by normalizing it to
  `<s>` at the iOS output-context boundary, matching desktop without moving
  sanitization into Core. Timeline sequencing remains a separate Core concern
  under A7 rather than duplicate presenter logic.
- A7 transports Core-owned sender, reply, thread, poll, reaction, and
  capability presentation without dropping non-message event metadata.
  Desktop message, poll, and sticker rows share the typed reply/thread/reaction
  presenter. The existing reply/edit/redact/react/pin UI consumes its
  applicable gates. Capability-driven poll vote, report, forward, and
  decline-call surfaces are now implemented across desktop and iOS with Core
  validation, typed exact readback, fail-closed forwarding, and duplicate-write
  exclusion. A pinned-SDK/Core suite proves redaction replacement, late
  decryption, pagination overlap, relation-before-parent ordering, and
  power-level capability reprojection without duplicate row identity. Live
  Matrix/two-client action interoperability remains **Not confirmed**; see
  [A7-TIMELINE-PRODUCT-ACTIONS.md](A7-TIMELINE-PRODUCT-ACTIONS.md).
- A8 is decided and tested: marked-unread alone is not a mention.
- A9 has a repaired iOS pusher-registration owner: duplicate triggers are
  single-flight, every binding retains a dedicated account-bound Core/client
  capability plus its exact token, stale in-flight results are removed, and
  failed rotation cleanup remains retryable instead of being overwritten.
  Logout uses that capability to enumerate and delete every exact Synara
  app+Matrix-device pusher before local credential teardown, including after a
  process restart with no redelivered APNs token. Its two-phase gate remains in
  force through Keychain deletion: remote failure blocks sign-out, while local
  deletion failure resumes and reconciles push for the still-signed-in session.
  App/NSE diagnostics use only fixed stage codes plus opaque local correlation
  IDs, and deadline races complete coordinator-owned fallbacks before recording
  only those request IDs. Physical
  foreground/background/terminated APNs and encrypted/unencrypted NSE proof
  remains **Not confirmed**. Desktop tray delivery remains **Failed** at its
  unwired Core candidate/decision source; no room-list polling or JS policy
  fallback was added. See [A9-NOTIFICATION-DELIVERY.md](A9-NOTIFICATION-DELIVERY.md).
- A10 measures the existing media byte-channel boundary without changing its
  architecture. A loopback Matrix SDK harness proves authenticated client-v1
  routing, exact declared/chunked caps, only the documented 404/405 legacy
  fallback, uncached repeated fetches, and caller-task cancellation. iOS adds
  content-free duration/byte-count/outcome signposts. Real-device process-memory,
  radio/network, and in-flight cancellation evidence remains **Not confirmed**,
  so the unused cache stays unwired and the existing platform caps remain.
- A11 removes the retired direct Matrix Swift package edge and makes CI lock
  behavior derive semantically from whether the reachable generated package
  graph actually has remote packages. The Core Swift/XCFramework pair was
  regenerated across device, simulator-arm64, simulator-x86_64, and macOS.
  After the final relation and receipt cleanup, the exact promoted head's iOS
  unit target executed 658 tests: 655 passed and 3 were intentionally skipped.
  Its iOS UI target executed 73 tests: 59 passed and 14 were intentionally
  skipped.

The prior A1-A5/A8/A11 remediation baseline also passed Core's 836-test unit
target (833 passed, 3 environment-gated tests ignored), every Core integration
binary and doc test, desktop modernization 910/910 after typecheck and
production build, and the complete Tauri compile. The offline cold-restart proof was
repaired at the Matrix SDK store-lifecycle boundary: the first client now
awaits `Client::pause()` before the same persistent root is reopened, replacing
the nondeterministic `drop` plus guessed delay.

Physical notification delivery (A9), real-device media measurement/cache policy
(A10), the remaining A2/A3/A4/A5 live proofs, and A7 live action
interoperability remain open. Deterministic tests must not be cited as those
live proofs.

## Prior remediation promotion status

The memos and original run remain docs-only research artifacts. The earlier
2026-09-02 A1-A5/A8/A11 product changes were separately authorized and
implemented in the remediation change set. Independent implementation review
completed and its concrete P1
reply-revision finding was remediated. The final Grok 4.6 High review then
identified an attachment-send revision-scope defect; that path was corrected,
passed the focused executable suite and full desktop typecheck, and a targeted
Grok re-review returned no P0-P2 findings. That prior change set's consolidated
deterministic validation was green. PR `#1092` promoted exact head `ce77bdcc` through 13
applicable successful checks with zero failures; four scope-inapplicable checks
were skipped and are not counted as evidence. The PR merged as `e9b5016e`, and
both local `main` and authoritative `origin/main` resolved to that merge commit
on 2026-09-02. None of those results validate the current follow-on branch.
Neither the research verdict nor compilation alone is proof that an external
operating path works.

## Follow-on validation status

The initial separately authorized A6/A7/A9/A10 candidate on
`feature/rust-ownership-follow-ons` completed deterministic validation. The
complete Rust workspace passed formatting, all-target Clippy with warnings
denied, check, and 1,072 tests with 3 environment-gated ignores. Tauri passed
formatting, all-target Clippy, check, 441 tests, and the 2,722-file Matrix
boundary/SDK guardrails. Desktop passed formatting, lint, both typechecks, a
production build, 926 modernization tests, and 6 real-layout Chromium tests;
the repository delivery suite passed 277 tests. Production npm audit is clean,
and both Rust lockfiles have no advisory outside the reviewed cargo-audit
allowlists.

The exact Apple candidate transactionally regenerated Core and NSE packages,
passed build-for-testing, and executed 777 tests: 760 passed, 17 intentionally
skipped, and 0 failed. Its fresh log has zero matches for the prior
background-publish, `ObservedObject`, main-actor-isolation, invalid-frame, or
non-finite-frame warnings. Generated Swift compiled the tri-state room
encryption DTO and both explicit forward-downgrade confirmation parameters
end-to-end.

Validation found and repaired stale test contracts for the required room
encryption field, the new HTTP-pusher owner, and the Tauri tri-state projection,
plus one desktop no-unused-binding violation. The message sanitizer was updated
from vulnerable 2.17.6 to fixed 2.17.7 and the affected desktop gates were
rerun. Independent review accepted the final iOS CI remediation and centralized
logout error presentation with no P0-P2 findings. The independent whole-change
review and final Grok 4.6 High review both returned `ACCEPT` with no actionable
P0-P2 findings. Remote PR CI is still a promotion gate; no merge or release is
implied. Physical notification delivery, real-device media measurement, and
every live/two-client gate listed above remain open.

That initial record is not the final promotion state. A later third-party
review on PR `#1094` identified stale-reaction readback handling, incomplete
remote-resource corpus assertions, collapsed iOS forward diagnostics, a
missing logout owner-rebind attempt, fragile pusher display-name matching,
missing encrypted-decrypt and production-projector test coverage, a misleading
decline-call error class, and an unused reaction path outside the flight
coordinator.

The accepted findings are remediated on the feature branch. Command commitment
and projection settlement are now separate for reactions; MXC image nodes are
reduced to inert alt text at the desktop output boundary and forbidden by both
platform harnesses; forwarding security races remain fail-closed but are
visible and retryable where confirmation is appropriate; pusher cleanup keeps
exact app/device authority with an exact last-known key as a secondary match;
and the missing decrypt/projector branches execute deterministically. Exact-head
validation on 2026-09-03 passed the complete Rust workspace, all desktop
format/lint/type/build/security gates and 927 modernization tests, Tauri's 441
tests and audit gates, and both iOS targets: 703 unit tests passed with 3
intentional skips and 59 UI tests passed with 14 intentional skips. Grok 4.6
High found one iOS 16 stacked-alert defect in the initial correction; the
forwarding sheet was reduced to one mutually exclusive item-driven alert and
the targeted re-review returned `ACCEPT` with no P0-P3 finding. Remote PR CI is
still a promotion gate. The physical APNs/NSE, live pusher readback, two-client
action, and real-device media gates remain open.

## Historical provenance

Worker PRs `#1081`–`#1091` were merged into the research branch. Their review
verdicts are recorded in [TRACKING.md](TRACKING.md) and memo headers. The
historical coordination protocol is archived in [OPERATING.md](OPERATING.md).
