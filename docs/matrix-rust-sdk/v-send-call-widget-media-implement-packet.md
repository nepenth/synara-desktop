# V-SEND.R-CALL-MEDIA — CallWidget media config/download native IPC implementation packet

| Field    | Value                                                                                                                                  |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Status   | **Implemented by #407** — this docs-only update records the merged media IPC vertical and does not change product code               |
| Residual | **V-SEND.R-CALL-UPLOAD** media-config/download closed by #407; [residual record](v-send-call-widget-residual.md)                  |
| Measured tip | `27a854d8` on `feature/matrix-rust-sdk-full-replacement`                                                                           |
| #407 delivery tip | `206d24f3` on `feature/matrix-rust-sdk-full-replacement`                                                                       |
| Refresh | **#476** — prior evidence anchor `c1e9c3be`                                                                                         |
| PR shape | Focused **draft docs-only** truth-up targeting `feature/matrix-rust-sdk-full-replacement`                                              |
| Policy   | [full-vertical-policy.md](full-vertical-policy.md): native UI → Tauri IPC → live `matrix-sdk`, physical JS-owner deletion, fail-closed |
| Guard    | Never `main`, umbrella **#39**, or V-BURN/#327; `dual_backend` is forbidden; #407 is merged and this update changes docs only         |

> **Scope guard.** Docs only. No product code in `product.rs`, `src-tauri/src/lib.rs`,
> generated permissions, or any TS is changed. It does not touch **#39**,
> V-BURN/#327, or any timeline/send slice. This refresh updates **#476**'s
> CallWidget evidence anchor to `27a854d8`.

---

## 1. Objective and completion bar

The former two fail-closed CallWidget media stubs are replaced by one native-only
operating path for a logged-in native desktop session:

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

The implementation slice must preserve:

- the widget API's exact `m.upload.size` config key and original-file download
  semantics (`downloadFile` asks for the original file, not a thumbnail);
- native session and command failures as terminal visible unavailable/error
  states; and
- no JS `getMediaConfig`, `mxcUrlToHttp`, `downloadMedia`, `fetch`, or
  `synara-media` fallback when the native session is live.

This packet records closure of the **media config/download** residual by
**#407**. Upload is closed by **#328**; `getKnownRooms` is native via **#362**.
The inventoried native desktop surfaces are complete, but this does not claim
full MatrixRTC/CallWidget parity or V-BURN readiness.

## 2. Frozen scope and prerequisites

### Delivered scope

- `CallWidgetDriver.getMediaConfig()` and `CallWidgetDriver.downloadFile(contentUri)`.
- A typed TypeScript native owner (alongside the existing call-widget owner),
  exact command argument/result validation, and focused IPC contract tests.
- The two Rust product commands `matrix_call_media_config` and
  `matrix_media_download` in the module-scoped widget command slice.

### Preflight recorded by #407

Before product implementation started, #407 verified:

1. The implementation landed at the approved integration tip now recorded as
   `206d24f3`; this docs-only record is revalidated at measured tip
   `27a854d8`. The CallWidget implementation paths are unchanged from the
   prior #476 evidence anchor `c1e9c3be`.
2. The PR target is `feature/matrix-rust-sdk-full-replacement`, never `main` or #39.
3. The managed native session exposes one live `matrix-sdk` client for the
   current session; no second Matrix client or selector is introduced.
4. The serial `product.rs` ownership was available for the #407 product slice;
   this docs update does not edit it.
5. `matrix_upload_media` remains the upload owner; this packet does not reuse it
   for download.

The merged implementation did not add a fallback, rename the commands, or
widen this packet.

### Explicit non-goals

