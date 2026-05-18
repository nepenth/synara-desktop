# Changelog

## 1.0.4 - 2026-05-18

- Fixed room timeline viewport restoration when leaving and returning to a channel after scrolling into history.
- Added explicit saved-anchor restore handling so historical restores load around the saved event before normal pagination resumes.
- Prevented initial bottom pinning and generic pagination from overwriting an in-progress historical viewport restore.
- Updated displayed client version to match the packaged app version.
