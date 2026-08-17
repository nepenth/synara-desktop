# Synara Desktop

Synara Desktop is a Tauri-based native desktop Matrix client focused on fast, secure conversations, smooth desktop UX, Linux support, and agent workflows.

## Documentation

**Start here for codebase orientation:** [`CODEBASE_KNOWLEDGE_BASE.md`](CODEBASE_KNOWLEDGE_BASE.md) — architecture, feature inventory, in-progress work, critical file paths, and expansion guidance. Review it before starting new tasks in this repository (including AI-assisted work).

| Area                          | Entry points                                                                                                                                                                                                                                     |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Monorepo layout               | [`docs/repository-layout.md`](docs/repository-layout.md)                                                                                                                                                                                         |
| Desktop shell & native bridge | [`docs/desktop-modernization.md`](docs/desktop-modernization.md), [`MODERNIZATION.md`](MODERNIZATION.md)                                                                                                                                         |
| Architecture decisions        | [`docs/adr/`](docs/adr/)                                                                                                                                                                                                                         |
| Desktop validation & release  | [`docs/build-and-release.md`](docs/build-and-release.md), [`docs/production-smoke-checklist.md`](docs/production-smoke-checklist.md), [`docs/desktop-validation-status.md`](docs/desktop-validation-status.md), [`docs/linux.md`](docs/linux.md) |
| Synapse audit guidance        | [`docs/synapse-production-audit-runbook.md`](docs/synapse-production-audit-runbook.md)                                                                                                                                                           |
| js-sdk → rust-sdk migration     | [`docs/matrix-rust-sdk/`](docs/matrix-rust-sdk/) (plans, ADRs, capability reviews, burn-down tracking)                                                                                                               |
| Shared native core plan        | [`docs/adr/0003-shared-native-rust-core.md`](docs/adr/0003-shared-native-rust-core.md) + [`docs/shared-native-core/`](docs/shared-native-core/)                                                                                                 |
| Rust language boundaries       | [`docs/adr/0004-rust-language-boundaries.md`](docs/adr/0004-rust-language-boundaries.md) — can/should rubric; stay-put list; no UI or CI rewrite                                                                                            |
| Timeline reliability          | [`docs/timeline-room-state-reliability-contract.md`](docs/timeline-room-state-reliability-contract.md)                                                                                                                                           |
| Shared Matrix contracts       | [`synara/docs/synara-contracts.md`](synara/docs/synara-contracts.md), [`synara/docs/contracts/`](synara/docs/contracts/)                                                                                                                         |
| App runtime (React/Vite)      | [`synara/README.md`](synara/README.md), [`synara/docs/`](synara/docs/)                                                                                                                                                                           |
| iOS app                       | [`synara-ios/README.md`](synara-ios/README.md), [`synara-ios/docs/`](synara-ios/docs/)                                                                                                                                                           |

## Download

Installers for macOS and Linux are built from this project. The in-app updater
is currently disabled until this project has a stable signed release metadata
channel. Local builds do not contact update servers.

| Operating System | Download          |
| ---------------- | ----------------- |
| macOS            | Build from source |
| Linux            | Build from source |

Windows packaging is not part of the current supported release matrix.
Published macOS releases require Developer ID signing and notarization in CI.
The release workflow fails if the required Apple distribution secrets are not
configured. Updater metadata remains disabled until this project has a stable
signed release metadata channel.
Published release jobs also run `npm run check:release-updater -- --require-enabled`;
until the Tauri updater plugin, signed metadata channel, and updater signing
secrets are configured, release artifact builds intentionally fail before
packaging.

### Platform support — session persistence

Native Matrix session persistence uses the platform credential store exposed by
the desktop bridge. Supported backends:

| Platform | Native session store | Persists across restarts |
| -------- | -------------------- | ------------------------ |
| macOS    | Keychain             | Yes                      |
| Linux    | Secret Service       | Yes (when available)     |
| Windows  | Not supported        | No                       |

Windows builds do **not** persist sessions to a native credential store. The
app falls back to the in-app session store on Windows; this is not equivalent
to macOS Keychain or Linux Secret Service security. Developer Tools reports
the native session store as unavailable on Windows.

## Local Development

