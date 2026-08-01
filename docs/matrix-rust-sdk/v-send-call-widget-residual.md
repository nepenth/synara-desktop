# V-SEND.R-CALL-UPLOAD — CallWidgetDriver native residual inventory

| Field    | Value                                                                                                                   |
| -------- | ----------------------------------------------------------------------------------------------------------------------- |
| Status   | **Native room-list reuse + fail-closed media boundary implemented**                                                     |
| Base tip | `4eeefa11` on `feature/matrix-rust-sdk-full-replacement`                                                                |
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
driver. This does not claim full call/widget cutover or start a burn slice.

## CallWidgetDriver inventory

| Source                        | Production operation                                                  | Native desktop status                                                                                                           | Residual decision                                                           |
| ----------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `CallWidgetDriver.ts`         | `getMediaConfig()`                                                    | **Fail-closed**; no native call-widget media-config command is present in this tip                                              | Keep blocked until an owning native IPC contract exists                     |
| `CallWidgetDriver.ts:317-326` | `uploadFile(file)` delegates to `uploadCallWidgetFileWithNativeOwner` | **Native-first and fail-closed** for a logged-in native session; uses `matrix_upload_media`                                     | Upload owner closed by **#328**; legacy callback remains for non-native use |
| `CallWidgetDriver.ts`         | `downloadFile(contentUri)`                                            | **Fail-closed**; no native call-widget media-download command is present in this tip                                            | Keep blocked until an owning native IPC contract exists                     |
| `CallWidgetDriver.ts`         | `getKnownRooms()`                                                     | **Native snapshot-backed**; uses the cached `matrix_room_list_snapshot` readback and returns `[]` until a valid snapshot exists | Room-list owner is native; no SDK visible-room fallback                     |

The media methods remain explicit blocked surfaces because no native IPC owner
exists for them. The room method is wired to the native snapshot owner. This
slice does not claim full call/widget cutover or start a burn slice.

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
download command, so this slice does not add a speculative `product.rs`
command.

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
