#!/usr/bin/env bash
set -euo pipefail

DERIVED_DATA_PATH="${DERIVED_DATA_PATH:-/private/tmp/synara-ios-derived}"
BUILD_DESTINATION="${IOS_BUILD_DESTINATION:-generic/platform=iOS Simulator}"
TEST_DESTINATION="${IOS_TEST_DESTINATION:-platform=iOS Simulator,name=iPhone 16}"

cd "$(dirname "$0")/.."

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
  build

xcodebuild \
  -project Synara.xcodeproj \
  -scheme Synara \
  -destination "$BUILD_DESTINATION" \
  -derivedDataPath "$DERIVED_DATA_PATH" \
  build-for-testing

if [[ "${RUN_IOS_TESTS:-0}" == "1" ]]; then
  xcodebuild \
    -project Synara.xcodeproj \
    -scheme Synara \
    -destination "$TEST_DESTINATION" \
    -derivedDataPath "$DERIVED_DATA_PATH" \
    test
fi
