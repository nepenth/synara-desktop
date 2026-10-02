# Native maturity retirement ledger

The 2026-09-30 consolidation retires executable migration experiments and
unused ownership scaffolding. Dated migration JSON and reports remain provenance;
they do not assert current product ownership or validation of this branch.

| Retired source | Reason and retained product route |
| --- | --- |
| Rust SDK 0.18 standalone probes and Swift Matrix SDK spike | The canonical root Cargo workspace and generated UniFFI bindings own the current SDK graph. Probe READMEs preserve historical recovery references; old executable manifests and sources are retired. |
| `app/legacy` and desktop `matrix/legacy` | The transition coordinator had no product consumer. Native session ownership and verified native deletion define completion; browser IndexedDB deletion is retired. |
| Domain `matrix_*_markers()` functions/constants and SDK link smoke | These returned labels or touched types without product calls. Real owner/adaptor code and native compiler checks establish SDK integration. Marker equality tests are retired; mixed feature/wire-policy assertions remain. |
| Core `app/supervisor`, `task`, desktop supervisor/task reexports, and `SdkClientHandle` | The exported actor/task graph was consumed only by its own unused orchestration and tests. Actual Core and SDK owner lifecycle paths remain. |
| `app/diagnostics` health/metrics/desktop projection models and desktop reexport | No product health emitter consumed this graph. Live diagnostic privacy filters remain, with adversarial secret/path/URL redaction fixtures. |
| Lifecycle `logout`, `recovery`, `remote_logout`, `remote_policy`, and `recovery_copy` models | These orchestrated the retired actor/task graph. Native session vault persist/restore/token rotation, exact account wipe, and their product error behavior remain. SharedCore teardown and desktop auth commands remain authoritative. |

`ClientBuilderError::to_factory_error()` remains live: the desktop native auth
adapter uses its category to select store-lock/unavailable diagnostics. Its
`FactoryError` projection moved into `app/client_builder/error.rs` without changing
fields, categories, or diagnostic IDs.

Core command adapters now live in `core/` domain modules. Typed UniFFI methods,
DTOs, and closed error conversions live in `shared_core_ffi/` domain modules; the
facade reexports preserve the existing UDL API plus the explicitly accepted typed
recovery additions. Before marker/model retirement, all 540 Core and 668 FFI
function bodies were preserved through extraction, ignoring formatter whitespace
and trailing commas outside string literals. Later graph retirement removes only
the explicit unused models listed above.

The retained desktop lifecycle tests exercise real encrypted SDK store open/drop,
native vault clearing, same-key reopen, verified exact-account wipe with a sibling
retained, wrong-key private failure, and actual SharedCore owner-queue retirement
on logout/restore. That queue regression attaches no owners and makes no
claim of generation advancement or post-restore old-generation rejection.
Separate production presence-owner registry regressions in
[`presence/native.rs`](../crates/synara-core/src/app/presence/native.rs) prove
user/generation filtering, stale subscription rejection, and rejection of new
and late work after retirement. The SDK-backed
[`presence/live.rs`](../crates/synara-core/src/app/presence/live.rs) uses that
registry for its event recipients. Config, credential-bearing proxy rejection, X509, store path
refusal, vault hooks, and product auth error mapping coverage remain.

Broad module `dead_code` and `unused_imports` exemptions are removed. Test-only
fixture parsers/imports use `cfg(test)`; they cannot satisfy production facade
boundary guards. The module source reader follows declared Rust files, rejects
missing modules, and excludes test modules/items, comments, and raw literal
examples from production declaration discovery. Compiler and hosted validation
must assess the exact final tree; the source inventory itself is not build proof.

## Recorded automated validation and proof limits

