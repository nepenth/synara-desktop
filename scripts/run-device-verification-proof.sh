#!/usr/bin/env bash
set -euo pipefail

required=(
  SYNARA_LIVE_HOMESERVER
  SYNARA_LIVE_USERNAME
  SYNARA_LIVE_PASSWORD
)

for name in "${required[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    echo "Missing required environment variable: ${name}" >&2
    exit 2
  fi
done

export SYNARA_VERIFICATION_DIAGNOSTICS=1
export CARGO_INCREMENTAL=0

cargo test \
  -p synara-core \
  --lib \
  live_two_device_sas_completes_through_product_owner_and_sync \
  -- \
  --ignored \
  --nocapture
