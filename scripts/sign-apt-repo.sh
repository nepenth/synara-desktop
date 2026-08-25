#!/usr/bin/env bash
set -euo pipefail

repo_dir="${1:-dist/apt-repo}"
private_key="${SYNARA_APT_SIGNING_PRIVATE_KEY:-}"
passphrase="${SYNARA_APT_SIGNING_PRIVATE_KEY_PASSWORD:-}"

if [[ -z "${private_key}" ]]; then
  printf 'SYNARA_APT_SIGNING_PRIVATE_KEY is required.\n' >&2
  exit 1
fi

for command in gpg gpgv; do
  if ! command -v "${command}" >/dev/null 2>&1; then
    printf '%s is required to sign the APT repository.\n' "${command}" >&2
    exit 1
  fi
done

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${repo_dir}" != /* ]]; then
  repo_dir="${root_dir}/${repo_dir}"
fi
repo_dir="$(cd "${repo_dir}" && pwd)"

if [[ ! -f "${repo_dir}/Release" ]]; then
  printf 'APT Release metadata was not found in %s.\n' "${repo_dir}" >&2
  exit 1
fi

gnupg_home="$(mktemp -d)"
trap 'rm -rf -- "${gnupg_home}"' EXIT
chmod 0700 "${gnupg_home}"

printf '%s' "${private_key}" | gpg \
  --homedir "${gnupg_home}" \
  --batch \
  --quiet \
  --import

fingerprint="$(
  gpg --homedir "${gnupg_home}" --batch --with-colons --list-secret-keys |
    awk -F: '$1 == "sec" { primary = 1; next } primary && $1 == "fpr" { print $10; exit }'
)"
if [[ ! "${fingerprint}" =~ ^[0-9A-Fa-f]{40}$ ]]; then
  printf 'Unable to identify one primary signing-key fingerprint.\n' >&2
  exit 1
fi

keyring="${repo_dir}/synara-archive-keyring.gpg"
inrelease="${repo_dir}/InRelease"
release_signature="${repo_dir}/Release.gpg"
rm -f -- "${keyring}" "${inrelease}" "${release_signature}"

gpg --homedir "${gnupg_home}" --batch --quiet --export "${fingerprint}" > "${keyring}"
if [[ ! -s "${keyring}" ]]; then
  printf 'Unable to export the Synara APT public keyring.\n' >&2
  exit 1
fi

sign_release() {
  printf '%s' "${passphrase}" | gpg \
    --homedir "${gnupg_home}" \
    --batch \
    --yes \
    --quiet \
    --pinentry-mode loopback \
    --passphrase-fd 0 \
    --local-user "${fingerprint}" \
    --digest-algo SHA512 \
    "$@"
}

sign_release --armor --detach-sign --output "${release_signature}" "${repo_dir}/Release"
sign_release --clearsign --output "${inrelease}" "${repo_dir}/Release"

gpgv --keyring "${keyring}" "${release_signature}" "${repo_dir}/Release" >/dev/null
gpgv --keyring "${keyring}" "${inrelease}" >/dev/null

printf '[apt-repo] signed Release with %s\n' "${fingerprint}"
