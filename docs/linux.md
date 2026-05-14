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
  appmenu-gtk-module \
  libappindicator-gtk3 \
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

Recommended first validation matrix:

- KDE Plasma 6 Wayland on one Debian-family workstation, such as KDE neon or Kubuntu.
- KDE Plasma 6 Wayland on one rolling workstation, such as Arch.
- One X11 fallback session for tray and global-shortcut comparison.
