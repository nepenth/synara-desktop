# Repository Layout

Reviewed: 2026-09-30

This repository owns all supported Synara clients and their shared core:

- the shared Rust core in `crates/synara-core/`;
- the macOS/Linux Tauri shell in `src-tauri/`;
- the embedded desktop runtime in `synara/`;
- the native SwiftUI client in `synara-ios/`.

The root Cargo workspace contains the desktop `synara` package, shared
`synara-core`, narrow `synara-nse-core`, and `synara-core-bindgen`. One root
`Cargo.lock` resolves their dependencies, with common versions declared in
`[workspace.dependencies]`. Standard desktop Cargo output is under `target/`,
including `target/release/bundle/`; Apple binding generators use their explicit
`target/synara-core-apple` output directory. Commands using
`--manifest-path src-tauri/Cargo.toml` still select the desktop package and
resolve the same root lockfile.

Standalone NSE validation must select the production NSE package and its
features explicitly. A feature-unified `cargo --workspace` graph is useful for
repository checks but is not evidence of a shipping notification extension.

Retired SDK experiments under `probes/` and
`synara-ios/spikes/matrix-sdk-probe/` retain README provenance records only.
Their original sources and manifests are recoverable from the dated Git
revision named there. They are excluded from current build instructions and
dependency maintenance.

`synara/` is now a normal tracked directory, not a Git submodule. Fresh clones
of `synara-desktop` do not need `--recursive` or any `git submodule` commands.
The former standalone runtime repository is not required for fresh clones,
builds, tests, or releases. Do not split product changes into a second runtime
repository.

Run this before committing repository-structure updates:

```sh
npm run check:repo-layout
```

CI runs the same check.

Repository layout acceptance criteria:

- `.gitmodules` does not exist.
- `synara/` is tracked directly by the parent repository.
- `synara/` does not contain nested Git metadata.
- `synara/` does not contain nested GitHub workflow automation.
- Fresh clones, local builds, and CI do not depend on another repository.

## Native domain ownership

`crates/synara-core/src/core.rs` contains the Core owner state and command registry.
Its `core/` modules keep command request validation and closed error mapping by
domain; `core/tests.rs` preserves transport and owner regression coverage.
`shared_core_ffi.rs` wires the UniFFI facade and reexports its established DTOs.
Its `shared_core_ffi/` modules keep typed domain methods, DTO projections, and
error conversions together. The UDL remains the generated binding contract.

The unused legacy-session transition harness and desktop marker-only linker
harness have been retired. Their dated migration documents remain historical
evidence. Production ownership is established through real Core/desktop callers
and compiler checks, rather than marker references or blanket module lint
exemptions.

See the [native maturity retirement ledger](rust-maturity-retirements.md) for
the retired graph, preserved live APIs, and validation boundaries.
