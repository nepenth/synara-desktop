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

repo_root="$(cd .. && pwd)"
checker="$repo_root/scripts/check-synara-core-swift-scaffold.mjs"
if [[ ! -f "$checker" ]]; then
  echo "SynaraCore Swift scaffold checker is required at $checker" >&2
  exit 127
fi
node "$checker"

generator="$repo_root/scripts/generate-synara-core-swift.sh"
if [[ ! -x "$generator" ]]; then
  echo "SynaraCore generator is required at $generator" >&2
  exit 127
fi

# The local SynaraCore package contains generated Swift and a generated binary
# XCFramework. Produce both before XcodeGen resolves the local package so a
# clean checkout cannot compile declarations without their Rust implementation.
"$generator"

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
for generated_ffi_file in synara_coreFFI.h module.modulemap; do
  if ! find "SynaraCore/Artifacts/SynaraCore.xcframework" \
    -path "*/Headers/$generated_ffi_file" -print -quit | grep -q .; then
    echo "SynaraCore XCFramework is missing generated FFI file: $generated_ffi_file" >&2
    exit 1
  fi
done

# This catches a malformed local binary target before XcodeGen. xcodebuild
# below still validates the actual unsigned simulator app and test bundles.
(
  cd SynaraCore
  swift build
)

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
