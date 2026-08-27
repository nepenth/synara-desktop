# TestFlight Internal Upload

Use the local upload script after `MARKETING_VERSION` and `CURRENT_PROJECT_VERSION`
are updated and committed:

```sh
synara-ios/scripts/upload-testflight-internal.sh
```

Defaults:

- Scheme: `Synara`
- Configuration: `Release`
- Bundle ID: `com.whylandcreative.synara`
- Distribution method: `app-store-connect`
- Destination: `upload`
- TestFlight scope: internal testing only

Required environment variables:

```sh
SYNARA_IOS_TEAM_ID=... \
SYNARA_IOS_PROVISIONING_PROFILE="..." \
SYNARA_IOS_NOTIFICATION_SERVICE_PROVISIONING_PROFILE="..." \
SYNARA_PUSH_GATEWAY_URL="https://push.example.com/_matrix/push/v1/notify" \
synara-ios/scripts/upload-testflight-internal.sh
```

Optional environment variables:

```sh
SYNARA_IOS_ARCHIVE_ROOT=/tmp \
SYNARA_IOS_PACKAGE_CACHE_PATH=/path/to/swift-package-cache \
synara-ios/scripts/upload-testflight-internal.sh
```

Project build operations always honor the committed `Package.resolved` and
skip dependency updates. `SYNARA_IOS_PACKAGE_CACHE_PATH` can point Xcode at an
existing package cache for deterministic local releases without a redundant
network refresh. Archive export does not receive project-only package flags.

For unattended App Store Connect authentication, create an API key in App Store
Connect and store the `.p8` outside this repository, for example:

```sh
mkdir -p ~/.private_keys/appstoreconnect
chmod 700 ~/.private_keys ~/.private_keys/appstoreconnect
mv ~/Downloads/AuthKey_ABC123DEFG.p8 ~/.private_keys/appstoreconnect/
chmod 600 ~/.private_keys/appstoreconnect/AuthKey_ABC123DEFG.p8
```

Then run:

```sh
SYNARA_ASC_KEY_PATH="$HOME/.private_keys/appstoreconnect/AuthKey_ABC123DEFG.p8" \
SYNARA_ASC_KEY_ID="ABC123DEFG" \
SYNARA_ASC_ISSUER_ID="00000000-0000-0000-0000-000000000000" \
synara-ios/scripts/upload-testflight-internal.sh
```

For local use, these values can also live in an untracked env file outside the
repo:

```sh
source "$HOME/.private_keys/appstoreconnect/synara.env"
synara-ios/scripts/upload-testflight-internal.sh
```

Use an App Store Connect API key with at least App Manager access for upload.
Admin access is acceptable for this local key if you want the automation to cover
future metadata work, but the private key must remain outside the repository.

Do not commit `.p8` files. The repo ignores `AuthKey_*.p8` and `*.p8` as a
defense-in-depth measure.

### Local TestFlight crash diagnostics

The same API key can retrieve the newest tester-submitted crash logs without
depending on Xcode Organizer or an App Store Connect browser session:

```sh
source "$HOME/.private_keys/appstoreconnect/synara.env"
SYNARA_ASC_APP_ID="6777089267" \
SYNARA_IOS_CRASH_DIAGNOSTICS_DIR="/tmp/synara-testflight-crashes" \
node synara-ios/scripts/fetch-testflight-crashes.mjs --build "X.Y.Z"
```

The command writes one standard `.crash` file per submission plus `index.json`.
The output can contain tester comments, device details, and process state, so
keep it outside the repository and do not attach it to public issues without
reviewing and redacting it first. Omitting `--build` downloads the newest page
across recent builds.

The script disables Xcode-managed build-number mutation so App Store Connect uses
the build number committed in `Synara.xcodeproj` and `project.yml`.

The upload command proves only that Apple accepted the package transport. After
upload, run the processing and internal-distribution gate with the exact values
reported by Xcode:

```sh
SYNARA_ASC_KEY_PATH="/secure/path/AuthKey_KEYID.p8" \
SYNARA_ASC_KEY_ID="KEYID" \
SYNARA_ASC_ISSUER_ID="issuer-uuid" \
SYNARA_IOS_MARKETING_VERSION="X.Y.Z" \
SYNARA_IOS_BUILD_NUMBER="X.Y.Z" \
SYNARA_TESTFLIGHT_INTERNAL_GROUP_IDS="group-uuid" \
node synara-ios/scripts/promote-testflight-internal.mjs
```

The gate follows the prerelease-version and build-upload relationships so it can
observe builds that Apple's ordinary build collection omits while processing. It
requires the upload to become `COMPLETE`, the exact build to become `VALID`, all
configured groups to be internal and assigned, and the internal build state to
become `IN_BETA_TESTING`. Missing export compliance, failed processing, an
expired build, or a bounded timeout fails the release.

The upload uses `testFlightInternalTestingOnly`, and App Store Connect makes the
processed build available to internal testers through the configured internal
distribution settings. Groups configured for automatic access to all builds do
not require a relationship mutation; the gate explicitly adds and verifies any
missing manual internal-group relationship. It writes a redacted Apple-state
snapshot to `SYNARA_IOS_DIAGNOSTICS_DIR`.

The upload script also records the archive and export logs and copies Xcode's
`.xcdistributionlogs` bundle when available. Keep these diagnostics for both
successful and failed uploads; never store the API private key in them.

## GitHub Release Integration

Pushing a `v<shared-version>` tag starts the singular
`.github/workflows/release.yml` workflow. Its protected `testflight` deployment
job runs on `macos-26`, validates the distribution identity and both provisioning
profiles, and uploads only after the common release gate passes.

Required `testflight` environment secrets:

- `IOS_DISTRIBUTION_CERTIFICATE_BASE64`
- `IOS_DISTRIBUTION_CERTIFICATE_PASSWORD`
- `IOS_APP_PROVISIONING_PROFILE_BASE64`
- `IOS_NOTIFICATION_PROVISIONING_PROFILE_BASE64`
- `SYNARA_ASC_KEY_BASE64`
- `SYNARA_ASC_KEY_ID`
- `SYNARA_ASC_ISSUER_ID`

The job also consumes the repository secret `APPLE_TEAM_ID` and repository
variables `SYNARA_PUSH_GATEWAY_URL` and
`SYNARA_TESTFLIGHT_INTERNAL_GROUP_IDS` (a comma-delimited list of App Store
Connect internal beta-group IDs). Internal-only upload remains the default;
external TestFlight still requires an explicit release decision and configured
external tester groups.

The final GitHub Release is not created unless iOS, macOS, both Linux builds,
notarization, updater metadata, and artifact verification all succeed. The iOS
upload and processing checks run as separate jobs, so rerunning failed jobs can
retry Apple processing without uploading the same build number again. The iOS
gate is not successful until the exact build is available to the configured
internal TestFlight groups.
