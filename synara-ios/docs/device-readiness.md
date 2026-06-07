# iOS Device Readiness

Reviewed: 2026-05-28

Purpose: capture what is ready for local iPhone testing once Apple Developer
Program enrollment, DUNS verification, team signing, and device access are
available.

## Current Verdict

The iOS app is ready for signed local-device build attempts after Apple account
setup. Simulator-completable Phase 6.7 and Phase 6.9 work is complete. The
remaining release gates require external state: Apple signing, a connected
physical iPhone, APNs credentials, and legal/privacy review.

## Before First Device Run

1. Complete Apple Developer Program enrollment for the LLC.
2. Add the Apple team in Xcode.
3. Connect an iPhone with Developer Mode enabled.
4. Open `synara-ios/Synara.xcodeproj`.
5. Select the `Synara` scheme and the connected iPhone.
6. Set the signing team for `com.whylandcreative.synara`.
7. Build and run from Xcode once to trust the profile on device.

## First Device Smoke

Use the disposable Matrix account only.

1. Launch Synara.
2. Sign in to the test homeserver.
3. Confirm session persists after force quit and relaunch.
4. Open the room list and verify Favorites/Other grouping.
5. Open the live test room.
6. Send a plain text message.
7. Open the attachment sheet.
8. Confirm Photo or Video opens the system picker.
9. Confirm unavailable attachment types show a clear unavailable alert.
10. Open the agent approval room/card if seeded.
11. Approve/reject a disposable agent card.
12. Open the encrypted test room.
13. Confirm encrypted-room status appears and no raw encrypted event dumps are
    shown.
14. Logout and confirm relaunch returns to signed-out state.

## Device Performance Pass

Required before external TestFlight:

- Cold launch trace.
- Warm resume trace.
- Signed-in session start trace.
- Room-list first render trace.
- Timeline first render and scroll trace.
- Message send trace.
- Media upload/picker trace.
- Encrypted room open trace.
- Memory graph after large timeline scroll.
- Memory graph after media viewer/upload path.

Record device model, iOS version, build configuration, commit, major CPU stacks,
main-thread stalls, memory growth, and any P0/P1 remediation.

## Still Blocked Externally

- Physical-device run and profiling.
- APNs registration and push receive/tap/badge validation.
- TestFlight archive/upload.
- Privacy policy and support URLs.
- AGPL/App Store legal review.
- Production E2EE release completion: recovery, verification/cross-signing, key
  backup restore, and encrypted media decryption.
