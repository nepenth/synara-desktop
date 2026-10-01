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

Validation status: the desktop shell and three shared packages passed strict Clippy over all targets
in the final bounded local compiler run. All 394 repository tooling tests also passed.
Clippy checks compilation and lints; native
unit/integration test execution remains pending. The rewritten desktop lifecycle
tests have source review and local compilation; hosted Linux compilation/execution is still pending.
Full Apple slices, NSE archive/export checks, app linkage, simulator tests,
unsigned device Release, and desktop packaging require their hosted lanes.

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
the retry budget. Those native/runtime results require their test lanes.
