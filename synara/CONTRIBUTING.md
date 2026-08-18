# Contributing To Synara

Use descriptive issue and pull-request titles, search for existing reports,
and keep each change focused enough to review and validate. Discuss substantial
new product scope with the maintainers before implementation.

## Legal

By contributing, you confirm that you authored the contribution or have the
necessary rights to submit it under the repository's AGPL-3.0-only license.
Preserve required copyright, license, and attribution notices.

## Architecture

Read the [codebase knowledge base](../CODEBASE_KNOWLEDGE_BASE.md) and relevant
ADRs before changing platform or Matrix boundaries.

- Matrix lifecycle and domain behavior belongs in the shared Rust core.
- React owns desktop presentation and reaches native behavior through the
  platform and Matrix facades.
- SwiftUI owns iOS presentation and Apple platform integration.
- Cross-platform behavior changes require contract and fixture updates.
- The Vite runtime is not a standalone browser product.
- Do not add credentials, private infrastructure identifiers, personal paths,
  or live account data to code, fixtures, documentation, logs, or screenshots.

## Validation

Run the gates appropriate to the changed surface. The baseline is:

```sh
cd ..
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

Follow [the build and release runbook](../docs/build-and-release.md) for package,
simulator, signing, and release validation.

## References

- [Matrix Rust SDK](https://github.com/matrix-org/matrix-rust-sdk)
- [Tauri documentation](https://v2.tauri.app/)
- [SwiftUI documentation](https://developer.apple.com/documentation/swiftui)
- [React documentation](https://react.dev/)
