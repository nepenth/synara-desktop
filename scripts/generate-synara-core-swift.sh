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
artifacts_dir="$package_root/Artifacts"
uniffi_version="0.28.3"
targets=(aarch64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin)

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
cargo_target_dir="$work_dir/target"
tool_root="$repo_root/target/uniffi-tools-$uniffi_version"
bindgen="$tool_root/bin/uniffi-bindgen"

# Install an exact, project-selected generator outside the source tree. It is
# cached under ignored target/ only after this explicit Apple build is invoked.
if [[ ! -x "$bindgen" ]]; then
  cargo install --locked --root "$tool_root" --version "$uniffi_version" uniffi_bindgen
fi

for target in "${targets[@]}"; do
  CARGO_TARGET_DIR="$cargo_target_dir" cargo build --locked --release --package synara-core --target "$target"
done

swift_tmp="$work_dir/Swift"
mkdir -p "$swift_tmp"
"$bindgen" generate "$core_udl" --language swift --out-dir "$swift_tmp"

framework_tmp="$work_dir/SynaraCore.xcframework"
xcodebuild -create-xcframework   -library "$cargo_target_dir/aarch64-apple-ios/release/libsynara_core.a"   -library "$cargo_target_dir/aarch64-apple-ios-sim/release/libsynara_core.a"   -library "$cargo_target_dir/aarch64-apple-darwin/release/libsynara_core.a"   -output "$framework_tmp"

# Publish only an all-target result. The generated package paths are ignored so
# source control never mistakes generated output for hand-maintained Swift.
rm -rf "$generated_dir" "$artifacts_dir"
mkdir -p "$generated_dir" "$artifacts_dir"
mv "$swift_tmp"/*.swift "$generated_dir/"
mv "$framework_tmp" "$artifacts_dir/SynaraCore.xcframework"
printf 'Generated SynaraCore Swift bindings and XCFramework at %s\n' "$package_root"