Results below were independently read back on 2026-10-02 for shipping validation
source `f756f60738c54ec10bf6e299f0e573e0bd493889`, tree
`6d0200e156d77338ab772568b918c99ac982b5a8` (3,080 source files). The
[full CI route](https://github.com/nepenth/synara-desktop/actions/runs/36954943741)
and [desktop package route](https://github.com/nepenth/synara-desktop/actions/runs/36954834984)
both completed successfully. The full route selected all Apple slices and
unsigned device Release checks. Its unit/UI and aggregate quality gates passed.
A separate compile-only job was unselected; the all-slice generation,
compilation, unit/UI and device proof below ran through the full route.

Rust, renderer and desktop package inputs are unchanged from candidate13.
Shipping14 retires temporary owner observation and control clocks. The original
named-stage composer fixture retains its commands, immediate native-refocus,
formatting/selection/replacement assertions and five-second deadlines. The
seventeen ownership regressions retain their target-action assertions after a
bounded paired UIKit witness and reviewed setup correction. Their final execution
is recorded below; prior failing executions remain separate history.

| Route | Actual result and scope |
| --- | --- |
| Node14 | 409 repository tooling tests, 1,201 modernization tests, and 126 Chromium cases passed: 7 timeline, 72 native timeline, 6 room-list, 34 approval, and 7 runtime-maturity. The ordinary hosted production npm audit found zero vulnerabilities; the separately recorded full local root/frontend audits found zero findings at every severity, including development dependencies. The hosted production audit is not a new full-development audit. |
| Native14 | Formatting, strict Clippy, compilation, and selected desktop/shared unit, integration, and doc-test commands passed: 1,864 tests passed, zero failed, six ignored across 81 result groups. The pass partition is 474 desktop, 1,111 Core unit, 277 Core integration across 72 executables, and two NSE unit tests. Six actual UniFFI async-bridge fixtures and the generated async export guard passed. Four Core unit, one Core integration, and one NSE live-preview case remain ignored. Development test features do not establish shipping NSE isolation; `indexed_message_search` was not selected because its required `search-index` feature was absent. |
| Synapse14 | Six separately selected actual native owner proofs each passed one test with zero failures or ignores: threads, receipts, reactions, polls, rich formatting, and attachments. These are distinct from browser harnesses and the six ordinary native ignores. |
| Rust audit14 | The 890-crate audit completed with zero vulnerability-class findings and zero yanked entries. Nine informative warnings remain: two unsoundness and seven unmaintained-crate advisories. Their owned dispositions are in [Rust dependency security](rust-dependency-security.md#remaining-informational-warnings); success is not an all-clear or remediation of those warnings. |
| Packages14 | Debian, Arch, macOS app bundle, macOS signature verification, and the aggregate required-package gate passed. The macOS bundle used ad-hoc signing; strict/deep codesign readback passed, and notarization was explicitly skipped. Upload logs record artifacts; they do not establish independent downloaded-byte verification, installation, app launch, signed distribution, updater behavior, or release publication. |
| Apple units14 | 820 passed, zero failed, three skipped (823 total). The original hosted composer ran once and passed in 4.801 seconds with its original five-second stage deadlines; native refocus, draft insertion, formatting, selection, replacement and focus assertions passed. All seventeen ownership regressions ran once and passed. Temporary owner probes and test clocks are retired in the executed source. |
| Apple UI14 | The fresh whole UI suite passed 71 tests with zero failures and 18 skips (89 total), preserving the previous named-case inventory and skip reasons. Eight selected composer/verification cases each ran once and passed. Completed comparison retained one Done tap, sheet-closed and replay-exclusion assertions and unverified-device assertions; independent screenshot readback showed the Rooms shell with the sheet absent. Two existing personal-note tests still emitted invalid-frame-dimension warnings, also present in actual9; no warning-free or remediation claim is made. |
| Apple generated pairs and strong archives14 | All four Core targets and three NSE targets regenerated their paired bindings/archives. Generated Swift compiled for native Darwin, arm64/x86_64 simulator and arm64 device. Production feature isolation passed. Seven logged positive primary-archive checks cover all three shipping NSE architectures and the post-unit device regeneration; the selected Rust LLVM decoder checks defined external symbols, required NSE function exports and absence of full Core exports. Unified development features are not this shipping proof. |
| Unsigned device Release14 | Generic-device Release app/extension compilation succeeded; its xcresult reports zero build errors and zero warnings. The exact fail-closed notification-service checker passed after that build, reporting a 20,537,144-byte extension executable below the unchanged 25,000,000-byte budget. Actual checker logs, exact source/invocation and positive primary archives establish the bounded linkage/isolation gate. Raw final Mach-O files and the text symbol report were not uploaded by the `*.xcresult` artifact route, so no independent downloaded-binary inspection or final per-architecture symbol-row counts are claimed. Executable disk bytes are not measured physical process footprint. |
| PDFv3 local release and CI routes | The normal cold-cache dual-engine release command passed 94 cases (12 room-list, 68 approval, 14 runtime-maturity); the normal Chromium CI command passed 47 (6, 34, 7). The spec-only v3 Retry oracle requires a new actual viewer paint failure after the first failure and excludes the standalone harness canvas. The unchanged production/compiled build, worker provenance, and 1,201-test evidence retained from v2 is explicitly separate from these fresh browser runs. |

The browser timeline performance assertions passed within their unchanged CI
budgets. Follow-live/wheel/prepend p95 frame times were approximately
16.8/33.3/16.8 milliseconds, with four/six/two dropped frames respectively;
the prepend maximum was 116.6 milliseconds. These are browser recordings, not
Apple UI frame metrics or zero-jank proof. Apple XCTest recorded three wall-clock
performance measurements, each with ten samples and no named baseline; UI
metrics were empty.

The normal same-SHA14 run36954835027 was intentionally superseded after PR head
synchronization by the full route; cancellation is neither failure nor pass.
Earlier failed runs remain failed. Diagnostic12 recorded 802 passes, one failure
and three skips: synchronous native resign took 22.244 seconds before returning
true, while the underlying UIKit/platform cause remained unconfirmed.
Diagnostic13 recorded 809 passes, eleven failures and three skips: all 806 old
cases produced 803 passes and three skips, while the seventeen added cases
produced six passes and eleven failures. Its original composer passed once for
instrumented source. The paired setup witness and shipping14 pass do not turn
these historical failures into successes or prove a universal transaction cause.
No final shipping event clock exists after probe retirement.

The six native ignores require authorized live Matrix credentials or a dedicated
live homeserver: retained-store key publication, direct-peer SAS transport,
own-device verification, live login-flow discovery, fresh-device eligibility,
and encrypted NSE preview after parent Core stops. Separate Synapse proofs do
not imply these six ignored scenarios executed. The three Apple unit skips are
unsigned simulator Keychain entitlement access and two opt-in live Core/NSE
smokes. The 18 UI skips comprise two iPad-only cases, 13 opt-in live fixtures,
and three opt-in mock screenshot cases. Their skipped status remains visible.

These named automated results do not establish physical APNs delivery, physical
minimum macOS13/WebKit16 compatibility, every installed OS notification or URL
interaction, signed Developer ID/TestFlight distribution, or updater upgrades.
The local current WebKit missing-builtin model retains its bounded scope.
The recorded routes cover the agreed Rust consolidation and maturity work,
application IndexedDB deletion retirement, and all 77 dependency proposal
dispositions within the stated automated scope. They do not broaden that scope
to the external physical, installed-platform or distribution proofs above.

The Rust source guard evaluates `cfg` Boolean combinations with `test=false`,
retaining feature/target combinations that can compile in production. It handles
same-line and multiline outer attributes and file-level inner `cfg` attributes.
Raw ASCII module identifiers resolve to their actual filename; unsupported
Unicode module identifiers fail explicitly. Path overrides, compiled `include!`
macros with any Rust delimiter (including literal paths), nested
external modules, nested inner test-only cfg, and cfg-adding `cfg_attr` are
explicitly unsupported and fail closed; they cannot silently hide source. A
module named `tests` is excluded only when its actual cfg disables production.

Encrypted notification ownership changed explicitly: Core waits through its
bounded decrypt retries and emits a resolved projection, or one final opaque
ciphertext observation if every lookup remains encrypted/unavailable. A resolved
edit, redaction, own event, or other non-candidate terminates observation without
ciphertext fallback. The renderer forwards the single final observation to Core.
When the authoritative SDK event remains ciphertext, Core returns nonsticky
`v-notify.event-not-ready` without push evaluation, pending enqueue, or event
dedup. This intentionally suppresses the previous generic platform alert while
plaintext/classification is unavailable. If a subsequent lookup resolves
plaintext, the same event can be classified and approval expiry rechecked; no
renderer retry schedule or unbounded Core polling is introduced.

Async raw-projection tests cover retry exhaustion, filtered plaintext, redacted
encrypted events, encrypted replacements, delayed approval classification, and
retirement during lookup. An SDK-backed decision
regression covers ciphertext refusal without dedup followed by successful
classification of the same event and ordinary duplicate suppression. The SDK
regression submits the generic message request with focus suppression enabled
while the room is focused; authoritative approval promotion overrides that flag.
Both renderer delivery routes use a shared presentation adapter based on the
returned candidate kind and matching source IDs. Its runtime tests cover approval
actions, time-sensitive action context, event dismissal keys, and rejection of
mismatched candidate IDs; no renderer classifier or expiry timer is introduced.
Frontend subscription execution and a message-route source contract cover
forwarding one final observation; they do not prove OS delivery or decryption arriving after
the retry budget. The [recorded native run](#recorded-automated-validation-and-proof-limits)
executed the SDK decision and late-room-key regressions, including same-event
push-rule evaluation without sticky ciphertext dedup; those owner results do
not establish OS delivery or an unbounded renderer retry policy.


The desktop lifecycle bridge's source-text installation counts and ordering test
was retired. It required releasing the auth session guard before Core open/close
based on a callback deadlock premise that those Core operations do not have.
Auth transitions now retain the session gate through Core wiring, attachments,
and rollback, and through both active and orphan logout closing. Actual async
coordinator and fault tests owned by the auth implementation provide the
serialization and cleanup oracle; the bridge's typed lifecycle, privacy, and
SDK behavior tests remain. Source counts or reversed string-order assertions
are not substitutes for those runtime tests. Execution of the final auth test
packet is recorded in the [native validation results](#recorded-automated-validation-and-proof-limits),
including origin-specific rollback and generation-bound owner/barrier tests;
this structural cleanup does not prove concurrency by itself.


Final notification follow-up checks also preserve the client binding generation
on retirement. Retirement permanently rejects decision work, including an SDK
lookup already in flight; it does not rebind the old client to the successor
index generation. Core's detached-owner pointer check remains in place. Renderer
room metadata and inbox triage no longer gate approval classification: the
returned candidate kind controls ordinary-message presentation gates. A shared
delivery helper makes a best-effort acknowledgement on every shown candidate,
including missing/stale renderer generation and delivery errors, with no stale
account delivery or acknowledgement retry. Protocol-relative and backslash
notification routes are discarded while a valid notification can still deliver.
The SDK retirement regression and the shared delivery helper fault tests are the
acceptance oracles; their native/helper execution is recorded above, while
installed native OS delivery requires separate platform evidence.


The final notification acceptance boundary binds Core candidates to their authenticated
session generation. The native command owns an acceptance task that keeps the auth
transition gate through the OS submission callback even if the renderer waiter is
cancelled. Bundled macOS uses UserNotifications request identifiers to remove both
pending and delivered items; Linux uses its existing notification handles. Exact
`candidate:notif-{generation}-{sequence}` keys isolate an old candidate from a
successor notification for the same event. A dismissal racing the macOS receipt
triggers a second removal after the callback. This contract governs acceptance;
it does not promise control over OS display timing. Unbound system notices retain
their existing route and deadline. Bound candidates fail closed in bare macOS
development executables, whose legacy sender cannot provide that callback boundary. Permission lookup remains bounded before banner
submission. Native compilation and owner/barrier execution are recorded above;
installed platform execution remains separate from source and mapping tests.

Renderer cache and sound commits occur after the asynchronous receipt and a second
session check. Browser fallback items have exact candidate ownership and close on
account teardown. Ordinary native banners explicitly request silent presentation;
the existing explicit message sound still follows the user preference and Core's
SDK push-action sound verdict, independently of the banner preference. Approval
banners retain native default sound and time-sensitive presentation; the web fallback
plays its existing explicit sound. Core candidate text removes controls and bidi
formatting, collapses whitespace, and keeps its Unicode character caps.

Observation-owner destruction retires and aborts its tracked decryption follow-ups,
including owners dropped without an explicit logout call. The regression uses the
real owner/task registry and projection loop with a blocked loader; it does not claim
an end-to-end sync-triggered SDK callback test. Native action clicks use volatile
in-flight exclusion and persist only after Core succeeds. A new completed-action
storage namespace ignores historical entries whose provisional status cannot be
proven; crash-before-success and failed actions remain retryable. Successful actions
remain bounded and account-scoped. Runtime helper regressions cover restart,
concurrent clicks, failure, and successful persistence.


Retained OS banner actions carry the original session generation in the native
response context and emitted action event. Critical actions with no binding fail
closed, as do stale bound action or review/navigation callbacks. The renderer checks
this binding before invoking Core and passes it through native admission. The
notification-bound approval command holds the auth transition gate throughout the
existing Core SDK decision in an owned task, preventing account replacement or
caller cancellation from rebinding a late mutation. Ordinary in-app decisions omit
the optional notification binding and retain their existing route. Desktop session
generation is never substituted into the distinct Core command-envelope counter.
The reusable auth gate's real mutex/barrier tests cover replacement and cancellation;
response-context and renderer tests cover preservation and forwarding of the original
binding. Native command compilation and owner/barrier execution are recorded
above; installed platform execution remains separate evidence.
