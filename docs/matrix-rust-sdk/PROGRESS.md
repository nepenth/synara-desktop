# Matrix Rust SDK replacement — live progress log

> **Remote-monitor file.** Open this on GitHub on the integration branch and refresh
> to see what the orchestrator has completed and what is next.
>
> **Branch:** [`feature/matrix-rust-sdk-full-replacement`](https://github.com/nepenth/synara-desktop/tree/feature/matrix-rust-sdk-full-replacement)  
> **This file on GitHub:**
> [docs/matrix-rust-sdk/PROGRESS.md](https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/PROGRESS.md)

| Field | Value |
| --- | --- |
| Last updated (UTC) | **2026-07-28 ~10:20** |
| Integration tip | `5db58a5` — Merge #161 P5.5 unread |
| Product runtime | Still **`matrix-js-sdk` only** until atomic sole-owner cutover |
| Dual backend | **`false`** (forbidden forever) |
| Operating model | [cutover-operating-model.md](cutover-operating-model.md) |
| Machine ledger | [program-status.md](program-status.md) (generated; do not hand-edit) |
| Short continuation | [CONTINUATION.md](CONTINUATION.md) |
| Full handoff | [implementation-handoff.md](implementation-handoff.md) |
| Umbrella → main | [PR #39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without explicit user approval** |

---

## Snapshot (read this first)

| | |
| --- | --- |
| **Now** | **#161 P5.5 unread merged** (tip `5db58a5`). Next **#162** raw-content, **#163** account-data. Parallel media #187–#188, #190, #192, #193. |
| **Inventory** | ~62/112 original task artifacts when program-status is synced (through P5.5 landed). |
| **Phase gates** | **0 / 15** strict gates closed (honest). |
| **Open PRs → integration** | [#162](https://github.com/nepenth/synara-desktop/pull/162) raw-content, [#163](https://github.com/nepenth/synara-desktop/pull/163) account-data, [#187](https://github.com/nepenth/synara-desktop/pull/187)/[#188](https://github.com/nepenth/synara-desktop/pull/188) media, [#190](https://github.com/nepenth/synara-desktop/pull/190) directory, [#192](https://github.com/nepenth/synara-desktop/pull/192) attachment-send, [#193](https://github.com/nepenth/synara-desktop/pull/193) crypto-bootstrap. |
| **Blocked on** | CI package for #162 after tip-merge. |
| **Dogfood path** | … → room-ops ✅ → **unread ✅ (#161)** → raw/account-data → media → crypto bootstrap → … |

---

## How this file is maintained

**Orchestrator / implementers must update this file** when:

1. A product or docs PR **merges** to the integration branch.
2. Active work **starts** (set “Now” + open PR link).
3. Priority or policy **changes** (user direction).

Update rules:

- Prepend new **Work log** entries (newest first).
- Keep **Snapshot** accurate (tip SHA, Now, open PRs).
- Prefer short bullets + PR numbers + one-line meaning.
- Do **not** claim phase-gate acceptance unless strict acceptance really closed.
- Commit as `docs(matrix): progress log — …` on a PR or as part of the landing PR.

---

## Work log (newest first)

### 2026-07-28

| When (UTC) | Item | Result | Notes |
| --- | --- | --- | --- |
| ~10:15 | **P5.5** unread | **Merged** [#161](https://github.com/nepenth/synara-desktop/pull/161) | tip `5db58a5`. |
| ~10:20 | **PROGRESS.md** after #161 | **This PR** | Next #162. |
| ~10:10 | **P8.9** crypto bootstrap | **PR open** [#193](https://github.com/nepenth/synara-desktop/pull/193) | Coordinator; local 6/6. |
| ~09:50 | **P7.4** attachment send | **PR open** [#192](https://github.com/nepenth/synara-desktop/pull/192) | AttachmentSendQueue. |
| ~09:35 | **P6.9** room-ops | **Merged** [#185](https://github.com/nepenth/synara-desktop/pull/185) | tip `f7981ea`. |
| ~09:35 | **PROGRESS.md** after #185 | **This PR** | Next #161. |
| ~09:10 | **P6.10** room directory | **PR open** [#190](https://github.com/nepenth/synara-desktop/pull/190) | RoomDirectorySession. |
| ~08:45 | **P5.7** polls | **Merged** [#160](https://github.com/nepenth/synara-desktop/pull/160) | tip `d71e8c6`. |
| ~08:45 | **PROGRESS.md** after #160 | **This PR** | Next #161–#163 + #185. |
| ~08:40 | **P7.3** media cache | **PR open** [#188](https://github.com/nepenth/synara-desktop/pull/188) | MediaCacheIndex; local 6/6. |
| ~08:25 | **P7.2** media download | **PR open** [#187](https://github.com/nepenth/synara-desktop/pull/187) | DownloadQueue. |
| ~08:00 | **P6.5** room profile | **Merged** [#183](https://github.com/nepenth/synara-desktop/pull/183) | tip `242ee57`. |
| ~08:00 | **P6.9** room ops | **PR open** [#185](https://github.com/nepenth/synara-desktop/pull/185) | RoomOpsQueue; local 7/7. |
| ~08:00 | **PROGRESS.md** after #183 | **This PR** | Mid-stack tip-merge after #183. |
| ~07:30 | **PROGRESS.md** after #182 | **This PR** | Tip-merge mid-stack; open #183 P6.5. |
| ~07:21 | **PROGRESS.md** after #165+#166 | **Merged** [#182](https://github.com/nepenth/synara-desktop/pull/182) | tip `f8fabae`. |
| ~07:25 | **P6.5** room profile | **PR open** [#183](https://github.com/nepenth/synara-desktop/pull/183) | RoomProfileIndex; local 8/8. |
| ~07:20 | **P8.7** UTD recovery | **Merged** [#166](https://github.com/nepenth/synara-desktop/pull/166) | tip `821177b`. |
| ~07:00 | **P8.6** room-keys | **Merged** [#165](https://github.com/nepenth/synara-desktop/pull/165) | tip `8d5cbd8`. |
| ~07:20 | **PROGRESS.md** after #165+#166 | **This PR** | Next mid-stack #160–#163. |
| ~06:35 | **P4.7** presence | **Merged** [#169](https://github.com/nepenth/synara-desktop/pull/169) | tip `fd87180`. |
| ~06:35 | **P4.7** presence stream | **Merged** [#169](https://github.com/nepenth/synara-desktop/pull/169) | PresenceIndex; tip `fd87180`. |
| ~06:05 | **P8.8** crypto-store | **Merged** [#168](https://github.com/nepenth/synara-desktop/pull/168) | tip `13dcb55`. |
| ~06:35 | **PROGRESS.md** after #168+#169 | **This PR** | Next #165/#166. |
| ~05:40 | **P6.6** user profile | **Merged** [#173](https://github.com/nepenth/synara-desktop/pull/173) | tip `edd6121`. |
| ~05:40 | **P6.6** user profile / ignore | **Merged** [#173](https://github.com/nepenth/synara-desktop/pull/173) | UserProfileIndex; tip `edd6121`. |
| ~05:40 | **PROGRESS.md** after #173 | **This PR** | Next #168 crypto-store. |
| ~05:15 | **PROGRESS.md** after #171 | **Merged** [#177](https://github.com/nepenth/synara-desktop/pull/177) | tip was `fc8dcaa`. |
| ~05:05 | **P3.4** UIA | **Merged** [#171](https://github.com/nepenth/synara-desktop/pull/171) | tip `3d1c46e`. |
| ~05:05 | **P3.4** UIA multi-stage | **Merged** [#171](https://github.com/nepenth/synara-desktop/pull/171) | Combined with SSO under auth; tip `3d1c46e`. |
| ~05:05 | **PROGRESS.md** after #171 | **This PR** | Next #173 profile + crypto stack. |
| ~04:31 | **P3.3** SSO | **Merged** [#170](https://github.com/nepenth/synara-desktop/pull/170) | tip `7d95461`. |
| ~04:32 | **PROGRESS.md** after #170 | **Merged** [#176](https://github.com/nepenth/synara-desktop/pull/176) | |
| ~04:30 | **P3.3** SSO / OAuth callback | **Merged** [#170](https://github.com/nepenth/synara-desktop/pull/170) | SsoCallbackFlow; tip `7d95461`. |
| ~04:30 | **PROGRESS.md** after #170 | **This PR** | Next #171 UIA. |
| ~04:05 | **P5.10** UTD | **Merged** [#157](https://github.com/nepenth/synara-desktop/pull/157) | tip `0c24e4b`. |
| ~04:11 | **PROGRESS.md** after #157 | **Merged** [#175](https://github.com/nepenth/synara-desktop/pull/175) | |
| ~04:05 | **P5.10** UTD / decrypt updates | **Merged** [#157](https://github.com/nepenth/synara-desktop/pull/157) | UtdIndex; tip `0c24e4b`. |
| ~04:05 | **PROGRESS.md** after #157 | **This PR** | Next #170/#171 auth + crypto stack. |
| ~03:45 | **PROGRESS.md** after #151 | **Merged** [#174](https://github.com/nepenth/synara-desktop/pull/174) | Tip was `6332042`. |
| ~03:40 | **P3.7** legacy transition | **Merged** [#151](https://github.com/nepenth/synara-desktop/pull/151) | Clean-break; tip `0698147`. |
| ~03:40 | **P3.7** legacy-session transition | **Merged** [#151](https://github.com/nepenth/synara-desktop/pull/151) | Clean-break; no JS/token continuity; tip `0698147`. |
| ~03:40 | **PROGRESS.md** after #151 | **This PR** | Next #157 UTD. |
| ~03:25 | **P6.6** user profile / ignore | **PR open** [#173](https://github.com/nepenth/synara-desktop/pull/173) | UserProfileIndex; local 6/6. |
| ~03:21 | **PROGRESS.md** after #154 | **Merged** [#172](https://github.com/nepenth/synara-desktop/pull/172) | Tip was `e6caca9`. |
| ~03:15 | **P5.4** timeline focus / context | **Merged** [#154](https://github.com/nepenth/synara-desktop/pull/154) | TimelineFocus Live/Unread/Focused; tip `5380471`. |
| ~03:15 | **PROGRESS.md** after #154 | **This PR** | Next #151; note #157 conflict. |
| ~02:55 | **P3.4** UIA multi-stage | **PR open** [#171](https://github.com/nepenth/synara-desktop/pull/171) | UiaSession; local auth 49/49. |
| ~02:44 | **PROGRESS.md** | **Merged** [#167](https://github.com/nepenth/synara-desktop/pull/167) | Tip was `d9009ca` before #154. |
| ~02:42 | **P3.3** SSO / OAuth callback | **PR open** [#170](https://github.com/nepenth/synara-desktop/pull/170) | SsoCallbackFlow; no tokens/codes; local auth 48/48. |
| ~02:35 | **P3.8** remote logout + recovery copy | **Merged** [#155](https://github.com/nepenth/synara-desktop/pull/155) | RemoteLogoutFlow + RecoveryCopyKey; tip `0e6399d`. |
| ~02:35 | **PROGRESS.md** after #155 | **This PR** | Next #151 legacy; tip-merged open stack. |
| ~02:30 | **P4.7** presence stream index | **PR open** [#169](https://github.com/nepenth/synara-desktop/pull/169) | PresenceIndex; local 8/8; clippy+guardrails. |
| ~02:30 | **PROGRESS.md** refresh | **This PR** | Open #168/#169; CI queue triage (cancel non-priority package smoke). |
| ~02:24 | **P8.8** crypto-store continuity | **PR open** [#168](https://github.com/nepenth/synara-desktop/pull/168) | Never auto-wipe; no keys. |
| ~02:15 | **PROGRESS.md** after #150 | **This PR** | Tip `c3c630e`; next #151 legacy. |
| ~02:13 | **P8.5** key backup / recovery | **Merged** [#150](https://github.com/nepenth/synara-desktop/pull/150) | BackupRecoveryFlow; no recovery keys; tip `c3c630e`. |
| ~02:15 | **P8.7** UTD recovery coordinator | **PR open** [#166](https://github.com/nepenth/synara-desktop/pull/166) | Room-level retry/history recovery. |
| ~01:58 | **P8.6** room-key transfer | **PR open** [#165](https://github.com/nepenth/synara-desktop/pull/165) | Export/import flow; no key material. |
| ~00:55 | **PROGRESS.md** after #145 | **This PR** | Tip `5799d16`; open stack tip-merged; next #147. |
| ~00:54 | **P8.2** device list / trust | **Merged** [#145](https://github.com/nepenth/synara-desktop/pull/145) | DeviceIndex; no keys; tip `5799d16`. |
| ~00:54 | **P5.10** UTD / decrypt updates | **PR open** [#157](https://github.com/nepenth/synara-desktop/pull/157) | UtdIndex; no session keys / bodies. |
| ~00:38 | **P3.8** remote logout + recovery copy | **PR open** [#155](https://github.com/nepenth/synara-desktop/pull/155) | RemoteLogoutFlow + RecoveryCopyKey. |
| ~00:32 | **P5.4** timeline focus / context | **PR open** [#154](https://github.com/nepenth/synara-desktop/pull/154) | TimelineFocus Live/Unread/Focused. |
| ~00:29 | **PROGRESS.md** after #143 | **Merged** [#153](https://github.com/nepenth/synara-desktop/pull/153) | After P9.1 widgets. |

### 2026-07-27

| When (UTC) | Item | Result | Notes |
| --- | --- | --- | --- |
| ~00:13 | **P9.1** widget / Element Call registry | **Merged** [#143](https://github.com/nepenth/synara-desktop/pull/143) | WidgetRegistry; no token URLs; tip `1e42d27`. |
| ~00:15 | **PROGRESS.md** after #143 | **This PR** | Next devices/verification/backup/legacy tip-merged. |
| ~23:36 | **PROGRESS.md** after #149 | **This PR** | Tip `f71be28`; open #143/#145/#147/#150/#151 tip-merged. |
| ~23:34 | **P8.4** cross-signing / identity | **Merged** [#149](https://github.com/nepenth/synara-desktop/pull/149) | CrossSigningStore presence + IdentityTrust; tip `f71be28`. |
| ~23:27 | **P3.7** legacy transition coordinator | **PR open** [#151](https://github.com/nepenth/synara-desktop/pull/151) | Clean-break; no JS client / token continuity. |
| ~23:15 | **P8.5** backup / recovery flow | **PR open** [#150](https://github.com/nepenth/synara-desktop/pull/150) | BackupRecoveryFlow; no recovery keys stored. |
| ~23:05 | **PROGRESS.md** after #141 | **Merged** [#148](https://github.com/nepenth/synara-desktop/pull/148) | After P6.8 search. |
| ~23:00 | **P6.8** search session foundation | **Merged** [#141](https://github.com/nepenth/synara-desktop/pull/141) | SearchSession request-id stale protection; tip `d6ef679`. Quality + package gate green. |
| ~22:53 | **P8.2** clippy fix + tip-merge | **Pushed** on [#145](https://github.com/nepenth/synara-desktop/pull/145) | `bool_assert_comparison` → `assert!`; tip after #141. |
| ~22:45 | **P8.3** verification inbox / SAS display | **PR open** [#147](https://github.com/nepenth/synara-desktop/pull/147) | VerificationInbox; no secrets; local 7/7. |
| ~22:34 | **P4.8** route / deep-link resolution | **Merged** [#139](https://github.com/nepenth/synara-desktop/pull/139) | resolve_path/build_path; tip `a74fb78`. |
| ~22:18 | **P8.2** device list / trust projection | **PR open** [#145](https://github.com/nepenth/synara-desktop/pull/145) | DeviceIndex; no keys; local 6/6. |
| ~22:09 | **P8.1** security status projection | **Merged** [#137](https://github.com/nepenth/synara-desktop/pull/137) | SecurityStatusStore no keys/secrets; tip `f461a00`. |
| ~21:50 | **P9.1** widget session registry | **PR open** [#143](https://github.com/nepenth/synara-desktop/pull/143) | WidgetRegistry forbids token URLs; local 7/7. |
| ~21:42 | **P7.1** notification candidate index | **Merged** [#135](https://github.com/nepenth/synara-desktop/pull/135) | NotificationIndex suppress/dedup/cap; tip `848dc14`. |
| ~21:27 | **P6.8** search session foundation | **PR open** [#141](https://github.com/nepenth/synara-desktop/pull/141) | SearchSession request-id stale protection; local 7/7. |
| ~21:17 | **P4.6** member / power-level index | **Merged** [#133](https://github.com/nepenth/synara-desktop/pull/133) | MemberIndex power-ordered; tip `22ec745`. |
| ~20:58 | **P4.8** route / deep-link resolution | **PR open** [#139](https://github.com/nepenth/synara-desktop/pull/139) | resolve_path/build_path; local 7/7. |
| ~20:50 | **P5.8** thread list index foundation | **Merged** [#131](https://github.com/nepenth/synara-desktop/pull/131) | ThreadIndex activity order + cap; tip `27f870e`. |
| ~20:30 | **P8.1** security status projection | **PR open** [#137](https://github.com/nepenth/synara-desktop/pull/137) | SecurityStatusStore; no keys/secrets; local 5/5. |
| ~20:24 | **P5.6** relations index foundation | **Merged** [#129](https://github.com/nepenth/synara-desktop/pull/129) | RelationIndex reactions/replaces/refs/threads; tip `116ed3d`. |
| ~20:06 | **P7.1** notification candidate index | **PR open** [#135](https://github.com/nepenth/synara-desktop/pull/135) | NotificationIndex suppress/dedup/cap; local 7/7. |
| ~19:58 | **P6.4** media upload queue foundation | **Merged** [#128](https://github.com/nepenth/synara-desktop/pull/128) | UploadQueue metadata-only; tip `f44bc5c`. Dogfood send/receipts/typing/media foundations landed. |
| ~19:42 | **P4.6** member / power-level index | **PR open** [#133](https://github.com/nepenth/synara-desktop/pull/133) | MemberIndex; local 6/6. |
| ~19:33 | **P6.3** typing index foundation | **Merged** [#127](https://github.com/nepenth/synara-desktop/pull/127) | TypingIndex + cap 32; tip `ef8bf60`. Quality + package gate green. |
| ~19:15 | **P5.8** thread list index foundation | **PR open** [#131](https://github.com/nepenth/synara-desktop/pull/131) | ThreadIndex over ThreadSummary; local 6/6. |
| ~19:06 | **P6.2** receipt index foundation | **Merged** [#125](https://github.com/nepenth/synara-desktop/pull/125) | ReceiptIndex over DTO Receipt; tip `ef75e3e`. Quality + package gate green. |
| ~18:52 | **P5.6** relations index foundation | **PR open** [#129](https://github.com/nepenth/synara-desktop/pull/129) | RelationIndex annotations/replace/reference/thread; local 8/8. |
| ~18:39 | **P6.1** outbound send queue foundation | **Merged** [#124](https://github.com/nepenth/synara-desktop/pull/124) | SendQueue + LocalEchoState; tip `c6cbc2c`. Quality ✅; package gate after Arch Docker Hub flake re-run. |
| ~18:35 | Tip-merge P6.2/P6.3/P6.4 onto tip | **Pushed** | #125/#127/#128 had green CI on pre-#124 tip; re-tip after #124. Local receipts 7/7 + send 8/8. |
| ~17:58 | **P5.3** timeline pagination foundation | **Merged** [#122](https://github.com/nepenth/synara-desktop/pull/122) | TimelinePagination state machine; tip `ed5b3c3`. |
| ~16:00 | **P4.2** room-list snapshot/delta | **Merged** [#115](https://github.com/nepenth/synara-desktop/pull/115) | Pure projection + ordered ops; tip `c2cdc0b`. iOS/Synapse skipped. |
| ~15:56 | **P6.1** outbound send queue foundation | **PR open** [#124](https://github.com/nepenth/synara-desktop/pull/124) | LocalEchoState queue; no Room::send; local 8/8. Disk pressure: cleaned cargo target (-34GB). |
| ~15:40 | **P4.1** sync readiness + reconnect | **Merged** [#114](https://github.com/nepenth/synara-desktop/pull/114) | `matrix/sync/`: readiness, reconnect table, SyncServiceOwner, guardrail confine. Tip `f9bfe0d`. Full CI green (iOS skipped via path filters). |
| ~15:38 | **P5.3** timeline pagination foundation | **PR open** [#122](https://github.com/nepenth/synara-desktop/pull/122) | Pure `TimelinePagination` state machine; local timeline 23/23. CI deferred until stack advances. |
| ~15:11 | **P5.2** timeline snapshot/diff projection | **PR open** [#121](https://github.com/nepenth/synara-desktop/pull/121) | `TimelineProjection` + ordered ops; local 16/16 then extended by P5.3. |
| ~15:40 | **#115** tip-merged after #114 | **Pushed** | Local matrix 270/270 + clippy + guardrails; CI re-run for P4.2 merge. |
| ~14:47 | P4.3+ clippy `needless_borrow` | **Fixed** on [#116](https://github.com/nepenth/synara-desktop/pull/116)–[#119](https://github.com/nepenth/synara-desktop/pull/119) | Lint Rust failed at `room_list/tests.rs` scope test; dropped extra `&` on `find().unwrap()`. Branches rebased/merged onto tip. |
| ~14:46 | Tip update-branch #114/#115/#109 | **Kicked** | After #111 merge so product PRs not BEHIND; #109 should get path-filtered skip of iOS. |
| ~14:45 | **PROGRESS.md** live work log | **Merged** [#111](https://github.com/nepenth/synara-desktop/pull/111) | Docs-only CI: heavy jobs skipped, Quality gate green. Tip `b3397db`. |
| ~14:42 | CI path filters for heavy jobs | **Merged** [#113](https://github.com/nepenth/synara-desktop/pull/113) | Job-level scopes; quality-gate accepts success\|skipped. Prior tip `168ca2b`. |
| ~14:30 | **P5.1** timeline registry foundation | **PR open** [#119](https://github.com/nepenth/synara-desktop/pull/119) | TimelineRegistry lifecycle; local 8/8. |
| ~14:25 | **P4.5** space hierarchy foundation | **PR open** [#118](https://github.com/nepenth/synara-desktop/pull/118) | SpaceHierarchy + filter/cycle; local 6/6. |
| ~14:21 | **P4.4** favorite/low-priority/folder/recent | **PR open** [#117](https://github.com/nepenth/synara-desktop/pull/117) | DTO tag fields + sorts; local room_list 15/15. |
| ~14:08 | **#114** rebased on tip after #112 | **Pushed** `d0ab3e5` | Combined lifecycle restore + sync guardrail zones; matrix tests 270/270. #115/#113 also tip-merged. |
| ~14:07 | **P3.6** session restore | **Merged** [#112](https://github.com/nepenth/synara-desktop/pull/112) | Vault → identity bind → `restore_session`. Tip `69f1087`. Full CI green (iOS ~23m). |
| ~14:00 | **P4.2** room-list snapshot/delta | **PR open** [#115](https://github.com/nepenth/synara-desktop/pull/115) | Pure `RoomListProjection` + delta ops + sequence gap resync. Local 10/10; stacks on #114. |
| ~13:53 | **P4.1** sync readiness foundation | **PR open** [#114](https://github.com/nepenth/synara-desktop/pull/114) | `matrix/sync/`: readiness map, reconnect table, SyncServiceOwner, guardrail confine `SyncService::builder`. Local 12/12 + clippy + guardrails green. |
| ~13:52 | CI path-filter policy checker fix | **Pushed** `09bd360` on [#113](https://github.com/nepenth/synara-desktop/pull/113) | First CI run failed `check:quality-gates` (expected needs lacked `changes` + skipped). Checker now matches path-filtered Quality gate. |
| ~13:48 | MiniMax tooling #109 | **CI fail** | iOS job cancelled (~45m hang); Quality gate failed. Not product path — deprioritize; merge after #113 if still wanted. |
| ~13:45 | CI path filters for heavy jobs | **PR open** [#113](https://github.com/nepenth/synara-desktop/pull/113) | Docs-only skip full suite; src-tauri skips iOS/Synapse. |
| ~13:40 | **P3.6** rustfmt CI fix | **Pushed** `78c61ea` on [#112](https://github.com/nepenth/synara-desktop/pull/112) | `cargo fmt --check` failed on test wrapping; local tests 5/5 + lifecycle 36/36 + guardrails PASS. CI re-run. |
| ~13:29 | **P3.6** session restore foundation | **PR open** [#112](https://github.com/nepenth/synara-desktop/pull/112) | Vault → identity bind → `restore_session` under lifecycle only. |
| ~13:30 | **PROGRESS.md** live work log introduced | **PR open** [#111](https://github.com/nepenth/synara-desktop/pull/111) | Remote-monitor file for orchestrator updates. |
| ~13:23 | **P3.5** session secret / refresh-token persistence | **Merged** [#110](https://github.com/nepenth/synara-desktop/pull/110) | Host keyring vault + `persist_session_after_login`. Tip `8b7d39e`. |
| ~12:57 | Cutover **operating model** docs | **Merged** [#108](https://github.com/nepenth/synara-desktop/pull/108) | Canonical capability slices + atomic sole-owner cutover. |
| ~12:36 | **P3.2** password/token login + device naming | **Merged** [#107](https://github.com/nepenth/synara-desktop/pull/107) | Harness login under `matrix/auth/`; D-NEW-DEVICE names; guardrail allowlist. |
| earlier | **R0.2-E1** traceability tooling | **Merged** [#82](https://github.com/nepenth/synara-desktop/pull/82) | Governance tooling; not product cutover. |
| earlier | R0.3–R0.8 Critical/High remediations | **Merged** #86–#104 band | Wipe, keyring, privacy, IPC, live adapters, formal residual reports. |
| policy | Product-first + clean-break | **User-approved** | Re-login/wipe OK; no dual-backend; no elaborate JS→Rust session migration. |
| tooling | Local MiniMax (Spark) for bulk draft/review | Config + open PR [#109](https://github.com/nepenth/synara-desktop/pull/109) | Free-token parallel text worker; Grok remains implementer. |

### Earlier foundation (condensed)

| Band | State |
| --- | --- |
| Phase 0 planning artifacts P0.1–P0.7 | Landed (strict gate **open**) |
| Phase 1 IPC/DTO/guardrails P1.1–P1.6 | Landed (strict gate **open**) |
| Phase 2 supervisor/store/builder/tasks/diagnostics/lifecycle P2.1–P2.6 | Landed harness (strict gate **open**) |
| P3.1 discovery + login-flow list | Landed |

---

## Roadmap strip (capability order)

| # | Slice | Status |
| ---: | --- | --- |
| 1 | Discovery / login-flow list (P3.1) | **Done** (artifact) |
| 2 | Password/token login + device name (P3.2) | **Done** (merged) |
| 3 | Session secret persist / refresh structure (P3.5) | **Done** (merged) |
| 4 | Session restore after restart (P3.6) | **Done** (merged #112) |
| 5 | Sync readiness / reconnect (P4.1) | **Done** (merged #114) |
| 6 | Room list snapshot/delta (P4.2) | **Done** (merged #115) |
| 7 | Membership / unread / invites (P4.3) | **Done** (merged #116) |
| 8 | Favorite / low-priority / recent (P4.4) | **Done** (merged #117) |
| 9 | Space hierarchy (P4.5) | **Done** (merged #118) |
| 10 | Timeline registry (P5.1) | **Done** (merged #119) |
| 11 | Timeline diffs (P5.2) | **Done** (merged #121) |
| 12 | Timeline pagination (P5.3) | **Done** (merged #122) |
| 13 | Send queue / local echo (P6.1) | **Done** (merged #124) |
| 14 | Receipt index (P6.2) | **Done** (merged #125) |
| 15 | Typing index (P6.3) | **Done** (merged #127) |
| 16 | Media upload queue (P6.4) | **Done** (merged #128) |
| 17 | Relations / reactions (P5.6) | **Done** (merged #129) |
| 18 | Thread list / summaries (P5.8) | **Done** (merged #131) |
| 19 | Member / power-level index (P4.6) | **Done** (merged #133) |
| 20 | Notification candidates (P7.1) | **Done** (merged #135) |
| 21 | Security status projection (P8.1) | **Done** (merged #137) |
| 22 | Route / deep-link resolution (P4.8) | **Done** (merged #139) |
| 23 | Search session (P6.8) | **Done** (merged #141) |
| 24 | Cross-signing / identity (P8.4) | **Done** (merged #149) |
| 25 | Widget / Element Call registry (P9.1) | **Done** (merged #143) |
| 26 | Device list / trust (P8.2) | **In PR** [#145](https://github.com/nepenth/synara-desktop/pull/145) |
| 27 | Verification inbox / SAS display (P8.3) | **In PR** [#147](https://github.com/nepenth/synara-desktop/pull/147) |
| 28 | Backup / recovery flow (P8.5) | **In PR** [#150](https://github.com/nepenth/synara-desktop/pull/150) |
| 29 | Legacy transition coordinator (P3.7) | **In PR** [#151](https://github.com/nepenth/synara-desktop/pull/151) |
| 30 | Remaining crypto / UTD / store continuity | Not started |
| 31 | Atomic sole-owner cutover + js-sdk burn-down (P11) | Not started |
| 32 | Merge to `main` (#39) | Needs **explicit user approval** |


---

## Links for phone / remote refresh

| What | URL |
| --- | --- |
| **This progress log** | https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/PROGRESS.md |
| Integration branch commits | https://github.com/nepenth/synara-desktop/commits/feature/matrix-rust-sdk-full-replacement |
| Open PRs into integration | https://github.com/nepenth/synara-desktop/pulls?q=is%3Apr+is%3Aopen+base%3Afeature%2Fmatrix-rust-sdk-full-replacement |
| Machine status ledger | https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/program-status.md |
| Umbrella PR (do not merge) | https://github.com/nepenth/synara-desktop/pull/39 |
