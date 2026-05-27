# Push Gateway Staging Runbook

Purpose: prepare the Matrix-to-APNs delivery path for iOS push validation.

## Current Implementation State

- Client-side Matrix pusher registration now includes `app_id`, `pushkey`, and
  gateway URL payload data from `SYNARA_PUSH_GATEWAY_URL`.
- Push routing and badge handling are fully implemented in app runtime.
- No active staging gateway has been attached in this repository yet.

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
3. APNs key + app bundle sandbox credentials bound to the target iOS app ID.

## Environment Wiring

- Set `SYNARA_PUSH_GATEWAY_URL` in runtime launch or CI/test environment:

```sh
export SYNARA_PUSH_GATEWAY_URL="https://push.example.internal"
```

Gateway URL is currently read from:

- `synara-ios/Synara/Services/AppEnvironment.swift`
- `SynaraPushService`

## Local Smoke Checklist

1. Launch a signed simulator/device build.
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