First, set up Rust, Node.js, and build tools by following the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

Edit the repository root `config.json` only. `scripts/build-runtime.mjs` and
`npm run tauri` keep `synara/config.json` synchronized before builds.

Then set up development locally:

- `git clone https://github.com/nepenth/synara-desktop.git`
- `cd synara-desktop/synara`
- `npm ci`
- `cd ..`
- `npm ci`

To build the app locally, run:

- `npm run tauri build`

On macOS, local unsigned smoke-test builds should use ad-hoc signing:

- `npm run tauri build -- --bundles app`

Published macOS releases require these GitHub Actions secrets:

- `APPLE_CERTIFICATE_BASE64`: base64-encoded Developer ID Application `.p12`.
- `APPLE_CERTIFICATE_PASSWORD`: password for that `.p12`.
- `APPLE_SIGNING_IDENTITY`: codesigning identity, for example `Developer ID Application: Example, Inc. (TEAMID)`.
- `APPLE_ID`: Apple Developer account email used for notarization.
- `APPLE_APP_SPECIFIC_PASSWORD`: app-specific password for notarization.
- `APPLE_TEAM_ID`: Apple Developer Team ID.

Published releases also require Tauri updater signing secrets once the signed
metadata channel is enabled:

- `TAURI_SIGNING_PRIVATE_KEY`: updater private key used to sign release metadata.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: password for the updater private key.

Published release jobs also read these repository variables to materialize the
release updater config before the strict updater gate runs:

- `SYNARA_UPDATER_PUBKEY`: public key generated from the updater signing key.
- `SYNARA_UPDATER_ENDPOINT`: optional HTTPS `latest.json` endpoint. If omitted,
  the release workflow uses the GitHub latest-release asset URL for this repo.

On Linux workstations, see [Linux builds](docs/linux.md) for distribution-specific system dependencies, KDE Plasma Wayland notes, and `.deb`/AppImage packaging commands.

Packaged builds expose their identity in the tray menu and About dialog as `Build <version> <branch>@<short-sha>`.

To replace an existing local macOS app bundle, move the old bundle aside first. Copying into an existing `.app` can nest the new bundle inside the old one:

- `mv /Applications/Synara.app "/Applications/Synara.app.$(date +%Y%m%d%H%M%S).previous"`
- `cp -R src-tauri/target/release/bundle/macos/Synara.app /Applications/Synara.app`

To start the local dev server, run:

- `npm run tauri dev`

### Linux build guide

For Linux workstation builds, install the required system packages and follow the steps in [`docs/linux.md`](docs/linux.md).

- Debian/Ubuntu/KDE neon and Fedora package commands for build dependencies are listed there.
- CachyOS and KDE Plasma Wayland smoke checklist is included for desktop integration verification.
- The page also links to:
  - [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
  - [Tauri distribution formats](https://v2.tauri.app/distribute/)

## License & Attribution

Synara Desktop is licensed under the **GNU Affero General Public License v3.0**
(see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE)).

The application is a derivative work of [Cinny](https://github.com/cinnyapp/cinny)
(AGPL-3.0), © the original Cinny authors, whose copyright remains intact in the
derived portions. Synara Desktop has been substantially rewritten by
**Whyland Creative LLC** (© 2026) — most notably replacing the matrix-js-sdk
renderer core with a native matrix-rust-sdk core. See `NOTICE` for third-party
attribution.

## Desktop Features

- Native tray/status-bar actions and close-to-tray behavior.
- Configurable global shortcuts.
- Native notification permissions and notification deep links.
- Dock/taskbar badge counts.
- macOS camera and microphone permission descriptions for Matrix calls.
- Hardened structured agent-action bridge for the Synara app runtime.

See [Desktop modernization](docs/desktop-modernization.md) for the native integration contract. Build/test notes, runtime smoke-test evidence, and bridge security notes are tracked in [MODERNIZATION.md](MODERNIZATION.md).
The native-first architecture decision is tracked in [Native-first architecture spike](docs/native-first-architecture-spike.md).
For a consolidated map of the whole monorepo, see [CODEBASE_KNOWLEDGE_BASE.md](CODEBASE_KNOWLEDGE_BASE.md).
