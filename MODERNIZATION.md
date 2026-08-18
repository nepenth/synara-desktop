# Desktop Modernization

Synara's macOS and Linux clients use a Tauri 2 shell around the embedded React
runtime. Matrix lifecycle and domain behavior are owned by the shared Rust
core; React owns presentation, routing, composer behavior, and timeline
virtualization; Tauri owns operating-system integration.

The standalone browser product and JavaScript Matrix backend have been retired.
The current architecture is documented in
[the codebase knowledge base](CODEBASE_KNOWLEDGE_BASE.md).

## Native Scope

- Application windows, tray/menu behavior, and close-to-tray lifecycle.
- Native notification permissions, delivery, and click activation.
- Dock/taskbar badges and configurable global shortcuts.
- Keychain or Secret Service session persistence.
- Native file access, downloads, clipboard media, and external URLs.
- Structured agent-action dispatch with native sanitization.
- Platform diagnostics, build identity, update checks, and release hardening.
- Signed macOS update installation; Linux package-manager update guidance.

Feature code reaches native behavior through the platform facade and reaches
Matrix through the shared-core facade. Direct ad hoc Tauri calls or parallel
Matrix clients are boundary violations.

## Development

Install both JavaScript dependency trees:

```sh
npm ci
npm --prefix synara ci
```

Run the complete desktop application:

```sh
npm run tauri dev
```

Build the embedded runtime or desktop package:

```sh
npm run build:runtime
npm run tauri build
```

For an ad-hoc local macOS application bundle:

```sh
npm run tauri build -- --bundles app
```

Production builds use protected Developer ID, notarization, and updater
credentials through the exact-tag release workflow. Do not add signing values
to commands, documentation, or tracked configuration.

## Validation

```sh
npm run check:repo-layout
npm run check:versions
npm run check:docs
npm run check:matrix-boundaries
npm run check:quality-gates
npm --prefix synara run typecheck
npm --prefix synara run test:modernization
npm --prefix synara run check:eslint
npm --prefix synara run check:prettier
cargo test --workspace --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

Package, signing, updater, and smoke instructions live in
[the build and release runbook](docs/build-and-release.md).
