#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
core_udl="$repo_root/crates/synara-nse-core/src/synara_nse_core.udl"
package_root="$repo_root/synara-ios/SynaraNseCore"
generated_dir="$package_root/Sources/SynaraNseCore/Generated"
artifacts_dir="$package_root/Artifacts"
apple_slices="${SYNARA_NSE_CORE_APPLE_SLICES:-all}"

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

installed_targets="$(rustup target list --installed)"
for target in "${targets[@]}"; do
  grep -Fxq "$target" <<<"$installed_targets" || {
    printf 'generate-synara-nse-core-swift: missing Rust target %s\n' "$target" >&2
    exit 1
  }
done

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/synara-nse-core-uniffi.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT
cargo_target_dir="${SYNARA_NSE_CORE_APPLE_TARGET_DIR:-$repo_root/target/synara-core-apple}"
rust_profile="nse-release"
mkdir -p "$cargo_target_dir"

for target in "${targets[@]}"; do
  IPHONEOS_DEPLOYMENT_TARGET=16.0 \
    CARGO_TARGET_DIR="$cargo_target_dir" \
    cargo build --locked --profile "$rust_profile" --package synara-nse-core --target "$target"
done

swift_tmp="$work_dir/Swift"
mkdir -p "$swift_tmp"
CARGO_TARGET_DIR="$cargo_target_dir" cargo run --locked --package synara-core-bindgen \
  -- generate "$core_udl" --language swift --out-dir "$swift_tmp" --no-format

headers_root="$work_dir/Headers"
headers_tmp="$headers_root/synara_nse_coreFFI"
mkdir -p "$headers_tmp"
mv "$swift_tmp"/synara_nse_coreFFI.h "$headers_tmp/synara_nse_coreFFI.h"
mv "$swift_tmp"/synara_nse_coreFFI.modulemap "$headers_tmp/module.modulemap"

framework_tmp="$work_dir/SynaraNseCore.xcframework"
create_xcframework=(xcodebuild -create-xcframework)
if [[ "$apple_slices" == "all" || "$apple_slices" == "device" ]]; then
  create_xcframework+=(
    -library "$cargo_target_dir/aarch64-apple-ios/$rust_profile/libsynara_nse_core.a"
    -headers "$headers_root"
  )
fi
if [[ "$apple_slices" == "all" || "$apple_slices" == "simulator" ]]; then
  simulator_library="$work_dir/libsynara_nse_core-simulator.a"
  xcrun lipo -create \
    "$cargo_target_dir/aarch64-apple-ios-sim/$rust_profile/libsynara_nse_core.a" \
    "$cargo_target_dir/x86_64-apple-ios/$rust_profile/libsynara_nse_core.a" \
    -output "$simulator_library"
  create_xcframework+=(
    -library "$simulator_library"
    -headers "$headers_root"
  )
elif [[ "$apple_slices" == "simulator-arm64" ]]; then
  create_xcframework+=(
    -library "$cargo_target_dir/aarch64-apple-ios-sim/$rust_profile/libsynara_nse_core.a"
    -headers "$headers_root"
  )
fi
create_xcframework+=(-output "$framework_tmp")
"${create_xcframework[@]}"

rm -f "$generated_dir/synara_nse_core.swift"
rm -rf "$artifacts_dir"
mkdir -p "$generated_dir" "$artifacts_dir"
mv "$swift_tmp"/synara_nse_core.swift "$generated_dir/"
mv "$framework_tmp" "$artifacts_dir/SynaraNseCore.xcframework"
printf 'Generated Synara NSE Core bindings at %s\n' "$package_root"
