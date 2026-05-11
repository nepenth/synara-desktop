# Synara desktop

Synara is a modern Matrix client focused on fast, secure conversations, desktop polish, and agent workflows. The desktop app is made with Tauri.

## Download

Installers for macOS, Windows and Linux are built from this fork. Distribution releases should be signed with an [Ed25519](https://ed25519.cr.yp.to/) public key.

| Operating System | Download          |
| ---------------- | ----------------- |
| Windows          | Build from source |
| macOS            | Build from source |
| Linux            | Build from source |

Decoded public key:

> RWRflTUQD3RHFtn25QNANCmePR9+4LSK89kAKTMEEB4OKpOFpLMgc64z

To verify release files, you need to download [minisign](https://jedisct1.github.io/minisign/) tool and [decode](https://www.base64decode.org/) the _.sig_ file before running:

> minisign -Vm **_RELEASE_FILE.msi.zip_** -P RWRflTUQD3RHFtn25QNANCmePR9+4LSK89kAKTMEEB4OKpOFpLMgc64z -x **_SINGATURE.msi.zip.sig_**

## Local development

Firstly, to setup Rust, NodeJS and build tools follow [Tauri documentation](https://v2.tauri.app/start/prerequisites/).

Now, to setup development locally run the following commands:

- `git clone --recursive https://github.com/nepenthe/synara-desktop.git`
- `cd synara-desktop/cinny`
- `npm ci`
- `cd ..`
- `npm ci`

To build the app locally, run:

- `npm run tauri build`

On macOS, local unsigned smoke-test builds should use ad-hoc signing:

- `APPLE_SIGNING_IDENTITY=- npm run tauri build -- --bundles app`

Packaged builds expose their identity in the tray menu and About dialog as
`Build <version> <branch>@<short-sha>`.

To replace an existing local macOS app bundle, move the old bundle aside first.
Copying into an existing `.app` can nest the new bundle inside the old one:

- `mv /Applications/Synara.app "/Applications/Synara.app.$(date +%Y%m%d%H%M%S).previous"`
- `cp -R src-tauri/target/release/bundle/macos/Synara.app /Applications/Synara.app`

To start local dev server, run:

- `npm run tauri dev`

## Desktop polish

This wrapper provides native tray/status-bar actions, close-to-tray behavior,
global shortcuts, updater wiring, native notification permissions, and macOS
camera/microphone permission descriptions for Matrix calls.
It also exposes the badge-count and structured agent-action bridge consumed by
the Synara modernization web branch.
See [Desktop modernization](docs/desktop-modernization.md) for the native
integration contract used by the Synara modernization branches.
Build/test notes, runtime smoke-test evidence, and bridge security notes are
tracked in [MODERNIZATION.md](MODERNIZATION.md).
