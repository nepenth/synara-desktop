#!/usr/bin/env bash
set -euo pipefail

required=(
  SYNARA_LIVE_HOMESERVER
  SYNARA_LIVE_USERNAME
  SYNARA_LIVE_PASSWORD
  SYNARA_LIVE_VERIFICATION_STORE_PASSPHRASE
)

for name in "${required[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    echo "Missing required environment variable: ${name}" >&2
    exit 2
  fi
done

if [[ ! "$SYNARA_LIVE_VERIFICATION_STORE_PASSPHRASE" =~ [^[:space:]] ]]; then
  echo "SYNARA_LIVE_VERIFICATION_STORE_PASSPHRASE must not be empty or whitespace" >&2
  exit 1
fi

export SYNARA_VERIFICATION_DIAGNOSTICS=1
export CARGO_INCREMENTAL=0

test_name="live_own_device_verification_is_authoritative_and_durable"
if [[ "${1:-}" == "--direct-peer" ]]; then
  test_name="live_direct_peer_sas_transport_completes_through_product_owner_and_sync"
elif [[ $# -gt 0 ]]; then
  echo "Usage: $0 [--direct-peer]" >&2
  exit 2
fi

cargo test \
  -p synara-core \
  --lib \
  "${test_name}" \
  -- \
  --ignored \
  --nocapture
