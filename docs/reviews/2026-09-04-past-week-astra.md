# Past-week architecture, quality, and security review

Dated review: 2026-09-04. This is a review snapshot, not a release certification
or a replacement for the ADRs.

Baseline: `07bb7903` on `main`. Open PR reviewed: [#1100](https://github.com/nepenth/synara-desktop/pull/1100)
at `b2ab6750d90163716179fccdaea79a0058949de6`.
The seven-day window is August 28 through September 4 at 19:20 UTC: 33 merged
PRs. The broader calendar-date query returned 36; #1063–#1065 were also inspected
as adjacent UI/release context. All PR descriptions and changed-file inventories
were reviewed; source/diff inspection concentrated on runtime, security, and
CI changes. This was a risk-focused review, not a line-by-line audit of every
file or a penetration test against deployed services.

## Assessment

The central architectural decision is good: one Rust application/Matrix owner,
with native platform presentation. Keep it. ADRs 0001–0005 distinguish authority,
observation, rendering, and native byte/secret transport more clearly than many
multi-client projects. The recent work generally follows those boundaries.

| Layer              | Technology and role                                                                                    | Assessment                                                                                                                                                                           |
| ------------------ | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Shared application | Rust 1.93, Matrix SDK/UI/crypto 0.18, Tokio, SQLite, UniFFI 0.28.3                                     | Good owner for crypto, lifecycle, sends, receipts, bounded models, and shared policy. The large timeline registry and command dispatcher deserve smaller cohesive modules over time. |
| macOS/Linux        | Tauri 2.11, platform credential/file/notification adapters                                             | Appropriate native integration. Privileged webview navigation needed an explicit origin boundary.                                                                                    |
| Desktop UI         | React 19, TypeScript, Vite, Slate, TanStack virtualization, Jotai, output-context HTML sanitation      | Keep the established presentation stack. The semantic color/spoiler work and viewport tests are valuable. Notification matching still contains protocol-policy duplication.          |
| iOS                | SwiftUI, generated project-owned Core bindings, Keychain, APNs/NSE                                     | Correctly native. Visibility and cancellation observations must remain honest across async Core transitions.                                                                         |
| Delivery           | Node/shell governance, pinned GitHub Actions, separate workspace/Tauri lockfiles, coordinated releases | Broad checks exist, but filename shortcuts could skip the very validation the checks purported to enforce.                                                                           |

Positive security evidence includes narrow native credential/media channels,
account-bound native owners, explicit encrypted-forwarding confirmation,
HTML sanitation in each output context, bounded streaming media/decryption,
and content-free notification diagnostics. Those are useful controls, not proof
that every deployed path is secure.

## Fixes on this branch

### P1: executable and dependency changes could bypass CI

`scripts/ci-metadata-only.mjs` treated JavaScript manifests/lockfiles, Tauri CSP
configuration, Xcode project settings, the Arch build script, generated HTML,
and some TSX components as metadata based solely on filenames. A security or
runtime change in those files could receive no heavy validation. The icon-only
shortcut also accepted edits to the CI workflow itself. Root Cargo manifests,
lockfiles, and toolchain changes were absent from the main Rust scope probe.

The shortcuts now accept release prose and narrowly scoped icon inputs;
executable/build/dependency inputs reach validation. Workflow changes run policy
checks, generated runtime and package changes reach the frontend/tooling lane,
and root Rust inputs run compile/tests. Release PRs **from** `release/**` into
`main`, release pushes, and explicit iOS labels can no longer exit through the
metadata shortcut before their simulator gates. Ordinary feature PRs retain the
existing opt-in iOS policy.

Regression tests execute the actual CI shell against temporary Git repositories
and assert its emitted job decisions. They cover individual build inputs,
release branch direction, labels, prose-only changes, and icon-plus-code changes.
The shared classifiers also improve package-smoke selection.

### P2: privileged desktop navigation was not origin-restricted

The packaged runtime uses a dynamically selected localhost port, and its
capability file allows `http://localhost:*/*`. New-window handling already opens
safe external links through the OS, but same-window navigation had no guard.
Loading another local service in that window must not inherit its privilege.

`desktop_navigation.rs` now restricts document navigation to the actual configured
dev origin or selected packaged origin, including its port. It rejects other
origins, embedded credentials, and non-HTTP document schemes while retaining
normal routes, reloads, and fragments. Unit tests cover both accepted routes and
adversarial URLs. This is defense in depth; this review did not demonstrate a
remote exploit through the current sanitized message renderer. The dynamic
localhost ACL remains, so future capability narrowing remains worthwhile.
[Tauri navigation API](https://docs.rs/tauri/2.11.2/tauri/webview/struct.WebviewWindowBuilder.html#method.on_navigation),
[Tauri remote capability guidance](https://v2.tauri.app/reference/acl/capability/).

### P2: notification history grew without a bound; rejection mutated state

The 128-candidate pending cap did not cap `seen_events`: each delivered event
remained until logout. Also, rejecting a reused candidate ID happened after
recording its event and evicting pending work. A rejected submission could
therefore lose an unrelated pending candidate and suppress a valid retry.

Core now retains the most recent 4,096 accepted event keys independently of the
pending queue, bounds retained identifiers to 512 bytes, and checks candidate-ID
collisions before either mutation. Dismissal and pending eviction preserve recent
dedup; retiring the session clears it. Tests exercise a full queue, rejection and
retry, identifier limits, and more than 4,096 delivered/dismissed events.

Tradeoff: dedup is now a documented recent-event window, not a lifetime promise.
An evicted event can notify again if resubmitted. The desktop observation pump
already limits its scans to recent events; unusually high traffic or a future
replay consumer must account for the finite window.

### P2: intentional mention metadata could still trigger legacy highlights

An event with `m.mentions: {}` could still highlight because its body mentioned a
name or `@room`. The desktop observation helper now disables legacy name and
`@room` matching whenever explicit mention metadata exists. Intentional mentions
and independent keyword matching still work; modern room mentions and legacy
`@room` settings are no longer conflated. Regression tests cover empty metadata,
real mentions, keywords, and independently disabled flags. This is a correction
to the existing helper, not a claim of complete SDK push-rule equivalence.
[Matrix intentional-mention semantics](https://spec.matrix.org/v1.15/client-server-api/#mentions).

### Dependency maintenance: three high-severity npm reports cleared

The complete dependency audit, including development tooling, reported
`browserslist`, `fast-uri`, and the affected `ajv` dependency path. Updated
Browserslist and its supporting data in the lockfile and changed the existing
fast-uri override from 3.1.5 to 3.1.6. The complete npm audit now reports zero
vulnerabilities. These were build-tool dependencies; no production messaging
exploit was established.
[Browserslist advisory](https://github.com/advisories/GHSA-c83g-rgw3-j3cx),
[fast-uri advisory](https://github.com/advisories/GHSA-f65p-4m7j-42xc).

## Open PR #1100: changes needed before merge

These findings concern code present only on the open PR, not on this branch's
`main` baseline. This branch does not import or rewrite that PR's unmerged work.
Both findings need behavioral regression tests on the PR before merge.

1. **P1 — iOS silent follow can acknowledge unseen history.** In
   `RoomTimelineView.silentlyFollowLive`, a valid-looking painted event ID is only
   a nonempty-string gate. The method calls `transitionToLive()` without the
   observed ID. That existing service method loads/replaces the latest window.
   Completion then sets `isTimelineBottomVisible = true` and schedules the new
   tail read without verifying the old tail equals the live tail or that the new
   tail became visible. Reproduce by opening an old event, reaching the bottom
   of that focused window while newer room events exist, or moving away while
   the async load runs. Preserve explicit Jump to latest for incomplete history;
   automatic follow must verify continuity and current viewport/session identity,
   and acknowledge only after the new row is actually painted and visible.

2. **P1 — desktop follow compares against the focused window, not an independent
   live edge.** In `NativeTimelineRegistry::follow_live_tail`, the comparison is
   `stream.timeline.latest_event_id()`. The pinned SDK's controller implements
   this by inspecting that timeline's `state.items.all_remote_events()`; an
   event-focused timeline can end before the room's live edge. Matching that
   window tail and changing only Synara's `position` to `LiveBottom` does not
   change the SDK provider's focus to live. Reproduce using a context window
   with newer events outside it. Require SDK-authoritative forward exhaustion
   or an independent live-tail comparison plus a real provider transition;
   retain the focused state on stale/incomplete observations.

The associated tests mostly cover policy predicates, transport routing, and
source strings. They do not establish that an old context window is rejected
or that moving away during an async follow prevents acknowledgement. At review
time #1100's iOS simulator checks were skipped despite Swift changes.

## Recommended follow-on work

- **Notification owner completion:** consume SDK-evaluated push actions through
  Core instead of reconstructing mention/keyword/default rules in TypeScript.
  Room power-level checks for room mentions and full rule ordering should have
  one owner. Add cross-client tests for muted rooms, encrypted/late-decrypted
  events, focus changes, and failed OS delivery. Current delivery swallows OS
  errors and acknowledges the candidate; test a receipt/ack design before
  adding automatic retries.
- **Media performance:** measure device RSS and duplicate downloads through the
  actual Swift/Tauri byte entry points. Existing A10 evidence explicitly leaves
  Swift-to-Rust in-flight cancellation and real-device memory unproven. The
  32 MiB iOS and 64 MiB desktop byte caps do not themselves cap total decoded
  image memory. Use measurements to decide caching/concurrency changes.
- **Timeline concurrency:** profile head-of-line blocking where the registry
  mutex is held through network/SDK awaits. A per-stream operation design may
  help, but changing ordering/receipt semantics without race tests is risky.
- **Security dependencies:** both Rust audits pass under existing policy but
  retain maintenance/unsoundness/yanked warnings and reviewed exceptions. Track
  the pinned SDK/Tauri dependency upgrades; do not describe this as zero Rust
  advisories or remove allowlists merely to make the report look cleaner.
- **Agent approvals:** Core checks expiry and terminal decisions, but prompt
  recognition still relies on textual headings and sender-not-self. The code
  explicitly assigns authorization to Hermes. A structured, authenticated
  approval identity and tests against the actual Hermes authority would reduce
  spoofed-prompt ambiguity; this client review cannot certify that external
  enforcement.
- **Evidence quality:** retain source guards as architecture drift checks, but
  prioritize executable async race and visibility tests. Keep real macOS/Linux
  delivery, physical APNs/NSE, and two-client interoperability release evidence
  distinct from deterministic test counts. Consider a cheap iOS compile gate
  for ordinary Swift/FFI PRs while retaining the simulator scheduling policy.

## Validation

Validation results are recorded in the accompanying PR. Local checks use Node
24.13.1 and Rust 1.93. No real-account, signing, deployment, or production
notification operations were performed. Initial Rust linking exhausted local
disk space; regenerable incremental caches were cleared and build concurrency
was reduced. The APT signing fixture required `TMPDIR=/tmp` to avoid the local
GPG socket-path limit. Playwright's pinned Chromium headless shell was installed
before the browser suite ran. These environment repairs are not product fixes.

## Merged PR inventory

The table records every PR in the seven-day window; grouping does not imply an
independent full test run of each historical commit.

| PR                                                           | Change reviewed                                                                      |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| [#1066](https://github.com/nepenth/synara-desktop/pull/1066) | fix: complete device verification and refine message UX                              |
| [#1067](https://github.com/nepenth/synara-desktop/pull/1067) | Synara 2.1.21: harden timeline restore and media retries                             |
| [#1068](https://github.com/nepenth/synara-desktop/pull/1068) | Fix desktop UX coherence and timestamp gestures                                      |
| [#1069](https://github.com/nepenth/synara-desktop/pull/1069) | Release Synara 2.1.22                                                                |
| [#1070](https://github.com/nepenth/synara-desktop/pull/1070) | Fix cross-client desktop operating paths                                             |
| [#1071](https://github.com/nepenth/synara-desktop/pull/1071) | Release v2.1.23                                                                      |
| [#1072](https://github.com/nepenth/synara-desktop/pull/1072) | chore(deps): bump the github-actions-updates group across 1 directory with 2 updates |
| [#1073](https://github.com/nepenth/synara-desktop/pull/1073) | Improve semantic rich-text readability across clients                                |
| [#1075](https://github.com/nepenth/synara-desktop/pull/1075) | Release v2.1.24                                                                      |
| [#1076](https://github.com/nepenth/synara-desktop/pull/1076) | docs: add future Rust ownership research portfolio                                   |
| [#1077](https://github.com/nepenth/synara-desktop/pull/1077) | docs: add Rust ownership expansion portfolio triage                                  |
| [#1078](https://github.com/nepenth/synara-desktop/pull/1078) | docs: synthesize Rust ownership triage                                               |
| [#1079](https://github.com/nepenth/synara-desktop/pull/1079) | docs: maintain architecture decision records                                         |
| [#1080](https://github.com/nepenth/synara-desktop/pull/1080) | docs: align Rust ownership plan with revised ADRs                                    |
| [#1081](https://github.com/nepenth/synara-desktop/pull/1081) | docs: ROE-01 orchestration census memo                                               |
| [#1082](https://github.com/nepenth/synara-desktop/pull/1082) | docs: ROE-08 agent-approval research memo                                            |
| [#1083](https://github.com/nepenth/synara-desktop/pull/1083) | docs: ROE-02 verification census memo                                                |
| [#1084](https://github.com/nepenth/synara-desktop/pull/1084) | docs: ROE-09 notes census memo                                                       |
| [#1085](https://github.com/nepenth/synara-desktop/pull/1085) | docs: ROE-07 notification-policy research memo                                       |
| [#1086](https://github.com/nepenth/synara-desktop/pull/1086) | docs: ROE-03 timeline-rows census memo                                               |
| [#1087](https://github.com/nepenth/synara-desktop/pull/1087) | docs: ROE-06 room-sort census memo                                                   |
| [#1088](https://github.com/nepenth/synara-desktop/pull/1088) | docs: ROE-10 drafts census memo                                                      |
| [#1089](https://github.com/nepenth/synara-desktop/pull/1089) | docs: ROE-04/12 message-format census memo                                           |
| [#1090](https://github.com/nepenth/synara-desktop/pull/1090) | docs: ROE-11 media-metadata census memo                                              |
| [#1091](https://github.com/nepenth/synara-desktop/pull/1091) | docs: ROE-05 visibility-contract census memo                                         |
| [#1092](https://github.com/nepenth/synara-desktop/pull/1092) | Close Rust ownership residuals                                                       |
| [#1093](https://github.com/nepenth/synara-desktop/pull/1093) | docs: record Rust ownership promotion                                                |
| [#1094](https://github.com/nepenth/synara-desktop/pull/1094) | Complete Rust ownership follow-on integrations                                       |
| [#1095](https://github.com/nepenth/synara-desktop/pull/1095) | Release v2.1.25                                                                      |
| [#1096](https://github.com/nepenth/synara-desktop/pull/1096) | chore(deps): bump the rust-updates group across 1 directory with 16 updates          |
| [#1097](https://github.com/nepenth/synara-desktop/pull/1097) | feat: add Core-owned desktop notification decision stream                            |
| [#1098](https://github.com/nepenth/synara-desktop/pull/1098) | Retry transient TestFlight export auth failures                                      |
| [#1099](https://github.com/nepenth/synara-desktop/pull/1099) | ci: run long iOS simulator jobs on release cuts, not feature PRs                     |
