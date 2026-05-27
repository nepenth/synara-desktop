# iOS Privacy Data Inventory

Reviewed: 2026-05-27

Status: draft App Store privacy input for internal TestFlight preparation. External TestFlight remains blocked until the final privacy policy URL is approved.

## Data Classes

| Data class | Examples | Stored locally | Sent to Synara-operated service | Notes |
| --- | --- | --- | --- | --- |
| Account identifiers | Matrix user ID, device ID, homeserver URL | Keychain/session state | No | Required for Matrix login/session restore. |
| Credentials and tokens | Matrix access token, APNs device token | Keychain/APNs registration state | Push token only when push is configured | Access tokens are never logged; APNs token logging is redacted. |
| Messages and rooms | Room IDs, event IDs, room metadata, timeline content, media references | SDK/app cache as needed | No | Data originates from the selected Matrix homeserver. |
| Media and files | Picked image bytes, Matrix media URLs | Transient upload path/cache as needed | No | Uploaded to the selected Matrix homeserver. |
| Notifications | Matrix pusher registration, generic notification route, badge count | Push service state | Push gateway only when configured | Payloads must avoid decrypted message bodies. |
| Diagnostics | Local redacted logs | Device local logs | No | No analytics or crash SDK is enabled. |
| Contacts | None | No | No | The app does not request Contacts access. |
| Analytics/tracking | None | No | No | No tracking SDK or tracking domains are configured. |

## Privacy Manifest

`Synara/Resources/PrivacyInfo.xcprivacy` declares no tracking, no tracking domains, no required-reason API use, and no Synara-collected data categories for the current app target. App Store privacy labels still need human/legal review because users exchange Matrix account and message data with their chosen homeserver.

## Release Gates

- Publish and approve a privacy policy URL before external TestFlight.
- Draft App Privacy labels from this inventory before App Store review.
- Require explicit approval before adding analytics, crash reporting, attribution, advertising, or any third-party SDK with separate data collection.
