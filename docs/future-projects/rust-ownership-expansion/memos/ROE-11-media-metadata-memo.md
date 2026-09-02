# ROE-11 Research Memo: Media Metadata and Cache Policy

Status: draft research; docs-only; not approved for implementation.

| Field              | Value                                                                                                                      |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-11                                                                                                                     |
| Research owner     | Isolated researcher on `roe/memo-11-media-metadata`                                                                        |
| Reviewers          | Unassigned                                                                                                                 |
| Source census      | 2026-09-01 against `5f81d9e7d1fccd762e16dad645bea8f07a216675`                                                              |
| ADR baseline       | ADR 0003, 0004, 0005 last reviewed 2026-09-01 (index in [`docs/adr/README.md`](../../../adr/README.md)); same census commit |

[program/CENSUS.md](../program/CENSUS.md) recorded media on `main` `011cf39a`
as opaque `TimelineMediaHandle`, bytes off `Core::command` (ADR 0005), desktop
`synara-media://` resolve, and iOS UniFFI bytes by handle. Re-read source on
this commit agrees with that channel split. Source wins on details the snapshot
does not name: the P7.3 `MediaCacheIndex` is an unused harness; leftover UniFFI
`media_download` / `media_thumbnail` / `media_upload` stay fail-closed; leftover
avatar/pack still uses a bounded plain `mxc://` as a control-plane identifier;
live `TimelineMediaHandle` does not project `size_bytes` or a thumbnail handle;
logout is not wipe.

This memo does not authorize product work, a Core cache engine, a new envelope
field, leftover registration on `Core::command`, or a shared-Core phase change.
Paths and bytes on the generic envelope remain prohibited even if a metadata
field is later added.

## Observable problem

A user opens an image, video, file, or sticker that may be encrypted, oversized,
corrupt, or leftover `mxc://`. The residual question is whether MXC identity,
encryption metadata, thumbnail identity, MIME/size/dimension claims, cache
eligibility, quota, retry, corruption recovery, logout wipe, or diagnostics
still have competing owners on desktop and iOS.

The user-visible risk is not whether React and SwiftUI look the same. It is
whether either presenter reconstructs an `mxc://` presentation source, decrypts
timeline media, or invents cache/wipe policy that Core already owns.

No current source evidence shows a second media engine. Timeline rows already
expose opaque handles plus bounded declared metadata. Dedicated native channels
carry bytes. Filesystem paths, file handoff, and NSE storage stay platform-side.

