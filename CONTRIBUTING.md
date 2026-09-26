# Contributing To Synara

Use descriptive issue and pull-request titles, search for existing reports,
and keep each change focused enough to review and validate. Discuss substantial
new product scope with the maintainers before implementation.

Public downloads are macOS and x86_64 Linux. The iOS app is internal TestFlight
only. Voice and video calling are not supported.

## Legal

By contributing, you confirm that you authored the contribution or have the
necessary rights to submit it under the repository's AGPL-3.0-only license,
including the Affero network-use source offer. Contributions stay under that
license. Synara is a derivative of Cinny. Preserve required copyright, license,
and attribution notices in `NOTICE`.

## Architecture

Read the [shared native core ADR](docs/adr/0003-shared-native-rust-core.md) and
the [language-boundary ADR](docs/adr/0004-rust-language-boundaries.md) before
changing platform or Matrix boundaries.

- Matrix lifecycle and domain behavior belongs in the shared Rust core.
- React owns desktop presentation and reaches native behavior through the
  platform and Matrix facades.
- SwiftUI owns iOS presentation and Apple platform integration.
- Cross-platform behavior changes require contract and fixture updates.
- The Vite runtime is not a standalone browser product.
- Do not add credentials, private infrastructure identifiers, personal paths,
  or live account data to code, fixtures, documentation, logs, or screenshots.
- `docs/matrix-rust-sdk/` and `docs/shared-native-core/` are historical
  migration records. Do not treat them as the current product.

## Validation

Source builds use Node.js 24.13.1 (`.node-version`) and Rust 1.96
(`rust-toolchain.toml`).

Run the gates appropriate to the changed surface. The baseline is:

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
```

A green pull-request check is not the full iOS simulator or desktop package
matrix. Those jobs run when the changed paths require them, or when a
maintainer adds `needs-ios`, `needs-ios-ui`, or `needs-package`. Fork pull
requests do not receive signing secrets.

Follow [the build and release runbook](docs/build-and-release.md) for package,
simulator, signing, and release validation.

## References

- [Matrix Rust SDK](https://github.com/matrix-org/matrix-rust-sdk)
- [Tauri documentation](https://v2.tauri.app/)
- [SwiftUI documentation](https://developer.apple.com/documentation/swiftui)
- [React documentation](https://react.dev/)
