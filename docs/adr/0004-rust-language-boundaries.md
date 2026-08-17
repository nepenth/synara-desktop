# ADR 0004: Rust language boundaries

Reviewed: 2026-08-17

Status: accepted.

Companion to [ADR 0003](0003-shared-native-rust-core.md). ADR 0003 decides
**that** desktop and iOS share one Rust application-logic core. This ADR
decides **what** may be written in Rust, what must stay platform-side, and
what must not be rewritten. It does not open a new crate, UI toolkit, or
parallel migration program.

**How to finish the work this ADR points at:**
[`docs/shared-native-core/11-implementer-playbook.md`](../shared-native-core/11-implementer-playbook.md).

## Context

Synara ships two clients from this monorepo:

- **Desktop** (macOS and Linux): Tauri 2 Rust shell in `src-tauri/` plus a
  React / Vite presenter in `synara/`.
- **iOS**: SwiftUI in `synara-ios/`, consuming `synara-core` through UniFFI.

There is no shipped Windows session store, standalone web client, Android
app, or server product. `integration/synapse/` is a test harness only.

Rust already owns the Matrix engine (`crates/synara-core`), the desktop
shell, and the IPC/UniFFI adapters. TypeScript owns desktop presentation.
Swift owns iOS presentation and Apple-only OS services. The remaining
question is not whether to adopt Rust. It is which leftovers still belong
in `synara-core`, and which proposed Rust rewrites would be a mistake.

Existing decisions this ADR does not reopen:

- [ADR 0002](0002-ios-architecture.md): SwiftUI, not Tauri iOS.
- [ADR 0003](0003-shared-native-rust-core.md): one `synara-core`; UI, OS
  integrations, credential stores, and app lifecycle stay platform-side.
- [Native-first architecture spike](../native-first-architecture-spike.md):
  do not rewrite the desktop UI in a Rust widget toolkit before iOS ships
  on the shared engine.

