#!/usr/bin/env bash
set -euo pipefail

package_dir="${1:-src-tauri/target/release/bundle/deb}"
output_dir="${2:-dist/apt-repo}"
repo_tag="${SYNARA_APT_REPO_TAG:-apt-repo}"

for command in dpkg-scanpackages apt-ftparchive; do
  if ! command -v "${command}" >/dev/null 2>&1; then
    printf '%s is required (install dpkg-dev and apt-utils).\n' "${command}" >&2
    exit 1
  fi
done

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

resolve_input_dir() {
  local dir="$1"
  if [[ "${dir}" = /* ]]; then
    cd "${dir}" && pwd
  else
    cd "${root_dir}/${dir}" && pwd
  fi
}

package_dir="$(resolve_input_dir "${package_dir}")"
if [[ "${output_dir}" != /* ]]; then
  output_dir="${root_dir}/${output_dir}"
fi

shopt -s nullglob
package_files=("${package_dir}"/Synara_*.deb)
shopt -u nullglob

if [[ ${#package_files[@]} -ne 1 ]]; then
  printf 'Expected exactly one Synara_*.deb in %s, found %s.\n' \
    "${package_dir}" "${#package_files[@]}" >&2
  exit 1
fi

package_file="${package_files[0]}"

rm -rf -- "${output_dir}"
mkdir -p -- "${output_dir}"
cp -- "${package_file}" "${output_dir}/"

(
  cd "${output_dir}"

  # A flat repository uses a suite ending in "/". APT resolves Filename
  # against the source URI, so include the fixed GitHub Release tag here.
  dpkg-scanpackages -m . /dev/null |
    sed "s|^Filename: \\./|Filename: ${repo_tag}/|" > Packages
  gzip -9n -c Packages > Packages.gz

  release_tmp="$(mktemp)"
  trap 'rm -f -- "${release_tmp}"' EXIT
  apt-ftparchive \
    -o APT::FTPArchive::Release::Origin="Synara" \
    -o APT::FTPArchive::Release::Label="Synara" \
    -o APT::FTPArchive::Release::Suite="${repo_tag}" \
    -o APT::FTPArchive::Release::Codename="${repo_tag}" \
    -o APT::FTPArchive::Release::Architectures="amd64" \
    -o APT::FTPArchive::Release::Description="Synara Desktop Debian-family repository" \
    release . > "${release_tmp}"
  mv -- "${release_tmp}" Release
)

printf '[apt-repo] wrote %s with %s\n' \
  "${output_dir}" "$(basename "${package_file}")"
