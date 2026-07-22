#!/usr/bin/env bash
set -euo pipefail

DERIVED_DATA_PATH="${DERIVED_DATA_PATH:-/private/tmp/synara-ios-derived}"
PACKAGE_CACHE_PATH="${IOS_PACKAGE_CACHE_PATH:-/private/tmp/synara-ios-package-cache}"
RESULT_BUNDLE_DIR="${IOS_RESULT_BUNDLE_DIR:-/private/tmp/synara-ios-results}"
RESULT_STAMP="${IOS_RESULT_STAMP:-$(date +%Y%m%d-%H%M%S)-$$}"
BUILD_DESTINATION="${IOS_BUILD_DESTINATION:-generic/platform=iOS Simulator}"
TEST_DESTINATION="${IOS_TEST_DESTINATION:-platform=iOS Simulator,name=iPhone 17}"
CLONED_SOURCE_PACKAGES_DIR_PATH="${IOS_CLONED_SOURCE_PACKAGES_DIR_PATH:-}"
export CLANG_MODULE_CACHE_PATH="${CLANG_MODULE_CACHE_PATH:-/private/tmp/synara-ios-module-cache}"
export SWIFTPM_MODULECACHE_OVERRIDE="${SWIFTPM_MODULECACHE_OVERRIDE:-/private/tmp/synara-ios-swiftpm-module-cache}"
export CFFIXED_USER_HOME="${CFFIXED_USER_HOME:-/private/tmp/synara-ios-home}"
UNSIGNED_BUILD_ARGS=(
  CODE_SIGNING_ALLOWED=NO
  CODE_SIGNING_REQUIRED=NO
  CODE_SIGN_STYLE=Manual
  DEVELOPMENT_TEAM=
)
PACKAGE_ARGS=(
  -packageCachePath "$PACKAGE_CACHE_PATH"
  -onlyUsePackageVersionsFromResolvedFile
  -skipPackageUpdates
  -scmProvider system
  -skipPackagePluginValidation
  -skipMacroValidation
  -skipPackageSignatureValidation
)

if [[ -n "$CLONED_SOURCE_PACKAGES_DIR_PATH" ]]; then
  PACKAGE_ARGS+=(-clonedSourcePackagesDirPath "$CLONED_SOURCE_PACKAGES_DIR_PATH")
fi

cd "$(dirname "$0")/.."

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

xcodegen generate --spec project.yml

xcodebuild \
  -project Synara.xcodeproj \
  -scheme Synara \
  -destination "$BUILD_DESTINATION" \
  -derivedDataPath "$DERIVED_DATA_PATH" \
  -resultBundlePath "$RESULT_BUNDLE_DIR/build-$RESULT_STAMP.xcresult" \
  "${PACKAGE_ARGS[@]}" \
  build \
  "${UNSIGNED_BUILD_ARGS[@]}"

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
  xcodebuild \
    -project Synara.xcodeproj \
    -scheme Synara \
    -destination "$TEST_DESTINATION" \
    -derivedDataPath "$DERIVED_DATA_PATH" \
    -resultBundlePath "$RESULT_BUNDLE_DIR/test-$RESULT_STAMP.xcresult" \
    -parallel-testing-enabled NO \
    -maximum-concurrent-test-simulator-destinations 1 \
    -retry-tests-on-failure \
    -test-iterations 2 \
    "${PACKAGE_ARGS[@]}" \
    test \
    "${UNSIGNED_BUILD_ARGS[@]}"
fi
