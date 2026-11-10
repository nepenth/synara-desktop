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

Use Node.js 24.13.1 from `.node-version`, Rust 1.96 from `rust-toolchain.toml`, and the Tauri Linux dependencies for your distribution. Published Linux packages are x86_64 only. The package lists below mirror the upstream Tauri 2 Linux prerequisites: <https://v2.tauri.app/start/prerequisites/>.

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
npm run cache:setup
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
npm run cache:setup
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
- `/usr/share/applications/Synara.desktop` desktop entry from `packaging/arch/synara.desktop`. It registers Synara for `synara://` links (see "`synara://` links").
- hicolor app icons (`Icon=synara` under `/usr/share/icons/hicolor/*/apps/`).

The wrapper keeps the app on native Wayland (`GDK_BACKEND=wayland`) and makes CPU drawing this package's default (`SYNARA_RENDERING_DEFAULT=safe`). The Synara binary applies the drawing mode itself when it starts; see the next section.

Keep `gst-plugins-good` installed so WebKitGTK can resolve the `autoaudiosink` GStreamer element during Matrix media setup.

#### Hardware acceleration (`SYNARA_RENDERING`)

On Wayland, the Arch-family package draws the interface on the CPU by default. Before the web view starts, Synara sets the two variables that turn off the WebKitGTK DMABUF and compositing paths, which can crash or blank the WebKit web process on CachyOS/KDE Plasma Wayland, especially on NVIDIA systems:

```sh
WEBKIT_DISABLE_DMABUF_RENDERER=1
WEBKIT_DISABLE_COMPOSITING_MODE=1
```

The `.deb`, the AppImage and `tauri dev` have no wrapper and no such default: until you choose, Synara sets nothing and WebKitGTK draws with the graphics card.

**Settings → Appearance → Window → Hardware acceleration** changes it; restart Synara afterwards. The switch is shown only on Linux under Wayland. Until you use it, it shows what is running and nothing is saved. Your choice is saved as one word (`gpu` or `safe`) in `~/.config/com.whylandcreative.synara.desktop/rendering-mode`; delete the file to return to the package's default.

With hardware acceleration turned on by the setting or by `SYNARA_RENDERING=gpu`, Synara leaves WebKitGTK's GPU compositing and DMABUF presentation on. With NVIDIA's driver (when `/proc/driver/nvidia/version` exists) it also sets `__NV_DISABLE_EXPLICIT_SYNC=1` for Synara only: without it the app exits at startup with `Error 71 (Protocol error)` (`wp_linux_drm_syncobj_surface_v1 … Missing acquire timeline`). Checked on Hyprland 0.56 with an RTX 4080 SUPER, driver 615 and WebKitGTK 2.54; other compositors and drivers are untested, which is why the Arch-family package keeps it off by default.

What decides, first match wins:

1. `SYNARA_RENDERING`, when set: `gpu` turns hardware acceleration on, any other value turns it off. The switch in Settings is then disabled and says so.
2. The saved setting, once you have chosen.
3. `SYNARA_RENDERING_DEFAULT`, the package's default, read the same way. Only the Arch-family wrapper sets it (to `safe`).
4. Otherwise Synara sets nothing.

Whichever decides, any of the three variables above that is already set in the environment is left exactly as it is.

`SYNARA_RENDERING` is the way to try a mode for one start, or to get back in when the window blanks or the app exits with hardware acceleration on:

```sh
SYNARA_RENDERING=safe synara
```

Then turn the setting off. For a launcher, put the variable in the session environment or in the desktop entry (`Exec=env SYNARA_RENDERING=gpu synara %U`).

If a `.deb` or AppImage build exits at startup on NVIDIA with the `Error 71` above and you have not chosen a mode, that is case 4: nothing was set, so explicit sync is still on. Start it once with `SYNARA_RENDERING=gpu` (which adds `__NV_DISABLE_EXPLICIT_SYNC=1`) or `SYNARA_RENDERING=safe`, then set the switch to match so the choice is saved.

#### Updating an existing clone-built CachyOS install

From the existing clone:

