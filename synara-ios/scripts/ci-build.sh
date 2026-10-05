#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/xcode-cache.sh"
synara_configure_xcode_cache "$SCRIPT_DIR/.."
DERIVED_DATA_PATH="${DERIVED_DATA_PATH:-$SYNARA_XCODE_DERIVED_DATA_ROOT/simulator}"
RESULT_BUNDLE_DIR="${IOS_RESULT_BUNDLE_DIR:-/private/tmp/synara-ios-results}"
RESULT_STAMP="${IOS_RESULT_STAMP:-$(date +%Y%m%d-%H%M%S)-$$}"
APPLE_SLICES="${SYNARA_CORE_APPLE_SLICES:-all}"
# Xcode 26 rejects `{ generic:1, platform:iOS Simulator, arch:arm64 }`.
# Named arm64 destinations exist; use one so the arm64-only XCFramework links.
if [[ -n "${IOS_BUILD_DESTINATION:-}" ]]; then
  BUILD_DESTINATION="$IOS_BUILD_DESTINATION"
elif [[ "$APPLE_SLICES" == "simulator-arm64" ]]; then
  BUILD_DESTINATION="platform=iOS Simulator,name=iPhone 17,arch=arm64"
else
  BUILD_DESTINATION="generic/platform=iOS Simulator"
fi
if [[ -n "${IOS_TEST_DESTINATION:-}" ]]; then
  TEST_DESTINATION="$IOS_TEST_DESTINATION"
elif [[ "$APPLE_SLICES" == "simulator-arm64" ]]; then
  TEST_DESTINATION="platform=iOS Simulator,name=iPhone 17,arch=arm64"
else
  TEST_DESTINATION="platform=iOS Simulator,name=iPhone 17"
fi
TEST_SUITE="${IOS_TEST_SUITE:-all}"
DEVICE_DERIVED_DATA_PATH="${IOS_DEVICE_DERIVED_DATA_PATH:-$SYNARA_XCODE_DERIVED_DATA_ROOT/device-release}"
export CFFIXED_USER_HOME="${CFFIXED_USER_HOME:-$SYNARA_XCODE_CACHE_ROOT/Home}"
PACKAGE_RESOLVED_PATH="Synara.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved"
PACKAGE_RESOLVED_BACKUP=""
UNSIGNED_BUILD_ARGS=(
  CODE_SIGNING_ALLOWED=NO
  CODE_SIGNING_REQUIRED=NO
  CODE_SIGN_STYLE=Manual
  DEVELOPMENT_TEAM=
)
if [[ "$APPLE_SLICES" == "simulator-arm64" ]]; then
  UNSIGNED_BUILD_ARGS+=(
    ARCHS=arm64
    ONLY_ACTIVE_ARCH=YES
    EXCLUDED_ARCHS=x86_64
  )
fi
case "$TEST_SUITE" in
  all)
    TEST_ONLY_ARGS=()
    PARALLEL_TESTING="${IOS_PARALLEL_TESTING:-NO}"
    MAX_TEST_SIMULATORS="${IOS_MAX_TEST_SIMULATORS:-1}"
    ;;
  unit)
    TEST_ONLY_ARGS=(-only-testing:SynaraTests)
    PARALLEL_TESTING="${IOS_PARALLEL_TESTING:-YES}"
    MAX_TEST_SIMULATORS="${IOS_MAX_TEST_SIMULATORS:-1}"
    ;;
  ui)
    TEST_ONLY_ARGS=(-only-testing:SynaraUITests)
    # Match the serial UI runner: one simulator owns foreground gestures.
    # Explicit caller overrides remain available for controlled experiments.
    PARALLEL_TESTING="${IOS_PARALLEL_TESTING:-NO}"
    MAX_TEST_SIMULATORS="${IOS_MAX_TEST_SIMULATORS:-1}"
    ;;
  *)
    echo "IOS_TEST_SUITE must be all, unit, or ui (got $TEST_SUITE)" >&2
    exit 1
    ;;
esac

cd "$(dirname "$0")/.."

cleanup() {
  if [[ -n "$PACKAGE_RESOLVED_BACKUP" ]]; then
    # Never leave a reviewed lock deleted if XcodeGen or a graph check exits
    # before the normal restore below.
    if [[ ! -f "$PACKAGE_RESOLVED_PATH" ]]; then
      mkdir -p "$(dirname "$PACKAGE_RESOLVED_PATH")"
      cp "$PACKAGE_RESOLVED_BACKUP" "$PACKAGE_RESOLVED_PATH"
    fi
    rm -f "$PACKAGE_RESOLVED_BACKUP"
  fi
}
trap cleanup EXIT

repo_root="$(cd .. && pwd)"
checker="$repo_root/scripts/check-synara-core-swift-scaffold.mjs"
if [[ ! -f "$checker" ]]; then
  echo "SynaraCore Swift scaffold checker is required at $checker" >&2
  exit 127
fi
node "$checker"

nse_checker="$repo_root/scripts/check-synara-nse-core-isolation.mjs"
if [[ ! -f "$nse_checker" ]]; then
  echo "SynaraNseCore isolation checker is required at $nse_checker" >&2
  exit 127
