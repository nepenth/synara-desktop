# iOS TestFlight Readiness

Reviewed: 2026-08-17

Status: internal TestFlight delivery is operational: Apple Developer enrollment,
bundle identifiers, signing, App Store Connect, API-key upload, and internal
tester assignment have been exercised by prior builds. The 2026-08-17 branch
proof covers simulator build, deterministic tests, and signed live Matrix tests;
it does not claim a new archive or upload from this unmerged branch. External
TestFlight and App Store release remain gated by the legal, privacy, push, E2EE,
and physical-device requirements below.

## Internal Tester Instructions

1. Install the TestFlight build.
2. Launch Synara and enter the test homeserver.
3. Log in with the disposable Matrix test account supplied out-of-band.
4. Confirm joined rooms load, including the encrypted test room.
5. Open a room, send a disposable message, and verify it appears in the timeline.
6. Open Settings, review About/Licenses/Privacy/Support, then log out and confirm the app returns to homeserver selection.

## Review Notes Draft

- Synara is a native Matrix client focused on agentic workflows.
- Test credentials use a disposable Matrix account and private test rooms.
- Push notification gateway is staging-gated; push behavior should be reviewed only when APNs and pusher credentials are configured.
- Encrypted-room production support remains a release gate until the app-facing Matrix Rust SDK crypto integration is complete.

## Known Issues Before External Testing

- Final support URL and privacy policy URL are placeholders until approved.
- App Store legal review is still required for AGPL/App Store distribution.
- Production push gateway setup is not complete.
- Physical-device push, memory, performance, and upgrade testing must be rerun
  against the exact release candidate.

## Release Candidate Command

```sh
xcodebuild -project Synara.xcodeproj -scheme Synara -configuration Release archive
```

The archive must be validated in Xcode Organizer or App Store Connect before
external testers are invited. Prefer the repository release workflow for an
actual candidate so versioning, desktop artifacts, TestFlight upload, and
provenance stay on the same immutable commit.
