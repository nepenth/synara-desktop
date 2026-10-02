#!/usr/bin/env bash
# Decode every generated Apple archive architecture using matching Rust LLVM.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/lib/rust-llvm-symbols.sh"
if [[ "$#" -eq 0 ]]; then
  echo "Usage: $0 <libsynara_nse_core.a> [archive...]" >&2
  exit 1
fi
for archive in "$@"; do
  [[ -f "$archive" ]] || { echo "SynaraNseCore archive is missing: $archive" >&2; exit 1; }
  [[ -s "$archive" ]] || { echo "SynaraNseCore archive is empty: $archive" >&2; exit 1; }
done
resolve_rust_llvm_nm "${SYNARA_NSE_ARCHIVE_NM:-}"
for archive in "$@"; do
  inspect_nse_apple_symbols "$archive" 1
done
