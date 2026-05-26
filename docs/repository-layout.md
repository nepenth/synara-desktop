# Repository Layout

Reviewed: 2026-05-26

The canonical local workspace for desktop client work is:

```text
/Users/nepenthe/git_repos/synara_project/synara-desktop
```

This repository owns both:

- the macOS/Linux Tauri shell in `src-tauri/`
- the Synara app runtime in `synara/`

`synara/` is now a normal tracked directory, not a Git submodule. Fresh clones
of `synara-desktop` do not need `--recursive` or any `git submodule` commands.
The former standalone GitHub repository `nepenth/synara` was archived on
2026-05-26 after `synara-desktop` package CI passed without it.

Do not use a sibling checkout at:

```text
/Users/nepenthe/git_repos/synara_project/synara
```

That path is not the canonical workspace for this desktop project. If a
standalone checkout exists for archival or comparison, archive or delete it
before starting new work so fixes are not split across two local copies.

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
- No active sibling checkout exists at `/Users/nepenthe/git_repos/synara_project/synara`.
- `nepenth/synara` is not required for fresh clones, local builds, or package
  smoke workflows.
