# 13 — Language-boundary goal graph (loop operator)

This is the loop that finishes the work [ADR 0004](../adr/0004-rust-language-boundaries.md)
named. It does **not** replace the implementer playbook. It tells the next
turn which node to run.

**How to pick the next slice:** playbook section 5, then this graph.
**What done means:** playbook section 12, minus P5 (operator/Apple gated).
**What may be written in Rust:** ADR 0004.

GitHub Actions minutes are exhausted. Verify locally (`cargo test` when
disk ≥ 20 Gi). Delay Actions. Do not treat a skipped Actions run as
failure.

---

## Goal

iOS and desktop share one live `synara-core` engine for session, sync,
room list, timeline, and crypto. Native media is the sole decrypt path.
UI, Keychain, APNs, NSE lifecycle, and Node scripts stay put.

This goal is **not** P5, not a release, not a Slint/Dioxus/egui rewrite,
and not Tauri iOS.

---

## Loop

```text
1. Read this file + playbook §5.
2. Take the first node whose status is `next`.
3. Implement one slice. Local tests. Commit. Push. Open or update the PR.
4. Mark that node `landed` or `in-pr`. Set the following node to `next`.
5. Repeat until every `required` node is `landed` or `blocked`.
6. Stop on `blocked` (P5, Apple generate, live homeserver, merge).
```

Do not invent 7B leftovers. Do not start P5. Do not claim iOS-on-engine
until S13–S15 are landed and a live session actually syncs.

---

## Graph

```mermaid
flowchart TD
  adr0004[ADR0004_rubric]
  s12[S12_start_sync]
  s13[S13_restore_bootstrap]
  s14[S14_emit_sinks]
  s15[S15_live_leftover_io]
  s16[S16_product_timeline_rows]
  s17[S17_owner_emit_poll]
  s18[S18_product_timeline_live]
  s19[S19_room_list_live]
  s20[S20_product_verification]
  s21[S21_product_typing]
  s22[S22_product_room_details]
  s23[S23_product_foreground_resume]
  s24[S24_product_read_markers]
  media[desktop_native_media_cutover]
  done[p4_engine_ready]
  p5[P5_operator_gated]

  adr0004 --> s12
  s12 --> s13
  s13 --> s14
  s13 --> s15
  s14 --> s16
  s16 --> s17
  s17 --> s18
  s18 --> s19
  s19 --> s20
  s20 --> s21
  s21 --> s22
  s22 --> s23
  s23 --> s24
  s24 --> done
  s15 --> done
  media --> done
  done --> p5
```

| Node | Status | Kind | Slice |
|---|---|---|---|
| ADR 0004 rubric | `in-pr` #1000 | required docs | Can/should filter. No UI/CI rewrite. |
| P4-S12 start_sync | `in-pr` #1001 | required | Start already-attached SyncService. NSE still cannot start. |
| P4-S13 restore bootstrap | `in-pr` #1001 | required | Cold-start: restore vault session → attach → start. One product path with login. |
| P4-S14 emit sinks | `in-pr` #1001 | required | Timeline view-delta poll queue. Summaries only. NSE cannot poll. Product consume is S18. |
| P4-S15 leftover I/O live | `in-pr` #1001 | required | Owner leftover status after attach. Recover/media/raw-send stay fail-closed (decision 15). No byte/secret envelopes. |
| P4-S16 product timeline rows | `in-pr` #1001 | required | Snapshot DTO keeps privacy-safe row bodies. Product maps them. No media bytes. |
| P4-S17 owner emit poll | `in-pr` #1001 | required | Presence/devices/join_rules/image_packs poll queue. Summaries only. NSE cannot poll. |
| P4-S18 product timeline live poll | `in-pr` #1001 | required | Product `timelineUpdates` consumes S14 summaries. One host poller. |
| P4-S19 room-list live poll | `in-pr` #1001 | required | After start_sync, wake-ups only. Product `roomUpdates` re-fetches snapshot. No room ids. |
| P4-S20 product verification | `in-pr` #1001 | required | Product crypto calls list/SAS. `verification` family on the S17 owner queue. |
| P4-S21 product typing live | `in-pr` #1001 | required | `typing` family on the S17 owner queue (room id only). Product `typingUsers` re-fetches snapshot. |
| P4-S22 product room details | `in-pr` #1001 | required | Product `roomDetails` maps list / members / power / join-rule / invite snapshots. No media bytes. |
| P4-S23 product foreground resume | `in-pr` #1001 | required | `resumeFromForeground` uses the S13 bootstrap. Second start is a restart. No NSE. |
| P4-S24 product read markers | `in-pr` #1001 | required | Product mark-as-read uses `timeline_set_read_state`. No HTTP access token. |
| Desktop native media cutover | `blocked` | required | Both shells do not yet get bytes from a native owner. iOS leftover media stays fail-closed (decision 15). Bytes must not cross `Core::command`. Do not register `matrix_send_attachment`. |
| P4 engine ready | pending | gate | Session + sync + room list + timeline + crypto product paths call Core on iOS. Not claimed. |
| P5 | `blocked` | operator | Do not start. Apple/TestFlight/physical-device. |

---

## Current pointer

**S12–S24 are on #1001.** Session, sync, room list, timeline,
verification, typing, room-details, foreground-resume, and read-marker
product paths call Core. Media cutover stays **blocked** (decision
15). Do not start P5. Do not claim P4 engine ready. Stop if only
media/P5/Apple remain.

---

## Stop conditions

- Disk under 20 Gi and the slice needs cargo/UniFFI.
- The next node is P5.
- A leftover secret/byte command would have to cross `Core::command`.
- The only remaining work is Apple generate, a live homeserver, or a merge.
- Desktop native media cutover would need a native byte channel that is
  not `Core::command`, and iOS leftover media is fail-closed (decision 15).
