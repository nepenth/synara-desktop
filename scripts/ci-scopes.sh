#!/usr/bin/env bash
# Decide which CI jobs run for one event. Writes key=value lines to
# $GITHUB_OUTPUT. Tested by scripts/__tests__/ci-scopes.test.mjs against real
# git diffs.
#
# Inputs: EVENT_NAME, BASE_SHA, HEAD_SHA, PR_LABELS, GITHUB_BASE_REF,
# GITHUB_HEAD_REF, GITHUB_REF_NAME, PACKAGES_INPUT (workflow_dispatch only).
set -euo pipefail

out() { echo "$1=$2" >> "$GITHUB_OUTPUT"; }

# Every output is written exactly once, at the end.
VALIDATE_RUST=false
RUST_AUDIT=false
VALIDATE_FRONTEND=false
IOS=false
IOS_UI=false
IOS_COMPILE=false
SYNAPSE=false
PACKAGES=false
ICON_ONLY=false

emit() {
  out icon_only "$ICON_ONLY"
  out validate_rust "$VALIDATE_RUST"
  out rust_audit "$RUST_AUDIT"
  out validate_frontend "$VALIDATE_FRONTEND"
  out ios "$IOS"
  out ios_ui "$IOS_UI"
  out ios_compile "$IOS_COMPILE"
  out synapse "$SYNAPSE"
  out packages "$PACKAGES"
  echo "CI scope: rust=${VALIDATE_RUST} audit=${RUST_AUDIT} frontend=${VALIDATE_FRONTEND} ios=${IOS} ios_ui=${IOS_UI} ios_compile=${IOS_COMPILE} synapse=${SYNAPSE} packages=${PACKAGES}"
}

full() {
  VALIDATE_RUST=true
  RUST_AUDIT=true
  VALIDATE_FRONTEND=true
  IOS=true
  IOS_UI=true
  SYNAPSE=true
}

has_label() { [[ ",${PR_LABELS:-}," == *",$1,"* ]]; }
changed() { ! git diff --quiet "$BASE_SHA" "$HEAD_SHA" -- "$@"; }

# Nightly and manual runs are the heavy suite: every lane, including the
# XCUITests, the live Synapse proofs, package builds and release-cache seeding.
if [[ "${EVENT_NAME}" == "schedule" ]]; then
  full
  PACKAGES=true
  emit
  exit 0
fi
if [[ "${EVENT_NAME}" == "workflow_dispatch" ]]; then
  full
  [[ "${PACKAGES_INPUT:-false}" == "true" ]] && PACKAGES=true
  emit
  exit 0
fi

if [[ -z "${BASE_SHA:-}" || -z "${HEAD_SHA:-}" || "${BASE_SHA}" == "0000000000000000000000000000000000000000" ]]; then
  # Missing diff metadata: run everything a push could need, UI tests stay opt-in.
  full
  IOS_UI=false
  emit
  exit 0
fi

