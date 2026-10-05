#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/lib/rust-cache.sh"
if [[ $# -eq 0 ]]; then
  printf 'Usage: %s <command> [args...]\n' "$0" >&2
  exit 2
fi
synara_configure_rust_cache "$repo_root"
exec "$@"
