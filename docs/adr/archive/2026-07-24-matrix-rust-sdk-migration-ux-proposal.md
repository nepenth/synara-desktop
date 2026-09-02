# Archived Proposal: Matrix Rust SDK Migration UX

Originally reviewed: 2026-07-24.

Status: archived proposal; never accepted as a numbered ADR.

This document was originally titled “ADR 0003: Matrix Rust SDK migration UX”
while its status remained “proposed for Phase 0 (P0.7) — pending review.” The
accepted ADR sequence later assigned 0003 to the
[shared native Rust core](../0003-shared-native-rust-core.md). This proposal is
archived here to remove that duplicate identity without rewriting history.

The migration policy selected a required clean-break reauthentication, a new
Matrix device, key recovery, inert legacy IndexedDB retention, and no dual
production backend. The full decision catalog and implementation-era evidence
remain in the historical
[migration UX record](../../matrix-rust-sdk/migration-ux-decision.md).

Current product architecture is governed by ADRs
[0003](../0003-shared-native-rust-core.md),
[0004](../0004-rust-language-boundaries.md), and
[0005](../0005-native-media-handle-channel.md). This archived proposal does not
authorize new migration behavior or override current session and release
requirements.
