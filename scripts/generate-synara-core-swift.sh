#!/usr/bin/env bash
# Generate project-owned UniFFI Swift bindings and an Apple XCFramework.
#
# This is deliberately explicit: normal host CI validates the scaffold without
# invoking Apple toolchains, while a requested binding build either produces
# all artifacts from source or exits before publishing any stale output.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
core_udl="$repo_root/crates/synara-core/src/synara_core.udl"
package_root="$repo_root/synara-ios/SynaraCore"
generated_dir="$package_root/Sources/SynaraCore/Generated"
ffi_include_dir="$package_root/Sources/synara_coreFFI/include"
artifacts_dir="$package_root/Artifacts"
apple_slices="${SYNARA_CORE_APPLE_SLICES:-all}"
case "$apple_slices" in
  all)
    targets=(aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios aarch64-apple-darwin)
    ;;
  simulator)
    # generic iOS Simulator XCFramework still needs a fat sim slice.
    targets=(aarch64-apple-ios-sim x86_64-apple-ios)
    ;;
  device)
    targets=(aarch64-apple-ios)
    ;;
  *)
    printf 'generate-synara-core-swift: SYNARA_CORE_APPLE_SLICES must be all, simulator, or device (got %s)\n' "$apple_slices" >&2
    exit 1
    ;;
esac

fail() {
  printf 'generate-synara-core-swift: %s\n' "$*" >&2
  exit 1
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  fail "Apple binding generation requires macOS with Xcode and Apple Rust targets; host is $(uname -s). Run scripts/check-synara-core-swift-scaffold.mjs for host-neutral validation."
fi

command -v cargo >/dev/null || fail "cargo is required"
command -v rustup >/dev/null || fail "rustup is required to verify Apple Rust targets"
command -v xcrun >/dev/null || fail "Xcode command-line tools are required"
command -v xcodebuild >/dev/null || fail "Xcode is required to create SynaraCore.xcframework"
xcrun --sdk iphoneos --show-sdk-path >/dev/null || fail "the iPhoneOS SDK is not configured"
xcrun --sdk iphonesimulator --show-sdk-path >/dev/null || fail "the iPhoneSimulator SDK is not configured"
xcrun --sdk macosx --show-sdk-path >/dev/null || fail "the macOS SDK is not configured"

installed_targets="$(rustup target list --installed)"
for target in "${targets[@]}"; do
  if ! grep -Fxq "$target" <<<"$installed_targets"; then
    fail "missing Rust target $target; install it with: rustup target add $target"
  fi
done

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/synara-core-uniffi.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT
# Keep target output stable so local reruns and the repository's `target/`
# Actions cache can reuse the expensive four-architecture Matrix SDK build.
# Temporary assembly/output still lives under work_dir and is atomically
# published only after every target succeeds.
cargo_target_dir="${SYNARA_CORE_APPLE_TARGET_DIR:-$repo_root/target/synara-core-apple}"
mkdir -p "$cargo_target_dir"

# Match the Swift package deployment floors. Without these, current Xcode C
# dependencies may compile for the host's newest SDK while Rust links the iOS
# archive at its legacy default deployment target.
for target in "${targets[@]}"; do
  IPHONEOS_DEPLOYMENT_TARGET=16.0 \
    MACOSX_DEPLOYMENT_TARGET=13.0 \
    CARGO_TARGET_DIR="$cargo_target_dir" \
    cargo build --locked --release --package synara-core --target "$target"
done

swift_tmp="$work_dir/Swift"
mkdir -p "$swift_tmp"
# Run the repository's own lockfile-pinned generator, never a user/global tool.
CARGO_TARGET_DIR="$cargo_target_dir" cargo run --locked --package synara-core-bindgen \
  -- generate "$core_udl" --language swift --out-dir "$swift_tmp" --no-format

# The generated Swift imports `synara_coreFFI`. Put its C header and module
# map in every XCFramework slice so the Swift package's binary target supplies
# both that Clang module and the matching static library. Keeping these files
# beside an unrelated source target would compile the declarations without
# linking the real Rust FFI library.
headers_tmp="$work_dir/Headers"
mkdir -p "$headers_tmp"
mv "$swift_tmp"/synara_coreFFI.h "$headers_tmp/synara_coreFFI.h"
mv "$swift_tmp"/synara_coreFFI.modulemap "$headers_tmp/module.modulemap"

framework_tmp="$work_dir/SynaraCore.xcframework"
create_xcframework=(xcodebuild -create-xcframework)
if [[ "$apple_slices" == "all" || "$apple_slices" == "device" ]]; then
  create_xcframework+=(
    -library "$cargo_target_dir/aarch64-apple-ios/release/libsynara_core.a"
    -headers "$headers_tmp"
  )
fi
if [[ "$apple_slices" == "all" || "$apple_slices" == "simulator" ]]; then
  # XCFramework cannot accept two otherwise-identical simulator definitions.
  # Make the required generic simulator slice explicitly fat so both Xcode
  # generic simulator architectures link the same generated C module/library.
  simulator_library="$work_dir/libsynara_core-simulator.a"
  xcrun lipo -create \
    "$cargo_target_dir/aarch64-apple-ios-sim/release/libsynara_core.a" \
    "$cargo_target_dir/x86_64-apple-ios/release/libsynara_core.a" \
    -output "$simulator_library"
  create_xcframework+=(
    -library "$simulator_library"
    -headers "$headers_tmp"
  )
fi
if [[ "$apple_slices" == "all" ]]; then
  create_xcframework+=(
    -library "$cargo_target_dir/aarch64-apple-darwin/release/libsynara_core.a"
    -headers "$headers_tmp"
  )
fi
create_xcframework+=(-output "$framework_tmp")
"${create_xcframework[@]}"

# Publish only an all-target result. The generated package paths are ignored so
# source control never mistakes generated output for hand-maintained Swift.
# Headers now belong only inside the binary XCFramework; delete old P4-1
# sidecar output if a developer generated it before this package became linkable.
rm -f "$generated_dir/synara_core.swift"
rm -rf "$artifacts_dir"
rm -f "$ffi_include_dir/synara_coreFFI.h" "$ffi_include_dir/module.modulemap"
mkdir -p "$generated_dir" "$artifacts_dir"
mv "$swift_tmp"/synara_core.swift "$generated_dir/"
mv "$framework_tmp" "$artifacts_dir/SynaraCore.xcframework"
printf 'Generated SynaraCore Swift bindings and XCFramework at %s\n' "$package_root"
