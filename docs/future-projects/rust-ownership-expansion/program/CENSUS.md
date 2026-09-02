# Historical starting source snapshot

Status: historical starting point for overnight memos. Not live inventory.

Recorded against `main` `011cf39a` on 2026-09-01 during the portfolio
review. Re-verify every path on the commit named in the memo. If source
and this table disagree, source wins and the memo must say so.

| Domain | Core / Rust | Desktop | iOS | Snapshot note |
| --- | --- | --- | --- | --- |
| Sync, restore, crypto/backup/cross-signing status | `app/lifecycle/`, `app/sync/`, `app/backup/`, `app/cross_signing/`, registered status commands | Thin Tauri bridges; password, keyring, recovery, and media-byte leftovers stay shell-side | `SharedCore*` session/sync wrappers; leftover recover/media/pusher fail-closed | Prior: already owned |
| Verification | `app/verification/live.rs`, `matrix_verification_*` | Presentation helpers in `nativeVerification.ts` | `SharedCoreVerification*`; extra device-key continuity policy may live in `MatrixClientPolicies.swift` | Prior: already owned; iOS continuity is the only open question |
| Timeline rows / relations | `app/timeline/live.rs`, `view.rs` (`TimelineViewRow`) | Viewport + HTML render (`nativeTimelineViewportPolicy.ts`, `nativeTimelineRichText.ts`) | `SharedCoreTimeline*`; `MatrixHTMLRenderer` semantic segments | Prior: extend rows only |
| Formatted body | Outbound `markdown_to_html()`; inbound `formatted_body` string | Sanitize-html → React parse + Prism | Typed semantic tree in `MatrixHTMLRenderer` (shipped 2.1.11) | Prior: stay platform-side; fixtures first |
| Read / unread | Room-list counts; `matrix_timeline_set_read_state`; `matrix_room_set_read_state` | Auto-read / focus gating in viewport policy | Foreground/background + `SharedCoreReadMarkers` | Prior: visibility contract remainder |
| Room sort / filter | `app/room_list/sort.rs` and `filters.rs` existed and were unused by product UIs at snapshot | Device-local sort in `homeRoomList.ts` | Swift sort/filter in `RoomListService.swift` | Prior: census whether Core helpers are consumed |
| Notifications | Push rules and room notification modes | OS delivery in `desktop_notifications.rs` | APNs / NSE platform-owned | Prior: policy yes, delivery no |
| Agent approval | `app/agent_approvals.rs`; `matrix_agent_approval_decide` | Parallel planner in `agentApprovals.ts` | Parallel planner in notification/PushService paths | Highest residual; do not implement tonight |
| Notes | `matrix_room_notes_*` | Presenter panel | `SharedCoreRoomNotes` + SwiftUI editor | Prior: already owned |
| Drafts | Reply/thread draft commands on the timeline owner | Slate / Jotai composer body local-only | Core reply draft + local SwiftUI composer state | Prior: split |
| Media | Opaque `TimelineMediaHandle`; bytes off `Core::command` (ADR 0005) | Shell / `synara-media://` resolve | UniFFI byte channel by handle | Prior: metadata only |
| Validation | Envelope `deny_unknown_fields`, IDs, notes limits, outbound HTML sanitize | Display HTML sanitizer + desktop route sanitize | `MatrixHTMLRenderer` allowlist | Prior: shared rules/fixtures, not one sanitizer |

`TimelineViewRow` is already the event/row semantic model. It is not a
formatted-body AST.

At snapshot time, `TimelineMessageRow.formatted_body` in
`crates/synara-core/src/app/timeline/view.rs` was documented in a way
that could be read as universally sanitized. Presenters still sanitize.
Memos that touch ROE-04/12 must confirm whether that comment remains
misleading.

Playbook leftover secret/byte command names are a closed desktop set.
Do not propose registering them on `Core::command`.
