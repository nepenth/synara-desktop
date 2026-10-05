#!/usr/bin/env bash
# Source this file; it never cleans or replaces existing build outputs.

synara_configure_xcode_cache() {
  local project_directory="$1"
  local repository_identity toolchain_identity generator_identity generator_version
  local developer_path swift_path xcode_version swift_version device_sdk simulator_sdk architecture xcode_major
  project_directory="$(cd "$project_directory" && pwd -P)" || return
  repository_identity="$(printf '%s' "$project_directory" | shasum -a 256 | awk '{print $1}')"
  developer_path="$(xcode-select --print-path)" || return
  swift_path="$(xcrun --find swift)" || return
  xcode_version="$(xcodebuild -version)" || return
  swift_version="$(xcrun swift --version 2>&1)" || return
  device_sdk="$(xcrun --sdk iphoneos --show-sdk-version)" || return
  simulator_sdk="$(xcrun --sdk iphonesimulator --show-sdk-version)" || return
  architecture="$(uname -m)" || return
  toolchain_identity="$(printf '%s\n' "$developer_path" "$swift_path" "$xcode_version" "$swift_version" "$device_sdk" "$simulator_sdk" "$architecture" | shasum -a 256 | awk '{print $1}')"
  SYNARA_XCODE_CACHE_ROOT="${SYNARA_IOS_CACHE_ROOT:-${HOME}/Library/Caches/Synara/Xcode/$repository_identity/$toolchain_identity}"
  SYNARA_XCODE_DERIVED_DATA_ROOT="$SYNARA_XCODE_CACHE_ROOT/DerivedData"
  SYNARA_XCODE_TOOLCHAIN_KEY="$toolchain_identity"
  SYNARA_XCODE_COMPILATION_CACHE_PATH="$SYNARA_XCODE_CACHE_ROOT/CompilationCache.noindex"
  export SYNARA_XCODE_CACHE_ROOT SYNARA_XCODE_DERIVED_DATA_ROOT SYNARA_XCODE_TOOLCHAIN_KEY SYNARA_XCODE_COMPILATION_CACHE_PATH
  SYNARA_XCODE_COMPILATION_ARGS=()
  xcode_major="$(printf '%s\n' "$xcode_version" | sed -n 's/^Xcode \([0-9][0-9]*\).*/\1/p')"
  if [[ ! "$xcode_major" =~ ^[0-9]+$ ]]; then
    echo "Unable to read the selected Xcode major version" >&2
    return 1
  fi
  if (( xcode_major >= 26 )); then
    SYNARA_XCODE_COMPILATION_ARGS=(
      COMPILATION_CACHE_ENABLE_CACHING=YES
      "COMPILATION_CACHE_CAS_PATH=$SYNARA_XCODE_COMPILATION_CACHE_PATH"
      COMPILATION_CACHE_KEEP_CAS_DIRECTORY=YES
      COMPILATION_CACHE_ENABLE_PLUGIN=NO
      COMPILATION_CACHE_REMOTE_SERVICE_PATH=
      "COMPILATION_CACHE_LIMIT_SIZE=${SYNARA_IOS_COMPILATION_CACHE_LIMIT:-1G}"
      "COMPILATION_CACHE_ENABLE_DIAGNOSTIC_REMARKS=${SYNARA_IOS_CACHE_REMARKS:-NO}"
    )
  fi
  export CLANG_MODULE_CACHE_PATH="${CLANG_MODULE_CACHE_PATH:-$SYNARA_XCODE_CACHE_ROOT/ModuleCache.noindex}"
  export SWIFTPM_MODULECACHE_OVERRIDE="${SWIFTPM_MODULECACHE_OVERRIDE:-$SYNARA_XCODE_CACHE_ROOT/SwiftModules.noindex}"
  PACKAGE_CACHE_PATH="${IOS_PACKAGE_CACHE_PATH:-$SYNARA_XCODE_CACHE_ROOT/PackageCache}"
  CLONED_SOURCE_PACKAGES_DIR_PATH="${IOS_CLONED_SOURCE_PACKAGES_DIR_PATH:-$SYNARA_XCODE_CACHE_ROOT/SourcePackages}"
  # XcodeGen's cache is scoped to its version as well as the selected Xcode.
  if command -v xcodegen >/dev/null 2>&1; then
    generator_version="$(xcodegen --version)" || return
    generator_identity="$(printf '%s' "$generator_version" | shasum -a 256 | awk '{print $1}')"
    XCODEGEN_CACHE_PATH="${XCODEGEN_CACHE_PATH:-$SYNARA_XCODE_CACHE_ROOT/XcodeGen/$generator_identity}"
  fi
}

synara_xcode_package_args() {
  local project_directory="$1"
  local checker="$project_directory/../scripts/check-xcode-local-package-graph.mjs"
  local resolved="$project_directory/Synara.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved"
  local graph_kind
  graph_kind="$(node "$checker" "$project_directory/Synara.xcodeproj/project.pbxproj" "$project_directory")" || return
  SYNARA_XCODE_PACKAGE_ARGS=(
    -packageCachePath "$PACKAGE_CACHE_PATH"
    -clonedSourcePackagesDirPath "$CLONED_SOURCE_PACKAGES_DIR_PATH"
    -scmProvider system
  )
  case "$graph_kind" in
    remote)
      if [[ ! -f "$resolved" ]]; then
        echo "A committed Swift package lock is required when remote packages are present: $resolved" >&2
        return 1
      fi
      SYNARA_XCODE_PACKAGE_ARGS+=(
        -onlyUsePackageVersionsFromResolvedFile
        -disableAutomaticPackageResolution
        -skipPackageUpdates
      )
      ;;
    local)
      if [[ -f "$resolved" ]]; then
        echo "Remove the stale Swift package lock; the generated project has only local packages: $resolved" >&2
        return 1
      fi
      ;;
    *)
      echo "Swift package graph checker returned an invalid result: $graph_kind" >&2
      return 1
      ;;
  esac
}
