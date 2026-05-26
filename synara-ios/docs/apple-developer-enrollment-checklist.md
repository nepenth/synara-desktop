# Apple Developer Enrollment Checklist

Reviewed: 2026-05-26

Status: owner-action checklist. No credentials or enrollment state are stored in
this repository.

## Planned Identifiers

| Purpose                        | Bundle ID                            | Phase                      | Notes                                                          |
| ------------------------------ | ------------------------------------ | -------------------------- | -------------------------------------------------------------- |
| iOS app                        | `app.synara.ios`                     | Phase 1                    | Primary native app target.                                     |
| Notification service extension | `app.synara.ios.NotificationService` | Post-MVP or push hardening | Add only if rich or mutable notification handling is required. |
| Share extension                | `app.synara.ios.ShareExtension`      | Deferred                   | Add after core messaging and push are stable.                  |
| Existing desktop app           | `app.synara.desktop`                 | Existing                   | Already used by the macOS/Linux Tauri app.                     |

## Owner-Only Tasks

- Confirm the LLC legal name exactly as registered.
- Obtain or locate the LLC D-U-N-S Number.
- Confirm who has legal authority to bind the LLC to Apple agreements.
- Enroll the LLC in the Apple Developer Program as an organization.
- Accept current Apple Developer Program License Agreement terms.
- Create or designate an Apple Account with two-factor authentication.
- Configure App Store Connect access for maintainers.
- Approve App Store Connect API key creation for CI when release automation is
  ready.
- Approve production APNs key creation and rotation policy.
- Approve privacy policy location and support URL.
- Approve legal review for AGPL/App Store distribution before external
  TestFlight or App Store submission.

## Engineering Tasks After Enrollment

- Create explicit App ID for `app.synara.ios`.
- Enable only required capabilities:
  - Push Notifications.
  - Associated Domains.
  - Keychain Sharing only if a second target must share credentials.
  - App Groups only if extensions need shared local state.
  - Background Modes for remote notifications only when justified by push
    architecture.
- Create sandbox APNs configuration for development.
- Add a staging Matrix pusher configuration for test homeservers.
- Add unsigned simulator CI first.
- Add signed device/archive CI only after secrets storage is approved.
- Document where signing secrets live. Prefer GitHub Actions secrets or a
  dedicated secret manager, never repository files.

## Secret Handling Rules

- Do not commit `.p8` APNs keys.
- Do not commit provisioning profiles unless they are public development
  placeholders with no private signing material.
- Do not commit App Store Connect API keys.
- Do not commit Apple ID credentials, team IDs paired with private keys, or
  test account passwords.
- Do not store production Matrix tokens or homeserver admin credentials in CI.

## Acceptance Criteria

- The organization enrollment path is known.
- Bundle IDs and capabilities are documented before Xcode project creation.
- Owner-only and engineering tasks are separated.
- Secret storage is identified before any signed CI or APNs work begins.
