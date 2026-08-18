# Desktop Matrix SDK Boundaries

Date: 2026-06-07

> **Historical pre-cutover audit.** The desktop product no longer uses
> `matrix-js-sdk` as its Matrix runtime. The shared Rust core is the current
> owner; see [the codebase knowledge base](../CODEBASE_KNOWLEDGE_BASE.md) and
> [ADR 0004](adr/0004-rust-language-boundaries.md). The details below are
> retained as migration evidence.

## Summary

The macOS and Linux desktop apps run the shared `synara/` runtime inside the
Tauri shell. That runtime uses `matrix-js-sdk` as the first-class Matrix
integration surface.

The desktop alignment goal is not to remove `matrix-js-sdk`. The goal is to
prevent Matrix behavior from being spread across arbitrary UI components. Domain
helpers should own common Matrix operations so desktop behavior can be compared
against iOS Matrix Rust SDK behavior with fixtures and contract tests.

## Current Accepted Matrix Stack

- Client lifecycle: `synara/src/client/initMatrix.ts`
- Shared Matrix helpers: `synara/src/app/utils/matrix.ts`
- Desktop media domain: `synara/src/app/matrix/media.ts`
- Room/state/timeline helpers: `synara/src/app/utils/room.ts`
- Notification/read-marker helpers: `synara/src/app/utils/notifications.ts`
- Synara account data:
  - `synara/src/app/utils/later.ts`
  - `synara/src/app/utils/roomNotes.ts`
- Upload state: `synara/src/app/state/upload.ts`
- Room list state: `synara/src/app/state/room-list/*`
- Room state atoms: `synara/src/app/state/room/*`
- Device/security flows: `synara/src/app/features/settings/devices/*`,
  `synara/src/app/components/DeviceVerification*.tsx`,
  `synara/src/app/components/BackupRestore.tsx`
- Call/widget integration: `synara/src/app/plugins/call/*`

## Direct Matrix Client Buckets

These are the repo-wide desktop buckets to burn down into narrower domain
services/hooks.

### Room List And Navigation

Current surface:

- `synara/src/app/state/room-list/*`
- `synara/src/app/hooks/router/*`
- `synara/src/app/utils/sort.ts`
- `synara/src/app/features/room-nav/*`
- `synara/src/app/pages/client/sidebar/*`

Target boundary:

- One room-list domain hook/service exposes room summaries, ordering,
  unread/highlight state, invite state, and space grouping.
- UI components should not call `mx.getRoom(...)` or `mx.getRooms()` directly
  unless they are inside that domain.

### Timeline And Message Actions

Current surface:

- `synara/src/app/features/room/*`
- `synara/src/app/components/message/*`
- `synara/src/app/hooks/useCommands.ts`
- `synara/src/app/utils/room.ts`
- `synara/src/app/utils/forward.ts`

Target boundary:

- One timeline domain owns live timeline access, pagination, reactions, edits,
  redactions, replies, receipts, typing, and thread semantics.
- Cross-platform fixtures should verify reaction, edit, redaction, reply, poll,
  and agent event content match iOS.

### Media

Current surface:

- `synara/src/app/matrix/media.ts`
- `synara/src/app/utils/matrix.ts`
- `synara/src/app/state/upload.ts`
- `synara/src/app/components/message/content/*`
- `synara/src/app/components/media/*`
- `synara/src/sw.ts`

Target boundary:

- One media domain owns authenticated MXC conversion, uploads, downloads,
  encrypted-media decrypt policy, save/share behavior, and retry/progress state.
- The service worker remains an approved desktop implementation detail for
  authenticated media fetches.

Current progress:

- `app/matrix/media.ts` now owns authenticated MXC URL resolution,
  thumbnail URL resolution, browser-service-worker-backed download, encrypted
  attachment decrypt policy, and object URL creation for message media.
- File, thumbnail, image, video, and audio message components have moved to this
  boundary. File-header downloads also use this boundary.
- Member tiles, user room profiles, lobby hero/header, and unjoined lobby
  room/space summaries use the media boundary for avatar thumbnail resolution.
  Main room message sender avatars also use this boundary.
