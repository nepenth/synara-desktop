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
`build-for-testing`.

Full `xcodebuild test` execution requires a concrete installed Simulator
runtime. Local machines can run:

```sh
RUN_IOS_TESTS=1 IOS_TEST_DESTINATION='platform=iOS Simulator,name=iPhone 16' scripts/ci-build.sh
```

Signed simulator builds do not require App Store distribution credentials on a
local Mac, but they do need normal simulator code signing. Signed device builds,
TestFlight archives, and App Store uploads are future release-lane work. They
will require Apple Developer Program membership, a registered bundle identifier,
signing assets, and App Store Connect API credentials stored as CI secrets.
