# Repository Layout

Reviewed: 2026-05-26

The canonical local workspace for desktop client work is:

```text
/Users/example/git_repos/synara_project/synara-desktop
```

This repository owns the macOS/Linux Tauri shell. Its app runtime is the
`synara` submodule:

```text
/Users/example/git_repos/synara_project/synara-desktop/synara
```

Use that nested submodule for Synara runtime/client changes. The submodule
tracks `https://github.com/nepenth/synara.git` on `main`, and the parent desktop
repo records the exact runtime commit through the submodule pointer.

This is still a real submodule relationship. Do not delete
`https://github.com/nepenth/synara.git` while `.gitmodules` points at it. A
fresh clone of `synara-desktop` needs that remote to populate
`synara-desktop/synara`.

The submodule shape is acceptable for now because it keeps the app runtime and
desktop shell as independently reviewable projects while the iOS architecture
is still being settled. If this stops helping, the next deliberate step is to
absorb `synara/` into `synara-desktop` as a normal tracked directory and remove
the submodule metadata in the same commit.

Run this before committing repository-structure or submodule updates:

```sh
npm run check:repo-layout
```

CI runs the same check.

Do not use a sibling checkout at:

```text
/Users/example/git_repos/synara_project/synara
```

That path is not the canonical workspace for this desktop project. If a
standalone checkout exists for archival or comparison, sync or archive it before
starting new work so fixes are not split across two local copies.

Safe push order for desktop work:

1. Push `synara-desktop/synara`.
2. Push `synara-desktop`.

This ensures the parent desktop repo never points at an unpublished submodule
commit.
