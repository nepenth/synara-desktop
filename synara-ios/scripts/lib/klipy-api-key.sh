#!/usr/bin/env bash
# Source this file. The Klipy key ships inside the app, as client-side GIF
# keys do, but never enters the repository: it is read from
# SYNARA_KLIPY_API_KEY at build time and written to an ignored xcconfig
# include that Config/Synara.xcconfig picks up. Same rules as desktop's
# withGifApiKey (scripts/lib/runtime-config.mjs).

SYNARA_KLIPY_API_KEY_ENV="SYNARA_KLIPY_API_KEY"

synara_klipy_api_key_xcconfig_path() {
  local project_directory="$1"
  printf '%s\n' "${SYNARA_IOS_KLIPY_XCCONFIG:-$project_directory/Config/KlipyAPIKey.local.xcconfig}"
}

# Prints the trimmed key, nothing when it is unset or blank, and fails when it
# is not 8-128 characters of A-Z, a-z, 0-9, "_" or "-".
synara_validated_klipy_api_key() {
  local key="${SYNARA_KLIPY_API_KEY:-}"
  key="${key#"${key%%[![:space:]]*}"}"
  key="${key%"${key##*[![:space:]]}"}"
  if [[ -z "$key" ]]; then
    return 0
  fi
  if [[ ! "$key" =~ ^[A-Za-z0-9_-]{8,128}$ ]]; then
    echo "${SYNARA_KLIPY_API_KEY_ENV} is not a valid API key." >&2
    return 1
  fi
  printf '%s\n' "$key"
}

# Writes the key for the next Xcode build of the project in $1. With no key,
# "replace" ($2) removes an earlier one, so a release ships exactly what its
# environment holds; the default leaves a developer's own file alone.
synara_write_klipy_api_key() {
  local project_directory="$1"
  local mode="${2:-keep}"
  local output key
  output="$(synara_klipy_api_key_xcconfig_path "$project_directory")"
  key="$(synara_validated_klipy_api_key)" || return 1
  if [[ -z "$key" ]]; then
    if [[ "$mode" == "replace" ]]; then
      rm -f -- "$output"
    fi
    return 0
  fi
  (
    umask 077
    printf '// Written by scripts/lib/klipy-api-key.sh from %s. Never commit.\nSYNARA_KLIPY_API_KEY = %s\n' \
      "$SYNARA_KLIPY_API_KEY_ENV" "$key" > "$output"
  )
}

synara_remove_klipy_api_key() {
  local project_directory="$1"
  rm -f -- "$(synara_klipy_api_key_xcconfig_path "$project_directory")"
}
