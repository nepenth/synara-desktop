# Linux Builds

Synara Desktop is a Tauri 2 app. Linux builds should be produced on Linux, because the final bundle links against system WebKitGTK, GTK, glibc, tray, and media libraries from the build host.

## Supported Targets

Start with these package formats:

- `.deb` for Debian, Ubuntu, Pop!_OS, KDE neon, and related distributions
  through the Synara GitHub Release-backed APT repository.
- Native pacman package for CachyOS, Arch, and other Arch-family installs
  through the Synara GitHub Release-backed pacman repository.

Tauri can also target RPM, Flatpak, Snap, and AppImage packaging, but those formats should be treated as follow-up distribution work until they have their own install and update testing. See Tauri's Linux distribution notes for the current package formats and per-format caveats: <https://v2.tauri.app/distribute/>.

For the current release goal, Linux updates are package-manager-owned. The app
may notify users that a newer Linux package is available, but it must not
self-update a package-manager installation. Debian-family users configure the
Synara APT repo once and update with `sudo apt update && sudo apt upgrade`.
Arch-family users configure the Synara pacman repo once, then update with
normal `paru -Syu` or `sudo pacman -Syu`.

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

Arch Linux, CachyOS, EndeavourOS, Manjaro:

```sh
sudo pacman -Syu
sudo pacman -S --needed \
  dbus \
  libsecret \
  webkit2gtk-4.1 \
  xdg-desktop-portal \
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

For KDE Plasma Wayland, also install the KDE portal integration if it is not already present:

```sh
sudo pacman -S --needed xdg-desktop-portal-kde
```

Session persistence uses the Secret Service API (`libsecret`). On KDE desktops,
`kwallet` can satisfy that API; on GNOME, `gnome-keyring` is the usual provider.
The Arch `PKGBUILD` depends on `dbus`, `libsecret`, and `xdg-desktop-portal`,
with wallet backends listed as `optdepends`.

If package names drift for a newer distribution, install the equivalent WebKitGTK 4.1, GTK 3, OpenSSL, librsvg, xdo, and appindicator or ayatana appindicator development packages.

## Fresh Clone Build

Install dependencies from the repository root first, then the embedded `synara/`
runtime. This matches the Arch packaging flow and CI release workflows.

```sh
git clone https://github.com/nepenth/synara-desktop.git
cd synara-desktop
npm ci
cd synara
npm ci
cd ..
npm run tauri dev
```

Production packaging:

```sh
npm run tauri build -- --bundles deb
npm run tauri build -- --bundles appimage
```

The Tauri `.deb` bundler supplies stock WebKitGTK, GTK, and tray runtime dependencies for Debian-family packages. Synara additionally declares `hunspell-en-us` so WebKitGTK has a default English spell-check dictionary. The Arch package declares `enchant` and `hunspell-en_us` for the same reason. Users who select another composer language must install the matching Hunspell dictionary through their distribution.

The desktop build runs `scripts/build-runtime.mjs`, which copies the repository
root `config.json` into `synara/config.json`, builds the `synara/` app runtime,
copies `synara/dist` into `devAssets`, then packages with Tauri. Edit the root
`config.json` only; the build pipeline keeps `synara/config.json` in sync.

### Debian / Ubuntu / Pop!_OS package

Configure the repository once:

```sh
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL \
  https://github.com/nepenth/synara-desktop/releases/download/apt-repo/synara-archive-keyring.gpg |
  sudo tee /etc/apt/keyrings/synara-archive-keyring.gpg >/dev/null
sudo chmod 0644 /etc/apt/keyrings/synara-archive-keyring.gpg
gpg --show-keys --with-fingerprint /etc/apt/keyrings/synara-archive-keyring.gpg

sudo tee /etc/apt/sources.list.d/synara.sources >/dev/null <<'EOF'
Types: deb
URIs: https://github.com/nepenth/synara-desktop/releases/download/
Suites: apt-repo/
Architectures: amd64
Signed-By: /etc/apt/keyrings/synara-archive-keyring.gpg
EOF

