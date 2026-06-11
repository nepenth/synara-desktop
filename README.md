# Synara Desktop

Synara Desktop is a Tauri-based native desktop Matrix client focused on fast, secure conversations, smooth desktop UX, Linux support, and agent workflows.

## Download

Installers for macOS and Linux are built from this project. The in-app updater
is currently disabled until this project has a stable signed release metadata
channel. Local builds do not contact update servers.

| Operating System | Download          |
| ---------------- | ----------------- |
| macOS            | Build from source |
| Linux            | Build from source |

Windows packaging is not part of the current supported release matrix.
Developer ID signing, notarization, and updater metadata are release-channel
work after the required distribution credentials are available.

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

## Desktop Features

- Native tray/status-bar actions and close-to-tray behavior.
- Configurable global shortcuts.
- Native notification permissions and notification deep links.
- Dock/taskbar badge counts.
- macOS camera and microphone permission descriptions for Matrix calls.
- Hardened structured agent-action bridge for the Synara app runtime.

See [Desktop modernization](docs/desktop-modernization.md) for the native integration contract. Build/test notes, runtime smoke-test evidence, and bridge security notes are tracked in [MODERNIZATION.md](MODERNIZATION.md).
The native-first architecture decision is tracked in [Native-first architecture spike](docs/native-first-architecture-spike.md).
