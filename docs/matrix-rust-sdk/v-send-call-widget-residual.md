# V-SEND.R-CALL-UPLOAD — CallWidgetDriver residual inventory

| Field        | Value                                                                                                                       |
| ------------ | --------------------------------------------------------------------------------------------------------------------------- |
| Status       | **Docs-only audit** — no product code changed                                                                               |
| Measured tip | `e38bfdab68bd57e4f3110a812c5e4c5d543c1ff5` (`feature/matrix-rust-sdk-full-replacement`)                                     |
| Scope        | `CallWidgetDriver` media/room methods, the call-widget upload owner, and production `createClient` paths under `synara/src` |
| Guard        | Do not touch `main`, merge umbrella PR **#39**, or introduce `dual_backend`                                                 |

## Finding

#328 closes the call-widget upload route for a logged-in native desktop
session. `CallWidgetDriver.uploadFile` now enters
`uploadCallWidgetFileWithNativeOwner`, checks the native session, converts the
widget body to bytes, and invokes `matrix_upload_media`. The native command
uses the live Rust Matrix SDK client. A missing command, invalid native result,
or unsupported widget body is terminal; it does not select
`client.uploadContent`.

The same driver still exposes three JS-SDK-backed widget surfaces on native
desktop: media configuration, media download, and known-room enumeration.
They are recorded here as call-widget adjacency residuals. This note does not
claim full call/widget cutover and does not invent a new product vertical.

## CallWidgetDriver inventory

| Source                                                | Production operation                                                                                 | Native desktop status                                                                                                   | Residual decision                                                                   |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `synara/src/app/plugins/call/CallWidgetDriver.ts:311` | `getMediaConfig()` returns `this.mx.getMediaConfig()`                                                | **Reachable**; no native call-widget media-config command was found                                                     | Keep as a documented widget-adjacent JS read; no product change in this slice       |
| `synara/src/app/plugins/call/CallWidgetDriver.ts:317` | `uploadFile(file)` enters `uploadCallWidgetFileWithNativeOwner`                                      | **Native-first and fail-closed** for a logged-in native session; calls `matrix_upload_media` → `Client::media().upload` | **Closed by #328**; the legacy callback remains only for web or logged-out sessions |
| `synara/src/app/plugins/call/CallWidgetDriver.ts:328` | `downloadFile(contentUri)` resolves `mxc://` with `mxcUrlToHttp` and fetches through `downloadMedia` | **Reachable**; the Rust P7.2 download queue is metadata-only and no call-widget media-download command was found        | Keep as a documented widget-adjacent JS download residual                           |
| `synara/src/app/plugins/call/CallWidgetDriver.ts:337` | `getKnownRooms()` returns `this.mx.getVisibleRooms().map(...)`                                       | **Reachable**; no call-widget room-list IPC readback is used here                                                       | Keep as a documented widget-adjacent JS read                                        |

These three remaining methods are not upload fallbacks. Removing them without
an owning native widget contract would break the current Element Call bridge,
so no JS owner is deleted in this docs-only slice.

## `createClient` inventory

The only production `createClient` call under `synara/src` is
`synara/src/client/initMatrix.ts:206`, in the general JS client bootstrap. It
is not constructed by `CallEmbed` or `CallWidgetDriver`; `CallEmbed.ts:138`
receives an existing `MatrixClient` and injects it into the widget driver.

The remaining `createClient` matches are in the Synapse integration harness
(`synara/scripts/run-synapse-two-client-integration.mjs`) or tests/guardrails.
No call-specific `createClient` path was found. This inventory does not claim
that the general bootstrap is deleted; that is outside the call-upload slice.

## Native upload path

The confirmed #328 route is:

```text
CallWidgetDriver.uploadFile
  → uploadCallWidgetFileWithNativeOwner
  → matrix_session_snapshot
  → matrix_upload_media
  → MatrixAuthState active client
  → matrix_sdk::Client::media().upload
  → mxc:// result
```

Evidence at the measured tip:

- `synara/src/app/plugins/call/CallWidgetDriver.ts:317-325` selects the native
  owner before the legacy callback.
- `synara/src/app/plugins/call/nativeCallMediaUploadOwner.ts:49-71` rejects
  unsupported bodies and returns native failures without fallback.
- `synara/src/app/state/nativeMediaUploadOwner.ts:22-30,45-59` gates on a
  logged-in native session and validates the native `mxc` response.
- `src-tauri/src/lib.rs:464` registers `matrix_upload_media`.
- `src-tauri/src/matrix/auth/product.rs:3115-3144` calls the live SDK media
  upload and returns the native `mxc://` URI.
- `synara/src/app/plugins/call/__tests__/nativeCallMediaUploadOwner.test.ts`
  covers native success, web legacy ownership, missing-command fail-closed,
  invalid-response fail-closed, and unsupported widget bodies.

## Verification

The production scan excluded tests and used:

```text
rg -n 'getMediaConfig|downloadFile|getKnownRooms|uploadFile|createClient' \
  synara/src src-tauri/src synara/scripts --glob '*.{ts,tsx,js,rs}'
```

The native-side scan found the upload command and the metadata-only P7.2
download foundation, but no native call-widget command for the three
remaining methods. No product code or new reliability machinery is proposed.
