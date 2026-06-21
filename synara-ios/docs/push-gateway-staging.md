# Push Gateway Staging Runbook

Purpose: prepare the Matrix-to-APNs delivery path for iOS push validation.

## Current Implementation State

- Client-side Matrix pusher registration now includes `app_id`, `pushkey`, and
  gateway URL payload data.
- Release builds read `SynaraPushGatewayURL` from the app bundle and currently
  default to `https://push.whyland.com/_matrix/push/v1/notify`.
- Local/debug runs can still override the bundled value with
  `SYNARA_PUSH_GATEWAY_URL`.
- Push routing and badge handling are fully implemented in app runtime.
- `https://push.whyland.com/_matrix/push/v1/notify` is the configured
  production/TestFlight pusher endpoint.
- The deployed gateway is expected to stay in mocked APNs mode until the Apple
  APNs auth key is installed and `SYNARA_APNS_MODE=real` is enabled.

In addition, unit coverage now verifies gateway registration payload shape and endpoint path for both set/delete operations:

- `MatrixPusherPayload` includes required `app_id`, `pushkey`, `kind: http`, and `data.url` / `data.format`.
- `pushers/set` and `pushers/delete` payload shapes are asserted in `PushServiceTests`.

Run the related tests:

```sh
cd synara-ios
xcodebuild -project Synara.xcodeproj -scheme Synara -destination 'platform=iOS Simulator,name=iPhone 16' -only-testing:SynaraTests/PushServiceTests test
```

## Required Inputs

1. Matrix homeserver with a writable `pushers/set` endpoint.
2. Staging Matrix push gateway that accepts Matrix Push Gateway API payloads.
3. Production APNs key + app bundle credentials bound to the target iOS app ID.
   TestFlight uses production APNs; do not use sandbox for the TestFlight smoke.

## Gateway URL Wiring

- Release/TestFlight builds use the bundled `SynaraPushGatewayURL` Info.plist
  value.
- Set `SYNARA_PUSH_GATEWAY_URL` in runtime launch or CI/test environment when a
  local override is needed:

```sh
export SYNARA_PUSH_GATEWAY_URL="https://push.example.internal"
```

Gateway URL is currently resolved from:

- `SYNARA_PUSH_GATEWAY_URL`
- `SynaraPushGatewayURL` in `Synara/App/Info.plist`

## Local Smoke Checklist

1. Launch a signed TestFlight/device build.
2. Log in with a test account.
3. Trigger notifications permission.
4. Confirm registration state updates in Settings (`Register Push`).
5. Confirm homeserver receives push registration and returns success.
6. Send a test room message to trigger Matrix gateway flow.
7. Confirm iOS device receives notification and badge updates.

## Production Gate Notes

- Keep gateway credentials in a secret store.
- Redact tokens and device IDs in logs.
- Keep payloads generic (`event_id_only` where possible).
- Verify sandbox and production APNs endpoints are partitioned.

## Known External Dependencies

- Matrix push gateway deployment.
- iOS device access (or simulator-equivalent with full APNs simulation).
- App Store entitlement profile for Push Notifications in signed builds.
