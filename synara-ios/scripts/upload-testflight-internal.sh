#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

SCHEME="${SYNARA_IOS_SCHEME:-Synara}"
CONFIGURATION="${SYNARA_IOS_CONFIGURATION:-Release}"
TEAM_ID="${SYNARA_IOS_TEAM_ID:-ABC123DEFG}"
BUNDLE_ID="${SYNARA_IOS_BUNDLE_ID:-com.whylandcreative.synara}"
PROVISIONING_PROFILE="${SYNARA_IOS_PROVISIONING_PROFILE:-Synara Matrix App Store}"
ARCHIVE_ROOT="${SYNARA_IOS_ARCHIVE_ROOT:-/tmp}"

xcode_auth_args=()
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
fi

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
		<string>${PROVISIONING_PROFILE}</string>
	</dict>
	<key>signingStyle</key>
	<string>manual</string>
	<key>teamID</key>
	<string>${TEAM_ID}</string>
	<key>testFlightInternalTestingOnly</key>
	<true/>
	<key>uploadSymbols</key>
	<true/>
</dict>
</plist>
PLIST

echo "Archiving Synara ${marketing_version} (${build_number}) to ${archive_path}"
xcodebuild \
  -project "$PROJECT_DIR/Synara.xcodeproj" \
  -scheme "$SCHEME" \
  -configuration "$CONFIGURATION" \
  -destination "generic/platform=iOS" \
  -archivePath "$archive_path" \
  archive \
  -allowProvisioningUpdates \
  "${xcode_auth_args[@]}"

echo "Uploading Synara ${marketing_version} (${build_number}) to App Store Connect"
xcodebuild \
  -exportArchive \
  -archivePath "$archive_path" \
  -exportOptionsPlist "$export_options" \
  -exportPath "$export_path" \
  -allowProvisioningUpdates \
  "${xcode_auth_args[@]}"

echo "Upload complete. Wait for App Store Connect processing, then install from TestFlight."
