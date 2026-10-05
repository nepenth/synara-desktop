#!/usr/bin/env bash
set -euo pipefail

[[ "${SYNARA_CACHE_FAMILY:-}" =~ ^[a-z0-9][a-z0-9-]*$ ]] || {
  echo "Invalid Rust cache family" >&2; exit 1;
}
case "${SYNARA_CACHE_WRITER:-false}" in
  true) [[ "${GITHUB_REF:-}" == refs/heads/main ]] || {
    echo "Only main may publish Rust caches" >&2; exit 1;
  } ;;
  false) ;;
  *) echo "Rust cache writer must be true or false" >&2; exit 1 ;;
esac

# Include compiler, Apple SDKs and selected installation; cross-target caches
# never fall back to objects from another SDK or Rust toolchain.
source scripts/lib/rust-cache.sh
identity="kache $SYNARA_KACHE_VERSION"$'\n'"$(rustc -vV)"
if [[ "${RUNNER_OS:-}" == macOS ]]; then
  identity+=$'\n'"${DEVELOPER_DIR:-$(xcode-select -p)}"
  identity+=$'\n'"$(xcodebuild -version)"
  identity+=$'\n'"$(xcrun --sdk macosx --show-sdk-version)"
  identity+=$'\n'"$(xcrun --sdk iphoneos --show-sdk-version)"
  identity+=$'\n'"$(xcrun --sdk iphonesimulator --show-sdk-version)"
fi
if command -v sha256sum >/dev/null 2>&1; then
  digest="$(printf '%s' "$identity" | sha256sum | cut -d ' ' -f 1)"
else
  digest="$(printf '%s' "$identity" | shasum -a 256 | cut -d ' ' -f 1)"
fi
echo "compiler-prefix=synara-kache-v1-${SYNARA_CACHE_FAMILY}-${RUNNER_OS}-${RUNNER_ARCH}-${digest}" >> "$GITHUB_OUTPUT"
{
  echo "CARGO_INCREMENTAL=0"
  echo "KACHE_HOST_CONFIG="
  echo "KACHE_CONFIG=$GITHUB_WORKSPACE/.kache.toml"
  echo "KACHE_AUTO_CLEAN_ORPHANED_TARGETS=0"
  echo "KACHE_AUTO_CLEAN_UNUSED_UNITS_DAYS=0"
} >> "$GITHUB_ENV"