sudo apt update
sudo apt install synara
```

Verify that the displayed primary-key fingerprint is exactly:

```text
EB88 3952 04C1 EE19 7EE8  3B2F 3E02 F509 BB6B 0D2B
```

The trailing slash in `apt-repo/` selects a flat repository. Its package index
is published at:

```text
https://github.com/nepenth/synara-desktop/releases/download/apt-repo/Packages
```

After setup, Synara participates in normal system updates:

```sh
sudo apt update
sudo apt upgrade
```

Verify the installed and candidate versions with `apt-cache policy synara`.
The fixed `apt-repo` GitHub Release is refreshed by production release CI from
the same `.deb` uploaded to each versioned release. CI signs `InRelease` and
`Release.gpg` with the dedicated Synara APT key; `Signed-By` scopes trust in that
key to this repository instead of granting it system-wide trust.

GitHub Release asset replacement is not transactional. Publication keeps the
prior package until signed metadata for the replacement is live, but the
long-term hardening target remains an atomically deployed static package host.

To remove Synara and the repository:

```sh
sudo apt remove synara
sudo rm /etc/apt/sources.list.d/synara.sources
sudo rm /etc/apt/keyrings/synara-archive-keyring.gpg
sudo apt update
```

### CachyOS / Arch-family package

Recommended install path for a CachyOS KDE Plasma Wayland workstation:

```sh
sudo install -d -m 0755 /etc/pacman.d
printf '%s\n' \
  '[synara]' \
  'SigLevel = Optional TrustAll' \
  'Server = https://github.com/nepenth/synara-desktop/releases/download/pacman-repo' |
  sudo tee /etc/pacman.d/synara.conf >/dev/null

if ! grep -q 'Include = /etc/pacman.d/synara.conf' /etc/pacman.conf; then
  printf '\nInclude = /etc/pacman.d/synara.conf\n' | sudo tee -a /etc/pacman.conf >/dev/null
fi

