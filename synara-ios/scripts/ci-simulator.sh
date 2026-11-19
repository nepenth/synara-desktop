#!/usr/bin/env bash
# Simulator setup for the iOS CI test jobs.
#
#   ci-simulator.sh settle
#       Start CoreSimulator and wait until every usable iOS runtime is Ready and
#       its volume is mounted, then print runtimes, device types and devices.
#   ci-simulator.sh boot <iOS version> <device type name>
#       Create a fresh device on exactly that runtime, boot it, wait until it has
#       finished booting, and print its UDID (the only line on stdout).
#   ci-simulator.sh diagnose
#       Print runtime state and the simdiskimaged/cryptexd log after a failure.
#
# Why: on the Xcode 27 runner image, simdiskimaged tidies up duplicate runtime
# registrations shortly after CoreSimulator first starts, and can unmount the
# volume a booted simulator runs from (actions/runner-images#14946, #15334;
# Apple lists it as a known issue in the Xcode 27.3 release notes, 191574717).
# Settling at the start of the job lets that happen during the Rust build, not
# under a booted simulator; booting an explicit runtime and waiting for it means
# a missing or broken runtime fails here with a clear message, not as
# "runtime path not found" inside xcodebuild.
set -euo pipefail

log() { echo "ci-simulator: $*" >&2; }

runtime_report() {
  xcrun simctl runtime list -j | python3 -I -c '
import json, os, sys
entries = json.load(sys.stdin).values()
ios = [e for e in entries if "iphonesimulator" in e.get("platformIdentifier", "").lower()]
pending = 0
for e in sorted(ios, key=lambda e: e.get("version", "")):
    state = e.get("state", "?")
    mount = e.get("mountPath") or ""
    mounted = bool(mount) and os.path.isdir(mount)
    usable = not state.startswith("Unusable")
    if usable and not (state == "Ready" and mounted):
        pending += 1
    version, build, ident = e.get("version"), e.get("build"), e.get("identifier", "")
    print(f"  iOS {version} ({build}) state={state} mounted={mounted} {ident}", file=sys.stderr)
if not ios:
    print("  no iOS simulator runtimes registered", file=sys.stderr)
    pending += 1
print(pending)
'
}

settle() {
  local deadline=$((SECONDS + ${CI_SIMULATOR_SETTLE_SECONDS:-300}))
  log "waiting for iOS simulator runtimes to be Ready and mounted"
  while true; do
    local pending
    pending="$(runtime_report)"
    if [[ "$pending" == "0" ]]; then
      break
    fi
    if (( SECONDS >= deadline )); then
      log "iOS simulator runtimes did not settle"
      diagnose
      return 1
    fi
    sleep 10
  done
  xcrun simctl runtime list >&2
  xcrun simctl list runtimes >&2
  xcrun simctl list devicetypes | grep -E '^(== |iPhone)' >&2 || true
  xcrun simctl list devices available >&2
}

boot() {
  local version="$1" device_type_name="$2"
  settle
  local runtime_id device_type_id
  runtime_id="$(xcrun simctl list runtimes -j | python3 -I -c '
import json, sys
want = sys.argv[1]
for r in json.load(sys.stdin)["runtimes"]:
    if r.get("platform") == "iOS" and r.get("version") == want and r.get("isAvailable"):
        print(r["identifier"]); break
' "$version")"
  if [[ -z "$runtime_id" ]]; then
    log "no available iOS $version simulator runtime for $(xcodebuild -version | head -1)"
    diagnose
    return 1
  fi
  device_type_id="$(xcrun simctl list devicetypes -j | python3 -I -c '
import json, sys
want = sys.argv[1]
for d in json.load(sys.stdin)["devicetypes"]:
    if d.get("name") == want:
        print(d["identifier"]); break
' "$device_type_name")"
  if [[ -z "$device_type_id" ]]; then
    log "no simulator device type named '$device_type_name'"
    return 1
  fi
  local udid
  udid="$(xcrun simctl create "Synara CI $device_type_name" "$device_type_id" "$runtime_id")"
  log "booting $device_type_name on $runtime_id ($udid)"
  local started=$SECONDS
  if ! xcrun simctl bootstatus "$udid" -b >&2; then
    log "simulator $udid did not finish booting"
    diagnose
    return 1
  fi
  log "booted in $((SECONDS - started)) s"
  echo "$udid"
}

diagnose() {
  log "runtime state"
  xcrun simctl runtime list >&2 || true
  xcrun simctl list devices >&2 || true
  log "simdiskimaged and cryptexd, last 90 minutes"
  /usr/bin/log show --last 90m --style compact \
    --predicate 'process == "simdiskimaged" OR process == "cryptexd"' 2>/dev/null \
    | grep -iE 'unmount|duplicate|eject|fail|error|prefer' | tail -n 200 >&2 || true
}

case "${1:-}" in
  settle) settle ;;
  boot)
    [[ $# -eq 3 ]] || { log "usage: $0 boot <iOS version> <device type name>"; exit 2; }
    boot "$2" "$3"
    ;;
  diagnose) diagnose ;;
  *) log "usage: $0 settle | boot <iOS version> <device type name> | diagnose"; exit 2 ;;
esac
