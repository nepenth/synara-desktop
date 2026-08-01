# V-SEND.R-CALL-UPLOAD — CallWidgetDriver native residual inventory

| Field    | Value                                                                                                                   |
| -------- | ----------------------------------------------------------------------------------------------------------------------- |
| Status   | **Native room-list reuse + fail-closed media boundary; media IPC design only**                                          |
| Base tip | `3d76402f` on `feature/matrix-rust-sdk-full-replacement`                                                                |
| Scope    | `CallWidgetDriver` upload, media-config, media-download, and known-room methods                                         |
| Guard    | Never touch `main` or umbrella PR **#39**; `dual_backend` is forbidden; **#327 remains HOLD and V-BURN is not started** |

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

## Native media IPC implementation plan

### Contract decision

Add two dedicated Tauri commands in the serial product slice:

| Command                    | Request                  | Successful response           | Native SDK operation                     |
| -------------------------- | ------------------------ | ----------------------------- | ---------------------------------------- |
| `matrix_call_media_config` | no fields                | `{ "m.upload.size": number }` | `client.load_or_fetch_max_upload_size()` |
| `matrix_media_download`    | `{ contentUri: string }` | `{ bytes: number[] }`         | `client.media().get_media_content(...)`  |

No existing media IPC is suitable for reuse. `matrix_upload_media` returns an
upload `mxc` and has upload-specific validation. `src-tauri/src/matrix/media`
and P7.2 currently hold metadata-only queues; they do not own a live SDK
client, network I/O, or production Tauri commands. The versioned JSON IPC
envelopes and domain DTOs also remain metadata-only. These two commands are
narrow product commands, not new stream topics.

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

The production inventory was checked with:

```text
rg -n 'getMediaConfig|downloadFile|getKnownRooms|uploadFile' \
  synara/src/app/plugins/call/CallWidgetDriver.ts
```

No Rust product command, dual-backend flag, V-BURN state, or #327 state was
changed. The TypeScript owner and this residual record were updated together.
