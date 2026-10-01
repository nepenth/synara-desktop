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
