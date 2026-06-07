#!/usr/bin/env bash
set -euo pipefail

DERIVED_DATA_PATH="${DERIVED_DATA_PATH:-/private/tmp/synara-ios-ui-tests-derived}"
TEST_DESTINATION="${IOS_TEST_DESTINATION:-platform=iOS Simulator,name=iPhone 17 Pro}"
SHARD="${SYNARA_UI_TEST_SHARD:-all}"

UNSIGNED_BUILD_ARGS=(
  CODE_SIGNING_ALLOWED=NO
  CODE_SIGNING_REQUIRED=NO
  CODE_SIGN_STYLE=Manual
  DEVELOPMENT_TEAM=
)

COMMON_XCODEBUILD_ARGS=(
  -project Synara.xcodeproj
  -scheme Synara
  -destination "$TEST_DESTINATION"
  -derivedDataPath "$DERIVED_DATA_PATH"
  -parallel-testing-enabled NO
  -maximum-concurrent-test-simulator-destinations 1
  -test-timeouts-enabled YES
  -default-test-execution-time-allowance 60
  -collect-test-diagnostics never
  COMPILER_INDEX_STORE_ENABLE=NO
  ONLY_ACTIVE_ARCH=YES
)

cd "$(dirname "$0")/.."

run_shard() {
  local shard_name="$1"
  shift
  local tests=("$@")
  local only_testing_args=()

  for test_name in "${tests[@]}"; do
    only_testing_args+=("-only-testing:SynaraUITests/SynaraUITests/${test_name}")
  done

  echo "Running SynaraUITests shard: ${shard_name}"
  xcodebuild \
    "${COMMON_XCODEBUILD_ARGS[@]}" \
    "${only_testing_args[@]}" \
    test-without-building \
    "${UNSIGNED_BUILD_ARGS[@]}"
}

echo "Building Synara for UI testing"
xcodebuild \
  "${COMMON_XCODEBUILD_ARGS[@]}" \
  build-for-testing \
  "${UNSIGNED_BUILD_ARGS[@]}"

auth_and_rooms=(
  testShellShowsHomeserverSelectionWhenSignedOut
  testInvalidHomeserverShowsErrorBeforeNavigation
  testValidHomeserverNavigatesToLoginPlaceholder
  testLoginValidationShowsNonSensitiveError
  testSuccessfulMockLoginShowsSignedInShell
  testRoomListShowsStableRoomRows
  testRoomHeaderAccountMenuShowsSettingsAndLogout
  testRoomManagementCreatesPrivateEncryptedRoom
  testRoomSearchFiltersByName
  testSpaceFilterScopesRoomList
  testRoomManagementPublicDirectorySearchMockFlow
)

timeline_and_composer=(
  testRoomRouteShowsTimeline
  testRoomDetailsInviteAndLeaveMockFlow
  testRoomDetailsProfileEditMockFlow
  testLargeRoomFixtureRendersAndScrolls
  testLargeTimelineFixtureRendersAndScrolls
  testComposerSendsMockMessage
  testMediaUploadAddsAttachmentPlaceholder
  testUnavailableAttachmentOptionShowsHonestState
  testThreadViewOpensAndRepliesFromTimeline
  testEncryptedTimelineShowsCryptoStatusRecoveryBannerAndSafePlaceholder
)

settings_and_workflows=(
  testLogoutReturnsToSignedOutShell
  testSettingsShowsNotificationSectionsAndReleaseLinks
  testSettingsShowsEncryptedRecoveryControlsWhenNeeded
  testAboutScreenShowsVersionBuildLicenseSupportAndPrivacyLinks
  testAcceptInviteTransitionsRowToJoinedRoom
  testRejectInviteRemovesInviteRow
  testLaterListRendersStatesAndUnavailableDestinations
  testLaterItemNavigatesToRoomAnchor
  testAgentCardApproveActionShowsSubmittedState
  testAgentCardApprovalFailureIsVisibleAndRetryable
)

live_and_visual=(
  testLiveSmokeWhenConfigured
  testLiveAgentApprovalSmokeWhenConfigured
  testLiveEncryptedRoomSmokeWhenConfigured
  testLiveRoomManagementSmokeWhenConfigured
  testLiveVisualMockupScreenshotsWhenConfigured
  testMockThreadVisualScreenshotWhenConfigured
  testMockAgentVisualScreenshotWhenConfigured
  testMockRoomsVisualScreenshotsWhenConfigured
)

case "$SHARD" in
  all)
    run_shard auth-and-rooms "${auth_and_rooms[@]}"
    run_shard timeline-and-composer "${timeline_and_composer[@]}"
    run_shard settings-and-workflows "${settings_and_workflows[@]}"
    run_shard live-and-visual "${live_and_visual[@]}"
    ;;
  auth-and-rooms)
    run_shard auth-and-rooms "${auth_and_rooms[@]}"
    ;;
  timeline-and-composer)
    run_shard timeline-and-composer "${timeline_and_composer[@]}"
    ;;
  settings-and-workflows)
    run_shard settings-and-workflows "${settings_and_workflows[@]}"
    ;;
  live-and-visual)
    run_shard live-and-visual "${live_and_visual[@]}"
    ;;
  *)
    echo "Unknown SYNARA_UI_TEST_SHARD '${SHARD}'." >&2
    echo "Expected all, auth-and-rooms, timeline-and-composer, settings-and-workflows, or live-and-visual." >&2
    exit 2
    ;;
esac
