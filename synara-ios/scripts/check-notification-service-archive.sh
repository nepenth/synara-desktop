#!/usr/bin/env bash
set -euo pipefail

product_path="${1:-}"
diagnostics_dir="${2:-}"
max_executable_bytes="${SYNARA_NSE_MAX_EXECUTABLE_BYTES:-25000000}"

if [[ -z "$product_path" || ! -d "$product_path" ]]; then
  echo "Usage: $0 <Synara.xcarchive|Synara.app> [diagnostics-directory]" >&2
  exit 1
fi

if [[ "$product_path" == *.app ]]; then
  app_path="$product_path"
else
  app_path="$product_path/Products/Applications/Synara.app"
fi
appex="$app_path/PlugIns/SynaraNotificationService.appex"
if [[ ! -d "$appex" ]]; then
  echo "Release archive is missing SynaraNotificationService.appex: $appex" >&2
  exit 1
fi

executable_name="$(plutil -extract CFBundleExecutable raw "$appex/Info.plist")"
executable="$appex/$executable_name"
if [[ ! -f "$executable" ]]; then
  echo "Release archive is missing the notification-service executable: $executable" >&2
  exit 1
fi

executable_bytes="$(stat -f '%z' "$executable")"
if [[ "$executable_bytes" -gt "$max_executable_bytes" ]]; then
  echo "Notification-service executable is ${executable_bytes} bytes; budget is ${max_executable_bytes}." >&2
  exit 1
fi

file_output="$(file "$executable")"
if [[ "$file_output" != *"arm64"* ]]; then
  echo "Notification-service executable has no arm64 device slice: $file_output" >&2
  exit 1
fi

linked_images="$(otool -L "$executable")"
if grep -Eiq 'SynaraCore|libsynara_core' <<<"$linked_images"; then
  echo "Notification-service executable links the forbidden full SynaraCore image." >&2
  exit 1
fi

if LC_ALL=C grep -a -q '_uniffi_synara_core_' "$executable"; then
  echo "Notification-service executable contains forbidden full Core UniFFI exports." >&2
  exit 1
fi

report="notification_service_archive_report.txt"
if [[ -n "$diagnostics_dir" ]]; then
  mkdir -p "$diagnostics_dir"
  report="$diagnostics_dir/$report"
fi
{
  echo "appex=$appex"
  echo "executable=$executable"
  echo "executable_bytes=$executable_bytes"
  echo "disk_budget_bytes=$max_executable_bytes"
  echo "note=disk size is a regression guard, not physical phys_footprint proof"
  echo "$file_output"
  echo "$linked_images"
} > "$report"

echo "Notification-service archive checks passed (${executable_bytes} bytes)."
