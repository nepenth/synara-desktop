# Native-First Architecture Spike

Reviewed: 2026-05-25

Status: accepted for planning

> **Historical decision record.** Its sequencing decision was implemented:
> desktop retained Tauri/React, iOS shipped as SwiftUI, and both now consume the
> shared Rust core. Use [the codebase knowledge base](../CODEBASE_KNOWLEDGE_BASE.md)
> for current architecture; use this spike for the original trade-off analysis.

Language-boundary follow-up (what may be written in Rust, including the
continued rejection of a Slint/Dioxus/egui desktop rewrite):
[ADR 0004](adr/0004-rust-language-boundaries.md).

## Decision

Keep Tauri as the internal macOS/Linux runtime while building the first native
iOS app in SwiftUI.

Do not start a full native desktop rewrite before iOS. The next engineering
phase should instead extract platform contracts from the current runtime:
notifications, badge counts, files, links, shortcuts, agent actions, settings,
and session storage. Those contracts should be the bridge between the current
desktop runtime and the native iOS implementation.

This is not a permanent endorsement of Tauri for every future product surface.
It is a sequencing decision: the current codebase is much closer to a polished
macOS/Linux app plus native iOS than it is to one shared Rust-native UI across
macOS, iOS, and Linux.

## Local Findings

The current architecture has three distinct layers:

1. The app runtime in `synara/` owns Matrix state, timeline rendering,
   composer behavior, room navigation, settings, Later, notifications,
   agent cards, calls, media, and account-data semantics.
2. The Tauri shell in `src-tauri/` owns native windows, tray, global shortcuts,
   badge count, file drops, native notifications, clipboard, external URL
   opening, packaging, signing, and Linux/macOS integration.
3. The current storage/session layer is still browser-shaped: Matrix sync and
   crypto stores use IndexedDB, session fallback uses localStorage, and the
   service worker handles authenticated Matrix media.

Observed coupling:

- `matrix-js-sdk` appears in 209 runtime files.
- Browser/session primitives appear in 29 runtime files.
- Direct desktop bridge usage is concentrated in 10 runtime files.
- The Tauri command surface is bounded to 14 user-facing `desktop_*` commands.

Interpretation:

- Replacing Tauri alone does not remove the browser-shaped app layer.
- A full native desktop rewrite would mean rebuilding the feature UI and a large
  Matrix integration surface before iOS work can start.
- Platform API abstraction is cheap and high-leverage because desktop bridge
  usage is concentrated.
- Matrix SDK migration is the deeper strategic question, not the shell toolkit.

## External Findings

Primary references:

