# Changelog

## Unreleased

- Hardened desktop composer drag/drop detection so file payloads that expose `files` or file items without a `Files` type marker still activate the upload drop zone.
- Improved desktop composer clipboard image paste so image-like native clipboard payloads are uploaded before rich-text insertion, with a rich-text fallback when the native image read yields no file.
- Routed desktop external link opens through the native `desktop_open_external_url` bridge across Hermes cards, profile/server actions, account-management links, auth/info anchors, and agent actions without unsafe desktop `window.open` fallback.
- Added a cross-platform timeline open-focus contract and expanded iOS focus-policy coverage for read-marker and jump-latest behavior.
- Added a desktop timeline viewport restore policy so unread rooms and stale historical anchors no longer override live/read-marker opening.
- Added Codex-Orchestrator-v2 persistent harness artifacts for production-readiness tracking.
- Expanded the living production-readiness backlog to cover the full KB Section 7 recommendation set and reconciliation constraints.
- Fixed existing Prettier drift in timeline, notification, app-link, and timeline lifecycle files so the formatting gate passes.

## 1.0.4 - 2026-05-18

- Fixed room timeline viewport restoration when leaving and returning to a channel after scrolling into history.
- Added explicit saved-anchor restore handling so historical restores load around the saved event before normal pagination resumes.
- Prevented initial bottom pinning and generic pagination from overwriting an in-progress historical viewport restore.
- Updated displayed client version to match the packaged app version.
