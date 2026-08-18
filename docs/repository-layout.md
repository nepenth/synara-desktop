# Repository Layout

Reviewed: 2026-08-18

This repository owns all supported Synara clients and their shared core:

- the shared Rust core in `crates/synara-core/`;
- the macOS/Linux Tauri shell in `src-tauri/`;
- the embedded desktop runtime in `synara/`;
- the native SwiftUI client in `synara-ios/`.

`synara/` is now a normal tracked directory, not a Git submodule. Fresh clones
of `synara-desktop` do not need `--recursive` or any `git submodule` commands.
The former standalone runtime repository is not required for fresh clones,
builds, tests, or releases. Do not split product changes into a second runtime
repository.

Run this before committing repository-structure updates:

```sh
npm run check:repo-layout
```

CI runs the same check.

Repository layout acceptance criteria:

- `.gitmodules` does not exist.
- `synara/` is tracked directly by the parent repository.
- `synara/` does not contain nested Git metadata.
- `synara/` does not contain nested GitHub workflow automation.
- Fresh clones, local builds, and CI do not depend on another repository.
