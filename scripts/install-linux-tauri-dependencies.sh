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

bounds=(
  -o Acquire::Retries=3
  -o Acquire::http::Timeout=20
  -o Acquire::https::Timeout=20
)
apt_options=(
  -o "Dir::Etc::sourcelist=$source_list"
  -o "Dir::Etc::sourceparts=-"
  "${bounds[@]}"
)

# `apt-get update` exits 0 when an index cannot be fetched, and the install
# then fails with "Unable to locate package". Make a fetch failure an error,
# and when the official archive cannot be reached (for example its name does
# not resolve on the runner) fall back to the sources the runner image ships
# with, under the same retry and timeout bounds.
if ! sudo apt-get "${apt_options[@]}" -o APT::Update::Error-Mode=any update; then
  echo "archive.ubuntu.com is unreachable; using the runner's own package sources." >&2
  apt_options=("${bounds[@]}")
  sudo apt-get "${apt_options[@]}" -o APT::Update::Error-Mode=any update
fi
sudo env DEBIAN_FRONTEND=noninteractive apt-get "${apt_options[@]}" install -y --no-install-recommends \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf
