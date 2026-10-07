#!/usr/bin/env bash
# Package the Linux release binary (built once on Ubuntu by the .deb job) as
# the synara-desktop-bin pacman package. Runs inside the Arch container.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$repo_root/target/release/synara"

if [[ ! -f "$binary" ]]; then
  echo "Missing $binary; download the synara-linux-binary artifact first." >&2
  exit 1
fi
chmod 755 "$binary"

# The binary is linked on Ubuntu 22.04. Prove every shared library it needs
# resolves against the Arch runtime packages before packaging it.
missing="$(ldd "$binary" | grep 'not found' || true)"
if [[ -n "$missing" ]]; then
  printf 'Unresolved shared libraries on Arch:\n%s\n' "$missing" >&2
  exit 1
fi

id builder >/dev/null 2>&1 || useradd -m builder
chown -R builder:builder "$repo_root"
su builder -c "cd '$repo_root/packaging/arch' && makepkg -f --noconfirm"