## Current ownership census

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| MXC / encrypted-source *identity* | Authority. `TimelineMediaRegistry` stores SDK `MediaSource` (plain or encrypted) behind `timeline-media-` + 64 hex. Handle JSON must not contain `mxc://`. Cap 4_096. Re-projection of the same item keeps the handle; `retain_items` / `revoke_item` / `Drop` revoke. | Observation of the opaque id only. `nativeTimelineMediaSrc` builds `synara-media://…/<handle>`. Protocol and leftover `matrix_media_download` resolve `timeline-media-*` through `resolve_timeline_media`. | Observation. `SharedCoreTimelineRows.mediaPlaceholder` builds `synara-timeline-media://<handle>`. Product loader calls `timelineMediaBytes`. | Core [`media.rs`](../../../../crates/synara-core/src/app/timeline/media.rs), [`view.rs`](../../../../crates/synara-core/src/app/timeline/view.rs) `TimelineMediaHandle`, [`live.rs`](../../../../crates/synara-core/src/app/timeline/live.rs) `resolve_media`. Tests: [`media.rs` unit tests](../../../../crates/synara-core/src/app/timeline/media.rs), [`p4_s33_timeline_media.rs`](../../../../crates/synara-core/tests/p4_s33_timeline_media.rs), [`p4_s36_desktop_media_cutover.rs`](../../../../crates/synara-core/tests/p4_s36_desktop_media_cutover.rs). Desktop: [`lib.rs`](../../../../src-tauri/src/lib.rs) protocol, [`product_commands.rs`](../../../../src-tauri/src/matrix/media/product_commands.rs), [`nativeTimelineView.ts`](../../../../synara/src/app/features/room/nativeTimelineView.ts). iOS: [`SharedCoreTimelineRows.swift`](../../../../synara-ios/Synara/Services/SharedCoreTimelineRows.swift), [`SharedCoreTimelineMedia.swift`](../../../../synara-ios/Synara/Services/SharedCoreTimelineMedia.swift). |
| Encryption metadata / decrypt | Authority. Encrypted-file descriptors stay inside `TimelineMediaSource`. `download_media_bounded` downloads then decrypts with the retained source. Presenter DTOs have no keys. | Must not decrypt. Leftover encrypted `mxc://` without a handle fail-closes in `downloadMatrixMedia`. Native protocol uses the resolved `MediaSource`. | Must not decrypt. Product timeline sets `isEncrypted: false` on handle URLs because Core already owns decrypt. Leftover `mxc` + `isEncrypted` fail-closes in `MockMediaLoader` / `SharedCoreMediaLoader`. Leftover UniFFI `mediaDownload` is unused by product. | [`bounded.rs`](../../../../crates/synara-core/src/app/media/bounded.rs); desktop [`media.ts`](../../../../synara/src/app/matrix/media.ts) + [`media.test.ts`](../../../../synara/src/app/matrix/__tests__/media.test.ts); iOS [`SharedCoreProductServices.swift`](../../../../synara-ios/Synara/Services/SharedCoreProductServices.swift) `SharedCoreMediaLoader`, [`MediaServiceTests.swift`](../../../../synara-ios/SynaraTests/MediaServiceTests.swift). |
| Thumbnails | Authority for *which source* is fetched: timeline handles always request `MediaFormat::File`. Invite avatars use a 96×96 thumbnail of a plain MXC. Plain leftover `thumbnail_plain_media` is a typed method, not `Core::command`. Live rows have no `thumbnail_handle_id`. | Rendering. Protocol serves original-file bytes; leftover avatar/pack hook `useNativeMatrixMediaSrc` may pass leftover `mxc://` into `matrix_media_download` (File format). `ImageContent` / pin / search / inbox leftovers still exist off the native timeline. | Rendering. Handle path: `loadThumbnail` returns the resource; `loadThumbnailData` fetches full handle bytes. Leftover `mxc` uses `SharedCorePlainMedia.thumbnail`. | Core [`plain.rs`](../../../../crates/synara-core/src/app/media/plain.rs); desktop protocol + [`useNativeMatrixMediaSrc.ts`](../../../../synara/src/app/hooks/useNativeMatrixMediaSrc.ts); iOS `SharedCoreMediaLoader`. |
| MIME, dimensions, duration | Authority projects *declared* protocol claims onto the handle (`mimetype`, width/height, duration). No `size_bytes` on the live handle. Filename/caption are separate row fields. | Rendering uses declared width/height for layout (`mediaStyle` caps 480). Dedicated protocol magic-sniffs bytes and requires them to match the declared MIME (or allowlisted fallback). | Rendering. `mediaMimeType` is copied onto `MediaResource`. Extension MIME is only for local file-picker uploads. | Core `project_message_type_and_media` in [`view.rs`](../../../../crates/synara-core/src/app/timeline/view.rs); UniFFI [`synara_core.udl`](../../../../crates/synara-core/src/synara_core.udl) `TimelineViewRowDto`; desktop [`lib.rs`](../../../../src-tauri/src/lib.rs) `timeline_media_content_type`; iOS `mediaPlaceholder`. |
| Size / integrity limits | Authority for bounded download/decrypt (`download_media_bounded` rejects Content-Length and streamed oversize). Product caps are per dedicated channel: iOS handle bytes 32 MiB; desktop protocol 64 MiB; leftover original-file download 300 MiB; content upload 32 MiB; invite avatars 1 MiB. `matrix_media_config` is the `m.upload.size` envelope only. | Observation of the homeserver upload size through Core; local composer cap `NATIVE_ATTACHMENT_MAX_BYTES` (32 MiB). File-save names are sanitized in the shell. | Observation. Attachment send uses dedicated UniFFI bytes (`SharedCoreMediaSend`), not leftover `mediaUpload`. | [`bounded.rs`](../../../../crates/synara-core/src/app/media/bounded.rs), [`plain.rs`](../../../../crates/synara-core/src/app/media/plain.rs), [`content.rs`](../../../../crates/synara-core/src/app/media/content.rs), [`live.rs`](../../../../crates/synara-core/src/app/timeline/live.rs) `media_bytes`; desktop [`nativeMediaLimits.ts`](../../../../synara/src/app/utils/nativeMediaLimits.ts), [`desktop_file_transfer.rs`](../../../../src-tauri/src/desktop_file_transfer.rs); iOS [`SharedCoreMediaSend.swift`](../../../../synara-ios/Synara/Services/SharedCoreMediaSend.swift). |
| Cache eligibility / quota / eviction | Unused harness. `MediaCacheIndex` (P7.3) tracks handle + optional mxc + declared size + last-access, plans LRU, and `retire_generation` on wipe. `#![allow(dead_code)]`; product never upserts. `DownloadQueue` / `UploadQueue` / `media_export` are the same class of metadata harness. Account layout has a `media/` slot that is **not** always bound as the SDK media store. | No product cache-eligibility engine. Protocol responses send `Cache-Control: no-store`. Blob URLs are presenter memory. | No product cache-eligibility engine. `BoundedLRUCache` is viewport/read-marker chrome, not media bytes. | [`media_cache/index.rs`](../../../../crates/synara-core/src/app/media_cache/index.rs), [`media_cache/mod.rs`](../../../../crates/synara-core/src/app/media_cache/mod.rs), [`store/paths.rs`](../../../../crates/synara-core/src/app/store/paths.rs); desktop protocol headers in [`lib.rs`](../../../../src-tauri/src/lib.rs). |
| Retry / corruption | Bounded download fail-closes (`TooLarge` / `DecryptionFailed` / `RequestFailed`). UTD retry is event decrypt, not media bytes. No shared media-download retry owner. | Protocol maps failure to 404 / 415 after magic-sniff mismatch. Leftover download maps to static diagnostics. | Handle fetch `try?` → nil / “could not be loaded.” | [`bounded.rs`](../../../../crates/synara-core/src/app/media/bounded.rs); desktop protocol; iOS `SharedCoreMediaLoader`. |
| Logout / wipe | Authority for *when* stores die. Logout drops the client and handle registries and **retains** the account tree (`D-LOGOUT-WIPE`). Explicit wipe deletes the exact `account_root` (includes the `media/` slot) after the client is dropped. Reports never include absolute paths. | Shell leftover `matrix_logout` is Keyring + app-data cleanup (playbook leftover; not `Core::command`). | Product `resetLocalState` calls leftover `logout` only. Tests assert it does **not** call `wipePersistedStores`. Composer/local caches clear in `AppLocalWipeService`. | [`logout.rs`](../../../../crates/synara-core/src/app/lifecycle/logout.rs), [`wipe.rs`](../../../../crates/synara-core/src/app/lifecycle/wipe.rs); iOS [`SharedCoreProductServices.swift`](../../../../synara-ios/Synara/Services/SharedCoreProductServices.swift) `resetLocalState`, [`LocalWipeService.swift`](../../../../synara-ios/Synara/Services/LocalWipeService.swift), [`LocalWipeServiceTests.swift`](../../../../synara-ios/SynaraTests/LocalWipeServiceTests.swift). |
| Diagnostics | Authority: static fail-closed codes; no handle / mxc / token echo on `timeline_media_bytes` errors. Export-job `Debug` redacts handle ids. Store layout DTO exposes only relative child names. | Protocol returns empty bodies on failure. JS leftover errors are generic. | Leftover media wrappers must not echo mxc/token. `MediaResource.safeDescription` is the filename basename. | [`p4_s33_timeline_media.rs`](../../../../crates/synara-core/tests/p4_s33_timeline_media.rs); [`media_export/queue.rs`](../../../../crates/synara-core/src/app/media_export/queue.rs); iOS [`SharedCorePlainMedia.swift`](../../../../synara-ios/Synara/Services/SharedCorePlainMedia.swift), `MediaServiceTests`. |
| NSE / notification storage | Hard stop. `timeline_media_bytes` returns `p4-s33-nse-forbids-media` when the SharedCore is NSE read-only. NSE preview is one-shot event text, not media. | N/A (no NSE). Tray delivery is a different lane (ROE-07). | `NotificationService` rewrites title/body only. No `mxc` / thumbnail / attachment fetch in the extension. | [`shared_core_ffi.rs`](../../../../crates/synara-core/src/shared_core_ffi.rs) `timeline_media_bytes` / `is_nse_read_only`; [`nse_preview.rs`](../../../../crates/synara-core/src/app/nse_preview.rs); iOS [`NotificationService.swift`](../../../../synara-ios/SynaraNotificationService/NotificationService.swift). |
| Filesystem paths / file handoff | Must not own. No path on `Core::command`. Attachment send/upload take dedicated byte arguments. | Platform. File picker, save, drag, `sanitize_download_filename`, `native-staged:` composer ids. | Platform. Photo library / camera / file `Data` in `MediaUploadRequest`; display-name basename. | Playbook leftover `matrix_send_attachment` / `matrix_upload_media` / `matrix_media_download`; iOS [`MediaService.swift`](../../../../synara-ios/Synara/Services/MediaService.swift). |
| Leftover plain `mxc://` (avatar / pack / leftover cells) | Typed leftover methods `download_plain_media` / `thumbnail_plain_media` reject `timeline-media-*` and encrypted sources. Leftover UniFFI `media_*` stay unavailable. | Control-plane identifier for leftover avatar/pack via `useNativeMatrixMediaSrc` → `matrix_media_download`. Not an `<img src=mxc://>`. | `SharedCorePlainMedia` is the live leftover path. `SharedCoreLeftovers.mediaDownload` is fail-closed and unused by product. | [`plain.rs`](../../../../crates/synara-core/src/app/media/plain.rs); [`core.rs`](../../../../crates/synara-core/src/core.rs) comments; playbook §6 leftovers. |

