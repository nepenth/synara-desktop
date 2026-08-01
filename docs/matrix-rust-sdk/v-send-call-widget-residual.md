# V-SEND.R-CALL-UPLOAD — CallWidgetDriver residual inventory

| Field        | Value                                                                                                                   |
| ------------ | ----------------------------------------------------------------------------------------------------------------------- |
| Status       | **Docs-only audit** — no product code changed                                                                           |
| Measured tip | `9eb4689b` on `feature/matrix-rust-sdk-full-replacement` |
| Scope        | `CallWidgetDriver` upload, media-config, media-download, and known-room methods                                         |
| Guard        | Never touch `main` or umbrella PR **#39**; `dual_backend` is forbidden; **#327 remains HOLD and V-BURN is not started** |

## Finding

#328 makes `CallWidgetDriver.uploadFile` native-first for a logged-in native
desktop session. It enters `uploadCallWidgetFileWithNativeOwner`, invokes
`matrix_upload_media`, and returns the native `mxc://` result. Unsupported
widget bodies, unavailable native commands, and invalid native responses are
terminal; the `client.uploadContent` callback is not selected on that native
path.

The same driver still exposes three JS-backed widget surfaces. They remain
explicit residuals here; this note does not claim full call/widget cutover or
start a burn slice.

## CallWidgetDriver inventory

| Source                        | Production operation                                                                                 | Native desktop status                                                                       | Residual decision                                                           |
| ----------------------------- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `CallWidgetDriver.ts:311-315` | `getMediaConfig()` returns `this.mx.getMediaConfig()`                                                | **JS residual**; no native call-widget media-config command is present in this tip          | Keep as a documented widget-adjacent residual; no product change here       |
| `CallWidgetDriver.ts:317-326` | `uploadFile(file)` delegates to `uploadCallWidgetFileWithNativeOwner`                                | **Native-first and fail-closed** for a logged-in native session; uses `matrix_upload_media` | Upload owner closed by **#328**; legacy callback remains for non-native use |
| `CallWidgetDriver.ts:328-335` | `downloadFile(contentUri)` resolves `mxc://` with `mxcUrlToHttp` and fetches through `downloadMedia` | **JS residual**; no native call-widget media-download command is present in this tip        | Keep as a documented widget-adjacent residual; no product change here       |
| `CallWidgetDriver.ts:337-339` | `getKnownRooms()` maps `this.mx.getVisibleRooms()` to room IDs                                       | **JS residual**; no native call-widget room-list readback is used here                      | Keep as a documented widget-adjacent residual; no product change here       |

The three remaining methods are not upload fallbacks. Removing them without
an owning native widget contract would change the current Element Call bridge,
so this docs-only slice records them without deleting or rewiring JS owners.

## Native upload evidence

The verified #328 route is:

```text
CallWidgetDriver.uploadFile
  → uploadCallWidgetFileWithNativeOwner
  → matrix_session_snapshot
  → matrix_upload_media
  → native Matrix SDK media upload
  → mxc:// result
```

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

## SCOREBOARD cross-link

`V-SEND.R-CALL-UPLOAD` remains closed for the **upload owner** by #328. The
three named methods above are widget-adjacent JS residuals, not evidence that
upload is still JS-backed. The scoreboard row links here so the distinction
between upload closure and remaining widget surfaces stays explicit.

## Verification

The production inventory was checked with:

```text
rg -n 'getMediaConfig|downloadFile|getKnownRooms|uploadFile' \
  synara/src/app/plugins/call/CallWidgetDriver.ts
```

No product code, native command, dual-backend flag, V-BURN state, or #327 state
was changed by this audit.
