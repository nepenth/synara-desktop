#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<'EOF'
Build a local Developer ID signed and notarized macOS release bundle.

Usage:
  npm run build:macos:local

Configuration:
  Copy .env.macos-signing.example to .env.macos-signing.local and fill in:
    APPLE_TEAM_ID
    APPLE_SIGNING_IDENTITY
    APPLE_ID
    APPLE_APP_SPECIFIC_PASSWORD
    SYNARA_MACOS_PROVISIONING_PROFILE (path to the matching Developer ID profile)

  App Store Connect API key notarization is also supported by setting:
    APPLE_API_KEY
    APPLE_API_ISSUER
    APPLE_API_KEY_PATH

Environment overrides:
  SYNARA_MACOS_SIGNING_ENV   Path to env file. Defaults to .env.macos-signing.local
  SYNARA_MACOS_TARGET        Defaults to universal-apple-darwin
  SYNARA_MACOS_BUNDLES       Defaults to dmg
EOF
  exit 0
fi

env_file="${SYNARA_MACOS_SIGNING_ENV:-$root/.env.macos-signing.local}"
if [[ -f "$env_file" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$env_file"
  set +a
elif [[ -n "${SYNARA_MACOS_SIGNING_ENV:-}" ]]; then
  cat >&2 <<EOF
Missing local signing env file:
  $env_file

Create it from:
  cp .env.macos-signing.example .env.macos-signing.local
EOF
  exit 1
fi

if [[ -z "${APPLE_PASSWORD:-}" && -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" ]]; then
  export APPLE_PASSWORD="$APPLE_APP_SPECIFIC_PASSWORD"
fi

missing=()
for name in APPLE_TEAM_ID APPLE_SIGNING_IDENTITY SYNARA_MACOS_PROVISIONING_PROFILE; do
  if [[ -z "${!name:-}" ]]; then
    missing+=("$name")
  fi
done

if [[ -n "${APPLE_API_KEY:-}" || -n "${APPLE_API_ISSUER:-}" || -n "${APPLE_API_KEY_PATH:-}" ]]; then
  for name in APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH; do
    if [[ -z "${!name:-}" ]]; then
      missing+=("$name")
    fi
  done
else
  for name in APPLE_ID APPLE_PASSWORD; do
    if [[ -z "${!name:-}" ]]; then
      missing+=("$name")
    fi
  done
fi

if (( ${#missing[@]} > 0 )); then
  printf 'Missing required local macOS signing values:\n' >&2
  printf ' - %s\n' "${missing[@]}" >&2
  exit 1
fi

if ! security find-identity -v -p codesigning | grep -F "$APPLE_SIGNING_IDENTITY" >/dev/null; then
  cat >&2 <<EOF
Developer ID signing identity is not visible to this shell:
  $APPLE_SIGNING_IDENTITY

Check Keychain Access > login > My Certificates, and verify the certificate has
its private key. Then confirm this command prints the identity:
  security find-identity -v -p codesigning | grep "Developer ID Application"
EOF
  exit 1
fi

target="${SYNARA_MACOS_TARGET:-universal-apple-darwin}"
bundles="${SYNARA_MACOS_BUNDLES:-dmg}"
signing_dir="$(mktemp -d "${TMPDIR:-/tmp}/synara-macos-signing.XXXXXX")"
trap 'rm -rf -- "$signing_dir"' EXIT
export SYNARA_MACOS_PROVISIONING_PROFILE="$(python3 -c 'from pathlib import Path; import sys; print(Path(sys.argv[1]).resolve())' "$SYNARA_MACOS_PROVISIONING_PROFILE")"
export SYNARA_MACOS_SIGNING_ENTITLEMENTS="$signing_dir/signing.entitlements"
python3 scripts/check-macos-provisioning-profile.py "$SYNARA_MACOS_PROVISIONING_PROFILE" \
  --team "$APPLE_TEAM_ID" --identity "$APPLE_SIGNING_IDENTITY" \
  --entitlements src-tauri/Entitlements.plist --entitlements-output "$SYNARA_MACOS_SIGNING_ENTITLEMENTS"

config_json="$(
  node - <<'NODE'
const config = {
  bundle: {
    createUpdaterArtifacts: false,
    macOS: {
      signingIdentity: process.env.APPLE_SIGNING_IDENTITY,
      providerShortName: process.env.APPLE_TEAM_ID,
      entitlements: process.env.SYNARA_MACOS_SIGNING_ENTITLEMENTS,
      files: { "embedded.provisionprofile": process.env.SYNARA_MACOS_PROVISIONING_PROFILE },
    },
  },
};
process.stdout.write(JSON.stringify(config));
NODE
)"

echo "Building Synara for macOS target: $target"
echo "Bundles: $bundles"
echo "Signing identity: $APPLE_SIGNING_IDENTITY"

npm run tauri -- build \
  --target "$target" \
  --bundles "$bundles" \
  --config "$config_json"

if [[ "$target" == "universal-apple-darwin" ]]; then
  bundle_root="target/universal-apple-darwin/release/bundle"
else
  bundle_root="target/release/bundle"
fi

app_path="$bundle_root/macos/Synara.app"
if [[ ! -d "$app_path" ]]; then
  echo "Expected app bundle not found: $app_path" >&2
  exit 1
fi

dmg_paths=()

codesign --verify --deep --strict --verbose=2 "$app_path"
python3 scripts/check-macos-provisioning-profile.py "$SYNARA_MACOS_PROVISIONING_PROFILE" \
  --team "$APPLE_TEAM_ID" --identity "$APPLE_SIGNING_IDENTITY" --app "$app_path"
spctl --assess --type execute --verbose=4 "$app_path"
xcrun stapler validate "$app_path"

if [[ "$bundles" == *dmg* ]]; then
  shopt -s nullglob
  dmg_paths=("$bundle_root"/dmg/Synara_*.dmg)
  shopt -u nullglob
  if (( ${#dmg_paths[@]} == 0 )); then
    echo "Expected DMG bundle not found under $bundle_root/dmg" >&2
    exit 1
  fi
  for dmg_path in "${dmg_paths[@]}"; do
    xcrun stapler validate "$dmg_path"
    spctl --assess --type open --context context:primary-signature --verbose=4 "$dmg_path"
  done
fi

echo "Local macOS release build verified:"
echo "  $app_path"
if (( ${#dmg_paths[@]} > 0 )); then
  for dmg_path in "${dmg_paths[@]}"; do
    echo "  $dmg_path"
  done
fi
