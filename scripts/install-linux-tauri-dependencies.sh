#!/usr/bin/env bash
set -euo pipefail

# Installs the system packages the Linux Tauri build needs.
#
#   --sources-only   only make sure apt has a reachable package source, and
#                    leave it in place for later steps (Playwright's
#                    `install --with-deps` runs apt itself).
#
# GitHub-hosted runners can stall on the Azure Ubuntu mirror, and on
# 2026-11-11 could not resolve any ubuntu.com name for hours. So the source is
# chosen from three, in order, each probed first with a short request:
#   1. the official archive (archive.ubuntu.com),
#   2. the sources the runner image ships with,
#   3. a mirror outside ubuntu.com (kernel.org).
# apt checks every index against Ubuntu's signing keys whichever host serves
# it, so a mirror cannot substitute packages.
sources_only=false
if [[ "${1:-}" == "--sources-only" ]]; then
  sources_only=true
fi

# The three paths are overridable so the test can run this against stubs.
os_release_file="${SYNARA_OS_RELEASE_FILE:-/etc/os-release}"
runner_mirrors_file="${SYNARA_APT_MIRRORS_FILE:-/etc/apt/apt-mirrors.txt}"
system_source_list="${SYNARA_APT_SOURCE_LIST:-/etc/apt/sources.list}"

# shellcheck disable=SC1090
source "$os_release_file"

if [[ -z "${VERSION_CODENAME:-}" ]]; then
  echo "Unable to determine the Ubuntu release codename." >&2
  exit 1
fi

official_url="https://archive.ubuntu.com/ubuntu"
fallback_mirror_url="https://mirrors.edge.kernel.org/ubuntu"

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

write_source_list() { # <base url> <file>
  cat >"$2" <<LIST
deb $1 ${VERSION_CODENAME} main restricted universe multiverse
deb $1 ${VERSION_CODENAME}-updates main restricted universe multiverse
deb $1 ${VERSION_CODENAME}-security main restricted universe multiverse
LIST
}

bounds=(
  -o Acquire::Retries=3
  -o Acquire::http::Timeout=20
  -o Acquire::https::Timeout=20
)

reachable() { # <base url>
  curl --fail --silent --show-error --head --max-time 15 --retry 1 \
    "$1/dists/${VERSION_CODENAME}/InRelease" >/dev/null
}

# `apt-get update` exits 0 when an index cannot be fetched, and the install
# then fails with "Unable to locate package": make a fetch failure an error.
update_with() { # <apt options...>
  sudo apt-get "$@" -o APT::Update::Error-Mode=any update
}

runner_url=""
if [[ -r "$runner_mirrors_file" ]]; then
  runner_url="$(awk 'NF { print $1; exit }' "$runner_mirrors_file")"
fi

apt_options=()
chosen=""
try_listed() { # <name> <base url>
  local list="$work_dir/$1.list"
  reachable "$2" || return 1
  write_source_list "$2" "$list"
  update_with -o "Dir::Etc::sourcelist=$list" -o "Dir::Etc::sourceparts=-" "${bounds[@]}" || return 1
  apt_options=(-o "Dir::Etc::sourcelist=$list" -o "Dir::Etc::sourceparts=-" "${bounds[@]}")
  chosen="$1"
}
try_runner() {
  [[ -n "$runner_url" ]] && { reachable "${runner_url%/}" || return 1; }
  update_with "${bounds[@]}" || return 1
  apt_options=("${bounds[@]}")
  chosen="runner"
}

if try_listed official "$official_url"; then
  :
elif echo "archive.ubuntu.com is unreachable; trying the runner's own package sources." >&2 && try_runner; then
  :
elif echo "The runner's package sources are unreachable; trying $fallback_mirror_url." >&2 &&
  try_listed mirror "$fallback_mirror_url"; then
  :
else
  echo "No Ubuntu package source is reachable from this runner." >&2
  exit 1
fi
echo "Using package source: $chosen"

if $sources_only; then
  # Later steps call apt without our options. The runner's own sources need
  # nothing; any other choice replaces the system list for this job.
  if [[ "$chosen" != "runner" ]] && { [[ -z "$runner_url" ]] || ! reachable "${runner_url%/}"; }; then
    sudo cp "$work_dir/$chosen.list" "$system_source_list"
    echo "Installed the $chosen source list as $system_source_list for this job."
  fi
  exit 0
fi

sudo env DEBIAN_FRONTEND=noninteractive apt-get "${apt_options[@]}" install -y --no-install-recommends \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf
