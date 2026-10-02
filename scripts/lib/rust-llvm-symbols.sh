#!/usr/bin/env bash
# Read-only Apple symbol inspection using the selected Rust toolchain's LLVM.
# The caller owns prerequisites; never install tools or infer symbols from bytes.
resolve_rust_llvm_nm() {
  local override="${1:-}" rust_info sysroot host rust_llvm tool_info tool_llvm
  rust_info="$(rustc -vV)" || return 1
  host="$(awk '$1 == "host:" { print $2 }' <<<"$rust_info")"
  rust_llvm="$(awk '$1 == "LLVM" && $2 == "version:" { print $3 }' <<<"$rust_info")"
  sysroot="$(rustc --print sysroot)" || return 1
  if [[ -z "$host" || -z "$rust_llvm" || -z "$sysroot" ]]; then
    echo "Cannot resolve selected Rust LLVM toolchain metadata." >&2
    return 1
  fi
  NM_BIN="${override:-$sysroot/lib/rustlib/$host/bin/llvm-nm}"
  if [[ ! -x "$NM_BIN" ]]; then
    echo "Missing selected Rust llvm-nm: $NM_BIN. Install llvm-tools-preview for the selected Rust toolchain before building Apple artifacts." >&2
    return 1
  fi
  tool_info="$("$NM_BIN" --version)" || return 1
  tool_llvm="$(awk '/LLVM version / { print $3; exit }' <<<"$tool_info")"
  tool_llvm="${tool_llvm%%-*}"
  if [[ "$tool_llvm" != "$rust_llvm" ]]; then
    echo "LLVM version mismatch: selected rustc uses $rust_llvm; llvm-nm uses ${tool_llvm:-unknown}." >&2
    return 1
  fi
}

inspect_nse_apple_symbols() {
  local artifact="$1" require_nse_export="$2" architectures architecture output errors status symbols expected
  expected='_uniffi_synara_nse_core_fn_method_nsepreviewrequest_resolve'
  architectures="$(lipo -archs "$artifact")" || {
    echo "Cannot inspect Apple artifact architectures: $artifact" >&2
    return 1
  }
  if [[ -z "$architectures" ]]; then
    echo "Apple artifact architecture inventory is empty: $artifact" >&2
    return 1
  fi
  for architecture in $architectures; do
    if [[ ! "$architecture" =~ ^[[:alnum:]_]+$ ]]; then
      echo "Invalid Apple architecture readback: $architecture" >&2
      return 1
    fi
    output="$(mktemp "${TMPDIR:-/tmp}/synara-nse-symbols.XXXXXX")"
    errors="$(mktemp "${TMPDIR:-/tmp}/synara-nse-symbol-errors.XXXXXX")"
    status=0
    "$NM_BIN" --arch="$architecture" --extern-only --defined-only --format=posix --quiet "$artifact" >"$output" 2>"$errors" || status=$?
    if [[ "$status" -ne 0 || -s "$errors" ]]; then
      echo "NSE symbol inspection failed: $artifact ($architecture)" >&2
      cat "$errors" >&2
      rm -f "$output" "$errors"
      return 1
    fi
    if LC_ALL=C grep -Eq '^_?uniffi_synara_core_[[:alnum:]_]+[[:space:]]' "$output"; then
      echo "NSE artifact contains forbidden full Core exports: $artifact ($architecture)" >&2
      rm -f "$output" "$errors"
      return 1
    fi
    symbols="$(LC_ALL=C grep -Ec '^[^[:space:]:]+[[:space:]]+[[:alpha:]?][[:space:]]' "$output" || true)"
    if LC_ALL=C grep -Eq "^${expected}[[:space:]]+[TW][[:space:]]+[^[:space:]]+" "$output"; then
      printf 'artifact=%s architecture=%s symbols=%s required_nse_export=%s\n' "$artifact" "$architecture" "$symbols" "$expected"
    elif [[ "$require_nse_export" == "1" ]]; then
      echo "NSE artifact has no required decoded NSE export: $artifact ($architecture); symbol rows=$symbols" >&2
      rm -f "$output" "$errors"
      return 1
    else
      printf 'artifact=%s architecture=%s symbols=%s required_nse_export=not-in-final-symbol-table (shipping symbol readback alone is not archive isolation proof)\n' "$artifact" "$architecture" "$symbols"
    fi
    rm -f "$output" "$errors"
  done
}
