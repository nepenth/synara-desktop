# Linux Builds

Synara Desktop is a Tauri 2 app. Linux builds should be produced on Linux, because the final bundle links against system WebKitGTK, GTK, glibc, tray, and media libraries from the build host.

## Supported Targets

Start with these package formats:

- `.deb` for Debian, Ubuntu, KDE neon, and related distributions.
- AppImage for broader workstation smoke testing.

Tauri can also target RPM, Flatpak, Snap, and AUR packaging, but those formats should be treated as follow-up distribution work until they have their own install and update testing. See Tauri's Linux distribution notes for the current package formats and per-format caveats: <https://v2.tauri.app/distribute/>.

## Workstation Prerequisites

Use the Node.js version in `.node-version`, Rust stable, and the Tauri Linux dependencies for your distribution. The package lists below mirror the upstream Tauri 2 Linux prerequisites: <https://v2.tauri.app/start/prerequisites/>.

Debian, Ubuntu, KDE neon:

```sh
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

Fedora:

```sh
sudo dnf check-update
sudo dnf install -y \
  webkit2gtk4.1-devel \
  openssl-devel \
  curl \
  wget \
  file \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  libxdo-devel
sudo dnf group install -y "c-development"
```

Arch Linux:

```sh
sudo pacman -Syu
sudo pacman -S --needed \
  webkit2gtk-4.1 \
  base-devel \
  curl \
  wget \
  file \
  openssl \
  gst-plugins-good \
  appmenu-gtk-module \
  libayatana-appindicator \
  librsvg \
  xdotool
```

If package names drift for a newer distribution, install the equivalent WebKitGTK 4.1, GTK 3, OpenSSL, librsvg, xdo, and appindicator or ayatana appindicator development packages.

## Fresh Clone Build

```sh
git clone --recursive https://github.com/nepenth/synara-desktop.git
cd synara-desktop/synara
npm ci
cd ..
npm ci
npm run tauri dev
```

Production packaging:

```sh
npm run tauri build -- --bundles deb
npm run tauri build -- --bundles appimage
```

The Tauri `.deb` bundler supplies stock WebKitGTK, GTK, and tray runtime dependencies for Debian-family packages. Keep `bundle.linux.deb.depends` unset unless Synara gains an extra native runtime dependency.

The desktop build runs `scripts/build-web.mjs`, builds the `synara/` submodule, copies `synara/dist` into `devAssets`, then packages with Tauri.

### Arch-family local installs

On CachyOS and other rolling Arch-family systems, `npm run tauri build -- --bundles appimage` can fail in Tauri's `linuxdeploy` step with errors like `unknown type [0x13] section .relr.dyn`. That failure is in the AppImage packaging tool's bundled `strip` binary handling newer system libraries; it does not mean the Synara binary failed to build.

For local workstation testing on CachyOS, build the release binary and then produce a native pacman package:

```sh
npm run tauri build -- --bundles deb
cd packaging/arch
makepkg -f
sudo pacman -U synara-desktop-bin-*.pkg.tar.zst
```

The Arch package installs a small `/usr/bin/synara` wrapper that keeps the app on native Wayland while disabling WebKitGTK's DMABUF and compositing renderer paths. Those renderer paths can crash or blank the WebKit web process on CachyOS/KDE Plasma Wayland, especially on NVIDIA systems. Keep `gst-plugins-good` installed so WebKitGTK can resolve the `autoaudiosink` GStreamer element during Matrix media setup.

Use AppImage as a release artifact only after validating it on a packaging host or container whose `linuxdeploy` toolchain can strip the host libraries successfully.

## KDE Plasma Wayland Scope

KDE Plasma Wayland is in scope for Synara, but it needs direct validation on a Linux workstation before we call it release-proven.

Expected to work:

- Matrix login, sync, timeline rendering, account data, local notes, and web-client UI behavior.
- Native Linux packaging through Tauri `.deb` and AppImage.
- StatusNotifier/AppIndicator tray integration on KDE Plasma when the tray widget and appindicator dependencies are present.
- Native notifications through the desktop notification stack.

Areas that require KDE Wayland smoke testing:

- Close-to-tray behavior and tray menu activation.
- Global shortcuts, because Wayland compositors can restrict global key capture differently than macOS, Windows, or X11.
- Notification click/deep-link routing back into the existing Synara window.
- File open/download handoff through the desktop portal.
- Camera, microphone, and any future screen-share behavior through WebKitGTK, PipeWire, GStreamer, and `xdg-desktop-portal-kde`.
- Long-room timeline responsiveness under active agent message edits.

### CachyOS / KDE Plasma Wayland smoke checklist

Use this checklist for one focused acceptance pass on CachyOS and other Arch-family Wayland sessions.

Environment report:

- Synara platform:
  - `desktop_environment` reports `KDE` or `KDE Plasma`.
  - `session_type` reports `Wayland`.
  - Distribution metadata resolves to `cachyos` in the id and a matching name/version.
- Tray diagnostics:
  - Tray icon appears in the KDE panel.
  - Tray menu contains `Show Synara`, unread summary, `Later`, `Notifications`, `Desktop Integration`, `Do Not Disturb`, build label, `Quit`.
  - `unread`, `later`, `highlights`, and `notifications` menu labels update when state changes in-app.
- Shortcut diagnostics:
  - `Desktop Shortcuts` can be edited and saved.
  - On failure, help text explains why and indicates KDE Wayland manual binding workaround.
- Notifications:
  - In-session notifications appear.
  - Permission state in desktop integration panel reflects current denial/allowance.
- Portals:
  - File and media portal readiness rows are present and accurate (Ready / Not Ready / Unavailable).
- Build/deploy check:
  - Confirm app starts with Tauri defaults (Debian family or AppImage as configured).
  - Validate using documented dependencies from Tauri Linux prerequisites and packaging format notes.

Reference links for local environment prep:

- <https://v2.tauri.app/start/prerequisites/>
- <https://v2.tauri.app/distribute/>

Recommended first validation matrix:

- KDE Plasma 6 Wayland on one Debian-family workstation, such as KDE neon or Kubuntu.
- KDE Plasma 6 Wayland on one rolling workstation, such as Arch.
- One X11 fallback session for tray and global-shortcut comparison.
