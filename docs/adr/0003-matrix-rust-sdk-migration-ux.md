# ADR 0003: Matrix Rust SDK migration UX

Reviewed: 2026-07-24

Status: proposed for Phase 0 (P0.7) — pending review. Single source of truth for
full decision text is under `docs/matrix-rust-sdk/`.

## Decision

Desktop cutover from `matrix-js-sdk` to Matrix Rust SDK uses a **required
one-time reauthentication**, a **new named Matrix device**, and **key recovery**
to establish a native SQLite-backed session. Legacy IndexedDB Matrix stores are
**not** converted and are **not** reopened by a JavaScript Matrix client after
cutover. They may remain **inert** for a bounded retention window, then are
removed only via **explicit, scoped, idempotent cleanup**.

There is **no** dual production backend, SDK selector, or concurrent reuse of
one device ID across two SDK stores. Access tokens and device IDs are **not**
copied into a fresh Rust crypto store by default (plan §8.1).

## Canonical record

- Full decision catalog, happy-path flow, failure modes, user copy, and
  acceptance checklist:
  [`../matrix-rust-sdk/migration-ux-decision.md`](../matrix-rust-sdk/migration-ux-decision.md)
- Machine twin:
  [`../matrix-rust-sdk/migration-ux-decision.json`](../matrix-rust-sdk/migration-ux-decision.json)
- Program plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)
  §3, §8, Phase 3

## Rationale

JavaScript IndexedDB state/crypto stores and native Rust SQLite stores are not
assumed compatible. Preferring new-device login plus recovery avoids identity and
decryption continuity hazards from token/device reuse against an empty crypto
store, matches full-replacement constraints, and keeps rollback operational
(prior build + inert legacy data) rather than dual-runtime.

## Consequences

- Phase 3 implementers (`P3.7` coordinator and auth tasks) must follow the
  canonical decision IDs (`D-LEGACY-DETECT` … `D-NO-DUAL-BACKEND`).
- FR-7.9-011 remains sequential single-active multi-account only; concurrent
  multi-account is out of scope for cutover UX.
- Product copy must set expectations for one-time sign-in and recovery without
  embedding secrets.
- P0.7 is documentation only; no production session/migration code ships with
  this ADR.