- Remaining media burn-down is upload/progress behavior, media viewer save/share
  paths, and less common non-message avatar/media call sites.

### Account Data

Current surface:

- `synara/src/app/utils/later.ts`
- `synara/src/app/utils/roomNotes.ts`
- `synara/src/app/utils/notifications.ts`
- `synara/src/app/utils/matrix.ts`
- `synara/src/app/plugins/recent-emoji.ts`
- `synara/src/app/plugins/custom-emoji/*`
- `synara/src/app/features/settings/developer-tools/*`

Target boundary:

- Synara account-data schemas live in one contract package/folder.
- Later, room notes, unread anchors, room anchors, spaces filters, and any
  iOS-visible account data use shared codecs and fixtures.
- Developer tools can remain a raw escape hatch but must not become production
  feature code.

### Profile, Avatar, Member, And Permissions

Current surface:

- `synara/src/app/components/user-profile/*`
- `synara/src/app/components/member-tile/*`
- `synara/src/app/components/room-avatar/*`
- `synara/src/app/components/room-card/*`
- `synara/src/app/components/editor/autocomplete/*`

Target boundary:

- One profile/member domain owns member display names, avatar URLs, power
  levels, permission checks, and mention autocomplete data.
- UI components receive display-ready profile/member models.

### Device, Crypto, And Verification

Current surface:

- `synara/src/app/features/settings/devices/*`
- `synara/src/app/components/DeviceVerification*.tsx`
- `synara/src/app/components/ManualVerification.tsx`
- `synara/src/app/components/SecretStorage.tsx`
- `synara/src/app/components/BackupRestore.tsx`
- `synara/src/app/state/backupRestore.ts`

Target boundary:

- One device/security domain owns cross-signing, verification, key backup,
  recovery, and local backup import/export.
- iOS and desktop expose the same device naming convention and security status
  vocabulary.

### Calls And Widgets

Current surface:

- `synara/src/app/plugins/call/*`
- `synara/src/app/features/call/*`
- `synara/src/app/features/call-status/*`
- `synara/src/app/hooks/useCall.ts`

Target boundary:

- Calls remain desktop-first until iOS voice/video is explicitly scoped.
- Call/widget Matrix access stays quarantined in call plugin code and does not
  leak into general timeline/domain helpers.

## Content-Security-Policy Exceptions

Synara Desktop sets its CSP in `src-tauri/tauri.conf.json`. The baseline is
`default-src 'self'` with hardening directives (`base-uri 'none'`,
`object-src 'none'`, `form-action 'none'`). The following sources deviate from
that baseline and are required for Matrix federation, desktop shell integration,
or embedded call UI.

