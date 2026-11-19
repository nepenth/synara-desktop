#!/usr/bin/env bash
# Simulator setup for the iOS CI test jobs and the simulator smoke workflow.
#
#   ci-simulator.sh settle
#       Start CoreSimulator and wait until every usable iOS runtime is Ready and
#       its volume is mounted, then print runtimes, device types and devices.
#   ci-simulator.sh prepare <iOS version> <device type name>
#       Settle, pick the runtime of that version that runs that device type,
#       and create a fresh device on it without booting it. Prints its UDID
#       (the only line on stdout).
#   ci-simulator.sh ready <udid>
#       Boot that device, wait until it has finished booting, and check it.
#   ci-simulator.sh boot <iOS version> <device type name>
#       prepare, then ready; prints the UDID.
#   ci-simulator.sh check <udid>
#       Fail unless that device is booted, its runtime is still mounted and it
#       answers a command.
#   ci-simulator.sh smoke <udid>
#       Prove the device works without building anything: run launchctl in it
#       and launch the built-in Settings app.
#   ci-simulator.sh remove <udid>
#       Shut the device down and delete it.
#   ci-simulator.sh diagnose
#       Print runtime state and the simdiskimaged/cryptexd log after a failure.
#
# Why: on the Xcode 27 runner image, simdiskimaged tidies up duplicate runtime
# registrations after CoreSimulator starts, and can unmount the volume a booted
# simulator runs from (actions/runner-images#14946, #15334; Apple lists it as a
# known issue in the Xcode 27.3 release notes, 191574717). The CI jobs run
# prepare as their first step, so a missing runtime or a runtime/device pair
# that does not fit fails in minutes with this script's diagnostics, and boot
# the device with ready only right before testing: on this image a booted
# simulator's first-boot work slowed the build steps beside it until a test
# timed out, and shutting it down again hung for minutes.
set -euo pipefail

log() { echo "ci-simulator: $*" >&2; }

# Run a command with a time limit; macOS has no timeout(1).
with_timeout() {
  local seconds="$1"
  shift
  perl -e 'alarm shift; exec @ARGV or die "exec: $!"' "$seconds" "$@"
}

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
    version, build, ident = e.get("version"), e.get("build"), e.get("runtimeIdentifier", "")
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

prepare() {
  local version="$1" device_type_name="$2"
  settle
  local runtime_id device_type_id
  # The image has two iOS 27.2 runtimes: the general one and one only for the
  # iPhone Duo (iOS-27-2-iPhone19-4). Take the one that runs this device type.
  runtime_id="$(xcrun simctl list runtimes -j | python3 -I -c '
import json, sys
want, device = sys.argv[1], sys.argv[2]
for r in json.load(sys.stdin)["runtimes"]:
    if r.get("platform") == "iOS" and r.get("version") == want and r.get("isAvailable") \
            and any(d.get("name") == device for d in r.get("supportedDeviceTypes", [])):
        print(r["identifier"]); break
' "$version" "$device_type_name")"
  if [[ -z "$runtime_id" ]]; then
    log "no available iOS $version simulator runtime runs '$device_type_name' with $(xcodebuild -version | head -1)"
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
  if ! udid="$(xcrun simctl create "Synara CI $device_type_name" "$device_type_id" "$runtime_id")"; then
    log "could not create '$device_type_name' on $runtime_id"
    diagnose
    return 1
  fi
  log "created $device_type_name on $runtime_id ($udid)"
  echo "$udid"
}

ready() {
  local udid="$1" started=$SECONDS
  log "booting $udid"
  if ! with_timeout "${CI_SIMULATOR_BOOT_SECONDS:-600}" xcrun simctl bootstatus "$udid" -b >&2; then
    log "simulator $udid did not finish booting"
    diagnose
    return 1
  fi
  log "booted in $((SECONDS - started)) s"
  check "$udid"
}

boot() {
  local udid
  udid="$(prepare "$1" "$2")"
  ready "$udid"
  echo "$udid"
}

check() {
  local udid="$1" problem runtimes started=$SECONDS
  runtimes="$(mktemp "${TMPDIR:-/tmp}/ci-simulator-runtimes.XXXXXX")"
  with_timeout 180 xcrun simctl runtime list -j > "$runtimes"
  log "runtime list took $((SECONDS - started)) s"
  started=$SECONDS
  problem="$(with_timeout 180 xcrun simctl list devices -j | python3 -I -c '
import json, os, sys
udid, runtimes_path = sys.argv[1], sys.argv[2]
devices = json.load(sys.stdin)["devices"]
found = [(runtime, d) for runtime, ds in devices.items() for d in ds if d.get("udid") == udid]
if not found:
    print("device " + udid + " no longer exists"); sys.exit()
runtime, device = found[0]
state = device.get("state")
if state != "Booted":
    print("device " + udid + " is " + str(state) + ", not Booted"); sys.exit()
with open(runtimes_path) as f:
    entries = json.load(f).values()
mounts = [e.get("mountPath") or "" for e in entries if e.get("runtimeIdentifier") == runtime]
if not any(m and os.path.isdir(m) for m in mounts):
    print("runtime " + runtime + " of device " + udid + " is no longer mounted (the image unmounted it)")
' "$udid" "$runtimes")"
  rm -f "$runtimes"
  log "device list took $((SECONDS - started)) s"
  started=$SECONDS
  if [[ -z "$problem" ]] && ! with_timeout 60 xcrun simctl spawn "$udid" launchctl print system > /dev/null 2>&1; then
    problem="device $udid is booted but does not answer simctl spawn"
  fi
  if [[ -n "$problem" ]]; then
    log "the test simulator is not usable: $problem"
    diagnose
    return 1
  fi
  log "device $udid is still booted, its runtime is mounted and it answered in $((SECONDS - started)) s"
}

smoke() {
  local udid="$1"
  check "$udid"
  log "launchctl in the device"
  with_timeout 60 xcrun simctl spawn "$udid" launchctl print system | head -n 20 >&2 || true
  log "launching Settings"
  with_timeout 120 xcrun simctl launch "$udid" com.apple.Preferences >&2
  sleep 5
  check "$udid"
}

remove() {
  local udid="$1"
  xcrun simctl shutdown "$udid" 2>/dev/null || true
  xcrun simctl delete "$udid"
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

usage="usage: $0 settle | prepare <iOS version> <device type name> | ready <udid> | boot <iOS version> <device type name> | check <udid> | smoke <udid> | remove <udid> | diagnose"
case "${1:-}" in
  settle) settle ;;
  prepare | boot)
    [[ $# -eq 3 ]] || { log "$usage"; exit 2; }
    "$1" "$2" "$3"
    ;;
  ready | check | smoke | remove)
    [[ $# -eq 2 ]] || { log "$usage"; exit 2; }
    "$1" "$2"
    ;;
  diagnose) diagnose ;;
  *) log "$usage"; exit 2 ;;
esac
