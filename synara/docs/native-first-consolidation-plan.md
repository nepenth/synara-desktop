# Native-First Consolidation Plan

> **Historical implementation plan.** The standalone web promise was retired,
> platform APIs were established, iOS was built in SwiftUI, and both shells now
> consume the shared Rust core. Use
> [the codebase knowledge base](../../CODEBASE_KNOWLEDGE_BASE.md) for current
> architecture.

Reviewed: 2026-05-26

## Decision Summary

Synara should stop treating the standalone web client as a public product
channel. The active desktop runtime lives directly in `synara-desktop/synara`,
and the native product channels should be:

- macOS and Linux: Tauri shell plus the Synara app runtime.
- iOS: native SwiftUI app backed by Matrix Rust SDK Swift bindings.

This does not mean removing HTML/CSS/TypeScript from the desktop product today.
It means the WebView runtime is an internal implementation detail of the native
desktop package, not a separately supported browser client or deployment target.

The first pre-iOS architecture spike is recorded in the parent desktop repo at
`docs/native-first-architecture-spike.md`. Its planning decision is to keep
Tauri as the internal macOS/Linux runtime while building the first native iOS
app in SwiftUI. A desktop rewrite should wait until we have stronger evidence
that it reduces total product complexity without regressing Linux.

The recommended decision sequence is:

1. Canonicalize the active app runtime.
2. Replace desktop-named browser APIs with platform contracts.
3. Extract shared notification, badge, route, agent-action, settings, and
   storage semantics.
4. Build iOS against those contracts.
5. Revisit a native desktop rewrite only after iOS and Matrix Rust SDK work
   produce evidence that it is the better route.

Current local pre-iOS status: local preparation is complete. The active runtime
is canonicalized, public web deployment support is retired, desktop-named
cross-platform calls are behind platform adapters, native credential/session
migration is implemented for the desktop shell, and shared contract artifacts
are inventoried in `docs/synara-contracts.md` and `docs/contracts/`. The next
gate is packaged macOS and Linux desktop validation.

## Native Desktop Rewrite Decision Spike

Status: completed for planning. The accepted next step is platform-contract
extraction, not a desktop rewrite.

Goal: decide whether to replace the Tauri/WebView desktop runtime before iOS.

Options to evaluate:

1. Keep Tauri for macOS/Linux, build native SwiftUI iOS.
2. Rewrite Apple platforms in SwiftUI, keep Tauri or another runtime for Linux.
3. Move to a shared Rust-native UI toolkit for macOS, Linux, and iOS.
4. Move Matrix/domain logic to Rust now, but keep the existing Tauri UI during
   the transition.

Questions:

- Can one native UI toolkit deliver App Store-grade iOS, polished macOS, and
  high-quality Linux without unacceptable compromises?
- If macOS uses SwiftUI, what is the Linux UI stack?
- Can Matrix Rust SDK replace `matrix-js-sdk` for the required desktop flows now?
- Can we preserve agent cards, composer behavior, timeline virtualization,
  notification center, Later, file/media flows, and Linux packaging during the
  rewrite?
- Does the rewrite reduce total complexity, or does it just move complexity
  into platform-specific UI forks?
- What is the migration path for existing desktop users' sessions, settings,
  caches, drafts, and Matrix crypto stores?

Spike deliverables:

- ADR comparing the four options: `../../docs/native-first-architecture-spike.md`
  in the nested desktop checkout.
- One small proof of concept for the leading native option.
- Matrix Rust SDK desktop feasibility report.
- Linux UI and packaging feasibility report.
- Migration impact estimate for current desktop users.
- Recommendation: rewrite before iOS, keep Tauri through iOS, or hybridize.

Acceptance criteria:

- The recommendation names a primary desktop architecture.
- The recommendation has a cost estimate in weeks/months, not just a preference.
- The Linux path is concrete.
- Existing desktop feature parity risks are listed.
- The iOS plan is updated to match the chosen path.

## Why Rewriting Desktop May Not Be The Right Pre-iOS Move

A native desktop rewrite would mean replacing the current Tauri/WebView UI with
SwiftUI on macOS and a separate Linux UI stack. That has serious cost:

- macOS and Linux would no longer share the same UI implementation.
- Linux would need a new native toolkit choice, packaging strategy, accessibility
  pass, media/file integration, and desktop-environment validation.
