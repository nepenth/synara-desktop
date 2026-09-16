#!/usr/bin/env bash
# Inspect generated SynaraNseCore static archives for full-Core UniFFI exports.
#
# Apple's Xcode nm (LLVM 21) cannot read rustc 1.96 / LLVM 22 bitcode
# (`Unknown attribute kind`). That is a reader mismatch, not a failed cargo
# build. Skip nm in that case and search the archive bytes, the same way the
# shipping .appex check does. Any other nm failure still fails closed.
set -euo pipefail

FORBIDDEN='_uniffi_synara_core_'
NM_BIN="${SYNARA_NSE_ARCHIVE_NM:-nm}"

if [[ "$#" -eq 0 ]]; then
  echo "Usage: $0 <libsynara_nse_core.a> [archive...]" >&2
  exit 1
fi

nm_cannot_read_rustc_196_llvm22() {
  local err="$1"
  [[ "$err" == *'Unknown attribute kind'* ]] &&
    { [[ "$err" == *'LLVM22'* ]] || [[ "$err" == *'rust-1.96'* ]]; }
}

for archive in "$@"; do
  if [[ ! -f "$archive" ]]; then
    echo "SynaraNseCore archive is missing: $archive" >&2
    exit 1
  fi
  if [[ ! -s "$archive" ]]; then
    echo "SynaraNseCore archive is empty: $archive" >&2
    exit 1
  fi

  nm_err_file="$(mktemp "${TMPDIR:-/tmp}/synara-nse-nm.XXXXXX")"
  nm_status=0
  nm_output="$("$NM_BIN" -gU "$archive" 2>"$nm_err_file")" || nm_status=$?
  nm_err="$(cat "$nm_err_file")"
  rm -f "$nm_err_file"

  if [[ "$nm_status" -eq 0 ]]; then
    if [[ "$nm_output" == *"$FORBIDDEN"* ]]; then
      echo "SynaraNseCore archive contains forbidden full Core exports: $archive" >&2
      exit 1
    fi
    continue
  fi

  if nm_cannot_read_rustc_196_llvm22 "$nm_err"; then
    echo "Apple nm cannot read rustc 1.96/LLVM22 objects in $archive; searching archive bytes for forbidden full Core exports." >&2
    grep_status=0
    LC_ALL=C grep -a -F -q "$FORBIDDEN" "$archive" || grep_status=$?
    if [[ "$grep_status" -eq 0 ]]; then
      echo "SynaraNseCore archive contains forbidden full Core exports: $archive" >&2
      exit 1
    fi
    if [[ "$grep_status" -ne 1 ]]; then
      echo "SynaraNseCore archive byte search failed: $archive" >&2
      exit 1
    fi
    continue
  fi

  echo "SynaraNseCore archive symbol inspection failed: $archive" >&2
  if [[ -n "$nm_err" ]]; then
    printf '%s\n' "$nm_err" >&2
  fi
  exit 1
done
