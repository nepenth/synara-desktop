# V-SEND.R-CALL-UPLOAD — CallWidgetDriver native residual inventory

| Field    | Value                                                                                                                   |
| -------- | ----------------------------------------------------------------------------------------------------------------------- |
| Status   | **Native room-list + native media boundary at the measured tip; #407 media config/download is merged; this inventoried residual is closed** |
| Tip      | `c1e9c3be` on `feature/matrix-rust-sdk-full-replacement`                                                                |
| Scope    | `CallWidgetDriver` upload, media-config, media-download, and known-room methods                                         |
| Guard    | Never touch `main` or umbrella PR **#39**; `dual_backend` is forbidden; **V-BURN remains HOLD and is not started** |

> **#407 is merged at this tip.** The merged product slice adds the native
> `matrix_call_media_config` and `matrix_media_download` commands, the typed
> CallWidget media owner, source-absence guards, contract tests, and the gated
> authenticated Synapse proof. The merge closes this document's
> media-config/download residual; it does not claim full MatrixRTC/CallWidget
> parity or start V-BURN.

> **Tip honesty.** This docs-only refresh supersedes **#466**, whose evidence
> anchor was `103a653f`. It is measured at `c1e9c3be`, a later integration tip
> carrying #458 presence and #461 room-directory work. The CallWidget
> implementation paths and focused evidence are unchanged from the prior
> anchor; this update changes the evidence anchor only.

> **Implementation record.** The frozen IPC contract, JS-owner deletion list,
> fail-closed rules, and test evidence for the merged media config/download
> vertical remain in [v-send-call-widget-media-implement-packet.md](v-send-call-widget-media-implement-packet.md).
> This residual records the final inventory and the #387 reuse scan.

## Finding

#328 makes `CallWidgetDriver.uploadFile` native-first for a logged-in native
desktop session. It enters `uploadCallWidgetFileWithNativeOwner`, invokes
`matrix_upload_media`, and returns the native `mxc://` result. Unsupported
widget bodies, unavailable native commands, and invalid native responses are
terminal; the `client.uploadContent` callback is not selected on that native
path.

The remaining widget surfaces now have explicit native outcomes:
`getKnownRooms` reads the latest native room-list snapshot already maintained by
the desktop state binder, and media config/download use the dedicated #407
native commands. No SDK room-list, media-config, MXC URL, or HTTP-download
fallback remains reachable from the desktop widget driver on the native desktop
path. This document records the delivered native media contract and its
boundaries; it does not claim full call/widget cutover or start a burn slice.

## CallWidgetDriver inventory

| Source                        | Production operation                                                  | Native desktop status                                                                                                           | Residual decision                                                           |
| ----------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `CallWidgetDriver.ts:316-320` | `getMediaConfig()`                                                    | **Native-owned and fail-closed**; invokes `matrix_call_media_config` through the typed native media owner                           | Media-config owner closed by **#407**; no JS fallback on native desktop      |
| `CallWidgetDriver.ts:319-327` | `uploadFile(file)` delegates to `uploadCallWidgetFileWithNativeOwner` | **Native-first and fail-closed** for a logged-in native session; uses `matrix_upload_media`                                     | Upload owner closed by **#328**; legacy callback remains for non-native use |
| `CallWidgetDriver.ts:333-337` | `downloadFile(contentUri)`                                            | **Native-owned and fail-closed**; invokes `matrix_media_download` and returns validated `Uint8Array` bytes                       | Media-download owner closed by **#407**; no JS fallback on native desktop     |
| `CallWidgetDriver.ts`         | `getKnownRooms()`                                                     | **Native snapshot-backed**; uses the cached `matrix_room_list_snapshot` readback and returns `[]` until a valid snapshot exists | Room-list owner is native; no SDK visible-room fallback                     |

All four inventoried surfaces now have an explicit native desktop owner. Native
failures remain terminal and the non-native web upload callback remains outside
this native desktop residual. This does not claim full call/widget cutover or
start a burn slice.

## Native route evidence