- Current agent, timeline, composer, notification center, Later, and desktop
  bridge work would need to be rebuilt before iOS even starts.
- We would be validating three new frontends at once instead of one.

The expected benefit is a cleaner native architecture, especially for Apple
platforms, but the rewrite is only a simplification if the Linux and migration
story stay coherent.

Until the spike proves otherwise, the lower-risk path is to extract contracts
and platform services while keeping Tauri for desktop.

## Canonical Repository Plan

Status: local consolidation complete. The active runtime is
`synara-desktop/synara` as a normal tracked directory. The previous submodule
relationship has been removed so the desktop shell and runtime share one
repository, one issue tracker, and one CI surface.

Current state:

- `synara-desktop/synara` is the active desktop runtime.
- The sibling `synara` checkout should not be recreated for active work.
- The iOS planning docs have been copied into `synara-desktop/synara/docs` so
  the active runtime carries the forward plan.

Tasks:

1. Inventory unique commits and files in the sibling `synara` checkout.
2. Cherry-pick or port any wanted changes into `synara-desktop/synara`.
3. Confirm all planning docs live in `synara-desktop/synara/docs`.
4. Update release/build docs to refer only to the desktop runtime.
5. Archive or delete any remaining sibling `synara` checkout only after explicit approval.

Acceptance criteria:

- `synara-desktop/synara` contains all wanted app-runtime changes.
- The standalone `synara` tree has no unique wanted work.
- New work instructions point to `synara-desktop/synara`.
- Deletion or archival is performed only after a final diff review.

## Platform API Abstraction

Status: local pre-iOS abstraction complete. `src/app/platform/index.ts` exports
focused platform modules for capabilities, badge, notification, file,
external-link, shortcut, tray state, diagnostics, session, secret-store, and
agent-action behavior. Existing UI callers use the platform API; the parent
Tauri shell bridge is represented by capability flags for integration status,
tray state, secure credentials, and structured shortcut registration results.
`src/app/utils/desktop.ts` remains as the desktop backing implementation for
this migration cycle.

Resolved local result:

- UI callers use `src/app/platform` instead of reaching into Tauri globals or
  `src/app/utils/desktop.ts`.
- `src/app/utils/desktop.ts` is limited to the desktop backing implementation
  and direct compatibility tests.
- Desktop-only concepts such as global shortcuts and tray state are capability
  gated while app badge, notification summary, routes, settings, sessions, and
  agent actions have platform-facing contracts.

Target shape:

```text
src/app/platform/
  index.ts
  capabilities.ts
  notifications.ts
  badge.ts
  agentActions.ts
  files.ts
  diagnostics.ts
  shortcuts.ts
  sessions.ts
  secrets.ts
  tray.ts
```

Naming rules:

- Use `Platform*`, `Native*`, or feature names for cross-platform concepts.
- Keep `Desktop*` only for genuinely desktop-only concepts such as tray and
  global shortcuts.
- Keep Tauri command names stable initially, but hide them behind platform
  adapter functions.

Completed local tasks:

1. Add platform capability types:
   - `channel`: `local-dev-runtime`, `desktop-tauri`, `ios-native`, `unknown`.
   - `supportsSystemNotifications`.
   - `supportsAppBadge`.
   - `supportsTray`.
   - `supportsGlobalShortcuts`.
   - `supportsNativeFileSave`.
   - `supportsNativeFileDrop`.
   - `supportsAgentActions`.
   - `supportsSecureSecretStore`.
   - `supportsIntegrationStatus`.
   - `supportsTrayState`.
2. Create `platform/notifications.ts` with shared request normalization and
   desktop delivery.
3. Create `platform/badge.ts` with shared count clamping.
4. Create `platform/agentActions.ts` with shared validation.
5. Create `platform/files.ts` for download/save/drop behavior.
6. Move desktop shortcut and tray behavior behind platform capability modules.
7. Update callers to use platform APIs instead of `utils/desktop.ts`.
8. Keep `utils/desktop.ts` as a compatibility wrapper for one migration cycle,
   then remove it.

Acceptance criteria:

- UI code does not call `window.__SYNARA_DESKTOP__` or `__TAURI_INTERNALS__`
  directly.
- Notification, badge, and agent action logic is tested independently of Tauri.
- Desktop-only settings are isolated from shared settings.
- The Tauri command surface remains bounded and validated in Rust.
- Local Vite/browser execution is treated as a development harness, not a
  product channel.