fi
node "$nse_checker"

generator="$repo_root/scripts/generate-synara-core-swift.sh"
if [[ ! -x "$generator" ]]; then
  echo "SynaraCore generator is required at $generator" >&2
  exit 127
fi

# The local SynaraCore package contains generated Swift and a generated binary
# XCFramework. Produce both before XcodeGen resolves the local package so a
# clean checkout cannot compile declarations without their Rust implementation.
"$generator"

nse_generator="$repo_root/scripts/generate-synara-nse-core-swift.sh"
if [[ ! -x "$nse_generator" ]]; then
  echo "SynaraNseCore generator is required at $nse_generator" >&2
  exit 127
fi
SYNARA_NSE_CORE_APPLE_SLICES="${SYNARA_NSE_CORE_APPLE_SLICES:-${SYNARA_CORE_APPLE_SLICES:-all}}" \
  "$nse_generator"

required_synara_core_artifacts=(
  "SynaraCore/Sources/SynaraCore/Generated/synara_core.swift"
  "SynaraCore/Artifacts/SynaraCore.xcframework/Info.plist"
)
for artifact in "${required_synara_core_artifacts[@]}"; do
  if [[ ! -f "$artifact" ]]; then
    echo "SynaraCore generation did not produce required artifact: $artifact" >&2
    exit 1
  fi
done

required_synara_nse_core_artifacts=(
  "SynaraNseCore/Sources/SynaraNseCore/Generated/synara_nse_core.swift"
  "SynaraNseCore/Artifacts/SynaraNseCore.xcframework/Info.plist"
)
for artifact in "${required_synara_nse_core_artifacts[@]}"; do
  if [[ ! -f "$artifact" ]]; then
    echo "SynaraNseCore generation did not produce required artifact: $artifact" >&2
    exit 1
  fi
done
for generated_ffi_file in synara_nse_coreFFI.h module.modulemap; do
  if ! find "SynaraNseCore/Artifacts/SynaraNseCore.xcframework" \
    -path "*/Headers/synara_nse_coreFFI/$generated_ffi_file" -print -quit | grep -q .; then
    echo "SynaraNseCore XCFramework is missing namespaced FFI file: $generated_ffi_file" >&2
    exit 1
  fi
done

# The live test fixture enables full Core only as a dev dependency. Validate
# the same normal/build feature graph used by the generated shipping archive.
node "$repo_root/scripts/check-synara-nse-core-production-features.mjs"
nse_archive_checker="$repo_root/scripts/check-synara-nse-core-archive-exports.sh"
if [[ ! -x "$nse_archive_checker" ]]; then
  echo "SynaraNseCore archive export checker is required at $nse_archive_checker" >&2
  exit 127
fi
nse_archives=()
while IFS= read -r nse_archive; do
  nse_archives+=("$nse_archive")
done < <(find "SynaraNseCore/Artifacts/SynaraNseCore.xcframework" -name 'libsynara_nse_core*.a' -type f)
if [[ "${#nse_archives[@]}" -eq 0 ]]; then
  echo "SynaraNseCore XCFramework is missing libsynara_nse_core archives" >&2
  exit 1
fi
# Decode each published archive architecture with the selected Rust LLVM tools;
# require the NSE ABI export and reject full-Core symbols. Decoder errors fail.
"$nse_archive_checker" "${nse_archives[@]}"
for generated_ffi_file in synara_coreFFI.h module.modulemap; do
  if ! find "SynaraCore/Artifacts/SynaraCore.xcframework" \
    -path "*/Headers/$generated_ffi_file" -print -quit | grep -q .; then
    echo "SynaraCore XCFramework is missing generated FFI file: $generated_ffi_file" >&2
    exit 1
  fi
done

# Host `swift build` links the darwin XCFramework slice. CI simulator/device
# generates omit that slice; xcodebuild below is the product check.
if [[ "${SYNARA_CORE_APPLE_SLICES:-all}" == "all" ]]; then
  (
    cd SynaraCore
    swift build
  )
fi

mkdir -p \
  "$DERIVED_DATA_PATH" \
  "$PACKAGE_CACHE_PATH" \
  "$RESULT_BUNDLE_DIR" \
  "$CLANG_MODULE_CACHE_PATH" \
  "$SWIFTPM_MODULECACHE_OVERRIDE" \
  "$CFFIXED_USER_HOME"

if ! command -v xcodegen >/dev/null 2>&1; then
  echo "xcodegen is required. Install with: brew install xcodegen" >&2
  exit 127
fi

# XcodeGen replaces the generated project directory, including Package.resolved.
# Preserve a reviewed lock when the generated graph contains remote packages.
# An all-local graph has no remote revisions to pin and must not require or carry
# a stale lock from a retired dependency.
if [[ -f "$PACKAGE_RESOLVED_PATH" ]]; then
  PACKAGE_RESOLVED_BACKUP="$(mktemp "${TMPDIR:-/tmp}/synara-package-resolved.XXXXXX")"
  cp "$PACKAGE_RESOLVED_PATH" "$PACKAGE_RESOLVED_BACKUP"
