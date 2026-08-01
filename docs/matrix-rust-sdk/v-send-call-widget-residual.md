# V-SEND.R-CALL-UPLOAD — CallWidgetDriver native residual inventory

| Field    | Value                                                                                                                   |
| -------- | ----------------------------------------------------------------------------------------------------------------------- |
| Status   | **Native room-list reuse + fail-closed media boundary; #378 media IPC design + reuse scan; implement packet frozen**     |
| Base tip | `b87f3a87` on `feature/matrix-rust-sdk-full-replacement`                                                                |
| Scope    | `CallWidgetDriver` upload, media-config, media-download, and known-room methods                                         |
| Guard    | Never touch `main` or umbrella PR **#39**; `dual_backend` is forbidden; **#327 remains HOLD and V-BURN is not started** |

> **Implement packet.** The frozen IPC contract, JS-owner deletion list,
> fail-closed rules, and test plan for the media config/download vertical live
> in [v-send-call-widget-media-implement-packet.md](v-send-call-widget-media-implement-packet.md).
> This residual records the inventory and the #387 reuse scan; the packet is the
> handoff for the next product vertical after members-read (**#395**).

## Finding

#328 makes `CallWidgetDriver.uploadFile` native-first for a logged-in native
desktop session. It enters `uploadCallWidgetFileWithNativeOwner`, invokes
`matrix_upload_media`, and returns the native `mxc://` result. Unsupported
widget bodies, unavailable native commands, and invalid native responses are
terminal; the `client.uploadContent` callback is not selected on that native
path.

The remaining widget surfaces now have one of two explicit native outcomes:
`getKnownRooms` reads the latest native room-list snapshot already maintained by
the desktop state binder, while media config/download fail closed because this
tip has no native command for either capability. No SDK room-list, media-config,
MXC URL, or HTTP-download fallback remains reachable from the desktop widget
driver. This document now records the proposed native media contract and its
implementation sequence; it does not wire that contract at this tip. This does
not claim full call/widget cutover or start a burn slice.

## CallWidgetDriver inventory

| Source                        | Production operation                                                  | Native desktop status                                                                                                           | Residual decision                                                           |
| ----------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `CallWidgetDriver.ts:315-317` | `getMediaConfig()`                                                    | **Fail-closed**; no native call-widget media-config command is present in this tip                                              | Implement `matrix_call_media_config`; no JS fallback on native desktop      |
| `CallWidgetDriver.ts:319-327` | `uploadFile(file)` delegates to `uploadCallWidgetFileWithNativeOwner` | **Native-first and fail-closed** for a logged-in native session; uses `matrix_upload_media`                                     | Upload owner closed by **#328**; legacy callback remains for non-native use |
| `CallWidgetDriver.ts:330-333` | `downloadFile(contentUri)`                                            | **Fail-closed**; no native call-widget media-download command is present in this tip                                            | Implement `matrix_media_download`; no JS fallback on native desktop         |
| `CallWidgetDriver.ts`         | `getKnownRooms()`                                                     | **Native snapshot-backed**; uses the cached `matrix_room_list_snapshot` readback and returns `[]` until a valid snapshot exists | Room-list owner is native; no SDK visible-room fallback                     |

The media methods remain explicit blocked surfaces until the serial product
slice lands. The room method is wired to the native snapshot owner. This slice
does not claim full call/widget cutover or start a burn slice.

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

`getMediaConfig` and `downloadFile` intentionally stop at the widget driver
with a terminal native-capability error. The native media module at this tip is
metadata-only queue scaffolding and exposes no production media-config or
download command. The existing `matrix_upload_media` command is an upload
owner, not a reusable config/download owner. Therefore this slice adds no
speculative `product.rs` command.

Relevant source evidence at the measured tip:

- `synara/src/app/plugins/call/CallWidgetDriver.ts:317-325` selects the
  native owner before the legacy callback.
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
by #378. The CallWidget path still needs its dedicated
`matrix_media_download` command, with bounded `mxc://` input and direct byte
response. No JS `mxcUrlToHttp`, browser `fetch`, `synara-media` URL, P7.2 queue,
or upload-command fallback is part of that route.

## Native media IPC implementation plan

### Contract decision

Add two dedicated Tauri commands in the serial product slice:

| Command                    | Request                  | Successful response           | Native SDK operation                     |
| -------------------------- | ------------------------ | ----------------------------- | ---------------------------------------- |
| `matrix_call_media_config` | no fields                | `{ "m.upload.size": number }` | `client.load_or_fetch_max_upload_size()` |
| `matrix_media_download`    | `{ contentUri: string }` | `{ bytes: number[] }`         | `client.media().get_media_content(...)`  |

The scan above confirms that no existing media IPC is suitable for direct
CallWidget download reuse. `matrix_upload_media` returns an upload `mxc` and
has upload-specific validation. The `synara-media` protocol serves
timeline-owned opaque handles, while `src-tauri/src/matrix/media` and P7.2
hold metadata-only queues; none accepts a widget `contentUri` as a direct
byte-returning command. The versioned JSON IPC envelopes and domain DTOs also
remain metadata-only. These two commands are narrow product commands, not new
stream topics.

### Command behavior

`matrix_call_media_config` should:

1. Require the live logged-in session from `MatrixAuthState`; logged-out,
   missing, or retired sessions return the existing structured native error.
2. Call `Client::load_or_fetch_max_upload_size()`. The SDK selects the
   authenticated media-config endpoint when supported and its legacy endpoint
   otherwise, while retaining the SDK cache.
3. Convert the result to the exact widget key `m.upload.size`. Reject a value
   that cannot be represented safely as a JavaScript number rather than
   truncating it.
4. Return only the config value; never return a token, URL, or SDK response.

`matrix_media_download` should:

1. Require the same live session and validate that `contentUri` is a bounded,
   valid `mxc://` URI. Reject `https://`, `data:`, `javascript:`, query-string
   credentials, empty input, and oversized identifiers before any SDK request.
2. Construct `MediaRequestParameters` with `MediaSource::Plain(uri)` and
   `MediaFormat::File`. `downloadFile` asks for the original file, not a
   thumbnail and not an HTTP URL.
3. Call `client.media().get_media_content(&request, true).await`, allowing the
   native SDK media cache to serve an already available file.
4. Enforce an explicit response-byte ceiling before serializing the result.
   Start with the existing 32 MiB attachment IPC ceiling as the policy target,
   subject to #375's product-boundary review; never silently truncate.
5. Return `{ bytes }` through this direct Tauri command only. The bytes must
   not enter a versioned JSON envelope, persistent DTO, diagnostic, or log.
   Errors contain stable diagnostic IDs only and never echo the URI or secrets.

The bare Widget API `contentUri` gives the native owner an MXC URI, not event
encryption metadata. This plan therefore matches the current JS behavior and
uses a plain MXC source. Extending this to encrypted-event media would require
an explicit Widget API request contract for the encryption metadata; it must
not be inferred from an untrusted string.

### Full-vertical route when #375 opens the product slice

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

The TypeScript part must land in the same product slice as the Rust commands:

- Add a small native owner helper (alongside the existing call-widget owner)
  that invokes exactly these command names, checks `available`, validates the
  response shape, and converts `bytes` to `Uint8Array`.
- Rewire `CallWidgetDriver.getMediaConfig` and `downloadFile` to that helper
  for the native desktop route. Native command absence, error, malformed
  response, logout, and stale session state remain terminal; none may select
  `this.mx.getMediaConfig`, `mxcUrlToHttp`, `downloadMedia`, or `fetch`.
- Delete the two current fail-closed method stubs and the generic
  `throwNativeCallWidgetCapabilityUnavailable` helper if no other caller
  remains, in the same change that proves the replacement route.
- Add focused tests for config success, download success, unavailable command,
  malformed config/bytes, invalid MXC rejection, and the no-fallback invariant.

The Rust half belongs to `src-tauri/src/matrix/auth/product.rs` and command
registration/permissions owned by **#375**. This docs slice intentionally does
not edit `product.rs`, `src-tauri/src/lib.rs`, generated permissions, or add
TypeScript stubs for commands that are not registered yet.

### Acceptance and boundaries

- The UI-to-Tauri-to-live-`matrix_sdk::Client` route is exercised for both
  methods before either fail-closed stub is removed.
- Native failures remain terminal on desktop; no `dual_backend` flag or JS
  network fallback is introduced.
- Config returns the widget API's exact `m.upload.size` key, and download
  returns original bytes with a hard size limit and no byte logging.
- Focused Rust/TypeScript tests cover validation, session retirement, command
  availability, response validation, and the no-fallback invariant.
- No V-BURN claim is made. `#327` remains HOLD and V-BURN remains not started.

## SCOREBOARD cross-link

`V-SEND.R-CALL-UPLOAD` remains closed for the **upload owner** by #328. The
media-config/download methods are blocked native capabilities, not evidence
that upload is still JS-backed. The scoreboard row links here so the
distinction between upload closure and the remaining widget boundary stays
explicit.

## Verification

The production inventory and media-route scan were checked with:

```text
rg -n 'getMediaConfig|downloadFile|getKnownRooms|uploadFile' \
  synara/src/app/plugins/call/CallWidgetDriver.ts
rg -n -i 'mx\.download(Media|File)|downloadMedia|downloadMatrixMedia|mxcUrlToHttp|synara-media|get_media_content' \
  synara/src src-tauri/src
```

The scan found no `mx.downloadMedia`/`mx.downloadFile` call and no CallWidget
path to the existing HTTP/Blob helpers. No Rust product command, dual-backend
flag, V-BURN state, or #327 state was changed; this remains a docs-only
alignment with #378.
