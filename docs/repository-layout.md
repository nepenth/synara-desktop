# Repository Layout

Reviewed: 2026-05-26

The canonical local workspace for desktop client work is:

```text
/Users/nepenthe/git_repos/synara_project/synara-desktop
```

This repository owns the macOS/Linux Tauri shell. Its app runtime is the
`synara` submodule:

```text
/Users/nepenthe/git_repos/synara_project/synara-desktop/synara
```

Use that nested submodule for Synara runtime/client changes. The submodule
tracks `https://github.com/nepenth/synara.git` on `main`, and the parent desktop
repo records the exact runtime commit through the submodule pointer.

Do not use a sibling checkout at:

```text
/Users/nepenthe/git_repos/synara_project/synara
```

That path is not the canonical workspace for this desktop project. If a
standalone checkout exists for archival or comparison, sync or archive it before
starting new work so fixes are not split across two local copies.

Safe push order for desktop work:

1. Push `synara-desktop/synara`.
2. Push `synara-desktop`.

This ensures the parent desktop repo never points at an unpublished submodule
commit.