RELEASE_PR=false
if [[ "${GITHUB_BASE_REF:-}" == release/* || "${GITHUB_HEAD_REF:-}" == release/* ]]; then
  RELEASE_PR=true
fi

# Release PRs and explicit iOS opt-ins must reach their gates even when the
# diff is only release notes or icons.
FORCE_CLIENT_GATES=false
if [[ "$RELEASE_PR" == "true" ]] || has_label needs-ios || has_label needs-ios-ui; then
  FORCE_CLIENT_GATES=true
fi

if [[ "$FORCE_CLIENT_GATES" != "true" ]] && git diff --name-only "$BASE_SHA" "$HEAD_SHA" | node scripts/ci-icon-only.mjs; then
  ICON_ONLY=true
  echo "CI scope: app-icon-only"
  node scripts/check-app-icons.mjs
  node --test scripts/__tests__/check-app-icons.test.mjs scripts/__tests__/ci-icon-only.test.mjs
  emit
  exit 0
fi

if [[ "$FORCE_CLIENT_GATES" != "true" ]] && git diff --name-only "$BASE_SHA" "$HEAD_SHA" | node scripts/ci-metadata-only.mjs; then
  echo "CI scope: metadata-only"
  node scripts/check-version-consistency.mjs
  emit
  exit 0
fi

RUST_CACHE_PATHS=(
  .github/actions/setup-rust-cache
  .kache.toml
  scripts/ci-rust-cache-identity.sh
  scripts/start-ci-rust-cache.mjs
  scripts/setup-rust-cache.sh
  scripts/with-rust-cache.sh
  scripts/lib/rust-cache.sh
)

if changed .github/workflows/ci.yml scripts/ci-scopes.sh "${RUST_CACHE_PATHS[@]}" \
  Cargo.toml Cargo.lock .cargo crates src-tauri rust-toolchain.toml; then
  VALIDATE_RUST=true
fi

if changed .github/workflows .github/actions devAssets packaging .node-version \
  config.json package.json package-lock.json scripts synara; then
  VALIDATE_FRONTEND=true
fi

if changed .github/workflows/ci.yml Cargo.lock Cargo.toml src-tauri/Cargo.toml \
  crates/synara-core/Cargo.toml crates/synara-nse-core/Cargo.toml .cargo/audit.toml; then
  RUST_AUDIT=true
fi

# Live Synapse proofs: the desktop tests in src-tauri drive the shared Core
# send, timeline, sync and auth owners, so Core sources count too.
if changed .github/workflows/ci.yml scripts/ci-scopes.sh scripts/synapse-integration.sh \
  integration/synapse Cargo.lock crates/synara-core/src crates/synara-core/Cargo.toml \
  src-tauri/src/matrix src-tauri/src/bridge src-tauri/Cargo.toml; then
  SYNAPSE=true
fi

IOS_PATHS_CHANGED=false
if changed rust-toolchain.toml Cargo.toml Cargo.lock .cargo \
  crates/synara-core crates/synara-nse-core crates/synara-core-bindgen \
  .github/workflows/ci.yml scripts/ci-scopes.sh \
  scripts/generate-synara-core-swift.sh scripts/generate-synara-nse-core-swift.sh \
  synara-ios/scripts/check-notification-service-archive.sh scripts/generate-xcode-project.mjs \
  "${RUST_CACHE_PATHS[@]}" \
  .github/actions/setup-xcode-cache .github/actions/save-xcode-cache \
  scripts/check-synara-nse-core-isolation.mjs scripts/check-synara-nse-core-production-features.mjs \
  scripts/check-synara-core-production-features.mjs scripts/lib/publish-generated-apple-pair.sh \
  scripts/lib/compare-generated-apple-pair.mjs scripts/check-synara-nse-core-archive-exports.sh \
  scripts/lib/rust-llvm-symbols.sh scripts/check-synara-core-swift-scaffold.mjs \
  synara-ios; then
  IOS_PATHS_CHANGED=true
fi

# iOS simulator lanes (unit ~20-30m, UI ~30-40m):
# - Release PRs and the needs-ios label run unit tests.
# - The needs-ios-ui label runs unit and UI tests. UI tests otherwise run
#   nightly and on manual dispatch only.
# - Push to main runs unit tests when iOS paths changed.
# - Other PRs that touch iOS/FFI paths run the cheap compile gate instead.
if [[ "${EVENT_NAME}" == "pull_request" ]]; then
  if has_label needs-ios-ui; then
    IOS=true
    IOS_UI=true
  elif [[ "$RELEASE_PR" == "true" ]] || has_label needs-ios; then
    IOS=true
  fi
elif [[ "${EVENT_NAME}" == "push" && "$IOS_PATHS_CHANGED" == "true" ]]; then
  IOS=true
fi

if [[ "$IOS_PATHS_CHANGED" == "true" && "$IOS" != "true" ]]; then
  IOS_COMPILE=true
fi

emit
git diff --name-only "$BASE_SHA" "$HEAD_SHA" | head -80 || true
