#!/usr/bin/env bash
set -euo pipefail

# GitHub-hosted runners can intermittently stall on the Azure Ubuntu mirror.
# Use a bounded, official Ubuntu source for this dependency-only operation.
source /etc/os-release

if [[ -z "${VERSION_CODENAME:-}" ]]; then
  echo "Unable to determine the Ubuntu release codename." >&2
  exit 1
fi

source_list="$(mktemp)"
trap 'rm -f "$source_list"' EXIT

cat >"$source_list" <<EOF
deb https://archive.ubuntu.com/ubuntu ${VERSION_CODENAME} main restricted universe multiverse
deb https://archive.ubuntu.com/ubuntu ${VERSION_CODENAME}-updates main restricted universe multiverse
deb https://archive.ubuntu.com/ubuntu ${VERSION_CODENAME}-security main restricted universe multiverse
EOF

apt_options=(
  -o "Dir::Etc::sourcelist=$source_list"
  -o "Dir::Etc::sourceparts=-"
  -o Acquire::Retries=3
  -o Acquire::http::Timeout=20
  -o Acquire::https::Timeout=20
)

sudo apt-get "${apt_options[@]}" update
sudo env DEBIAN_FRONTEND=noninteractive apt-get "${apt_options[@]}" install -y --no-install-recommends \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf
