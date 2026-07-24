# P0.5 — Tauri 2 + matrix-sdk 0.18 coexistence probe

Isolated compile-only probe for **P0.5 toolchain compatibility**.

## Purpose

Prove that:

- Rust **1.93**
- edition **2024**
- **Tauri 2.11** (aligned with production `src-tauri`)
- **matrix-sdk** / **matrix-sdk-ui** `=0.18.0`

can coexist in a single Cargo package (dependency resolution + type-check).

This probe is **not** a full Tauri app (no frontend, no `tauri-build` app
manifest, no production integration). Production `src-tauri` is intentionally
unmodified and must not gain matrix-sdk deps until Phase 1 (after P0.5 gates).

## Related

- API-shape probe: [`../matrix-rust-sdk-0.18/`](../matrix-rust-sdk-0.18/)
- Report: [`../../docs/matrix-rust-sdk/toolchain-compatibility-report.md`](../../docs/matrix-rust-sdk/toolchain-compatibility-report.md)

## Validate

```sh
cd probes/tauri-matrix-sdk-compat
cargo check --locked
# optional:
cargo test --locked
```

Host expectation: `rustc`/`cargo` 1.93.x (stable).