## Shared Route And Notification Semantics

Status: route contract and schema completed. `docs/synara-route-contract.md`
defines app-relative route rules, destination kinds, current desktop path
shapes, notification routing expectations, and proposed iOS transport mapping.
`docs/contracts/` contains the canonical route schema and fixtures.

Resolved local result:

- Notification routes use bounded app-relative path strings.
- Unknown, external, or unsupported route targets are rejected before platform
  notification delivery.
- iOS can implement a native router against the same route schema and fixtures.

Completed local tasks:

1. Keep the route contract synchronized with `src/app/pages/paths.ts`.
2. Add parser/normalizer tests for the runtime route contract.
3. Route notifications, Later reminders, and agent approval jumps only to
   safe internal destinations.
4. Treat unknown or unauthorized route targets as safe fallbacks.

Acceptance criteria:

- Route payloads never contain tokens, device identifiers, decrypted message
  bodies, or private notification preview text.
- Desktop notification delivery continues to accept only app-relative paths.
- iOS Phase 0 can implement route parsing against the shared contract.

## Shared Notification Semantics

Status: local pre-iOS contract slice complete. `src/app/notifications` now contains
the shared notification/badge summary contract and the
`SystemNotificationRequest` normalization contract used by platform
notification delivery. The portable contract is documented in
`docs/synara-notification-contract.md`, with summary schema fixtures in
`docs/contracts/`.

Resolved local result:

- Badge count and notification summary derivation are pure tested helpers.
- Platform notification delivery uses the shared `SystemNotificationRequest`
  model and strips unsafe routes.
- The runtime updates desktop badge and tray state from the shared summary.

Target shape:

```text
src/app/notifications/
  notificationSummary.ts
  notificationEligibility.ts
  notificationEvents.ts
  reminderScheduler.ts
  badgeSummary.ts
  __tests__/
```

Completed local tasks:

1. Define a `NotificationSummary` contract for unread, highlights, invites,
   Later, notification inbox, and agent approvals.
2. Define a `SystemNotificationRequest` contract with title, body, route,
   privacy level, and sound policy.
3. Extract badge count derivation into a pure tested function.
4. Keep message/invite/reminder eligibility on the existing runtime path while
   routing portable payloads through the shared request model.
5. Route delivery through platform adapters.
6. Add fixtures that can be reused by the future iOS implementation.

Acceptance criteria:

- Badge counts are identical before and after refactor.
- Development fallback and Tauri notification delivery use the same request
  model.
- Later reminders still mark `remindedAt` exactly once.
- The iOS project spec can reference the same summary and request contracts.

## Shared Later Account Data

Status: local pre-iOS contract slice complete. `docs/synara-later-contract.md`
defines the `in.synara.later` account-data contract. `docs/contracts/`
contains the canonical v1 JSON Schema and fixtures for valid, invalid, and
legacy-normalized payloads.

Resolved local result:

- Later account data uses documented room/event/thread anchors.
- The runtime strips legacy plaintext preview fields during normalization.
- iOS can decode the same fixtures without learning desktop UI state or storing
  decrypted message previews.

Completed local tasks:

1. Keep `src/app/utils/later.ts` synchronized with the documented v1 schema.
2. Add runtime conformance tests against the contract fixtures.
3. Defer generated-versus-manual Swift type choice to iOS Phase 0.
4. Route Later taps through the shared route contract.
5. Keep reminder notification behavior aligned with the shared notification
   contract.

Acceptance criteria:

- Later account data stores only anchors, kind, and timestamps.
- Legacy plaintext preview fields are stripped during normalization.
- Malformed items fail closed.
- Future iOS Later reads can use the same fixtures as desktop-runtime tests.

## Session And Storage Direction

Status: local pre-iOS implementation complete. The current fallback `localStorage`
session behavior now sits behind a `SessionStore` interface and a
`createLocalStorageSessionStore` adapter. Settings now sit behind a
`SettingsStore` contract with separate shared and desktop-platform settings.
The desktop secure-secret storage decision is recorded in the parent desktop
repo at `../../docs/desktop-secure-secret-storage-plan.md`.

The first implementation step is now in place: startup resolves an async session
bootstrap before the router mounts, then serves the chosen session through a
sync cache for existing route and Matrix-client startup paths.