The shared-core program is in progress, not done. Desktop routes one
hundred eleven `matrix_*` commands through Core. Twenty-one secret- or
byte-sensitive names stay unregistered on purpose. iOS has typed
`SharedCore` wrappers and P4-S12–S37 product consume on `main` (#1001);
product `MatrixRustSDK` callers are retired; iOS is not yet on-engine
(Apple generate, paused hosted iOS CI, and paused live homeserver proof
still sit in front of that claim; leftover I/O that needs a live
homeserver stays fail-closed).

## Decision

Use the rubric below before proposing any new Rust code. Follow the
existing shared-native-core playbook for everything that **should** be
Rust. Do not start a second core crate, a Rust UI rewrite, or a
migration program that bypasses that playbook.

### Should be Rust

Write or finish the work in `synara-core` when most of these are true:

- Shared by desktop and iOS (parity by construction).
- Security- or correctness-sensitive (E2EE, session material, verification,
  store identity).
- A hot path over large Matrix state (sync, timeline projection, search,
  media-queue metadata).
- Needs a single supervisor / concurrency owner (no dual backend).
- Testable without a WebView or SwiftUI (Rust unit tests and Synapse proofs).

### Can be Rust but should not

Leave the current language in place when:

- The work is presentation, layout, animation, or composer UX.
- The work is OS-specific (Keychain, APNs, tray, file dialogs, NSE
  lifecycle).
- Crossing FFI or the Core envelope would move passwords, recovery
  passphrases, `client_secret`, file paths, or media/attachment bytes.
- A mature JS or Swift library already owns the problem (Slate, pdf.js,
  SwiftUI, the Element Call widget).
- The work is CI or governance scripting where Node already fits.

### Must not be Rust (for now)

- A full native desktop UI rewrite (Slint, Dioxus, egui, or similar).
- Replacing SwiftUI with a Rust UI on iOS.
- Shipping Tauri iOS as the product path.
- Putting passwords, key-export passphrases, or attachment bytes on the
  generic `Core::command` / UniFFI command envelope.
- A Rust rewrite of Node build, CI, or guardrail scripts.
- A third Matrix engine in Swift or TypeScript.

## Layer map

| Layer | Today | Verdict |
|---|---|---|
| Matrix engine (sync, crypto, timeline DTOs, room list, auth policy) | Mostly `synara-core`; thin leftovers in `src-tauri/src/matrix/` | Should be Rust. Finish extraction through the playbook. |
| Desktop IPC adapters | `src-tauri/src/bridge/` | Stay Rust. Stay thin. |
| Desktop OS shell | `desktop_*.rs` (tray, keyring, notifications, file transfer) | Stay Rust. Stay out of core. |
| iOS UI | SwiftUI | Stay Swift. Consume UniFFI. |
| iOS Matrix adapters | `SharedCore*` wrappers; leftover I/O fail-closed | Thin Swift over Rust. Not a second engine. NSE stays a narrow read-only store surface and never starts sync. |
| Desktop UI | React / Jotai / Slate / vanilla-extract | Stay TypeScript. Presenter and virtualization only. |
| Media bytes / decrypt-for-display | Native queues; leftover `matrix_send_attachment` / `matrix_media_download` (not `Core::command`) | Rust-owned delivery. Desktop JS encrypt/decrypt and SW token injection are retired. Leftover encrypted `mxc://` without a handle fail-closes. Leftover avatar `<img src=mxc://>` display is still a later visual pass. |
| Markdown / HTML render / PDF | TypeScript and pdf.js | Stay TypeScript. |
| Element Call / MatrixRTC | Placeholder WebView widget | A Rust widget bridge may come later. Do not start a Rust WebRTC stack. |
| Agent / Hermes workflows | React cards plus native action bridge | Policy and approval state may move to core if iOS must share those semantics. Composer and card UI stay put. |
| Build / CI / guardrail scripts | Node (`scripts/*.mjs`) | Stay Node. |
| Stale WASM / IndexedDB / js-sdk CSP | Leftover from the former browser client | Delete. Do not rewrite in Rust. |

## Prescribed next Rust work

This is finish-the-migration work on the existing playbook, not a new
program. Do not invent routes, crates, or bindgen paths to satisfy this
list.

1. **Live Matrix I/O that is not secret- or byte-sensitive** continues
   through playbook 7B: remaining non-secret `product_commands` / `live`
   owners become Core commands with thin Tauri adapters. As of the
   playbook evidence tip, there is no unregistered census name whose
   write already lives on an attached owner. Do not invent one.
2. **The twenty-one shell leftovers stay desktop.** Password
   continuation, export/import passphrases, setup/restore/repair, and
   attachment/media bytes must not cross the 1 MiB Core envelope. That
   is a should-not, not unfinished work. See playbook section 6.
3. **iOS-on-engine (P4, then P5)** is the highest-leverage Rust outcome:
   start SyncService through Core, replace remaining fail-closed
   leftovers that need a live homeserver, and keep NSE read-only. Do not
   start P5 from this ADR. Follow playbook section 5 and section 9.
4. **Native media as the sole decrypt/delivery path.** Desktop composer
   send and timeline/leftover `mxc://` download use the native owner.
   `browser-encrypt-attachment` is removed; `synara/src/sw.ts` is a stub
   (no Matrix token injection). Leftover encrypted `mxc://` without a
   handle fail-closes. iOS leftover media I/O stays fail-closed
   (decision 15). Byte-bearing commands stay shell-side until a written
   owner decision defines a byte channel. Do not register
   `matrix_send_attachment` / `matrix_upload_media` /
   `matrix_media_download` on `Core::command`. Leftover avatar
   `<img src=mxc://>` display is a later visual pass, not a JS decrypt
   rewrite.
5. **Harness-only domains go live in Core** when they are shared product
   behavior (search is the example). Do not promote a harness just to
   have a merge.
6. **Optional, only after iOS-on-engine:** a typed agent-action policy
   module in Core if iOS must share approval and Hermes card semantics.
   Do not move the React agent-card UI.
7. **Housekeeping, not new Rust:** fold `src-tauri` into the workspace
   when the adapter swap is real; remove orphaned WASM, CSP, and
   IndexedDB paths.

## Stay put

These can be written in Rust and should not be:

- **Timeline viewport math and virtualization** in
  `synara/src/app/features/room/`. Rust already projects DTOs. JavaScript
  owns the rendered window. Moving scroll policy into Rust adds IPC
  chatter and does not help iOS, which has its own viewport.
- **Composer, Slate, and markdown parsers.** UI-adjacent. `ruma` already
  handles wire markdown in the SDK.
- **pdf.js viewer.** A Rust PDF renderer would not pay off.
- **Keyring and APNs.** Platform shell by ADR 0003.
- **Node guardrail scripts.** They enforce the Rust boundary. Rewriting
  them in Rust is vanity.

## Rejected alternatives

### Rewrite the desktop UI in Slint, Dioxus, or egui

Rejected. The native-first spike already sequenced iOS-on-engine ahead of
any desktop UI rewrite. A Rust widget toolkit would rebuild the feature
UI without moving the remaining shared Matrix work.

### Ship Tauri iOS, or replace SwiftUI with a Rust UI

Rejected. ADR 0002 stands. Apple-only UI, Keychain, APNs, and NSE stay
Swift. Logic moves; UI does not.

### Register the twenty-one leftover commands on `Core::command`

Rejected. Passwords, recovery secrets, file paths, and media bytes must
not cross the generic envelope. A watchdog prompt is not a new owner
decision.

### Rewrite CI, release, or guardrail scripts in Rust

Rejected. Those scripts are not product runtime. Node already fits.

### Expand to Windows, Android, or a web client in Rust first

Rejected until the two existing apps share one live engine. New platforms
are not a language-boundary escape hatch.

### Start a second core crate or a parallel migration ledger

Rejected. `crates/synara-core` and `docs/shared-native-core/` are the
implementation path.

## Consequences

- Future slices apply this rubric, then the playbook's "how to pick the
  next slice" checklist. The rubric does not authorize skipping disk,
  UniFFI, or leftover-envelope rules.
- iOS feature work that would duplicate sync, room list, timeline, or
  crypto in Swift is out of bounds. Add a Core owner or wait.
- Desktop UI, SwiftUI, and Node tooling stay in their current languages
  unless a later ADR supersedes this one.
- Secret- and byte-sensitive commands remain documented shell leftovers
  until a written owner decision or Platform ADR defines another channel.

## Related documents

- [ADR 0002 — iOS architecture](0002-ios-architecture.md)
- [ADR 0003 — shared native Rust core](0003-shared-native-rust-core.md)
- [Shared native core program](../shared-native-core/README.md)
- [Implementer playbook](../shared-native-core/11-implementer-playbook.md)
- [Native-first architecture spike](../native-first-architecture-spike.md)