Classification:

- Opaque handle allocation, `MediaSource` retention, decrypt, bounded
  download, handle revocation, `m.upload.size` envelope, logout≠wipe of the
  account tree, NSE media forbid, and fail-closed diagnostics are **Core
  authority** and a **hard invariant** (ADR 0003: no second Matrix engine;
  ADR 0004 invariant 3 / ADR 0005: no paths or bytes on `Core::command`; no
  presenter decrypt; no raw `mxc://` on required timeline presentation DTOs).
- `synara-media://` / `synara-timeline-media://` URL construction, magic-sniff
  at the dedicated desktop protocol, blob-URL lifetime, file picker/save/share,
  composer local `Data`, NSE process lifecycle, and leftover avatar/pack
  display are **platform observation / rendering** and an **accepted platform
  boundary**.
- Per-channel byte caps (32 / 64 / 300 MiB) are **product and platform policy**
  that ADR 0005 already allows to differ. They are not a missing shared owner.
- React `<img>` versus SwiftUI media cells, and whether a presenter shows a
  thumbnail chrome before fetching original-file bytes, are a **current
  technology preference**.
- Historical `MediaHandle` DTO / `valid_media_handle.json` (optional `mxcUri`,
  `thumbnailHandleId`, `sizeBytes`) is a **fixture schema**, not the live
  timeline row. Treating it as a missing Core field would invent work.

