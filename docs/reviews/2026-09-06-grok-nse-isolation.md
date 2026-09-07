# NSE production isolation review

Grok 4.6 High, source head `e440c3530fb392d9be49482c8e91d38ed4f1ba39`, session `01a079ac-c132-7e51-b560-993e21431ad1`. The review accepted the target feature repair but identified actionable adjacent archive/wiring gaps. Integration is held for those corrections; the ACCEPT heading does not waive them.

I'll start by loading the review skill and inspecting the branch against the given base and head refs.The user asked for a source-only review with Read/Grep/Glob and no subagents, so I’ll inspect the branch files and prior review evidence directly.I’ll inspect the existing review notes and the production-features checker, then trace owners and tests around that path.Next I’ll trace CI scope wiring, quality-gate ownership, and the archive-symbol check the feature guard is meant to complement.**ACCEPT**

The production-feature guard is the right repair for the original CI false positive: it asks Cargo for the **normal/build** graph (not dev), on **each Apple triple the NSE generator actually builds**, fails closed on any Cargo error, and still rejects `full-uniffi`. Archive symbol checking is unchanged. I found no P1 correctness defect in the new checker, its real-Cargo fixtures, or the iOS scope wiring.

## What this change does

The shipping NSE crate depends on Core with `default-features = false` and `nse-preview` only; `full-uniffi` is a **dev-dependency** for live fixtures (`crates/synara-nse-core/Cargo.toml:13`, `:25`). Resolver 2 keeps those off `cargo build`. The old CI query used `-e features`, which still displays dev-activated features, so a clean production graph looked dirty.

The new checker (`scripts/check-synara-nse-core-production-features.mjs`) runs:

```text
cargo tree --locked -p synara-nse-core -e normal,build,features -i synara-core --target <triple>
```

for `aarch64-apple-ios`, `aarch64-apple-ios-sim`, and `x86_64-apple-ios` — the same `all` list as `scripts/generate-synara-nse-core-swift.sh:14`. It inspects completed stdout (no `grep -q` pipeline) and treats any spawn error or non-zero Cargo status as failure.

## Risk-area assessment

| Area | Judgment |
|---|---|
| **False negatives** | Host-only graphs can no longer hide `target_os = "ios"` or per-triple leaks; the four new fixtures lock that. Dev-only `full-uniffi` is allowed. Empty/failed Cargo output cannot look clean. Remaining gaps are defense-in-depth (archive `nm \| grep -q`) and future target-list drift, not a hole in the current triples. |
| **Dependency edge selection** | `-e normal,build,features` is the correct production set under resolver 2: normal + build, features displayed, **dev excluded**. Matches `cargo build -p synara-nse-core` (no `--tests`). Build-script edges are included on purpose; the `build` fixture covers implicit default features. |
| **Cargo output parsing** | Match string `synara-core feature "full-uniffi"` is Cargo’s Debug feature-edge format. The tests assert that exact substring against real `cargo tree`. `includes` on a quoted name avoids prefix collisions. `spawnSync` is not a TTY, so color is auto-off. |
| **Supported target coverage** | Checker triples == generator `all` slices. Darwin is correctly omitted (NSE is not built for host). CI only *installs* `aarch64-apple-ios-sim`; `cargo tree --target` is metadata and does not need stdlib. |
| **Workflow scope wiring** | `.github/workflows/ci.yml:267` puts the checker on the iOS path probe. `ci-scopes.test.mjs:164-171` executes the **live** workflow script on a main push and requires `ios` and `ios_ui`. Feature PRs still skip iOS unless labeled — existing policy; `scripts/` still runs `node --test scripts/__tests__/*.test.mjs` via validate-frontend. `ci-build.sh:160` is the runtime invocation; exact-tag release iOS also goes through that script. |
| **Test quality** | Eight real-Cargo tests: dev-only pass, normal leak, build leak, failed query, `cfg(target_os = "ios")`, and each of the three triples with a proven-narrow macOS graph. Lockfiles are generated offline. That is the right kind of fixture, not string-mocked Cargo. |

No production Rust or Swift owners need to change for this guard. Core still gates UniFFI on `#[cfg(feature = "full-uniffi")]` (`crates/synara-core/src/lib.rs:15`).

## P2 (non-blocking)

**P2 — `synara-ios/scripts/ci-build.sh:161-165`** — The adjacent archive check still uses `nm -gU … \| grep -q` under `set -o pipefail`, with `nm` stderr discarded. That is the same producer-failure class the new checker was written to close: `grep -q` can SIGPIPE `nm` (pipeline 141 → `if` false) when forbidden `_uniffi_synara_core_` symbols **are** present, and a failed `nm` is treated as clean. This is pre-existing, not introduced here, and the Cargo graph check is now the primary CI signal. Action: capture `nm` stdout, fail if `nm` is non-zero, then search the buffer (same pattern as the feature checker).

**P2 — `scripts/check-synara-nse-core-isolation.mjs` / `scripts/check-synara-core-swift-scaffold.mjs:14-69` and `:1666-1677`** — Isolation/scaffold checkers pin the Core scaffold invocation in `ci-build.sh` and many NSE generator strings, but they do not require `check-synara-nse-core-production-features.mjs` to exist or to be called. Deleting only the `ci-build.sh:160` line would leave the eight unit tests green and drop the CI graph proof. Action: `requireText` on the `ci-build.sh` invocation (and add the script to the scaffold required list). Optionally also require the three generator `all` triples to appear in the checker so a fourth slice cannot silently recreate the host-only miss.

**P2 — `scripts/__tests__/nse-production-features.test.mjs:39-40`** — Target-specific fixtures only add **normal** `[target.*.dependencies]`, not `[target.*.build-dependencies]`. The checker uses the same `-e normal,build,features` for every triple, so a target build leak should already fail; a fixture would lock that. Not a product bug.

## Proof limitations (not defects)

- This review is source-only (Read/Grep). I did not `git diff` `e440c353` vs `76ab6cdb`, did not re-run the eight Cargo fixtures, the three-target repo checker, or the extracted CI feature/archive block, and did not execute `cargo tree --help` on 1.93. Claims that those runs passed are taken from `docs/reviews/2026-09-06-nse-production-isolation-check.md` and the user brief, not re-verified here.
- Changed-file set is inferred from owners + that review: the checker, its tests, `ci-build.sh`, `ci.yml` iOS path probe, `ci-scopes.test.mjs`, and the review doc. I did not independently confirm an empty Rust/Swift diff.
- `-i synara-core` is empirically the invert-root used by the fixtures; I did not re-parse Cargo 1.93 clap. If those tests passed, the invocation is accepted and detects leaks on this toolchain.

No P1. Do not block on the P2s; they are hardening around a guard that already closes the reported hole.