The verified #328 route is:

```text
CallWidgetDriver.uploadFile
  → uploadCallWidgetFileWithNativeOwner
  → matrix_session_snapshot
  → matrix_upload_media
  → native Matrix SDK media upload
  → mxc:// result
```

The room-list route is:

```text
useBindAllRoomsAtom
  → matrix_session_snapshot
  → matrix_room_list_snapshot
  → native room-list snapshot cache
  → CallWidgetDriver.getKnownRooms
```

The #407 media routes are:

```text
CallWidgetDriver.getMediaConfig
  → getMediaConfigWithNativeOwner
  → matrix_session_snapshot
  → matrix_call_media_config
  → live Matrix SDK client
  → Client::load_or_fetch_max_upload_size
  → {"m.upload.size": number}

CallWidgetDriver.downloadFile(contentUri)
  → downloadFileWithNativeOwner
  → matrix_session_snapshot
  → matrix_media_download
  → live Matrix SDK client
  → Client::media().get_media_content(MediaFormat::File)
  → {bytes: number[]}
  → Uint8Array
```

The native media queue remains metadata-only scaffolding for other product
surfaces; #407 deliberately uses dedicated product commands. The existing
`matrix_upload_media` command remains an upload owner, not a reusable
config/download owner.

Relevant source evidence at the measured tip:

- `synara/src/app/plugins/call/CallWidgetDriver.ts:316-337` selects the
  native owners for config/download; upload remains separately owned.
- `synara/src/app/plugins/call/nativeCallWidgetMediaOwner.ts` validates the
  native session, exact commands, response shapes, and `Uint8Array` conversion.
- `synara/src/app/plugins/call/nativeCallMediaUploadOwner.ts:49-71` rejects
  unsupported bodies and keeps native failures terminal.
- `synara/src/app/state/nativeMediaUploadOwner.ts:45-58` invokes
  `matrix_upload_media` and validates the native `mxc` response.
- `synara/src/app/plugins/call/__tests__/nativeCallMediaUploadOwner.test.ts`
  covers native success, web ownership, missing-command failure,
  invalid-response failure, and unsupported widget bodies.

## Remaining media-route scan

The source scan found no literal `mx.downloadMedia` or `mx.downloadFile` call
(`mx.downloadKeysForUsers` is an unrelated crypto path). The remaining Matrix
download/display paths are either browser HTTP/Blob helpers or a separate
native timeline protocol; neither is a compatible CallWidget download owner.

