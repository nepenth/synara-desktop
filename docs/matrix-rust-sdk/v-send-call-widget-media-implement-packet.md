# V-SEND.R-CALL-MEDIA — CallWidget media config/download native IPC implementation packet

| Field    | Value                                                                                                                                  |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Status   | **Implementation packet** — this PR is docs-only and does **not** claim the media IPC vertical is implemented                          |
| Residual | **V-SEND.R-CALL-UPLOAD** media-config/download from [v-send-call-widget-residual.md](v-send-call-widget-residual.md)                  |
| Base tip | `b87f3a87` on `feature/matrix-rust-sdk-full-replacement`                                                                               |
| PR shape | Focused **draft** PR targeting `feature/matrix-rust-sdk-full-replacement`                                                              |
| Policy   | [full-vertical-policy.md](full-vertical-policy.md): native UI → Tauri IPC → live `matrix-sdk`, physical JS-owner deletion, fail-closed |
| Guard    | Never `main`, umbrella **#39**, or V-BURN/#327; `dual_backend` is forbidden; **#375** owns `product.rs` now, **members-read #395** next |

> **Scope guard.** Docs only. No product code in `product.rs`, `src-tauri/src/lib.rs`,
> generated permissions, or any TS. Does not touch open **#375** (moderation writes),
> **#395** (members-read product), **#39** (umbrella), or any timeline/send slice.

---

## 1. Objective and completion bar

Replace the two fail-closed CallWidget media stubs with one native-only
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

This packet closes the **media config/download** residual only. Upload is
already closed by **#328**; `getKnownRooms` is already native. Do not claim the
whole V-SEND.R-CALL-UPLOAD vertical closed from this packet alone.

## 2. Frozen scope and prerequisites

### In scope

- `CallWidgetDriver.getMediaConfig()` and `CallWidgetDriver.downloadFile(contentUri)`.
- A typed TypeScript native owner (alongside the existing call-widget owner),
  exact command argument/result validation, and focused IPC contract tests.
- The two Rust product commands `matrix_call_media_config` and
  `matrix_media_download` in the serial product slice.

### Required preflight

Before product implementation starts, the writer must verify:

1. `HEAD` is exactly `b87f3a87` or the approved integration tip that explicitly
   includes it.
2. The PR target is `feature/matrix-rust-sdk-full-replacement`, never `main` or #39.
3. The managed native session exposes one live `matrix-sdk` client for the
   current session; no second Matrix client or selector is introduced.
4. **#375** has landed its `product.rs` ownership and **#395** (members-read)
   has released the serial lock on `product.rs` before this slice edits it.
5. `matrix_upload_media` remains the upload owner; this packet does not reuse it
   for download.

If a prerequisite is false, stop and escalate. Do not add a fallback, rename the
commands, or widen this packet.

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
   Start with the existing 32 MiB attachment IPC ceiling as the policy target,
   subject to #375's product-boundary review; never silently truncate.
5. Return `{ bytes }` through this direct Tauri command only. The bytes must
   not enter a versioned JSON envelope, persistent DTO, diagnostic, or log.
   Errors contain stable diagnostic IDs only and never echo the URI or secrets.

### 3.3 Native owner semantics

Add one SDK-neutral owner at:

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

## 4. Physical deletion list

The following JS ownership is removed in the same product implementation slice.

| Path                                                                        | Delete from path                                                                                                                              | Retain or replace with                                                                                                                          |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `synara/src/app/plugins/call/CallWidgetDriver.ts`                           | the two fail-closed stubs `getMediaConfig()` and `downloadFile()` (lines ~315-317 and ~330-333) and their `throwNativeCallWidgetCapabilityUnavailable` calls | rewire both methods to `nativeCallWidgetMediaOwner` for the native desktop route |
| `synara/src/app/plugins/call/nativeCallWidgetOwner.ts`                      | the generic `throwNativeCallWidgetCapabilityUnavailable` helper **if no other caller remains**                                                | keep `getKnownRoomsFromNativeSnapshot`; the helper may be deleted only when the replacement route is proven                                   |
| `synara/src/app/plugins/call/__tests__/nativeCallWidgetOwner.test.ts`       | the two `throwNativeCallWidgetCapabilityUnavailable` fail-closed assertions if the helper is deleted                                          | add focused native media owner tests (see §5)                                                                                                   |
| `synara/src/app/plugins/call/nativeCallWidgetMediaOwner.ts`                 | New file in the implementation PR; no JS SDK imports or fallback branch                                                                        | The sole desktop media config/download owner for both exact commands                                                                            |

