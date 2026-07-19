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
SYNARA_PUSH_GATEWAY_URL="https://push.example.com/_matrix/push/v1/notify" \
synara-ios/scripts/upload-testflight-internal.sh
```

Optional environment variables:

```sh
SYNARA_IOS_NOTIFICATION_SERVICE_PROVISIONING_PROFILE="..." \
SYNARA_IOS_ARCHIVE_ROOT=/tmp \
synara-ios/scripts/upload-testflight-internal.sh
```

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

The script disables Xcode-managed build-number mutation so App Store Connect uses
the build number committed in `Synara.xcodeproj` and `project.yml`.

After upload, Apple still performs server-side processing. If App Store Connect
shows "Missing Compliance", answer the export-compliance prompt before the build
can be installed through TestFlight.

The upload uses `testFlightInternalTestingOnly`, and App Store Connect makes the
processed build available to internal testers through the configured internal
distribution settings. Do not run `promote-testflight-internal.rb` as part of the
normal upload path.


## GitHub Workflow Integration (Release)

The iOS TestFlight upload is now wired into the release workflow (`.github/workflows/release-desktop.yml`).

When a GitHub Release for a `v*` tag is published:
- The `ios-testflight` job runs on `macos-latest`.
- It generates the Xcode project, imports certs/profiles from secrets, and runs the upload script.
- By default uploads with `testFlightInternalTestingOnly: true` (internal testers).

**Required GitHub Secrets** (set in repo Settings > Secrets and variables > Actions):
- `SYNARA_IOS_TEAM_ID`
- `SYNARA_IOS_PROVISIONING_PROFILE` (profile name)
- `SYNARA_PUSH_GATEWAY_URL` (production)
- `IOS_DISTRIBUTION_CERT_BASE64` (p12 for Apple Distribution)
- `IOS_CERT_PASSWORD`
- `IOS_APP_PROVISIONING_PROFILE_BASE64` (and optional `IOS_NSE_...`)
- `SYNARA_ASC_KEY_BASE64`, `SYNARA_ASC_KEY_ID`, `SYNARA_ASC_ISSUER_ID` (for auth)

To push for external / promote:
- The job hardcodes internal. To change, edit the job env `SYNARA_TESTFLIGHT_INTERNAL_ONLY: "false"` (requires external groups configured in App Store Connect).
- Or promote the processed build manually in App Store Connect.

Pre-release validation: Use the `iOS Skeleton` workflow (unsigned simulator) or local `scripts/ci-build.sh`.
