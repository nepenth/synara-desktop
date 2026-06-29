# Synara App Runtime

Synara is a native-app-first Matrix client focused on fast secure conversations,
desktop polish, Linux support, and agent workflows.

This package contains the React/Vite runtime used by the Synara desktop app. The
former public browser-client and self-hosting positioning has been retired. The
app product channels are native packages:

- macOS and Linux through the Tauri desktop shell in `../`.
- iOS through the planned native SwiftUI app.

The runtime can still be served locally during development because Tauri uses
Vite for fast iteration, but standalone browser releases are not supported.
Browser-only behavior is treated as development-runtime behavior unless it also
affects packaged native apps.

## Development

Install dependencies from this directory:

```sh
npm ci
```

Run the local runtime for desktop-shell development:

```sh
npm start
```

Build the runtime assets consumed by the desktop shell:

```sh
npm run build
```

Run the paired desktop app from the parent `synara-desktop` project:

```sh
cd ..
npm ci
npm run tauri dev
```

## Direction

- [`../CODEBASE_KNOWLEDGE_BASE.md`](../CODEBASE_KNOWLEDGE_BASE.md) is the
  monorepo onboarding map (architecture, features, contracts, and expansion
  guidance). Start there for new work in this repository.
- `docs/synara-namespaces.md` documents Synara Matrix account-data and event
  metadata contracts.
- `docs/synara-ios-app-store-plan.md` and
  `docs/synara-ios-project-spec.md` describe the native iOS direction.
- `docs/native-first-consolidation-plan.md` tracks the pre-iOS simplification
  work needed to keep macOS, Linux, and iOS aligned.

## Validation

Focused checks:

```sh
npm run test:modernization
npm run test:timeline-performance
npm run typecheck:modernization
```

Desktop wrapper validation lives in the parent `synara-desktop` repository.
