#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$repo_root/scripts/lib/rust-cache.sh"

install_kache() (
  local target digest install_dir archive stage actual
  case "$(uname -s)/$(uname -m)" in
    Darwin/arm64|Darwin/aarch64)
      target=aarch64-apple-darwin
      digest=253096ab5972fe179b86301abbcfb5944bc9807e3fa3d1720b44b129b86fcbff ;;
    Darwin/x86_64)
      target=x86_64-apple-darwin
      digest=05180b56a4eb8e0600ee47682017a7a81fd4a7cc1e781d481654002af8b082a6 ;;
    Linux/aarch64|Linux/arm64)
      target=aarch64-unknown-linux-musl
      digest=b2110805eb24afe9b408e5b9a1e3d0584b7fdd0b79fe8cbe548ab29fbaf50176 ;;
    Linux/x86_64)
      target=x86_64-unknown-linux-musl
      digest=68635a9f92ce5015a2c48c98b968d1b3707b6dbee0ca725e51d1671c9346f38f ;;
    *) synara_rust_cache_fail 'supported hosts: macOS/Linux x86_64 and arm64'; return 1 ;;
  esac
  install_dir="$(synara_rust_cache_tools_dir)/kache-$SYNARA_KACHE_VERSION"
  if [[ -x "$install_dir/kache" ]]; then
    synara_validate_kache "$install_dir/kache"
    return
  fi
  command -v curl >/dev/null || { synara_rust_cache_fail 'curl is required'; return 1; }
  command -v tar >/dev/null || { synara_rust_cache_fail 'tar is required'; return 1; }
  mkdir -p "$(dirname "$install_dir")"
  stage="$(mktemp -d "$(dirname "$install_dir")/.kache-install.XXXXXX")"
  trap 'rm -rf -- "$stage"' EXIT
  archive="$stage/kache.tar.gz"
  curl --fail --location --silent --show-error \
    --proto '=https' --tlsv1.2 \
    "https://github.com/kunobi-ninja/kache/releases/download/v$SYNARA_KACHE_VERSION/kache-$target.tar.gz" \
    --output "$archive"
  if command -v sha256sum >/dev/null; then
    actual="$(sha256sum "$archive")"
  elif command -v shasum >/dev/null; then
    actual="$(shasum -a 256 "$archive")"
  else
    synara_rust_cache_fail 'sha256sum or shasum is required for archive verification'
    return 1
  fi
  [[ "${actual%% *}" == "$digest" ]] || { synara_rust_cache_fail 'Kache archive SHA256 verification failed'; return 1; }
  # The pinned release archive contains a single executable named kache.
  tar -xzf "$archive" -C "$stage" kache
  chmod 755 "$stage/kache"
  synara_validate_kache "$stage/kache"
  mkdir -p "$install_dir"
  mv "$stage/kache" "$install_dir/kache"
  printf 'Installed verified kache %s: %s\n' "$SYNARA_KACHE_VERSION" "$install_dir/kache" >&2
)

case "${1:---install}" in
  --install)
    [[ $# -le 1 ]] || exit 2
    install_kache
    printf 'Local build entrypoints now use Kache. For direct cargo commands:\n  eval "$(scripts/setup-rust-cache.sh --env)"\n' ;;
  --env)
    [[ $# -eq 1 ]] || exit 2
    synara_configure_rust_cache "$repo_root"
    for name in RUSTC_WRAPPER CARGO_INCREMENTAL CMAKE_C_COMPILER_LAUNCHER CMAKE_CXX_COMPILER_LAUNCHER KACHE_CACHE_DIR KACHE_RUNTIME_DIR KACHE_CONFIG KACHE_LOCAL_ONLY KACHE_CACHE_EXECUTABLES KACHE_AUTO_CLEAN_ORPHANED_TARGETS KACHE_AUTO_CLEAN_IDLE_TARGETS_DAYS KACHE_AUTO_CLEAN_UNUSED_UNITS_DAYS KACHE_MAX_SIZE; do
      if [[ -n "${!name+x}" ]]; then printf 'export %s=%q\n' "$name" "${!name}"; fi
    done ;;
  --status)
    [[ $# -eq 1 ]] || exit 2
    synara_configure_rust_cache "$repo_root"
    "$RUSTC_WRAPPER" stats
    "$RUSTC_WRAPPER" targets ;;
  --clean)
    shift
    synara_configure_rust_cache "$repo_root"
    if [[ $# -eq 0 ]]; then
      "$RUSTC_WRAPPER" clean --tracked --stale 14d --dry-run
    elif [[ $# -eq 1 && "$1" == "--yes" ]]; then
      "$RUSTC_WRAPPER" clean --tracked --stale 14d --yes
    else
      printf 'Usage: %s --clean [--yes]\n' "$0" >&2
      exit 2
    fi ;;
  --help|-h)
    printf 'Usage: %s [--install | --env | --status | --clean [--yes]]\n' "$0"
    printf 'Installs SHA256-verified Kache %s without editing global Cargo or shell settings.\n' "$SYNARA_KACHE_VERSION" ;;
  *) printf 'Unknown argument: %s\n' "$1" >&2; exit 2 ;;
esac