| Existing surface                                                                                                                           | Measured route and consumers                                                                                                                                                                                                                                                                      | CallWidget reuse decision                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/matrix/media.ts:19-83` → `synara/src/app/utils/matrix.ts:226-260`                                                          | `resolveMatrixMediaUrl` calls `mx.mxcUrlToHttp`; `downloadMatrixMedia` resolves an HTTP URL and calls `downloadMedia`/`downloadEncryptedMedia`, which uses `fetch` and returns a `Blob`. FileHeader/FileContent, text/audio/video/image viewers, and thumbnail/media renderers consume this path. | **Do not reuse.** It is a webview HTTP/Blob boundary, performs optional client-side decryption, and requires event encryption metadata that the Widget API `contentUri` does not provide. It also violates the native desktop no-HTTP/no-JS-fallback boundary in #378.                              |
| `synara/src/app/utils/room.ts:297-314`, `synara/src/app/plugins/react-custom-html-parser.tsx:546`, and `resolveMatrixThumbnailUrl` callers | MXC values become authenticated HTTP image/thumbnail URLs for avatars, member images, room images, HTML media, and message thumbnails. These are URL/display paths, not byte-returning download owners.                                                                                           | **Do not reuse.** A URL-producing helper cannot satisfy `downloadFile(contentUri) → { file: Uint8Array }`, and routing the widget through an HTTP URL would reintroduce the forbidden fallback.                                                                                                     |
| `src-tauri/src/lib.rs:135-177` → `MatrixAuthState::resolve_timeline_media` → `synara-media`                                                | Native timeline rows use opaque, stream/session-bound handles. The protocol resolves a retained SDK `MediaSource`, fetches a file with `get_media_content`, validates size/type, and serves a main-window GET response.                                                                           | **Do not reuse as CallWidget IPC.** It accepts a timeline handle rather than a raw widget `contentUri`, is a URI protocol rather than an invoke response, and has timeline-specific handle lifetime and MIME policy. Its SDK ownership pattern is evidence for #378, not a shared CallWidget route. |
| `src-tauri/src/matrix/media/` and P7.2/P7.3                                                                                                | P7.2 download jobs and P7.3 cache entries contain metadata, source IDs, progress, and handles only; they do not own a live SDK client, network I/O, or media bytes.                                                                                                                               | **Do not reuse at this tip.** They are foundations, not production download commands or a byte-delivery boundary.                                                                                                                                                                                   |
| `matrix_upload_media`                                                                                                                      | Native upload command returning an upload `mxc`; it does not resolve or return downloaded media.                                                                                                                                                                                                  | **Do not reuse.** Upload validation and response semantics are different from CallWidget original-file download.                                                                                                                                                                                    |

The compatible reuse is therefore limited to the native session/SDK ownership
and the SDK `Client::media().get_media_content(...)` primitive already selected
by #378; #407 supplies the dedicated `matrix_media_download` command, with
bounded `mxc://` input and direct byte response. No JS `mxcUrlToHttp`, browser
`fetch`, `synara-media` URL, P7.2 queue, or upload-command fallback is part of
that route.

## Native media IPC delivery record

### Delivered contract

The serial product slice delivered two dedicated Tauri commands:

| Command                    | Request                  | Successful response           | Native SDK operation                     |
| -------------------------- | ------------------------ | ----------------------------- | ---------------------------------------- |
| `matrix_call_media_config` | no fields                | `{ "m.upload.size": number }` | `client.load_or_fetch_max_upload_size()` |
| `matrix_media_download`    | `{ contentUri: string }` | `{ bytes: number[] }`         | `client.media().get_media_content(...)`  |

The scan above confirmed that no existing media IPC was suitable for direct
CallWidget download reuse. `matrix_upload_media` returns an upload `mxc` and
has upload-specific validation. The `synara-media` protocol serves
timeline-owned opaque handles, while `src-tauri/src/matrix/media` and P7.2
hold metadata-only queues; none accepts a widget `contentUri` as a direct
byte-returning command. The versioned JSON IPC envelopes and domain DTOs also
remain metadata-only. These two commands are narrow product commands, not new
stream topics.

### Delivered command behavior

`matrix_call_media_config`:

1. Require the live logged-in session from `MatrixAuthState`; logged-out,
   missing, or retired sessions return the existing structured native error.
2. Call `Client::load_or_fetch_max_upload_size()`. The SDK selects the
   authenticated media-config endpoint when supported and its legacy endpoint
   otherwise, while retaining the SDK cache.
3. Convert the result to the exact widget key `m.upload.size`. Reject a value
   that cannot be represented safely as a JavaScript number rather than
   truncating it.
4. Return only the config value; never return a token, URL, or SDK response.

`matrix_media_download`:

1. Require the same live session and validate that `contentUri` is a bounded,
   valid `mxc://` URI. Reject `https://`, `data:`, `javascript:`, query-string
   credentials, empty input, and oversized identifiers before any SDK request.
2. Construct `MediaRequestParameters` with `MediaSource::Plain(uri)` and
   `MediaFormat::File`. `downloadFile` asks for the original file, not a
   thumbnail and not an HTTP URL.
3. Call `client.media().get_media_content(&request, true).await`, allowing the
   native SDK media cache to serve an already available file.
4. Enforce an explicit response-byte ceiling before serializing the result.
   Use the existing 32 MiB attachment IPC ceiling as the product boundary;
   never silently truncate.