Do **not** delete in this packet:

- `uploadCallWidgetFileWithNativeOwner` / `nativeCallMediaUploadOwner.ts` (upload owner, #328);
- `getKnownRoomsFromNativeSnapshot` (room-list owner);
- `matrix_upload_media` or any P7.2/P7.3 media queue;
- `synara/src/app/matrix/media.ts` / `utils/matrix.ts` HTTP/Blob helpers (other consumers);
- any test that proves SDK-neutral retained behavior; or
- `src-tauri/src/matrix/auth/product.rs` in this docs-only PR.

The implementation PR must include a negative source scan proving that
`CallWidgetDriver.ts` contains none of `getMediaConfig` (JS), `mxcUrlToHttp`,
`downloadMedia`, `fetch`, or a JS fallback for these two media methods.
Repository-wide JS Matrix imports may remain nonzero because other verticals are
still open; that is not permission to retain either media stub.

## 5. Required focused tests

### 5.1 Native owner tests

Add:

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

Add:

`synara/src/app/plugins/call/__tests__/callWidgetMediaSourceGuard.test.ts`

The guard reads `CallWidgetDriver.ts` and asserts:

- no `this.mx.getMediaConfig`, `mxcUrlToHttp`, `downloadMedia`, or `fetch` call
  remains for either media method;
- no `Legacy*` component, `legacy` return sentinel, or `isNative ? rust : js`
  writer selector is introduced; and
- the fail-closed stubs are physically absent.

The guard must not assert repository-wide `matrix-js-sdk` usage is zero.

### 5.3 Rust/IPC contract tests

The implementation PR must add focused contract coverage outside the product
command body for:

- camelCase request serialization for both exact command names;
- typed result serialization with `m.upload.size` and `bytes`;
- rejection of invalid `contentUri` (non-`mxc://`, oversized, empty, query-string
  credentials) and non-finite config values;
- response-byte ceiling enforcement without truncation; and
- safe error categories with no raw SDK error, URI, or byte leakage.

The contract tests must not depend on a live homeserver. An authenticated
disposable Synapse proof belongs to the eventual product implementation PR and
must cover one config call and one original-file download; this docs packet does
not claim that proof.

## 6. Acceptance checklist for the eventual implementation PR

- [ ] Exact commands are registered and permissioned without aliases or a
      generic media escape hatch.
- [ ] Both commands use the one managed live native Matrix client.
- [ ] `CallWidgetDriver.getMediaConfig` and `downloadFile` have no JS media path.
- [ ] Config returns the widget API's exact `m.upload.size` key; download returns
      original bytes with a hard size limit and no byte logging.
- [ ] Native failures are terminal and visibly reported; no JS fallback or
      `dual_backend` selector exists.
- [ ] Physical deletion and source-absence tests pass.
- [ ] Focused TypeScript and Rust/IPC contract tests pass.
- [ ] Prettier **2.8.1**, repository Matrix guardrails, and relevant lint/type
      checks pass.
- [ ] Live proof is recorded separately; this packet is not a V-BURN or full
      V-SEND.R-CALL-UPLOAD completion claim.

The docs PR itself is complete when this packet is reviewed, linked from the
CallWidget residual, and contains no product implementation or `product.rs`
change.

## 7. Honest status

This is an **implementation packet only**. The media config/download IPC is
**not implemented** at this tip: `getMediaConfig` and `downloadFile` remain
fail-closed stubs in `CallWidgetDriver.ts`, and no `matrix_call_media_config` or
`matrix_media_download` command exists in `product.rs`. The reuse scan in
**#387** confirmed no existing media IPC is suitable for direct CallWidget
download reuse; the dedicated commands above are required. This packet does not
claim the media IPC product is done.
