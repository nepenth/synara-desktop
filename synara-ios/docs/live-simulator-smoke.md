# Live Simulator Smoke

Use this checklist for live Matrix validation on a local iOS Simulator. Do not
commit test account credentials, access tokens, screenshots with private rooms,
or raw logs that contain secrets.

## Preconditions

- Use a signed local simulator build. The unsigned CI build path is compile-only
  and is not valid for Keychain/session validation.
- Use a dedicated test Matrix account and test room.
- Confirm the app version, simulator model, simulator OS, homeserver, and commit
  SHA before recording results.

## Flow

1. Launch the signed simulator app.
2. Enter the test homeserver and continue to login.
3. Log in with the test account.
4. If the iOS password-save prompt appears, dismiss it unless that prompt is
   part of the validation target.
5. Confirm the Rooms shell appears and no sensitive values are logged.
6. Confirm invites render with accept and decline controls.
7. Accept or reject only invites that are safe to consume; record the state
   mutation in validation notes.
8. Open a joined test room.
9. Confirm the room title matches the room list display name when opened from
   the list.
10. Confirm routine Matrix state events do not dominate the chat timeline.
11. Send a harmless message only when test-room mutation is acceptable.
12. Force quit and relaunch the app to confirm secure session restore.
13. Log out and relaunch to confirm the account is not restored.

## Gated XCTest Smoke

The UI test suite includes `testLiveSmokeWhenConfigured`. It is skipped by
default and can validate a signed simulator session against a disposable room:

```sh
SYNARA_LIVE_SMOKE=1 \
SYNARA_LIVE_ROOM_ID='!room-id:example.org' \
xcodebuild -project Synara.xcodeproj -scheme Synara \
  -destination 'platform=iOS Simulator,id=<simulator-id>' \
  -only-testing:SynaraUITests/SynaraUITests/testLiveSmokeWhenConfigured test
```

If no signed-in session exists on the simulator, also provide
`SYNARA_LIVE_HOMESERVER`, `SYNARA_LIVE_USERNAME`, and `SYNARA_LIVE_PASSWORD` in
the local environment. Never commit those values or paste them into docs.

## Evidence To Capture

- Build mode: signed simulator, unsigned simulator, device, or archive.
- Simulator model and OS version.
- App version and git commit.
- Homeserver domain only, not credentials.
- Redacted runtime logs.
- Screenshots only from test rooms and test accounts.
- Any server-side mutations performed, such as invite acceptance or sent test
  messages.

## Known Automation Notes

- Deterministic UI tests use mock services and may launch directly into a signed
  in room or settings tab for specific feature coverage.
- Live simulator automation should prefer accessibility identifiers, but visual
  smoke may still be needed when XCTest or simulator accessibility snapshots are
  unreliable on a specific iOS runtime.
