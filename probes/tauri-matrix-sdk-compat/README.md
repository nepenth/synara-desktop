# Retired SDK experiment

Retired: 2026-09-30.

This directory is a provenance record. Its executable sources, package
manifests, and lockfiles have been removed now that the product uses the
project-owned Rust Core and Matrix Rust SDK 0.19.1. It is not an active build,
test, dependency-update, or supported Matrix client path.

The original experiment can be inspected at repository commit `0a3d2ce95622e7ae4326dd8a3be45cb5240a3ea2`:

```sh
git show 0a3d2ce95622e7ae4326dd8a3be45cb5240a3ea2:probes/tauri-matrix-sdk-compat/README.md
git ls-tree -r 0a3d2ce95622e7ae4326dd8a3be45cb5240a3ea2 -- probes/tauri-matrix-sdk-compat
```

Historical measurements and capability reports retain their original version,
source paths, and conclusions; they do not establish current runtime behavior.
Current architecture is [ADR 0003](../../docs/adr/0003-shared-native-rust-core.md)
and [ADR 0004](../../docs/adr/0004-rust-language-boundaries.md).

---

## Original experiment record

The following README is retained as historical evidence. Its commands refer
to files at the commit above and cannot be run from this retired directory.

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
