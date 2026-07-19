# Synara iOS CI Notes

The iOS app supports two simulator build modes:

- Local signed simulator runs for live smoke testing. This is the correct mode
  for validating Keychain-backed login, session restore, logout, and any other
  entitlement-sensitive behavior.
- Unsigned simulator compilation for CI. This does not require Apple Developer
  Program credentials, provisioning profiles, App Store Connect API keys, APNs
  keys, or signing certificates, but it must not be used as proof that live
  Keychain/session behavior works.

CI should run:

```sh
scripts/ci-build.sh
```

That script regenerates the Xcode project with XcodeGen, performs an unsigned
generic iOS Simulator app build, and compiles the test bundles with
`build-for-testing`. It keeps Xcode derived data, SwiftPM package cache, Clang
module cache, SwiftPM module cache, and result bundles under `/private/tmp` by
default so local CI runs do not depend on mutable user-level Xcode cache state.

Full `xcodebuild test` execution requires a concrete installed Simulator
runtime. Local machines can run:

```sh
RUN_IOS_TESTS=1 IOS_TEST_DESTINATION='platform=iOS Simulator,name=iPhone 17' scripts/ci-build.sh
```

Set `IOS_TEST_DESTINATION` to any installed simulator shown by
`xcodebuild -showdestinations -project Synara.xcodeproj -scheme Synara` when a
runner has a different simulator set.

## Cache And Sandbox Controls

The script accepts these overrides when a runner needs explicit cache or result
locations:

```sh
DERIVED_DATA_PATH=/private/tmp/synara-ios-derived \
IOS_PACKAGE_CACHE_PATH=/private/tmp/synara-ios-package-cache \
IOS_RESULT_BUNDLE_DIR=/private/tmp/synara-ios-results \
CLANG_MODULE_CACHE_PATH=/private/tmp/synara-ios-module-cache \
SWIFTPM_MODULECACHE_OVERRIDE=/private/tmp/synara-ios-swiftpm-module-cache \
CFFIXED_USER_HOME=/private/tmp/synara-ios-home \
scripts/ci-build.sh
```

For an offline or network-restricted runner, prewarm the Swift package checkout
once in a normal macOS shell, then pass the existing Xcode SourcePackages path:

```sh
IOS_CLONED_SOURCE_PACKAGES_DIR_PATH="$HOME/Library/Developer/Xcode/DerivedData/Synara-fndcpnswhlujwfdbtgdwzppgoujz/SourcePackages" \
scripts/ci-build.sh
```

If the DerivedData hash changes, locate the active checkout with:

```sh
find "$HOME/Library/Developer/Xcode/DerivedData" -path '*Synara*/SourcePackages' -type d
```

The script pins package resolution to `Package.resolved`, skips package updates,
uses the system SCM provider, and writes timestamped `.xcresult` bundles under
`IOS_RESULT_BUNDLE_DIR`.

## Codex Sandbox Limitation

Codex can syntax-check this script, but it is not a reliable place to complete
iOS Xcode verification. Xcode and SwiftPM require access to CoreSimulator and
SwiftPM state under `~/Library`, and SwiftPM invokes Apple's `sandbox-exec`
during package resolution. Inside Codex's existing filesystem sandbox, that
nested sandbox application can fail even when cache paths are redirected.

The known failure signature is:

```text
sandbox-exec: sandbox_apply: Operation not permitted
```

If that appears while parsing the Swift package manifest, stop trying to tune
cache flags inside Codex and run the same script from a normal macOS Terminal or
macOS CI runner.

The release gate is therefore:

1. Run `scripts/ci-build.sh` from a normal macOS Terminal or a macOS CI runner.
2. Run the same command with `RUN_IOS_TESTS=1` on a machine that has the named
   simulator runtime installed.
3. Preserve the console log and the `.xcresult` bundles from
   `/private/tmp/synara-ios-results` for handoff if either command fails.

Signed simulator builds do not require App Store distribution credentials on a
local Mac, but they do need normal simulator code signing. Internal TestFlight
archives are produced by the protected `testflight` environment in the
`Release Synara` workflow whenever a matching `v<shared-version>` GitHub Release
is published. The release job uses environment secrets for the iOS Apple
Distribution identity, provisioning profiles, and App Store Connect API key;
these credentials are never exposed to pull-request or ordinary branch builds.

External TestFlight and App Store submission remain separate release gates. The
internal upload explicitly uses `testFlightInternalTestingOnly` and does not
submit a build for external review.