5. Return `{ bytes }` through this direct Tauri command only. The bytes must
   not enter a versioned JSON envelope, persistent DTO, diagnostic, or log.
   Errors contain stable diagnostic IDs only and never echo the URI or secrets.

The bare Widget API `contentUri` gives the native owner an MXC URI, not event
encryption metadata. This plan therefore matches the current JS behavior and
uses a plain MXC source. Extending this to encrypted-event media would require
an explicit Widget API request contract for the encryption metadata; it must
not be inferred from an untrusted string.

### Delivered full-vertical route in #407

```text
CallWidgetDriver.getMediaConfig
  → native call-widget media owner
  → invoke('matrix_call_media_config')
  → MatrixAuthState live Client
  → Client::load_or_fetch_max_upload_size
  → {"m.upload.size": number}
  → IGetMediaConfigResult

CallWidgetDriver.downloadFile(contentUri)
  → native call-widget media owner
  → invoke('matrix_media_download', {contentUri})
  → MatrixAuthState live Client
  → Client::media().get_media_content(MediaFormat::File)
  → {bytes: number[]}
  → Uint8Array
  → {file: Uint8Array}
```

The TypeScript and Rust parts landed in the same #407 product slice:

- `nativeCallWidgetMediaOwner.ts` invokes exactly these command names, checks
  `available`, validates response shapes, and converts `bytes` to `Uint8Array`.
- `CallWidgetDriver.getMediaConfig` and `downloadFile` use that owner on the
  native desktop route. Command absence, errors, malformed responses, logout,
  and stale session state remain terminal; none select `this.mx.getMediaConfig`,
  `mxcUrlToHttp`, `downloadMedia`, or `fetch`.
- The two fail-closed method stubs and the unused generic
  `throwNativeCallWidgetCapabilityUnavailable` helper were removed.
- Focused tests cover config/download success, unavailable commands, malformed
  config/bytes, invalid MXC rejection, and the no-fallback invariant.

The Rust command implementations now live in the module-scoped
`src-tauri/src/matrix/widgets/product_commands.rs`, with shared product types
and re-exports in `src-tauri/src/matrix/auth/product.rs`; they are registered
and permissioned in the merged #407 slice. This docs truth-up does not edit
those product files.

### Acceptance and boundaries

- The UI-to-Tauri-to-live-`matrix_sdk::Client` route is implemented for both
  methods and covered by the merged focused/live-proof test paths.
- Native failures remain terminal on desktop; no `dual_backend` flag or JS
  network fallback is introduced.
- Config returns the widget API's exact `m.upload.size` key, and download
  returns original bytes with a hard size limit and no byte logging.
- Focused Rust/TypeScript tests cover validation, session retirement, command
  availability, response validation, and the no-fallback invariant.
- No V-BURN claim is made. V-BURN remains HOLD and not started.

## SCOREBOARD cross-link

`V-SEND.R-CALL-UPLOAD` is closed for the inventoried native desktop surfaces:
the upload owner by #328, room-list owner by #362, and media config/download
owners by #407. The scoreboard row links here so the separate legacy web upload
callback and broader CallWidget/MatrixRTC parity are not mistaken for a native
desktop residual.

## Verification

The production inventory and media-route scan were checked with:

```text
rg -n 'getMediaConfig|downloadFile|getKnownRooms|uploadFile' \
  synara/src/app/plugins/call/CallWidgetDriver.ts
rg -n -i 'mx\.download(Media|File)|downloadMedia|downloadMatrixMedia|mxcUrlToHttp|synara-media|get_media_content' \
  synara/src src-tauri/src
```

The scan found no `mx.downloadMedia`/`mx.downloadFile` call and no CallWidget
path to the existing HTTP/Blob helpers. #407 supplies the dedicated Rust
product commands and native owner; this docs truth-up changes no product code,
dual-backend flag, or V-BURN state.

The scan and source anchors above were revalidated at tip `c1e9c3be`. No
CallWidget implementation path changed between the superseded #466 anchor
`103a653f` and this tip. The unrelated #461 room-directory slice does not
change the CallWidget residual boundary.
