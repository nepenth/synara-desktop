# ADR 0001: iOS Repository Layout

Originally accepted: 2026-05-26.

Last reviewed: 2026-09-01.

Status: accepted and implemented.

## Decision

The native iOS project lives inside the canonical `synara-desktop` monorepo at
`synara-ios/`:

```text
synara-desktop/
  crates/synara-core/
  src-tauri/
  synara/
  synara-ios/
```

The repository owns application source, shared contracts, generated-binding
inputs, CI policy, versioning, and release gates for macOS, Linux, and iOS.
Generated XCFrameworks and Swift bindings are build outputs, not a second source
repository.

## Current evidence

- `synara-ios/Synara.xcodeproj` and the `Synara` shared scheme are tracked here.
- `crates/synara-core-bindgen/` generates the project-owned Swift bindings.
- Cross-client contract schemas remain under `synara/docs/contracts/`.
- `synara/` is a normal tracked directory, not a submodule, and the repository
  layout check enforces the canonical topology.
- Root version and release workflows coordinate all supported clients.

Current build commands belong in the root [README](../../README.md) and
[build/release guide](../build-and-release.md), not in this ADR; those living
documents may evolve without changing the repository decision.

## Rationale

- One repository prevents contract, version, CI, and release-policy drift.
- iOS tests and binding generation can consume local source and fixtures
  without a package registry, submodule, or sibling checkout.
- Shared-Core changes can be reviewed atomically with both presenters and their
  adapters.
- A split repository would require a demonstrated access-control or independent
  release-cadence need, plus an explicit replacement ADR.

## Rejected alternatives

- **Separate iOS repository:** rejected because it creates immediate
  cross-repository synchronization and release risk.
- **Sibling checkout:** rejected because it is easy to confuse with the
  canonical product tree and makes automation non-deterministic.
- **Submodule or subtree for contracts:** rejected because ordinary tracked
  files are simpler while all consumers live in this monorepo.

## Consequences

- Changes to shared contracts or Core APIs must consider desktop and iOS in one
  repository review.
- Platform-specific source may remain isolated under its platform directory;
  monorepo ownership does not imply identical UI implementations.
- ADR 0003 adds the shared Rust Core within this layout; it does not supersede
  the layout decision.
