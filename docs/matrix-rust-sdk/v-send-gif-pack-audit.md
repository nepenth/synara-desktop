# V-SEND.R-GIF-PACK — audit result

| Field        | Value                                                               |
| ------------ | ------------------------------------------------------------------- |
| Status       | **NOOP** — no product surface exists; docs-only residual correction |
| Measured tip | `c22515fa` on `feature/matrix-rust-sdk-full-replacement` |
| Base         | `feature/matrix-rust-sdk-full-replacement`                          |
| Scope guard  | No product code, no `main`, no umbrella PR **#39**, no cutover      |

## Conclusion

`V-SEND.R-GIF-PACK` is not an actionable Matrix Rust replacement residual on
the measured tip. Synara has a provider-backed GIF **search picker** and a
one-shot GIF download/send path, but it has no GIF pack or collection feature:
there is no saved GIF collection, favorites list, pack account-data/state,
collection mutation, or GIF-specific native command to replace.

This is therefore a product **NOOP**, not an implementation slice. The
residual is removed from the active queue and retained only as this audit
record so a future GIF collection feature would require a new, evidence-backed
scope decision.

## Evidence

| Area                   | Branch-tip evidence                                                                                                                                             | Finding                                                                      |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Picker UI              | `synara/src/app/features/room/gif/GifPicker.tsx` — local state is only `query`, `loading`, `error`, and transient `results`; the only action is `onSelect(gif)` | Search/results/send selection; no pack or collection controls                |
| Provider utility       | `synara/src/app/utils/gifProvider.ts` — `GifPickerConfig`, `GifResult`, `buildGifSearchUrl`, `searchGifProvider`, and `fetchGifForUpload`                       | Provider search and one-shot download only; no persistence or collection API |
| Composer owner         | `synara/src/app/features/room/RoomInput.tsx` — `handleGifSelect` delegates to `sendComposerGifWithNativeOwner` and otherwise uses the legacy web fallback       | GIF send is the already-landed #264 send surface, not pack management        |
| Native owner           | `synara/src/app/features/room/nativeSendGifOwner.ts` — `matrix_send_attachment` with `image/gif` bytes                                                          | Shared native attachment send; no GIF collection command                     |
| Native command surface | `src-tauri/src/lib.rs` — GIF references are MIME detection plus the generic attachment registration; no GIF/collection command is registered                    | No native GIF collection owner is missing                                    |
| Settings               | `synara/src/app/state/settings.ts` and `RoomInput.tsx` — `gifSearchEnabled` and `gifOnboardingDismissed`                                                        | User opt-in/onboarding only; no collection state                             |
| Message metadata       | `synara/src/app/features/room/msgContent.ts` — `in.synara.gif` provider/source attribution                                                                      | Sent-message metadata, not a collection or pack owner                        |

Exact call-site evidence:

- Native owner: `synara/src/app/features/room/nativeSendGifOwner.ts:17-20,22-52`
  checks the native session, downloads bytes, and sends `image/gif` through
  `matrix_send_attachment`; failures throw instead of selecting JS.
- Adapter: `synara/src/app/features/room/nativeSendGif.ts:5-12` wires the
  composer to that native owner through the desktop availability wrapper.
- Composer boundary: `synara/src/app/features/room/RoomInput.tsx:899-904`
  returns after native ownership; the remaining JS fallback is
  `RoomInput.tsx:907-923` (`mx.uploadContent` + `mx.sendMessage`).
- Provider/picker only: `synara/src/app/utils/gifProvider.ts:3-22,175-191,211-239`
  and `synara/src/app/features/room/gif/GifPicker.tsx:38-60,148-171` contain
  provider search, one-shot download, transient results, and selection—not
  pack or collection state.
- Owner-route tests: `synara/src/app/features/room/__tests__/nativeSendGifOwner.test.ts:17-60,89-104`
  cover native send, legacy web/logged-out routing, and no native-to-JS
  fallthrough.

The scoped source scan was:

```text
rg -n -i 'gif|collection|favorite|pack' \
  synara/src/app/features/room/gif \
  synara/src/app/utils/gifProvider.ts \
  synara/src/app/features/room/nativeSendGifOwner.ts \
  synara/src/app/state/settings.ts \
  src-tauri/src
```

The GIF hits are limited to search, download, send, MIME detection, and
message metadata. The `favorite`/`pack` matches elsewhere in `src-tauri/src`
belong to unrelated room-list or sticker/emoji image-pack code, not GIF
collections.

A negative search for GIF-specific collection naming returned no matches
(ripgrep exit `1`):

```text
rg -n -i 'gif.*(pack|collection|favorite)|(?:pack|collection|favorite).*gif|gif_(pack|collection)|Gif(Pack|Collection)' \
  synara/src/app src-tauri/src --glob '*.{ts,tsx,rs}'
```

## Boundary

- GIF **send** remains native-first through #264 and is documented in
  [v-send-sticker-gif.md](v-send-sticker-gif.md).
- GIF provider search/download is external-provider behavior, not Matrix
  account-data/state ownership.
- Timeline GIF playback/display remains **V-TIMELINE** and is out of scope
  for this audit.
- If a future feature adds GIF collections, favorites, or packs, open a new
  residual with explicit product owners and Matrix state/IPC requirements.