- [Tauri 2.0](https://v2.tauri.app/) supports Linux, macOS, Windows, Android,
  and iOS from one codebase, with JavaScript frontend logic and Rust application
  logic.
- [Tauri architecture](https://v2.tauri.app/concept/architecture/) is explicitly
  HTML rendered in a WebView plus Rust APIs bridged by message passing. Tauri
  does not eliminate WebView; it makes WebView an app shell.
- [Tauri WebView versions](https://v2.tauri.app/reference/webview-versions/)
  confirm macOS and iOS use the OS-provided WebKit/WKWebView.
- [Matrix Rust SDK](https://github.com/matrix-org/matrix-rust-sdk) is described
  by Matrix as production ready and used by Element X on iOS and Android.
- [Matrix Rust components for Swift](https://github.com/matrix-org/matrix-rust-components-swift)
  provide Swift Package Manager distribution, but the Swift components currently
  warn that their API is unstable.
- [SwiftUI](https://developer.apple.com/documentation/technologyoverviews/swiftui)
  is Apple's first-party path for new apps across Apple platforms.
- [Dioxus platform support](https://dioxuslabs.com/learn/0.7/guides/platforms/)
  is Rust-first and cross-platform, but desktop and mobile use the same WebView
  rendering model. It does not solve the "drop WebView" goal.
- [Slint desktop support](https://docs.slint.dev/latest/docs/slint/guide/platforms/desktop/)
  covers macOS and Linux, and
  [Slint iOS support](https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/ios/)
  exists for Rust applications. Adopting it would still mean a Rust-only UI
  rewrite and a custom Xcode/cargo build path rather than the conservative
  first-party SwiftUI route for this project.

## Options

### Option A: Keep Tauri For macOS/Linux, Build Native SwiftUI iOS

Summary:

- Current desktop app stays on Tauri.
- iOS is a new SwiftUI app.
- Shared behavior is expressed as contracts, schemas, fixtures, and tests.
- Matrix Rust SDK is adopted first on iOS, then reconsidered for desktop.

Pros:

- Preserves the working macOS/Linux app.
- Keeps Linux quality on the known Tauri packaging path.
- Lets iOS use native Apple UI, Keychain, APNs, background modes, share sheet,
  file providers, and App Store tooling directly.
- Makes the next work incremental: platform APIs and shared contracts.

Cons:

- Two UI implementations long term.
- Desktop remains WebView-based.
- Feature parity requires discipline around shared contracts and fixtures.

Assessment: recommended.

### Option B: SwiftUI For iOS And macOS, Keep Tauri Or Another UI For Linux

Summary:

- Apple platforms move native.
- Linux remains Tauri or gets a separate native toolkit.

Pros:

- Strong Apple UX and shared SwiftUI patterns across iOS/macOS.
- Easier Apple-specific features than Tauri.

Cons:

- Linux diverges immediately.
- Current macOS app work is largely rebuilt before iOS ships.
- Three platform surfaces still exist: SwiftUI iOS, SwiftUI macOS, Linux.

Assessment: possible future direction, not the next phase.

### Option C: Shared Rust-Native UI Toolkit For macOS, Linux, And iOS

Summary:

- Rewrite the UI in a Rust-native toolkit such as Slint, egui/eframe, Makepad,
  iced, Xilem, or similar.

Pros:

- Potentially one language and one app UI stack.
- Natural path to a Rust Matrix/domain core.

Cons:

- No conservative, proven App Store-grade choice currently beats SwiftUI for iOS
  while also giving first-class Linux desktop UX.
- Some options still use WebView, some are not mobile-ready enough, and some
  would require custom platform integrations for accessibility, text input,
  notifications, share sheets, background work, deep links, and packaging.
- Existing React/Matrix JS UI would be largely discarded.

Assessment: not recommended now. Revisit only after a focused proof of concept
demonstrates timeline, composer, encrypted media, accessibility, and packaging
quality on all three target platforms.

### Option D: Move Matrix/Domain Logic To Rust, Keep Current Tauri UI

Summary:

- Keep current React/Tauri UI.
- Gradually replace Matrix JS SDK usage with a Rust-backed local core exposed
  to the runtime.

Pros:

- Could unify Matrix behavior with the future iOS Rust SDK path.
- Reduces browser storage/session dependence over time.
- Avoids immediate UI rewrite.

Cons:

- Large integration surface: sync, timeline, crypto, media, account data,
  notifications, search, rooms, spaces, calls, and custom Synara semantics.
- Requires a bridge model between Rust state and React UI.

Assessment: promising after iOS Phase 0 proves the Matrix Rust SDK shape.

## Recommendation

Proceed in this order:

1. Keep Tauri as internal macOS/Linux runtime.
2. Build platform contracts in the current runtime.
3. Build iOS Phase 0 as native SwiftUI with Matrix Rust SDK Swift components.
4. Use iOS implementation experience to decide whether Matrix/domain logic
   should migrate into a shared Rust core for desktop later.
5. Revisit native desktop rewrite only with evidence that it reduces total
   product complexity without regressing Linux.

## Next Phase: Platform Contracts

Scope:

- Add `synara/src/app/platform/`.
- Move bridge detection, capabilities, notifications, badges, file save/read,
  external URL opening, agent actions, diagnostics, and shortcuts behind
  platform APIs.
- Keep `utils/desktop.ts` as a compatibility wrapper during migration.
- Extract notification and badge request contracts into pure tested modules.
- Split shared settings from desktop-only platform settings.
- Add a `SessionStore` interface before changing token storage.

Acceptance criteria:

- UI code does not call `window.__SYNARA_DESKTOP__` or `__TAURI_INTERNALS__`
  directly.
- Notification, badge, and agent-action behavior remains covered by tests.
- Desktop-only shortcuts and tray capabilities are isolated from shared app
  settings.
- Development browser execution is still possible for Tauri iteration but is
  labeled as a development harness, not a product channel.
- The iOS project spec can reference the same notification, badge, route,
  settings, session, and agent-action contracts.

## iOS Phase 0 Entry Criteria

Start iOS scaffolding after the platform-contract phase has:

- A stable route contract for room/event/thread/Later/notification navigation.
- A `SystemNotificationRequest` contract.
- A `BadgeSummary` contract.
- Shared account-data schemas and fixtures for Later, room notes, favorites,
  folders, unread anchors, and agent cards.
- A session/storage abstraction design mapping desktop storage to future iOS
  Keychain-backed storage.

## Revisit Criteria

Reconsider a native desktop rewrite only if at least one is true:

- Tauri/WebView causes a reproducible macOS or Linux product-quality blocker
  that cannot be fixed inside the current shell.
- Matrix Rust SDK iOS work proves a shared Rust domain core can replace enough
  Matrix JS SDK behavior to reduce desktop complexity.
- A Rust-native UI proof of concept demonstrates App Store-grade iOS,
  polished macOS, and high-quality Linux behavior for the hard parts: timeline
  virtualization, rich composer, encrypted media, file drop/save, notifications,
  accessibility, deep links, and packaging.
- Linux has a concrete native toolkit and packaging plan that is lower risk
  than the current Tauri path.

## Non-Goals

- Do not attempt Tauri mobile for iOS as the first App Store target.
- Do not rewrite macOS before iOS just to remove WebView.
- Do not adopt a Rust UI toolkit without a proof of concept covering the hard
  chat-client surfaces.
- Do not move session tokens or crypto stores until migration and rollback
  behavior is documented.