- `uploadFile` (closed by **#328**) or `getKnownRooms` (already native);
- encrypted-event media download (requires an explicit Widget API contract for
  encryption metadata; must not be inferred from an untrusted `contentUri`);
- `synara-media` timeline protocol, P7.2/P7.3 queues, or browser HTTP/Blob helpers;
- changes to `main`, #39, V-BURN/#327, or `dual_backend`; and
- any edit to `src-tauri/src/matrix/auth/product.rs` in this docs-only PR.

## 3. Exact IPC contract

These command names and wire shapes are frozen. Do not add aliases, a generic
media command, or an `eventType`/`stateKey` escape hatch.

| Exact command                | Request                  | Successful response           | Native SDK operation                     |
| ---------------------------- | ------------------------ | ----------------------------- | ---------------------------------------- |
| `matrix_call_media_config`   | no fields                | `{ "m.upload.size": number }` | `client.load_or_fetch_max_upload_size()` |
| `matrix_media_download`      | `{ contentUri: string }` | `{ bytes: number[] }`         | `client.media().get_media_content(...)`  |

The frontend wire field is camelCase. `contentUri` must be a bounded, valid
`mxc://` URI. These are narrow product commands, not new stream topics.

### 3.1 `matrix_call_media_config` behavior

1. Require the live logged-in session from `MatrixAuthState`; logged-out,
   missing, or retired sessions return the existing structured native error.
2. Call `Client::load_or_fetch_max_upload_size()`. The SDK selects the
   authenticated media-config endpoint when supported and its legacy endpoint
   otherwise, while retaining the SDK cache.
3. Convert the result to the exact widget key `m.upload.size`. Reject a value
   that cannot be represented safely as a JavaScript number rather than
   truncating it.
4. Return only the config value; never return a token, URL, or SDK response.

### 3.2 `matrix_media_download` behavior

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

### 3.3 Delivered native owner semantics

The merged slice adds one SDK-neutral owner at:

`synara/src/app/plugins/call/nativeCallWidgetMediaOwner.ts`

The owner must:

1. require the desktop/native environment and a `logged_in`
   `matrix_session_snapshot` before either call;
2. validate `contentUri` (bounded `mxc://`) before invoking Tauri;
3. invoke only the exact command associated with the method;
4. check `available`, validate the response shape, and convert `bytes` to
   `Uint8Array`; and
5. return a typed native result or throw a safe unavailable/error result.

It must not return a `legacy` sentinel, inspect an `isNative ? rust : js`
selector, call `this.mx.getMediaConfig`, `mxcUrlToHttp`, `downloadMedia`,
`fetch`, or retry through a second backend. On every native failure the owner
makes zero calls to a legacy JS media path.

## 4. Physical deletion delivered by #407

The following JS ownership was removed in the #407 product implementation
slice.

| Path                                                                        | Deleted or changed in #407                                                                                                                    | Retain or replace with                                                                                                                          |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/plugins/call/CallWidgetDriver.ts`                           | the two fail-closed stubs and their `throwNativeCallWidgetCapabilityUnavailable` calls                                                          | both methods now use `nativeCallWidgetMediaOwner` for the native desktop route                                                              |
| `synara/src/app/plugins/call/nativeCallWidgetOwner.ts`                      | the generic `throwNativeCallWidgetCapabilityUnavailable` helper                                                                                | `getKnownRoomsFromNativeSnapshot` remains the room-list owner                                                                            |
| `synara/src/app/plugins/call/__tests__/nativeCallWidgetOwner.test.ts`       | the two unavailable-capability assertions                                                                                                      | focused native media owner tests                                                                                                              |
| `synara/src/app/plugins/call/nativeCallWidgetMediaOwner.ts`                 | added with no JS SDK imports or fallback branch                                                                                                | sole desktop media config/download owner for both exact commands                                                                              |

Do **not** delete in this packet:

- `uploadCallWidgetFileWithNativeOwner` / `nativeCallMediaUploadOwner.ts` (upload owner, #328);
- `getKnownRoomsFromNativeSnapshot` (room-list owner);
- `matrix_upload_media` or any P7.2/P7.3 media queue;
- `synara/src/app/matrix/media.ts` / `utils/matrix.ts` HTTP/Blob helpers (other consumers);
- any test that proves SDK-neutral retained behavior; or
- `src-tauri/src/matrix/auth/product.rs` in this docs-only PR.

The implementation includes a negative source scan proving that
`CallWidgetDriver.ts` contains none of `getMediaConfig` (JS), `mxcUrlToHttp`,
`downloadMedia`, `fetch`, or a JS fallback for these two media methods.
Repository-wide JS Matrix imports may remain nonzero because other verticals are
still open; that is not permission to retain either media stub.

## 5. Focused test evidence

### 5.1 Native owner tests

Present in #407:

`synara/src/app/plugins/call/__tests__/nativeCallWidgetMediaOwner.test.ts`

Use an injected invoke harness and assert exact command names, arguments, and
result validation for:

1. logged-in native preflight followed by `matrix_call_media_config` returning
   `{ "m.upload.size": number }`;
2. logged-in native preflight followed by `matrix_media_download` returning
   `{ bytes }` converted to `Uint8Array`;
3. desktop unavailable, logged-out, missing command, invoke rejection, malformed
   config/bytes, and invalid `contentUri` rejection becoming visible
   unavailable/error results; and
4. every native failure making zero calls to any legacy JS media path. The owner
   must reject rather than return a `legacy` sentinel.

### 5.2 Source-absence tests

Present in #407:

`synara/src/app/plugins/call/__tests__/callWidgetMediaSourceGuard.test.ts`

The guard reads `CallWidgetDriver.ts` and asserts:

- no `this.mx.getMediaConfig`, `mxcUrlToHttp`, `downloadMedia`, or `fetch` call
  remains for either media method;
- no `Legacy*` component, `legacy` return sentinel, or `isNative ? rust : js`
  writer selector is introduced; and
- the fail-closed stubs are physically absent.

The guard must not assert repository-wide `matrix-js-sdk` usage is zero.

### 5.3 Rust/IPC contract tests

The merged implementation adds focused contract coverage outside the product
command body for:

- camelCase request serialization for both exact command names;
- typed result serialization with `m.upload.size` and `bytes`;
- rejection of invalid `contentUri` (non-`mxc://`, oversized, empty, query-string
  credentials) and non-finite config values;
- response-byte ceiling enforcement without truncation; and
- safe error categories with no raw SDK error, URI, or byte leakage.

The contract tests do not depend on a live homeserver. The merged slice also
contains an authenticated disposable-Synapse proof covering one config call
and one original-file download; the proof remains gated by its explicit CI
environment variable.

## 6. Acceptance checklist recorded for #407

- [x] Exact commands are registered and permissioned without aliases or a
      generic media escape hatch.
- [x] Both commands use the one managed live native Matrix client.
- [x] `CallWidgetDriver.getMediaConfig` and `downloadFile` have no JS media path.
- [x] Config returns the widget API's exact `m.upload.size` key; download returns
      original bytes with a hard size limit and no byte logging.
- [x] Native failures are terminal and visibly reported; no JS fallback or
      `dual_backend` selector exists.
- [x] Physical deletion and source-absence tests pass.
- [x] Focused TypeScript and Rust/IPC contract tests pass.
- [x] Prettier **2.8.1**, repository Matrix guardrails, and relevant lint/type
      checks pass.
- [x] Live proof is recorded separately; this packet is not a V-BURN or full
      V-SEND.R-CALL-UPLOAD completion claim.

The docs PR itself is complete when this packet is reviewed, linked from the
CallWidget residual, and contains no product implementation or `product.rs`
change.

## 7. Honest status

This packet is now an **implementation record**. The media config/download IPC
is implemented by **#407**: `getMediaConfig` and `downloadFile` use the typed
native owner, and `matrix_call_media_config` plus `matrix_media_download` are
registered product commands. The reuse scan in **#387** confirmed that no
existing media IPC was suitable for direct CallWidget download reuse; the
dedicated commands above were therefore delivered. This packet does not claim
full MatrixRTC/CallWidget parity or V-BURN readiness.
