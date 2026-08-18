# Synara Desktop Runtime

This package is the React/Vite user interface embedded in Synara's Tauri shell
for macOS and Linux. It is not a standalone browser product and must not grow a
browser fallback for native security, persistence, or Matrix operations.

Matrix lifecycle and domain behavior are owned by the repository's shared Rust
core. This package consumes that behavior through the platform and Matrix
facades under `src/app/platform/` and `src/app/matrix/`. Presentation state,
routing, the Slate composer, and timeline virtualization remain in React.

The native SwiftUI client lives in `../synara-ios/` and consumes the same shared
core through generated Swift bindings.

## Development

Install package dependencies:

```sh
npm ci
```

Run Vite for development of the embedded runtime:

```sh
npm start
```

Build the runtime assets:

```sh
npm run build
```

Run the complete desktop application from the repository root:

```sh
cd ..
npm ci
npm run tauri dev
```

The Vite server is a development transport for Tauri. A successful browser
render does not establish that a native product flow works.

## Validation

```sh
npm run typecheck
npm run test:modernization
npm run test:browser:timeline
npm run check:eslint
npm run check:prettier
```

Repository, Rust, package, and release validation runs from the parent. See the
[root README](../README.md) and [build and release runbook](../docs/build-and-release.md).

## Documentation

- [Codebase knowledge base](../CODEBASE_KNOWLEDGE_BASE.md)
- [Shared contracts](docs/synara-contracts.md)
- [Synara namespaces](docs/synara-namespaces.md)
- [Documentation index](../docs/README.md)

Plans and audit records under `../docs/matrix-rust-sdk/` describe the completed
JavaScript-to-Rust Matrix migration. They are historical evidence unless a file
explicitly identifies itself as a current operating guide.
