# Rust dependency security dispositions

Reviewed against the consolidated production workspace lockfile on 2026-09-30.
The dependency owner is the Synara maintainer responsible for Cargo and SDK
updates. `cargo audit` runs without an advisory ignore list; informational
warnings remain visible in CI. Reassess the source evidence below whenever
Matrix SDK, Tantivy, Tauri, GTK/WebKit, or their lockfile entries change.

The [dependency PR disposition record](dependency-update-dispositions.md)
captures the four dependency update PRs at the recorded open-PR snapshot,
every npm package outcome, compatibility alternatives, renderer retirement,
and the [recorded automated results and proof limits](rust-maturity-retirements.md#recorded-automated-validation-and-proof-limits).
Shipping14 (`f756f60738c54ec10bf6e299f0e573e0bd493889`) independently
recorded 1,864 native passes with six ignores, all three desktop package builds
and their required gate, and an 890-crate audit with zero vulnerability-class
findings or yanked entries. Nine informative advisories remain: two unsoundness
and seven unmaintained-crate warnings.

Its [full CI route](https://github.com/nepenth/synara-desktop/actions/runs/36954943741)
passed Apple units (820 passed, zero failed, three skipped), fresh whole UI
(71 passed, zero failed, 18 skipped), all four Core/three NSE paired slices and
strong primary archives, and unsigned device Release compilation with the
fail-closed final checker. The checker logged 20,537,144 executable bytes below
the unchanged 25,000,000-byte budget. Raw final binaries/symbol-report rows were
not uploaded; this bounded gate uses actual checker/source/invocation/positive
archive readback, with no direct downloaded-binary inspection claim.
Temporary owner probes and test clocks are retired. Historical failed runs remain
failed; these results do not remediate the nine informative advisories or remove
physical/installed platform limits. Ad-hoc codesign success does not establish
notarization, signed distribution or installed updater behavior.

## Remediated entries and dependency PRs

- `wayland-scanner` 0.31.10 → 0.31.11 replaces `quick-xml` 0.39.4 with
  0.41.0. The older XML parser was a Linux protocol-code-generation dependency
  through RFD and clipboard support. Both [RUSTSEC-2026-0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html)
  and [RUSTSEC-2026-0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html)
  are fixed at >=0.41.0; the former desktop audit exceptions are removed.
- Yanked `chacha20` 0.10.1 → 0.10.2, including Matrix encryption and RNG users.
- [PR #1159](https://github.com/nepenth/synara-desktop/pull/1159) is incorporated
  into the root lock: Tauri 2.11.6, updater 2.12.0, single-instance 2.4.5,
  zbus 5.19.0. Direct requirements encode Tauri `~2.11.6` and updater
  `~2.12.0` so updates remain in their validated minor families; single-instance
  and zbus use compatible major requirements with floors 2.4.5 and 5.19.0.
  Supporting Tauri crates retain upstream-compatible ranges rather than
  gratuitous exact pins. Production builds use `--locked`, and version checks
  validate the resolved Tauri API/CLI and updater families.
- [PR #1162](https://github.com/nepenth/synara-desktop/pull/1162) is incorporated
  as exact UniFFI 0.32.2 across Core, NSE, and the project-owned generator.
  Its upstream metadata comments fix the obsolete 0.28.3 Clippy workaround.
  The explicit Tokio bridge is retained on all generated async UDL exports.
  Release validation must regenerate Swift and native archives together,
  compile the Apple graph, and inspect NSE exports; a manifest or source scan
  alone does not establish ABI compatibility.
- Matrix SDK and its direct sibling crates remain exactly 0.19.1. Desktop
  and Apple packages share one lockfile, while their shipping feature graphs
  are selected and checked independently. Apple generators build each package
  in a separate `-p` invocation. The NSE checker inspects normal, build, and
  feature edges for every supported Apple target and the all-target build-edge
  readback. Inverse Core feature nodes reject full FFI and search; forward
  feature nodes reject forwarding and X.509, and the normal/build crate tree
  rejects local search dependencies. Every query disables Cargo colors. Both
  Apple generators run this checker before any build or publication. Unified workspace and test builds deliberately
  enable development fixtures and do not establish the shipping feature graph.
  No additional compile-time rejection is added because it would also reject
  those intentional fixtures.
- `matrix-sdk-test` 0.19.1 declares only the opt-in
  `experimental-encrypted-state-events` feature and has no default features
  in its published manifest. Its exact pin therefore cannot implicitly enable
  forwarding, search, or X.509 defaults; adding `default-features = false`
  would not change this version's behavior.

## Remaining informational warnings

| Dependency and advisory | Reachability and disposition | Update owner and exit condition |
| --- | --- | --- |
| `lru` 0.16.4, [RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html) | Desktop local search: Matrix SDK search 0.19.1 → Tantivy 0.26.2. Tantivy's only `LruCache` is `LruCache<usize, Block>` in `src/store/reader.rs`; its production methods call `get`, `put`, and `len`, and its test-only helper calls `peek_lru`; none calls `pop`. Primitive `usize` keys have no `Drop` implementation. The advisory requires `pop`, a panicking key destructor, caught unwinding, and continued cache access; those conditions are absent in this pinned use. This is specific non-applicability, not a claim that the old crate is fixed. | Cargo/Matrix SDK maintainer: upgrade when SDK search admits Tantivy with `lru >=0.18.2`, or reassess immediately if cache key types, operations, or other consumers change. Tantivy currently requires `lru ^0.16.3`; forcing a 0.18 patch would violate that requirement. |
| `glib` 0.18.5, [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) | Linux Tauri GTK3/WebKit graph. The affected API is `Variant::array_iter_str` / `VariantStrIter`. Source inspection of Synara, Tauri 2.11.6, runtime-wry 2.11.4, Wry 0.55.1, GTK 0.18.2, GIO 0.18.4, GDK 0.18.2, WebKit2GTK 2.0.2, Soup 0.5.0, and GLib 0.18.5 found only GLib's own implementation, examples, and tests using that API. This bounds current evidence; it does not prove every transitive path unreachable. Warning is retained pending upstream migration. | Cargo/Tauri maintainer: follow the GTK/WebKit stack's upgrade to GLib >=0.20; reassess on Linux integration changes. Independently upgrading GLib would create incompatible GTK types. |
| `derivative` 2.2.0, [RUSTSEC-2024-0388](https://rustsec.org/advisories/RUSTSEC-2024-0388.html) | Lockfile-only optional WASM path: Matrix IndexedDB futures → `wasm_evt_listener` → derivative. `cargo tree --workspace --target all --edges all -i derivative` has no active consumer in the selected native workspace graphs. This SDK optional browser support is not the retired application IndexedDB implementation. Unmaintained macro crate; no reported vulnerability in this advisory. | Cargo/Matrix SDK maintainer: remove through an upstream optional-dependency replacement or removal; re-evaluate before enabling a WASM feature. |
| `proc-macro-error` 1.0.4, [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) | Linux GTK3/GLib macro generation via `glib-macros` and `gtk3-macros`. Build-time macro diagnostics; unmaintained status, no reported vulnerability in this advisory. | Cargo/Tauri maintainer: migrate with upstream GTK/GLib macro dependencies; reassess if it becomes a direct project dependency. |
| `unic-char-property` 0.9.0, [RUSTSEC-2025-0081](https://rustsec.org/advisories/RUSTSEC-2025-0081.html); `unic-char-range` 0.9.0, [RUSTSEC-2025-0075](https://rustsec.org/advisories/RUSTSEC-2025-0075.html); `unic-common` 0.9.0, [RUSTSEC-2025-0080](https://rustsec.org/advisories/RUSTSEC-2025-0080.html); `unic-ucd-ident` 0.9.0, [RUSTSEC-2025-0100](https://rustsec.org/advisories/RUSTSEC-2025-0100.html); `unic-ucd-version` 0.9.0, [RUSTSEC-2025-0098](https://rustsec.org/advisories/RUSTSEC-2025-0098.html) | Tauri utils 2.9.3 → URLPattern 0.3.0 Unicode identifier support in desktop build/runtime graph. Unmaintained status; these advisories report maintenance risk rather than a specific exploit. | Cargo/Tauri maintainer: migrate through a compatible Tauri utils/URLPattern release that replaces UNIC; keep package and security validation on the proposed upgrade. |

## Reproduction

Run from the repository root with the pinned Rust toolchain:

```sh
cargo audit
cargo metadata --locked --no-deps --format-version 1
cargo tree --locked -p synara-core --target all -i matrix-sdk@0.19.1
cargo tree --locked -p synara-core --target all -i uniffi@0.32.2
cargo tree --locked -p synara --target all -i quick-xml@0.41.0
cargo tree --locked -p synara --target all -i chacha20@0.10.2
cargo tree --locked -p synara --target all -i tauri@2.11.6
cargo tree --locked -p synara --target all -i zbus@5.19.0
cargo tree --locked -p synara --target all -i wayland-scanner
cargo tree --locked -p synara --target all -i lru
cargo tree --locked -p synara --target all -i glib
cargo tree --locked --workspace --target all --edges all -i derivative
node scripts/check-synara-nse-core-production-features.mjs
```

`metadata --no-deps` proves workspace membership and inherited metadata only;
the locked tree queries above prove the named dependency versions and consumers.

Review Tantivy `src/store/reader.rs` and its manifest against the locked version,
and scan the above Linux consumers for `VariantStrIter` and `array_iter_str`.
A future audit warning or dependency update must be assessed on its own
conditions; this document never authorizes adding broad ignores.