Earliest actual divergence is **dedicated-channel presentation**, not competing
MXC or decrypt authority. Desktop native timeline builds `synara-media://`
and the shell resolves the handle. iOS builds `synara-timeline-media://` and
calls UniFFI bytes. Both keep `MediaSource` in Core. Leftover inbox / pin /
search `ImageContent` cells still take leftover `mxc://` through
`downloadMatrixMedia`; encrypted leftovers fail-close without a handle. That
is leftover presenter consumption, not a second decrypt engine.

The unused P7.3 index is leftover harness, not a latent second cache owner.
Neither client implements a product cache-eligibility policy that would race
Core.

## Boundary constraints

- ADR 0005 (authoritative): opaque handles plus dedicated native byte
  channels. Must not put media/attachment bytes or local paths on
  `Core::command`. Must not put raw `mxc://` or encryption material on
  timeline presentation DTOs when a handle is required. A bounded Matrix
  content URI may be a control-plane identifier on a typed leftover
  download; it is not a byte channel or a presenter decryption path. Cache,
  integrity, quota, and retry *may* live in Core when shared; filesystem
  paths, OS file coordination, and display remain platform-owned. Channel
  byte caps may differ.
- ADR 0003: one Core for session, timeline, crypto, and Matrix writes.
  Swift/JS must not become a second media/decrypt owner.