| Directive | Exception | Justification |
| --- | --- | --- |
| `script-src` | `'wasm-unsafe-eval'` | `matrix-js-sdk` initializes the Rust crypto backend via `initRustCrypto()` in `synara/src/client/initMatrix.ts`. WebAssembly compilation requires this CSP keyword; without it E2EE, verification, and encrypted media fail. |
| `style-src` | `'unsafe-inline'` | Runtime theme and layout code sets inline styles (`ThemeManager.tsx`, `ClientNonUIFeatures.tsx`, `CallEmbed.ts`). UI libraries also emit inline style attributes. A nonce/hash-only policy would require a large refactor. |
| `img-src` | `data:` | Inline data-URI avatars, placeholders, and generated thumbnails in the message timeline. |
| `img-src` | `blob:` | Object URLs created by the media domain (`synara/src/app/matrix/media.ts`) for decrypted attachments and local previews before upload. |
| `img-src` | `filesystem:` | Legacy Chromium/Tauri file-picker preview URLs for local image selection flows. |
| `img-src` | `http:` `https:` | Matrix federation media: MXC URLs resolve to arbitrary homeserver and CDN hosts (`/_matrix/media/...`). Users connect to any federated homeserver; host allowlists are not practical. |
| `font-src` | `data:` | Embedded icon and font data URIs shipped with the bundled frontend assets. |
| `media-src` | `data:` `blob:` | Local and decrypted audio/video playback via object URLs and inline media previews in the timeline. |
| `media-src` | `http:` `https:` | Matrix federation media playback (voice messages, video, call recordings) from homeserver media endpoints. |
| `connect-src` | `blob:` | Fetch/XHR against blob URLs produced by the media and crypto layers (e.g. decrypted attachment buffers). |
| `connect-src` | `ipc:` `http://ipc.localhost` | Tauri v2 IPC custom protocol for native command invocation (`@tauri-apps/api`). Required for desktop shell features (session persistence, filesystem, notifications). |
| `connect-src` | `ws:` `wss:` | Matrix sync long-polling fallback and live sync WebSockets to the connected homeserver and federated endpoints. |
| `connect-src` | `http:` `https:` | Matrix Client-Server API, identity lookups, push gateways, Element Call widget API traffic, and authenticated media fetches via the service worker (`synara/src/sw.ts`). Homeserver host is user-configured and federated. |
| `worker-src` | `blob:` | Service worker and module-worker bootstrap paths used by the authenticated media fetch worker (`synara/src/sw.ts`) and bundler-generated worker chunks. |
| `frame-src` | `'self'` | Element Call is embedded from the same-origin bundled path `/public/element-call/index.html` via `CallEmbed.ts`; no remote iframe origins are used. |
| `frame-src` | `blob:` | Retained for blob-backed iframe/worker fallbacks inside the Element Call embed; same-origin `'self'` covers the primary call surface. |

### Residual risk

`http:` and `https:` wildcards on `img-src`, `media-src`, and `connect-src`
remain because Matrix clients must reach arbitrary federated homeservers and
their media CDNs. Tightening these further would break login, sync, media, and
calls for non-default servers. `frame-src` was narrowed from `http: https:` to
`'self' blob:` because call embeds load only from the bundled Element Call
assets (see `synara/src/app/plugins/call/CallEmbed.ts`).

## External URL policy (MIP1 remediation)

Desktop external navigation is enforced in two layers:

- **Rust (authoritative):** `is_safe_external_url` and `is_safe_agent_url` in
  `src-tauri/src/desktop.rs` reject credentialed URLs, non-loopback HTTP, private
  IPv4/IPv6 targets, and local host suffixes. `mailto:` and `matrix:` schemes require
  minimal structure validation.
- **TypeScript (defense in depth):** `isSafeHttpsUrl` / `safeRemoteContentUrl` in
  `synara/src/app/utils/remoteContent.ts` and `isSafeDesktopExternalUrl` in
  `synara/src/app/utils/desktop.ts` mirror the public HTTPS rules before IPC.

`global-shortcut:allow-register-all` remains broad at the Tauri capability layer;
shortcut strings are validated and registered only through Rust
`apply_desktop_shortcuts`.

## Guardrails

- `npm run check:matrix-boundaries` blocks new direct Matrix REST usage outside
  approved exceptions.
- New desktop Matrix behavior should prefer existing `matrix-js-sdk` helpers or
  a domain helper before UI-level client calls.
- New shared Synara feature state must add a fixture/contract test if it writes
  Matrix account data or custom events.

## Recommended Burn-Down Order

1. Account-data contracts: Later, room notes, unread anchors, room anchors,
   spaces filters.
2. Timeline action contracts: reaction, edit, redaction, reply, poll, agent
   custom events.
3. Media domain: authenticated MXC, encrypted media, download/share, upload
   progress.
4. Profile/member domain: avatars, display names, mentions, permissions.
5. Device/security domain: device names, verification status, key backup
   vocabulary.
6. Room-list domain: summaries, spaces, unread/highlight, room ordering.

## Acceptance Criteria

- Desktop direct `MatrixClient` access is allowed only inside documented
  domains or migration exceptions.
- Cross-platform fixtures exist for shared account-data and event-content
  contracts before iOS or desktop change those schemas.
- macOS and Linux continue to use the same desktop runtime behavior.
- CI continues to block new unapproved direct Matrix REST usage.
