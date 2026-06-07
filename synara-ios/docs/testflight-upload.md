# TestFlight Internal Upload

Use the local upload script after `MARKETING_VERSION` and `CURRENT_PROJECT_VERSION`
are updated and committed:

```sh
synara-ios/scripts/upload-testflight-internal.sh
```

Defaults:

- Scheme: `Synara`
- Configuration: `Release`
- Team ID: `ABC123DEFG`
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

The script disables Xcode-managed build-number mutation so App Store Connect uses
the build number committed in `Synara.xcodeproj` and `project.yml`.

After upload, Apple still performs server-side processing. If App Store Connect
shows "Missing Compliance", answer the export-compliance prompt before the build
can be installed through TestFlight.