```sh
cd synara-desktop
git pull
npm ci
cd synara
npm ci
cd ..
npm run cache:setup
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

The `.deb` (and an `.rpm`, which is not published) takes its desktop entry from `packaging/deb/synara.desktop`, set as `bundle.linux.deb.desktopTemplate` in `src-tauri/tauri.conf.json`. The bundler fills in the name, command and icon; the rest is the Arch entry line for line, and `scripts/__tests__/linux-desktop-entry.test.mjs` fails if the two drift apart. The bundler has no desktop template for AppImage, so an AppImage's entry would name the `synara` scheme but pass no link to the app.

AppImage is not part of the current supported Linux update strategy. Revisit it
only after the pacman repo path is stable and tested.

#### GitHub Release-backed pacman repo notes

The in-repo `packaging/arch/PKGBUILD` expects a release binary already built in
`target/release/synara`. CI builds that binary inside an Arch
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

## Tiling Window Managers

Under Hyprland, Sway, Niri, i3 and similar window managers Synara starts in tiling mode: it does not
draw its own title strip, because the window manager frames and moves the window. Detection uses
`HYPRLAND_INSTANCE_SIGNATURE`, `SWAYSOCK`, `NIRI_SOCKET`, `I3SOCK` and the names in
`XDG_CURRENT_DESKTOP`. Settings > Appearance > Tiling mode overrides it (Auto, On, Off) and applies
at once. Ctrl+Q quits in either mode.

The window can be as small as 360×480; under a tiling window manager there is no such floor (240×160), because the compositor sizes the tile. Below 750px wide the app switches to its single-pane layout.

### Window class and window rules

On Wayland the window's app id is `synara` (the `class` and `initialClass` in `hyprctl clients`, the
`app_id` in `swaymsg -t get_tree`), for the installed packages and for a development build alike.
Nothing sets it in code: Tauri leaves the GTK application id unset (`app.enableGTKAppId` is off), so
GTK uses the program name, which is the file name of the binary, `synara` (`mainBinaryName` in
`src-tauri/tauri.conf.json`). It changes only if the binary is renamed or started through a
differently named link. Both desktop entries carry `StartupWMClass=synara` to match.

On X11 (including XWayland) GTK sets `WM_CLASS` from the same name: instance `synara`, class
`Synara`, with a capital. That is how GTK 3 behaves; it has not been checked on an X11 session.

Hyprland 0.55 and later (Lua configuration):

```lua
hl.window_rule({ match = { class = "^synara$" }, workspace = "3" })
```

Hyprland 0.53 and 0.54 (`hyprland.conf`):

```ini
windowrule = match:class ^synara$, workspace 3
```

Hyprland 0.52 and earlier:

```ini
windowrulev2 = workspace 3, class:^(synara)$
```

Sway:

```
for_window [app_id="synara"] move container to workspace number 3
```

i3 (X11; `class` is case-sensitive, so match the instance or the capitalised class):

```
for_window [instance="synara"] move container to workspace number 3
```

## `synara://` Links

Synara opens `synara://` links from other apps, the terminal and scripts:

```sh
xdg-open synara://activity
xdg-open 'synara://room/!roomid:example.org'
```

`synara://home`, `settings`, `approvals`, `later`, `activity` (also `inbox`, `notifications`, with
`/later` or `/invites`) and `room/<room id or alias>[/<event id>]` are accepted; anything else is
ignored. If Synara is already running, the link goes to that window.

What registers the scheme:

| Platform | Registration |
| --- | --- |
| Arch package | `MimeType=x-scheme-handler/synara;` and `Exec=synara %U` in `packaging/arch/synara.desktop` |
| `.deb` | The same two lines in `packaging/deb/synara.desktop`, the bundler's desktop template |
| macOS | `CFBundleURLTypes` in `src-tauri/Info.plist` |
| Windows | Registry entries written by the NSIS and MSI installers (untested; see `docs/windows-support-spike.md`) |

If links open in something else, make Synara the handler and check it:

```sh
xdg-mime default Synara.desktop x-scheme-handler/synara
xdg-mime query default x-scheme-handler/synara
```

## Status Command

The running app publishes its counts to `$XDG_RUNTIME_DIR/synara/status.json`. `synara status` reads
that file without starting the app:

```sh
synara status            # 2 mentions, 1 approval, 5 unread messages
synara status --json     # the raw status
synara status --waybar   # JSON for a Waybar custom module
synara status --waybar --follow   # print a line whenever the status changes
```

It exits with 3 when Synara is not running. A Waybar module:

```jsonc
"custom/synara": {
  "exec": "synara status --waybar --follow",
  "return-type": "json",
  "format": "{icon} {text}",
  "format-icons": { "none": "󰭹", "unread": "󰭹", "mention": "󰍡", "approval": "󰒃", "offline": "󰭻" },
  "on-click": "synara",
  "tooltip": true
}
```

`text` is the number of things waiting on you (mentions, approvals and invites) and is empty when
there are none. `alt` and `class` are one of `none`, `unread`, `mention`, `approval`, `offline`;
`class` also carries `dnd` while notifications are paused, so the module can be styled in
`style.css` (`#custom-synara.approval { ... }`).
