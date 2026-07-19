#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

SCHEME="${SYNARA_IOS_SCHEME:-Synara}"
CONFIGURATION="${SYNARA_IOS_CONFIGURATION:-Release}"
TEAM_ID="${SYNARA_IOS_TEAM_ID:-}"
BUNDLE_ID="${SYNARA_IOS_BUNDLE_ID:-com.whylandcreative.synara}"
PROVISIONING_PROFILE="${SYNARA_IOS_PROVISIONING_PROFILE:-}"
NOTIFICATION_SERVICE_BUNDLE_ID="${SYNARA_IOS_NOTIFICATION_SERVICE_BUNDLE_ID:-${BUNDLE_ID}.NotificationService}"
NOTIFICATION_SERVICE_PROVISIONING_PROFILE="${SYNARA_IOS_NOTIFICATION_SERVICE_PROVISIONING_PROFILE:-}"
PUSH_GATEWAY_URL="${SYNARA_PUSH_GATEWAY_URL:-}"
ARCHIVE_ROOT="${SYNARA_IOS_ARCHIVE_ROOT:-/tmp}"

require_env() {
  local name="$1"
  local value="$2"
  if [[ -z "$value" ]]; then
    echo "Set ${name} before running the TestFlight upload script." >&2
    exit 1
  fi
}

require_env "SYNARA_IOS_TEAM_ID" "$TEAM_ID"
require_env "SYNARA_IOS_PROVISIONING_PROFILE" "$PROVISIONING_PROFILE"
require_env \
  "SYNARA_IOS_NOTIFICATION_SERVICE_PROVISIONING_PROFILE" \
  "$NOTIFICATION_SERVICE_PROVISIONING_PROFILE"
require_env "SYNARA_PUSH_GATEWAY_URL" "$PUSH_GATEWAY_URL"

TESTFLIGHT_INTERNAL_ONLY="${SYNARA_TESTFLIGHT_INTERNAL_ONLY:-true}"
if [[ "$TESTFLIGHT_INTERNAL_ONLY" != "true" && "$TESTFLIGHT_INTERNAL_ONLY" != "false" ]]; then
  echo "SYNARA_TESTFLIGHT_INTERNAL_ONLY must be 'true' or 'false'." >&2
  exit 1
fi
xcode_auth_args=()
has_xcode_auth_args=0
if [[ -n "${SYNARA_ASC_KEY_PATH:-}" || -n "${SYNARA_ASC_KEY_ID:-}" || -n "${SYNARA_ASC_ISSUER_ID:-}" ]]; then
  if [[ -z "${SYNARA_ASC_KEY_PATH:-}" || -z "${SYNARA_ASC_KEY_ID:-}" || -z "${SYNARA_ASC_ISSUER_ID:-}" ]]; then
    echo "Set SYNARA_ASC_KEY_PATH, SYNARA_ASC_KEY_ID, and SYNARA_ASC_ISSUER_ID together." >&2
    exit 1
  fi
  xcode_auth_args=(
    -authenticationKeyPath "$SYNARA_ASC_KEY_PATH"
    -authenticationKeyID "$SYNARA_ASC_KEY_ID"
    -authenticationKeyIssuerID "$SYNARA_ASC_ISSUER_ID"
  )
  has_xcode_auth_args=1
fi

run_xcodebuild() {
  if [[ "$has_xcode_auth_args" == "1" ]]; then
    xcodebuild "$@" "${xcode_auth_args[@]}"
    return
  fi
  xcodebuild "$@"
}

read_build_setting() {
  local key="$1"
  awk -F '= ' -v setting="$key" '$1 ~ setting"[[:space:]]*$" { print $2; exit }'
}

build_settings="$(
  xcodebuild \
    -project "$PROJECT_DIR/Synara.xcodeproj" \
    -scheme "$SCHEME" \
    -configuration "$CONFIGURATION" \
    -destination "generic/platform=iOS" \
    DEVELOPMENT_TEAM="$TEAM_ID" \
    SYNARA_IOS_PROVISIONING_PROFILE="$PROVISIONING_PROFILE" \
    SYNARA_IOS_NOTIFICATION_SERVICE_PROVISIONING_PROFILE="$NOTIFICATION_SERVICE_PROVISIONING_PROFILE" \
    SYNARA_PUSH_GATEWAY_URL="$PUSH_GATEWAY_URL" \
    -showBuildSettings
)"

marketing_version="$(printf "%s" "$build_settings" | read_build_setting MARKETING_VERSION)"
build_number="$(printf "%s" "$build_settings" | read_build_setting CURRENT_PROJECT_VERSION)"

if [[ -z "$marketing_version" || -z "$build_number" ]]; then
  echo "Unable to resolve MARKETING_VERSION or CURRENT_PROJECT_VERSION from Xcode build settings." >&2
  exit 1
fi

archive_path="$ARCHIVE_ROOT/Synara-${marketing_version}-${build_number}.xcarchive"
export_path="$ARCHIVE_ROOT/Synara-${marketing_version}-${build_number}-export"
export_options="$(mktemp "${TMPDIR:-/tmp}/synara-export-options.XXXXXX.plist")"
notification_service_profile_entry=""

if [[ -n "$NOTIFICATION_SERVICE_PROVISIONING_PROFILE" ]]; then
  notification_service_profile_entry="
		<key>${NOTIFICATION_SERVICE_BUNDLE_ID}</key>
		<string>${NOTIFICATION_SERVICE_PROVISIONING_PROFILE}</string>"
fi

cleanup() {
  rm -f "$export_options"
}
trap cleanup EXIT

cat > "$export_options" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>destination</key>
	<string>upload</string>
	<key>manageAppVersionAndBuildNumber</key>
	<false/>
	<key>method</key>
	<string>app-store-connect</string>
	<key>provisioningProfiles</key>
	<dict>
		<key>${BUNDLE_ID}</key>
		<string>${PROVISIONING_PROFILE}</string>${notification_service_profile_entry}
	</dict>
	<key>signingStyle</key>
	<string>manual</string>
	<key>teamID</key>
	<string>${TEAM_ID}</string>
	<key>testFlightInternalTestingOnly</key>
	<${TESTFLIGHT_INTERNAL_ONLY}/>
	<key>uploadSymbols</key>
	<true/>
</dict>
</plist>
PLIST

echo "Archiving Synara ${marketing_version} (${build_number}) to ${archive_path}"
run_xcodebuild \
  -project "$PROJECT_DIR/Synara.xcodeproj" \
  -scheme "$SCHEME" \
  -configuration "$CONFIGURATION" \
  -destination "generic/platform=iOS" \
  -archivePath "$archive_path" \
  DEVELOPMENT_TEAM="$TEAM_ID" \
  SYNARA_IOS_PROVISIONING_PROFILE="$PROVISIONING_PROFILE" \
  SYNARA_IOS_NOTIFICATION_SERVICE_PROVISIONING_PROFILE="$NOTIFICATION_SERVICE_PROVISIONING_PROFILE" \
  SYNARA_PUSH_GATEWAY_URL="$PUSH_GATEWAY_URL" \
  archive \
  -allowProvisioningUpdates

echo "Uploading Synara ${marketing_version} (${build_number}) to App Store Connect"
run_xcodebuild \
  -exportArchive \
  -archivePath "$archive_path" \
  -exportOptionsPlist "$export_options" \
  -exportPath "$export_path" \
  -allowProvisioningUpdates

echo "Upload complete. Wait for App Store Connect processing, then install from TestFlight."
