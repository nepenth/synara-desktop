#!/usr/bin/env bash
# Validate restricted capability opt-ins against the final code signatures.
set -euo pipefail
app_path="${1:?Usage: check-notification-capabilities.sh <Synara.app>}"
appex="$app_path/PlugIns/SynaraNotificationService.appex"
for bundle in "$app_path" "$appex"; do
  if ! plutil -lint "$bundle/Info.plist" >/dev/null 2>&1; then
    echo "Notification capability validation requires valid app and extension Info.plists." >&2
    exit 1
  fi
done

flag_enabled() {
  local value
  value="$(plutil -extract "$2" raw -o - "$1/Info.plist" 2>/dev/null || true)"
  case "$value" in
    YES|yes|TRUE|true|1) return 0 ;;
    NO|no|FALSE|false|0|'') return 1 ;;
    *) echo "Unresolved or invalid notification capability build flag: $2" >&2; exit 1 ;;
  esac
}

require_signed_entitlement() {
  local signed_xml value value_type key_path
  if ! signed_xml="$(codesign -d --entitlements :- "$1" 2>/dev/null)"; then
    echo "Cannot inspect the final $3 signature for notification capabilities." >&2
    exit 1
  fi
  # Entitlement names contain literal dots; plutil otherwise interprets these
  # as a nested key path.
  key_path="${2//./\\.}"
  value_type="$(plutil -type "$key_path" - <<< "$signed_xml" 2>/dev/null || true)"
  value="$(plutil -extract "$key_path" raw -o - - <<< "$signed_xml" 2>/dev/null || true)"
  if [[ "$value_type" != bool || "$value" != true ]]; then
    echo "$3 enables a restricted notification capability without its signed entitlement: $2" >&2
    exit 1
  fi
}

app_critical=0
extension_critical=0
app_filtering=0
extension_filtering=0
if flag_enabled "$app_path" SynaraCriticalAlertsEnabled; then app_critical=1; fi
if flag_enabled "$appex" SynaraCriticalAlertsEnabled; then extension_critical=1; fi
if flag_enabled "$app_path" SynaraNotificationFilteringEnabled; then app_filtering=1; fi
if flag_enabled "$appex" SynaraNotificationFilteringEnabled; then extension_filtering=1; fi
if [[ "$app_critical" != "$extension_critical" || "$app_filtering" != "$extension_filtering" ]]; then
  echo "App and notification-service capability build flags disagree." >&2
  exit 1
fi
if [[ "$app_critical" == 1 ]]; then
  require_signed_entitlement "$app_path" com.apple.developer.usernotifications.critical-alerts App
  require_signed_entitlement "$appex" com.apple.developer.usernotifications.critical-alerts Notification-service
fi
if [[ "$extension_filtering" == 1 ]]; then
  require_signed_entitlement "$appex" com.apple.developer.usernotifications.filtering Notification-service
fi
echo "Notification capability flags match required signed entitlements."
