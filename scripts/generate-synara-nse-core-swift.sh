#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
core_udl="$repo_root/crates/synara-nse-core/src/synara_nse_core.udl"
package_root="$repo_root/synara-ios/SynaraNseCore"
generated_dir="$package_root/Sources/SynaraNseCore/Generated"
artifacts_dir="$package_root/Artifacts"
apple_slices="${SYNARA_NSE_CORE_APPLE_SLICES:-all}"
space_bounded="${SYNARA_NSE_CORE_APPLE_SPACE_BOUNDED:-${SYNARA_APPLE_SPACE_BOUNDED:-0}}"

case "$apple_slices" in
  all)
    targets=(aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios)
    ;;
  simulator)
    targets=(aarch64-apple-ios-sim x86_64-apple-ios)
    ;;
  simulator-arm64)
    targets=(aarch64-apple-ios-sim)
    ;;
  device)
    targets=(aarch64-apple-ios)
    ;;
  *)
    printf 'generate-synara-nse-core-swift: invalid slice selection: %s\n' "$apple_slices" >&2
    exit 1
    ;;
esac
case "$space_bounded" in
  0|1)
    ;;
  *)
    printf 'generate-synara-nse-core-swift: SYNARA_NSE_CORE_APPLE_SPACE_BOUNDED must be 0 or 1 (got %s)\n' "$space_bounded" >&2
    exit 1
    ;;
esac

fail() {
  printf 'generate-synara-nse-core-swift: %s\n' "$*" >&2
  exit 1
}

publication_helper="$repo_root/scripts/lib/publish-generated-apple-pair.sh"
[[ -r "$publication_helper" ]] || fail "missing Apple pair publication helper: $publication_helper"
[[ -x "$publication_helper" ]] || fail "Apple pair publication helper is not executable: $publication_helper"

installed_targets="$(rustup target list --installed)"
for target in "${targets[@]}"; do
  grep -Fxq "$target" <<<"$installed_targets" || {
    printf 'generate-synara-nse-core-swift: missing Rust target %s\n' "$target" >&2
    exit 1
  }
done

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/synara-nse-core-uniffi.XXXXXX")"
cleanup_work_dir() {
  rm -rf -- "$work_dir"
}
trap cleanup_work_dir EXIT
cargo_target_dir="${SYNARA_NSE_CORE_APPLE_TARGET_DIR:-$repo_root/target/synara-core-apple}"
rust_profile="nse-release"
staged_archives_dir="$work_dir/Archives"
mkdir -p "$staged_archives_dir"

if [[ "$space_bounded" == "1" && -n "${SYNARA_NSE_CORE_APPLE_TARGET_DIR+x}" ]]; then
  fail "SYNARA_NSE_CORE_APPLE_TARGET_DIR cannot be combined with space-bounded mode; bounded builds own isolated temporary target directories"
fi

remove_bounded_target_dir() {
  local target_dir="$1"
  case "$target_dir" in
    "$work_dir"/cargo-target-*|"$work_dir"/cargo-bindgen)
      rm -rf -- "$target_dir"
      ;;
    *)
      fail "refusing to remove non-generator target directory: $target_dir"
      ;;
  esac
}

archive_for_target() {
  local target="$1"
  if [[ "$space_bounded" == "1" ]]; then
    printf '%s/%s/libsynara_nse_core.a\n' "$staged_archives_dir" "$target"
  else
    printf '%s/%s/%s/libsynara_nse_core.a\n' "$cargo_target_dir" "$target" "$rust_profile"
  fi
}

if [[ "$space_bounded" == "0" ]]; then
  mkdir -p "$cargo_target_dir"
fi

for target in "${targets[@]}"; do
  target_build_dir="$cargo_target_dir"
  if [[ "$space_bounded" == "1" ]]; then
    target_build_dir="$work_dir/cargo-target-$target"
    mkdir -p "$target_build_dir"
  fi
  IPHONEOS_DEPLOYMENT_TARGET=16.0 \
    CARGO_TARGET_DIR="$target_build_dir" \
    cargo build --locked --profile "$rust_profile" --package synara-nse-core --target "$target"
  if [[ "$space_bounded" == "1" ]]; then
    built_archive="$target_build_dir/$target/$rust_profile/libsynara_nse_core.a"
    [[ -f "$built_archive" ]] || fail "Rust build did not produce $built_archive"
    staged_archive="$(archive_for_target "$target")"
    mkdir -p "$(dirname "$staged_archive")"
    cp "$built_archive" "$staged_archive"
    remove_bounded_target_dir "$target_build_dir"
  fi
done

swift_tmp="$work_dir/Swift"
mkdir -p "$swift_tmp"
bindgen_target_dir="$cargo_target_dir"
if [[ "$space_bounded" == "1" ]]; then
  bindgen_target_dir="$work_dir/cargo-bindgen"
  mkdir -p "$bindgen_target_dir"
fi
CARGO_TARGET_DIR="$bindgen_target_dir" cargo run --locked --package synara-core-bindgen \
  -- generate "$core_udl" --language swift --out-dir "$swift_tmp" --no-format
if [[ "$space_bounded" == "1" ]]; then
  remove_bounded_target_dir "$bindgen_target_dir"
fi

headers_root="$work_dir/Headers"
headers_tmp="$headers_root/synara_nse_coreFFI"
mkdir -p "$headers_tmp"
mv "$swift_tmp"/synara_nse_coreFFI.h "$headers_tmp/synara_nse_coreFFI.h"
mv "$swift_tmp"/synara_nse_coreFFI.modulemap "$headers_tmp/module.modulemap"

framework_tmp="$work_dir/SynaraNseCore.xcframework"
create_xcframework=(xcodebuild -create-xcframework)
if [[ "$apple_slices" == "all" || "$apple_slices" == "device" ]]; then
  create_xcframework+=(
    -library "$(archive_for_target aarch64-apple-ios)"
    -headers "$headers_root"
  )
fi
if [[ "$apple_slices" == "all" || "$apple_slices" == "simulator" ]]; then
  simulator_library="$work_dir/libsynara_nse_core-simulator.a"
  xcrun lipo -create \
    "$(archive_for_target aarch64-apple-ios-sim)" \
    "$(archive_for_target x86_64-apple-ios)" \
    -output "$simulator_library"
  create_xcframework+=(
    -library "$simulator_library"
    -headers "$headers_root"
  )
elif [[ "$apple_slices" == "simulator-arm64" ]]; then
  create_xcframework+=(
    -library "$(archive_for_target aarch64-apple-ios-sim)"
    -headers "$headers_root"
  )
fi
create_xcframework+=(-output "$framework_tmp")
"${create_xcframework[@]}"

"$publication_helper" \
  "$swift_tmp/synara_nse_core.swift" \
  "$generated_dir/synara_nse_core.swift" \
  "$framework_tmp" \
  "$artifacts_dir/SynaraNseCore.xcframework"
printf 'Generated Synara NSE Core bindings at %s\n' "$package_root"