- ADR 0004 layer map: Core owns media metadata/policy and opaque handles;
  platforms own dedicated byte transfer, filesystem paths, caching
  integration, and display. Hard invariant 3: no generic-envelope secret or
  byte transport. Invariant 6: NSE stays narrow.
- Playbook leftover set: `matrix_send_attachment`, `matrix_upload_media`,
  `matrix_media_download` stay unregistered on `Core::command`. iOS leftover
  `media_download` / `media_thumbnail` / `media_upload` / `room_avatar_bytes`
  stay fail-closed. Native media cutover must not register byte commands on
  the envelope. Do not invent S38 or start P5.
- Goal-graph stop: leftover secret/byte commands must not cross the envelope.
  Docs-only memos are still allowed.
- Workstream prior: metadata only, subordinate to ADR 0005.

Behaviors that must stay platform-side: file dialogs, save/share/drag,
photo-library/camera capture, webview protocol and blob lifetime, SwiftUI /
React media cells, NSE process and App Group storage, and leftover avatar
HTTP/blob display.

## Alternatives

1. **No ownership change (stay-put / close).** Keep Core as the only
   timeline-handle and decrypt owner. Keep leftover plain `mxc://` as a typed
   control-plane identifier for avatar/pack. Keep paths, file handoff, NSE
   storage, and display platform-owned. Leave P7.3 / P7.2 / P6.4 / P7.5
   harnesses unwired. Do not add `size_bytes` or a thumbnail handle unless a
   later product owner wants a row-field convenience. Falsified if a shipped
   client decrypts timeline media in JS/Swift, uses leftover `mxc://` as a
   required timeline `<img src>`, registers leftover byte commands on
   `Core::command`, or downloads media from NSE.

2. **Bounded extraction or shared fixture.** Wire `MediaCacheIndex` as the
   shared eligibility/quota owner, or project `info.size` / a thumbnail
   handle onto `TimelineViewRow`. That would be *new* product surface, not a
   census of a missing engine. A shared fixture of handle metadata would not
   change ownership. Falsified as *necessary tonight* because neither client
   implements a competing cache or MIME/size policy; the unused harness is
   not a second owner. Wiring it would still leave disk delete and OS
   coordination platform-side (ADR 0005) and is implementation, which this
   charter forbids.

3. **Broader Core model** (Core-owned filesystem cache, Core-owned
   `synara-media://` / Photos handoff, Core-owned NSE media, or bytes/paths
   on `Core::command`). Would require replacing ADR 0005 and would fight
   envelope size, NSE narrowness, and platform file APIs. Falsified only by
   a new accepted ADR that dedicated channels are insufficient.

Strongest stay-put case: unused `MediaCacheIndex`, the historical
`MediaHandle` DTO with `mxcUri`, leftover UniFFI `media_*`, leftover
`ImageContent` cells, and diverging 32/64/300 MiB caps look like dual
ownership if adapters and harnesses are mistaken for engines. Product
timeline writes already go through one handle registry. Leftover `mxc://`
is the documented avatar/pack control plane. Byte-cap differences are
explicit ADR 0005 policy. Treating those leftovers as a missing shared
owner would invent P7.3 activation the prior told this lane not to start.

## Recommendation

**Already correctly owned.**

