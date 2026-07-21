#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
harness_dir="$repo_root/integration/synapse"
runtime_dir="$harness_dir/runtime"
env_file="$runtime_dir/.env"
config_file="$runtime_dir/homeserver.yaml"
signing_key_file="$runtime_dir/localhost.signing.key"
compose_file="$harness_dir/compose.yml"

usage() {
  cat <<'EOF'
Usage: scripts/synapse-integration.sh <command>

Commands:
  up           Generate local secrets and start Synapse/PostgreSQL.
  down         Stop the harness while retaining its disposable database.
  reset        Stop the harness, delete its volume, and remove generated secrets.
  status       Show service state.
  logs         Follow Synapse logs.
  create-user  Interactively create a local Matrix account.
EOF
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required for the disposable Synapse harness." >&2
    exit 127
  fi
}

compose() {
  docker compose \
    --project-directory "$harness_dir" \
    --env-file "$env_file" \
    -f "$compose_file" \
    "$@"
}

initialize_runtime() {
  require_command openssl
  umask 077
  mkdir -p "$runtime_dir"

  if [[ ! -f "$env_file" ]]; then
    requested_port="${SYNARA_PORT:-8008}"
    if [[ ! "$requested_port" =~ ^[1-9][0-9]{3,4}$ ]] || (( requested_port < 1024 || requested_port > 65535 )); then
      echo "SYNARA_PORT must be an unprivileged TCP port between 1024 and 65535." >&2
      exit 64
    fi
    printf 'SYNARA_POSTGRES_PASSWORD=%s\n' "$(openssl rand -hex 32)" > "$env_file"
    printf 'SYNARA_PORT=%s\n' "$requested_port" >> "$env_file"
    printf 'SYNARA_UID=%s\n' "$(id -u)" >> "$env_file"
    printf 'SYNARA_GID=%s\n' "$(id -g)" >> "$env_file"
  fi

  load_runtime

  if [[ ! -f "$config_file" ]]; then
    registration_secret="$(openssl rand -hex 32)"
    macaroon_secret="$(openssl rand -hex 32)"
    form_secret="$(openssl rand -hex 32)"
    sed \
      -e "s/__POSTGRES_PASSWORD__/$runtime_password/g" \
      -e "s/__PUBLIC_PORT__/$runtime_port/g" \
      -e "s/__REGISTRATION_SECRET__/$registration_secret/g" \
      -e "s/__MACAROON_SECRET__/$macaroon_secret/g" \
      -e "s/__FORM_SECRET__/$form_secret/g" \
      "$harness_dir/homeserver.yaml.template" > "$config_file"
  fi

  if [[ ! -f "$signing_key_file" ]]; then
    key_material="$(openssl rand -base64 32 | tr -d '\n=')"
    printf 'ed25519 a_synara %s\n' "$key_material" > "$signing_key_file"
  fi
}

load_runtime() {
  runtime_password="$(sed -n 's/^SYNARA_POSTGRES_PASSWORD=//p' "$env_file")"
  runtime_port="$(sed -n 's/^SYNARA_PORT=//p' "$env_file")"
  runtime_uid="$(sed -n 's/^SYNARA_UID=//p' "$env_file")"
  runtime_gid="$(sed -n 's/^SYNARA_GID=//p' "$env_file")"
  if [[ ! "$runtime_password" =~ ^[a-f0-9]{64}$ ]]; then
    echo "Generated PostgreSQL password is missing or malformed; run '$0 reset'." >&2
    exit 65
  fi
  if [[ ! "$runtime_port" =~ ^[1-9][0-9]{3,4}$ ]] || (( runtime_port < 1024 || runtime_port > 65535 )); then
    echo "Generated harness port is missing or malformed; run '$0 reset'." >&2
    exit 65
  fi
  if [[ ! "$runtime_uid" =~ ^[0-9]+$ || ! "$runtime_gid" =~ ^[0-9]+$ ]]; then
    echo "Generated harness UID/GID is missing or malformed; run '$0 reset'." >&2
    exit 65
  fi
}

require_initialized_runtime() {
  if [[ ! -f "$env_file" || ! -f "$config_file" || ! -f "$signing_key_file" ]]; then
    echo "The disposable Synapse harness is not initialized; run '$0 up' first." >&2
    return 65
  fi
  load_runtime
}

clear_runtime() {
  find "$runtime_dir" -mindepth 1 -maxdepth 1 ! -name .gitkeep -exec rm -rf -- {} +
}

require_command docker
if ! docker compose version >/dev/null 2>&1; then
  echo "Docker Compose v2 is required for the disposable Synapse harness." >&2
  exit 127
fi

command_name="${1:-}"
case "$command_name" in
  up)
    initialize_runtime
    compose up -d --wait
    echo "Synapse is ready at http://127.0.0.1:$runtime_port"
    echo "Run '$0 create-user' to create disposable test accounts."
    ;;
  down)
    if [[ ! -f "$env_file" ]]; then
      echo "The disposable Synapse harness is not initialized."
      exit 0
    fi
    compose down
    ;;
  reset)
    if [[ -f "$env_file" ]]; then
      compose down --volumes --remove-orphans
    fi
    clear_runtime
    echo "Removed disposable Synapse state and generated secrets."
    ;;
  status)
    if ! require_initialized_runtime; then
      exit 0
    fi
    compose ps
    ;;
  logs)
    require_initialized_runtime
    compose logs --follow synapse
    ;;
  create-user)
    initialize_runtime
    compose exec synapse register_new_matrix_user \
      --config /data/homeserver.yaml \
      http://127.0.0.1:8008
    ;;
  *)
    usage >&2
    exit 64
    ;;
esac