The second interface step is also in place: the runtime platform session store
now exposes secure-store status, read, write, and remove operations. The desktop
shell has matching scoped commands and a native credential adapter. macOS uses
Keychain. Linux treats Secret Service as the persistent backend and keeps
keyutils session-scoped until we explicitly accept that persistence tradeoff.

The migration/write step is now in place: legacy fallback sessions migrate to
native credentials only after Matrix client initialization succeeds. Login and
registration write native credentials first when available, and logout/data
reset clear both native and legacy session locations.

Resolved local result:

- Legacy `localStorage` access-token fields remain only as the intentional
  fallback path when native credentials are unavailable or fail.
- Settings reads preserve legacy compatibility while writes split shared app
  settings from desktop platform settings.

Target shape:

- `SessionStore` interface with browser fallback and desktop native adapter.
- `SettingsStore` split into shared settings and platform settings.
- Desktop secure-secret storage migration with native-first writes and legacy
  fallback only when needed.

Current settings migration behavior:

- Existing `settings` blobs are read as the legacy merged schema.
- Desktop shortcut fields are exposed through desktop platform settings.
- New writes store shared app settings in `settings` and desktop shortcut
  settings in `platformSettings`.
- The merged `settingsAtom` remains for compatibility while new platform-only
  consumers use `desktopPlatformSettingsAtom`.

Completed local tasks:

1. Add `SessionStore` and `SettingsStore` interfaces.
2. Move current `localStorage` usage behind those interfaces.
3. Split settings into:
   - shared app settings.
   - desktop platform settings.
   - future iOS platform settings.
4. Evaluate and implement OS credential storage for desktop secrets.
5. Document migration behavior for existing users.

Acceptance criteria:

- Existing users keep their settings after migration.
- Access-token handling is centralized and testable.
- Desktop-only shortcut fields are not part of shared settings.
- The iOS spec has a compatible storage model for Keychain-backed sessions.

## Public Web Retirement

Status: complete for local pre-iOS prep. The desktop package still uses a
React/Vite/WebView runtime, but standalone browser distribution is no longer a
supported product surface. Netlify, Docker, nginx, caddy, and public web deploy
workflows have been removed from the active runtime path; Vite remains only as
the desktop-shell development/runtime asset pipeline.

Completed local tasks:

1. Remove self-hosting language from active runtime docs.
2. Remove Netlify, Docker, nginx, and caddy deployment docs from the active
   app-runtime path.
3. Keep development server instructions because Tauri still uses Vite.
4. Rename docs and comments from "web client" to "app runtime" where the code is
   no longer a public web product.
5. Update product copy to "native-app-first".
6. Treat browser-only bugs as development-runtime issues unless they also affect
   the packaged desktop app.

Acceptance criteria:

- README no longer advertises self-hosted browser deployment.
- Desktop docs describe the runtime as implementation detail.
- No build script needed by the desktop shell is removed accidentally.
- Release checklists target packaged macOS/Linux apps, not standalone browser
  deployment.

## Native Desktop Rewrite Option

This becomes the recommended pre-iOS path only if the decision spike shows that
it simplifies the primary macOS/Linux channels without derailing parity.

Possible target:

- macOS: SwiftUI app sharing concepts and possibly some Swift packages with iOS.
- Linux: either keep Tauri, build a GTK/libadwaita app, or evaluate another Rust
  UI stack.

Prerequisites before reconsidering:

- iOS native app has shipped or reached TestFlight-quality core messaging.
- Shared contracts are stable.
- Matrix Rust SDK usage is proven in production-like iOS flows.
- Desktop Tauri limitations are clearly blocking product quality.

Acceptance criteria for starting the rewrite:

- Written ADR comparing Tauri desktop, native macOS plus Tauri Linux, and fully
  native desktop alternatives.
- Linux toolkit and packaging decision made.
- Migration plan for all existing desktop features.
- Resourcing accepted for a multi-month rewrite.

## Recommended Sequence

1. Finish canonical repo consolidation. Completed locally.
2. Refactor desktop APIs into platform APIs. Completed locally.
3. Extract route, notification, and badge contracts. Completed locally.
4. Split shared versus platform settings. Completed locally.
5. Add session/storage abstraction. Completed locally.
6. Validate packaged macOS and Linux desktop builds. Package CI complete;
   human smoke validation remains.
7. Start iOS Phase 0 and Matrix SDK spike.
8. Revisit native desktop architecture only after iOS and shared Rust-domain
   evidence exists.