The ADR 0005 split holds. Shared authority is **not** missing for MXC
identity, encryption metadata, declared MIME/dimensions/duration, bounded
integrity, logout-versus-wipe of the account tree, NSE media forbid, or
privacy-safe diagnostics. Those already have one Core owner. Desktop and
iOS filesystem paths, file handoff, protocol/blob display, and NSE
storage/lifecycle remain platform-owned and must stay there.

Unused cache/download/export harnesses and the absence of `size_bytes` /
`thumbnail_handle_id` on the live row are **not** a second engine and do
not justify extract or proceed.

Confidence: high for the handle/decrypt/channel split and for leftover
adapters not being a second engine; high that P7.3 is unwired on this
commit; medium only for whether leftover inbox/pin/search `ImageContent`
still receives encrypted `mxc://` in live product (those paths fail-close
without a handle either way).

Supporting evidence:

- CENSUS.md channel split still matches source; source only adds leftover
  and harness detail.
- `TimelineMediaHandle` serializes handle + declared metadata, never
  `mxc://` or keys.
- Desktop native timeline uses `synara-media://` + `resolve_timeline_media`.
- iOS product timeline uses `synara-timeline-media://` +
  `timeline_media_bytes`.
- Leftover encrypted `mxc://` fail-closes on desktop; leftover UniFFI
  media I/O fail-closes on iOS.
- NSE cannot call the handle byte API; the extension does not fetch media.
- `MediaCacheIndex` is `dead_code` harness; no desktop/iOS product caller.
- Playbook leftovers remain unregistered on `Core::command`.

Strongest objection: someone can read ADR 0005’s “cache … may live in Core
when shared,” plus the unused P7.3 index and missing `size_bytes`, as a
mandate to extract cache policy now. That sentence is permission, not a
gap. No shipped client has a competing eligibility/quota owner. Activating
the harness would be implementation behind a human gate, and it still
could not put paths or bytes on the generic envelope.

Unresolved questions that do **not** reopen ownership:

- Live `TimelineMediaHandle` omits protocol `info.size`. Presenters do not
  invent size from MXC. Adding the field later is a row-metadata convenience.
- Timeline handles always fetch original-file bytes; “thumbnail” chrome is
  presenter layout, not a second MXC identity.
- Desktop leftover `ImageContent` (inbox, pins, search) still accepts
  leftover `mxc://` through `downloadMatrixMedia`. That is leftover
  consumption of the existing native download owner.
- iOS `mediaPlaceholder` always sets `isEncrypted: false` for handle URLs.
  That is correct presenter ignorance of Core-owned decrypt, not a missing
  flag.
- `synara/docs/synara-media-policy.md` still prefers `mxc://` in prose
  (2026-05-25). ADR 0005 and current source supersede it for timeline rows.
- Per-channel byte caps differ. ADR 0005 already allows that.

Regression proof to keep the boundary stable:

- Core: `timeline-media-*` handles stay opaque; registry JSON/tests still
  forbid `mxc://`; `download_media_bounded` remains the decrypt path;
  `timeline_media_bytes` stays off `Core::command` and still fail-closes
  in NSE and without a session without echoing the handle.
- Desktop: native timeline `src` remains `synara-media://` + handle;
  leftover encrypted `mxc://` still throws without a handle; leftover
  `matrix_media_download` stays a shell leftover, not a Core envelope
  command.
- iOS: product timeline still maps handles to `synara-timeline-media://`
  and `SharedCoreTimelineMedia`; leftover `mediaDownload` stays unused by
  product; `NotificationService` still has no media fetch; product logout
  still does not wipe stores by default.
- No local path, attachment bytes, or encryption material appear on
  `Core::command`.

## Next gate

Already owned. Close ROE-11. Do not write an implementation plan. Do not
wire `MediaCacheIndex`. Do not register leftover media commands on
`Core::command`. Do not invent S38 or start P5. Do not put paths or bytes
on the generic envelope.

A later product owner may project `info.size` onto the existing handle, or
activate a metadata-only cache index, without changing this ownership
decision. Those would be additions to the owner already censused here, and
they still require a human implementation gate.
