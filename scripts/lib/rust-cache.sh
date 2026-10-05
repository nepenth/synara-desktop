#!/usr/bin/env bash
# One environment owner for local Cargo, Tauri, and Apple Rust builds.
# Source this file and call synara_configure_rust_cache before invoking Cargo.

SYNARA_KACHE_VERSION=0.28.1

synara_rust_cache_fail() {
  printf 'synara-rust-cache: %s\n' "$*" >&2
  return 1
}

synara_rust_cache_tools_dir() {
  printf '%s\n' "${SYNARA_RUST_CACHE_TOOLS_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/synara/rust-cache/tools}"
}

synara_rust_cache_binary() {
  if [[ -n "${SYNARA_KACHE_BIN:-}" ]]; then
    printf '%s\n' "$SYNARA_KACHE_BIN"
  elif [[ -n "${RUSTC_WRAPPER:-}" ]]; then
    printf '%s\n' "$RUSTC_WRAPPER"
  else
    printf '%s/kache-%s/kache\n' "$(synara_rust_cache_tools_dir)" "$SYNARA_KACHE_VERSION"
  fi
}

synara_validate_kache() {
  local binary="$1" version
  command -v "$binary" >/dev/null 2>&1 || {
    synara_rust_cache_fail "Kache $SYNARA_KACHE_VERSION is required. Run npm run cache:setup, or set SYNARA_KACHE_BIN to a verified installation."
    return 1
  }
  version="$("$binary" --version 2>/dev/null)" || {
    synara_rust_cache_fail "cannot run compiler wrapper $binary; no uncached fallback was selected"
    return 1
  }
  [[ "$version" == "kache $SYNARA_KACHE_VERSION" ]] || {
    synara_rust_cache_fail "compiler wrapper must be kache $SYNARA_KACHE_VERSION (found $version); remove conflicting RUSTC_WRAPPER or select the pinned installation"
    return 1
  }
}

synara_configure_rust_cache() {
  local repo_root="${1%/}" binary cache_base
  [[ -z "${RUSTC_WORKSPACE_WRAPPER:-}" && -z "${CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER:-}" ]] || {
    synara_rust_cache_fail "RUSTC_WORKSPACE_WRAPPER is already configured; remove it explicitly before enabling Kache (wrappers are not chained)"
    return 1
  }
  [[ -z "${CARGO_BUILD_RUSTC_WRAPPER:-}" || "${CARGO_BUILD_RUSTC_WRAPPER}" == "${RUSTC_WRAPPER:-}" ]] || {
    synara_rust_cache_fail "CARGO_BUILD_RUSTC_WRAPPER conflicts with the Kache owner; remove it explicitly"
    return 1
  }
  [[ "${KACHE_DISABLED:-0}" != "1" && "${KACHE_DISABLED:-false}" != "true" ]] || {
    synara_rust_cache_fail "KACHE_DISABLED is set; remove it explicitly before enabling the repository cache owner"
    return 1
  }
  binary="$(synara_rust_cache_binary)" || return 1
  synara_validate_kache "$binary" || return 1
  if [[ -n "${RUSTC_WRAPPER:-}" && "$RUSTC_WRAPPER" != "$binary" ]]; then
    synara_rust_cache_fail "RUSTC_WRAPPER differs from SYNARA_KACHE_BIN; select one compiler wrapper explicitly"
    return 1
  fi
  binary="$(command -v "$binary")"
  if [[ "$binary" != /* ]]; then
    binary="$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary")"
  fi
  [[ -x "$binary" ]] || { synara_rust_cache_fail "Kache must be an executable file, not a shell function"; return 1; }
  case "$(uname -s)" in
    Darwin) cache_base="${XDG_CACHE_HOME:-$HOME/Library/Caches}" ;;
    Linux) cache_base="${XDG_CACHE_HOME:-$HOME/.cache}" ;;
    *) synara_rust_cache_fail "local Rust caching supports macOS and Linux"; return 1 ;;
  esac
  export RUSTC_WRAPPER="$binary"
  export CARGO_INCREMENTAL=0
  # cc-rs selects the correct target compiler before applying RUSTC_WRAPPER.
  # CMake native dependencies use its launchers without changing CC/CXX.
  if [[ -z "${CMAKE_C_COMPILER_LAUNCHER+x}" ]]; then
    export CMAKE_C_COMPILER_LAUNCHER="$binary"
  fi
  if [[ -z "${CMAKE_CXX_COMPILER_LAUNCHER+x}" ]]; then
    export CMAKE_CXX_COMPILER_LAUNCHER="$binary"
  fi
  export KACHE_CACHE_DIR="${KACHE_CACHE_DIR:-${SYNARA_RUST_CACHE_ROOT:-$cache_base/Synara/rust-cache}/kache}"
  export KACHE_RUNTIME_DIR="${KACHE_RUNTIME_DIR:-$KACHE_CACHE_DIR/run}"
  export KACHE_CONFIG="${KACHE_CONFIG:-$repo_root/.kache.toml}"
  # The repository config keeps caching local; leave an explicit environment
  # override intact without manufacturing a redundant operational flag.
  export KACHE_CACHE_EXECUTABLES="${KACHE_CACHE_EXECUTABLES:-1}"
  export KACHE_AUTO_CLEAN_ORPHANED_TARGETS=0
  export KACHE_AUTO_CLEAN_IDLE_TARGETS_DAYS=0
  export KACHE_AUTO_CLEAN_UNUSED_UNITS_DAYS=0
  if [[ -n "${SYNARA_RUST_CACHE_MAX_SIZE:-}" ]]; then
    export KACHE_MAX_SIZE="${KACHE_MAX_SIZE:-$SYNARA_RUST_CACHE_MAX_SIZE}"
  fi
  mkdir -p "$KACHE_CACHE_DIR" "$KACHE_RUNTIME_DIR"
}