sudo pacman -Sy synara-desktop-bin
```

If `synara-desktop-bin` was already installed from a local package or an older
helper path, reinstall it after adding the repo so pacman records the package
against the `synara` repository:

```sh
sudo pacman -R synara-desktop-bin
sudo pacman -Syy synara-desktop-bin
pacman -Q synara-desktop-bin
```

After that one-time setup, updates are normal package-manager updates:

```sh
paru -Syu
```

or:

```sh
sudo pacman -Syu
```

The `synara` repository is backed by the fixed public GitHub Release tag
`pacman-repo`. Production release CI owns package creation, `repo-add`, pacman
database generation, and release asset replacement. Do not manually run
`repo-add` for the production repository.

Current trust policy is `SigLevel = Optional TrustAll` because package signing
is not enabled yet. Before broad public distribution, add a dedicated package
signing key, publish its public key, and tighten this to a required signature
policy.

To remove the package:

```sh
sudo pacman -R synara-desktop-bin
```

### CachyOS / Arch-family local build

Use this only for local development or emergency smoke builds. Build on the
CachyOS machine, then install the local pacman package.

From a fresh clone:

```sh
git clone https://github.com/nepenth/synara-desktop.git
cd synara-desktop
npm ci
cd synara
npm ci
cd ..
npm run tauri build
cd packaging/arch
makepkg -f
sudo pacman -U synara-desktop-bin-*.pkg.tar.zst
```

After install, launch Synara from the KDE application launcher or run:

```sh
synara
```

The package installs:

- `/usr/bin/synara` wrapper.
- `/usr/lib/synara/synara` release binary.
- `/usr/share/applications/Synara.desktop` desktop entry from `packaging/arch/synara.desktop`.
- hicolor app icons (`Icon=synara` under `/usr/share/icons/hicolor/*/apps/`).

The wrapper keeps the app on native Wayland and disables WebKitGTK DMABUF/compositing renderer paths that can crash or blank the WebKit web process on CachyOS/KDE Plasma Wayland, especially on NVIDIA systems:

```sh
GDK_BACKEND=wayland
WEBKIT_DISABLE_DMABUF_RENDERER=1
WEBKIT_DISABLE_COMPOSITING_MODE=1
```

Keep `gst-plugins-good` installed so WebKitGTK can resolve the `autoaudiosink` GStreamer element during Matrix media setup.

#### Updating an existing clone-built CachyOS install

From the existing clone:

```sh
cd synara-desktop
git pull
npm ci
cd synara
npm ci
cd ..
npm run tauri build
cd packaging/arch
makepkg -f
sudo pacman -U synara-desktop-bin-*.pkg.tar.zst
```

`pacman -U` upgrades the installed package in place when `pkgver` or `pkgrel`
changes. If you are rebuilding the same package version for local testing,
`pacman -U` will still reinstall the local package.

The Arch `PKGBUILD` derives `pkgver` from `src-tauri/tauri.conf.json`, so a desktop app version bump automatically changes the local pacman package version. Run `npm run check:versions` before packaging if you need to verify all desktop, runtime, Cargo, and Arch package metadata are aligned.

#### AppImage and `.deb` bundler notes

On CachyOS and other rolling Arch-family systems, `npm run tauri build -- --bundles appimage` can fail in Tauri's `linuxdeploy` step with errors like `unknown type [0x13] section .relr.dyn`. That failure is in the AppImage packaging tool's bundled `strip` binary handling newer system libraries; it does not mean the Synara binary failed to build.

The Arch pacman package no longer depends on Tauri's `.deb` desktop entry output. `packaging/arch/synara.desktop` is installed directly, so `npm run tauri build` (release binary only) is sufficient before `makepkg -f`.

AppImage is not part of the current supported Linux update strategy. Revisit it
only after the pacman repo path is stable and tested.

#### GitHub Release-backed pacman repo notes

The in-repo `packaging/arch/PKGBUILD` expects a release binary already built in
`src-tauri/target/release/synara`. CI builds that binary inside an Arch
container, runs `makepkg`, then runs:

```sh
scripts/build-pacman-repo.sh
```

That script creates a `synara` pacman database with:

- `synara.db`
- `synara.db.tar.gz`
- `synara.files`
- `synara.files.tar.gz`
- `synara-desktop-bin-<version>-<pkgrel>-x86_64.pkg.tar.zst`

Release-branch CI currently builds and uploads a `synara-linux-arch-pkg`
artifact from `packaging/arch/PKGBUILD`. This artifact is suitable for
release-candidate smoke with:

```sh
sudo pacman -U synara-desktop-bin-*.pkg.tar.zst
```

The production release workflow uploads the package to the versioned GitHub
Release for traceability and replaces the fixed `pacman-repo` release assets
for package-manager updates.

Do not point Linux installs at Tauri self-updater `.tar.gz` sidecar artifacts.
Those artifacts are for app-managed update flows; Synara Linux updates are
package-manager-owned for this release goal.

## KDE Plasma Wayland Scope

KDE Plasma Wayland is in scope for Synara, but it needs direct validation on a Linux workstation before we call it release-proven.

Expected to work:

- Matrix login, sync, timeline rendering, account data, local notes, and app-runtime UI behavior.
- Native Linux packaging through Tauri `.deb` and the Arch-family pacman repo.
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
  - `desktop_environment` reports `KDE Plasma Wayland` on KDE Wayland sessions
    (Rust `desktop_environment_label()`), or `KDE` / `KDE Plasma` on other KDE
    sessions.
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
  - Confirm app starts with Tauri defaults (Debian family package or pacman repo
    package as configured).
  - Validate using documented dependencies from Tauri Linux prerequisites and packaging format notes.

Reference links for local environment prep:

- <https://v2.tauri.app/start/prerequisites/>
- <https://v2.tauri.app/distribute/>

Recommended first validation matrix:

- KDE Plasma 6 Wayland on one Debian-family workstation, such as KDE neon or Kubuntu.
- KDE Plasma 6 Wayland on one rolling workstation, such as Arch.
- One X11 fallback session for tray and global-shortcut comparison.
