#!/usr/bin/env bash
set -euo pipefail

DERIVED_DATA_PATH="${DERIVED_DATA_PATH:-/private/tmp/synara-ios-derived}"
PACKAGE_CACHE_PATH="${IOS_PACKAGE_CACHE_PATH:-/private/tmp/synara-ios-package-cache}"
RESULT_BUNDLE_DIR="${IOS_RESULT_BUNDLE_DIR:-/private/tmp/synara-ios-results}"
RESULT_STAMP="${IOS_RESULT_STAMP:-$(date +%Y%m%d-%H%M%S)-$$}"
BUILD_DESTINATION="${IOS_BUILD_DESTINATION:-generic/platform=iOS Simulator}"
TEST_DESTINATION="${IOS_TEST_DESTINATION:-platform=iOS Simulator,name=iPhone 17}"
TEST_SUITE="${IOS_TEST_SUITE:-all}"
DEVICE_DERIVED_DATA_PATH="${IOS_DEVICE_DERIVED_DATA_PATH:-/private/tmp/synara-ios-device-derived}"
CLONED_SOURCE_PACKAGES_DIR_PATH="${IOS_CLONED_SOURCE_PACKAGES_DIR_PATH:-}"
export CLANG_MODULE_CACHE_PATH="${CLANG_MODULE_CACHE_PATH:-/private/tmp/synara-ios-module-cache}"
export SWIFTPM_MODULECACHE_OVERRIDE="${SWIFTPM_MODULECACHE_OVERRIDE:-/private/tmp/synara-ios-swiftpm-module-cache}"
export CFFIXED_USER_HOME="${CFFIXED_USER_HOME:-/private/tmp/synara-ios-home}"
PACKAGE_RESOLVED_PATH="Synara.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved"
PACKAGE_RESOLVED_BACKUP=""
UNSIGNED_BUILD_ARGS=(
  CODE_SIGNING_ALLOWED=NO
  CODE_SIGNING_REQUIRED=NO
  CODE_SIGN_STYLE=Manual
  DEVELOPMENT_TEAM=
)
PACKAGE_ARGS=(
  -packageCachePath "$PACKAGE_CACHE_PATH"
  -scmProvider system
  -skipPackagePluginValidation
  -skipMacroValidation
  -skipPackageSignatureValidation
)

if [[ -n "$CLONED_SOURCE_PACKAGES_DIR_PATH" ]]; then
  PACKAGE_ARGS+=(-clonedSourcePackagesDirPath "$CLONED_SOURCE_PACKAGES_DIR_PATH")
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
    PARALLEL_TESTING="${IOS_PARALLEL_TESTING:-YES}"
    MAX_TEST_SIMULATORS="${IOS_MAX_TEST_SIMULATORS:-2}"
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

if cargo tree -p synara-nse-core -e features | grep -q 'synara-core feature "full-uniffi"'; then
  echo "SynaraNseCore must not enable the full Core UniFFI feature" >&2
  exit 1
fi
while IFS= read -r nse_archive; do
  if nm -gU "$nse_archive" 2>/dev/null | grep -q '_uniffi_synara_core_'; then
    echo "SynaraNseCore archive contains forbidden full Core exports: $nse_archive" >&2
    exit 1
  fi
done < <(find "SynaraNseCore/Artifacts/SynaraNseCore.xcframework" -name 'libsynara_nse_core*.a' -type f)
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
xcodegen generate --spec project.yml

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
  cp "$PACKAGE_RESOLVED_BACKUP" "$PACKAGE_RESOLVED_PATH"
  PACKAGE_ARGS+=(
    -onlyUsePackageVersionsFromResolvedFile
    -skipPackageUpdates
  )
elif [[ -n "$PACKAGE_RESOLVED_BACKUP" ]]; then
  mkdir -p "$(dirname "$PACKAGE_RESOLVED_PATH")"
  cp "$PACKAGE_RESOLVED_BACKUP" "$PACKAGE_RESOLVED_PATH"
  echo "Remove the stale Swift package lock; the generated project has only local packages: $PACKAGE_RESOLVED_PATH" >&2
  exit 1
fi

xcodebuild \
  -project Synara.xcodeproj \
  -scheme Synara \
  -destination "$BUILD_DESTINATION" \
  -derivedDataPath "$DERIVED_DATA_PATH" \
  -resultBundlePath "$RESULT_BUNDLE_DIR/build-for-testing-$RESULT_STAMP.xcresult" \
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
    "${PACKAGE_ARGS[@]}" \
    build \
    "${UNSIGNED_BUILD_ARGS[@]}"

  scripts/check-notification-service-archive.sh \
    "$DEVICE_DERIVED_DATA_PATH/Build/Products/Release-iphoneos/Synara.app" \
    "$RESULT_BUNDLE_DIR"
fi
