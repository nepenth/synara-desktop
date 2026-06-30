#!/usr/bin/env bash
set -euo pipefail

repo_name="${SYNARA_PACMAN_REPO_NAME:-synara}"
package_dir="${1:-packaging/arch}"
output_dir="${2:-dist/pacman-repo}"

if ! command -v repo-add >/dev/null 2>&1; then
  printf 'repo-add is required. Install pacman-contrib on Arch runners.\n' >&2
  exit 1
fi

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="$(cd "${root_dir}/${package_dir}" && pwd)"
output_dir="${root_dir}/${output_dir}"

startdir="${package_dir}"
# shellcheck source=/dev/null
source "${package_dir}/PKGBUILD"

package_file="${package_dir}/${pkgname}-${pkgver}-${pkgrel}-x86_64.pkg.tar.zst"
if [[ ! -f "${package_file}" ]]; then
  printf 'Expected package was not found: %s\n' "${package_file}" >&2
  printf 'Run makepkg from %s before generating the pacman repo.\n' "${package_dir}" >&2
  exit 1
fi

rm -rf "${output_dir}"
mkdir -p "${output_dir}"
cp "${package_file}" "${output_dir}/"

(
  cd "${output_dir}"
  repo-add "${repo_name}.db.tar.gz" "$(basename "${package_file}")"

  rm -f "${repo_name}.db" "${repo_name}.files"
  cp "${repo_name}.db.tar.gz" "${repo_name}.db"
  cp "${repo_name}.files.tar.gz" "${repo_name}.files"
)

printf '[pacman-repo] wrote %s with %s\n' "${output_dir}" "$(basename "${package_file}")"
