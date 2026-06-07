# TestFlight Internal Upload

Use the local upload script after `MARKETING_VERSION` and `CURRENT_PROJECT_VERSION`
are updated and committed:

```sh
synara-ios/scripts/upload-testflight-internal.sh
```

Defaults:

- Scheme: `Synara`
- Configuration: `Release`
- Team ID: `NK6CM9YJC6`
- Bundle ID: `com.whylandcreative.synara`
- Provisioning profile: `Synara Matrix App Store`
- Distribution method: `app-store-connect`
- Destination: `upload`
- TestFlight scope: internal testing only

Override defaults with environment variables:

```sh
SYNARA_IOS_TEAM_ID=... \
SYNARA_IOS_PROVISIONING_PROFILE="..." \
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
synara-ios/scripts/promote-testflight-internal.rb
```

Use an App Store Connect API key with at least App Manager access for upload and
TestFlight build management. Admin access is acceptable for this local key if you
want the automation to cover future metadata and tester-management work, but the
private key must remain outside the repository.

Do not commit `.p8` files. The repo ignores `AuthKey_*.p8` and `*.p8` as a
defense-in-depth measure.

The script disables Xcode-managed build-number mutation so App Store Connect uses
the build number committed in `Synara.xcodeproj` and `project.yml`.

After upload, Apple still performs server-side processing. If App Store Connect
shows "Missing Compliance", answer the export-compliance prompt before the build
can be installed through TestFlight.

`promote-testflight-internal.rb` checks the uploaded build status and attempts to
associate it with the configured internal TestFlight group. App Store Connect may
reject explicit assignment to built-in internal groups; in that case the script
reports the build as valid and leaves final propagation/compliance handling to
App Store Connect.