fi
node "$repo_root/scripts/generate-xcode-project.mjs" \
  --project-dir "$PWD" --cache-path "$XCODEGEN_CACHE_PATH"

package_graph_checker="$repo_root/scripts/check-xcode-local-package-graph.mjs"
if [[ ! -f "$package_graph_checker" ]]; then
  echo "Swift package graph checker is required at $package_graph_checker" >&2
  exit 127
fi
package_graph_kind="$(
  node "$package_graph_checker" Synara.xcodeproj/project.pbxproj .
)"
case "$package_graph_kind" in
  local)
    has_remote_package_reference=0
    ;;
  remote)
    has_remote_package_reference=1
    ;;
  *)
    echo "Swift package graph checker returned an invalid result: $package_graph_kind" >&2
    exit 1
    ;;
esac

if [[ "$has_remote_package_reference" == "1" ]]; then
  if [[ -z "$PACKAGE_RESOLVED_BACKUP" ]]; then
    echo "A committed Swift package lock is required when remote packages are present: $PACKAGE_RESOLVED_PATH" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$PACKAGE_RESOLVED_PATH")"
  if [[ ! -f "$PACKAGE_RESOLVED_PATH" ]] || ! cmp -s "$PACKAGE_RESOLVED_BACKUP" "$PACKAGE_RESOLVED_PATH"; then
    cp "$PACKAGE_RESOLVED_BACKUP" "$PACKAGE_RESOLVED_PATH"
  fi
elif [[ -n "$PACKAGE_RESOLVED_BACKUP" ]]; then
  mkdir -p "$(dirname "$PACKAGE_RESOLVED_PATH")"
  cp "$PACKAGE_RESOLVED_BACKUP" "$PACKAGE_RESOLVED_PATH"
  echo "Remove the stale Swift package lock; the generated project has only local packages: $PACKAGE_RESOLVED_PATH" >&2
  exit 1
fi

# Use the same reachable graph and lock policy as local UI and signed builds.
synara_xcode_package_args "$PWD"
PACKAGE_ARGS=(
  "${SYNARA_XCODE_PACKAGE_ARGS[@]}"
  ${SYNARA_XCODE_COMPILATION_ARGS[@]+"${SYNARA_XCODE_COMPILATION_ARGS[@]}"}
  -skipPackagePluginValidation
  -skipMacroValidation
  -skipPackageSignatureValidation
)

xcodebuild \
  -project Synara.xcodeproj \
  -scheme Synara \
  -destination "$BUILD_DESTINATION" \
  -derivedDataPath "$DERIVED_DATA_PATH" \
  -resultBundlePath "$RESULT_BUNDLE_DIR/build-for-testing-$RESULT_STAMP.xcresult" \
  -showBuildTimingSummary \
  "${PACKAGE_ARGS[@]}" \
  build-for-testing \
  "${UNSIGNED_BUILD_ARGS[@]}"

if [[ "${RUN_IOS_TESTS:-0}" == "1" ]]; then
  test_command=(
    xcodebuild
    -project Synara.xcodeproj
    -scheme Synara
    -destination "$TEST_DESTINATION"
    -derivedDataPath "$DERIVED_DATA_PATH"
    -resultBundlePath "$RESULT_BUNDLE_DIR/test-$RESULT_STAMP.xcresult"
    -parallel-testing-enabled "$PARALLEL_TESTING"
    -maximum-concurrent-test-simulator-destinations "$MAX_TEST_SIMULATORS"
  )
  if [[ "$TEST_SUITE" != "all" ]]; then
    test_command+=("${TEST_ONLY_ARGS[@]}")
  fi
  test_command+=(
    "${PACKAGE_ARGS[@]}"
    test-without-building
    "${UNSIGNED_BUILD_ARGS[@]}"
  )
  "${test_command[@]}"
fi

# The simulator product cannot prove the architecture, linkage, or stripped
# size of the extension that ships to users. PR CI opts into this second pass
# after simulator tests so the final arm64 Release appex is checked before a
# TestFlight archive is attempted.
if [[ "${CHECK_IOS_DEVICE_RELEASE:-0}" == "1" ]]; then
  SYNARA_CORE_APPLE_SLICES=device "$generator"
  SYNARA_NSE_CORE_APPLE_SLICES=device "$nse_generator"

  xcodebuild \
    -project Synara.xcodeproj \
    -scheme Synara \
    -configuration Release \
    -destination "generic/platform=iOS" \
    -derivedDataPath "$DEVICE_DERIVED_DATA_PATH" \
    -resultBundlePath "$RESULT_BUNDLE_DIR/device-release-$RESULT_STAMP.xcresult" \
    -showBuildTimingSummary \
    "${PACKAGE_ARGS[@]}" \
    build \
    "${UNSIGNED_BUILD_ARGS[@]}"

  scripts/check-notification-service-archive.sh \
    "$DEVICE_DERIVED_DATA_PATH/Build/Products/Release-iphoneos/Synara.app" \
    "$RESULT_BUNDLE_DIR"
fi
