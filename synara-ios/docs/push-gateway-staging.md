# Push Gateway Staging Runbook

Purpose: prepare the Matrix-to-APNs delivery path for iOS push validation
without publishing operator-specific infrastructure details.

Detailed proxy requirements and the implementation handoff template live in
[`../../docs/agent-approval-notification-proxy-spec.md`](../../docs/agent-approval-notification-proxy-spec.md).

## Public Configuration Shape

Keep live hostnames, IP addresses, container IDs, usernames, and private deploy
paths in operator-private notes or environment files outside this repository.
The public repository should only describe the required variables:

| Variable | Purpose |
| --- | --- |
| `SYNARA_PUSH_GATEWAY_URL` | Matrix pusher endpoint, for example `https://push.example.com/_matrix/push/v1/notify` |
| `SYNARA_PUSH_PUBLIC_BASE_URL` | Public gateway origin, for example `https://push.example.com` |
| `SYNARA_PUSH_APP_ID` | Matrix pusher app id / APNs topic |
| `SYNARA_APNS_TOPIC` | APNs topic for the iOS app |
| `SYNARA_APNS_TEAM_ID` | Apple team identifier |
| `SYNARA_APNS_KEY_ID` | APNs AuthKey identifier |
| `SYNARA_APNS_KEY_PATH` | Secret-mounted APNs `.p8` path |
| `SYNARA_APNS_MODE` | `mock`, `sandbox`, or `production` |

The APNs provider `.p8` key must never be committed. Mount it from a secret
store or deploy-host-only path.

## Current Client Contract

- Synara iOS registers Matrix pushers with `format: event_id_only`.
- Release/TestFlight builds read `SynaraPushGatewayURL` from
  `SYNARA_PUSH_GATEWAY_URL` at archive time.
- Debug/local runs can override `SYNARA_PUSH_GATEWAY_URL` in the launch
  environment.
- Debug builds use development APNs tokens. TestFlight and App Store builds use
  production APNs tokens.
- The push runtime handles routing, badge updates, and agent approval native
  actions.
- The Notification Service Extension can enrich sparse room-message
  notifications on-device when message-content previews are enabled. It uses
  a notification-only, get-only native binding and restores only the target
  room before asking the Matrix SDK notification client to resolve the event.
  Encrypted events can be decrypted when the device already has the required
  keys. Missing sessions or keys, timeouts, unsupported events, and agent
  approval notifications retain the generic gateway-provided text.

Run focused iOS push tests:

```sh
cd synara-ios
xcodebuild -project Synara.xcodeproj -scheme Synara -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:SynaraTests/PushServiceTests -only-testing:SynaraTests/NotificationPreviewSupportTests test
```

## Build-Time Gateway Wiring

The public XcodeGen project intentionally does not hardcode the live gateway URL.
Provide it when archiving a signed release:

```sh
export SYNARA_PUSH_GATEWAY_URL="https://push.example.com/_matrix/push/v1/notify"
```

Gateway URL resolution order:

- `SYNARA_PUSH_GATEWAY_URL` process environment
- `SynaraPushGatewayURL` in `Synara/App/Info.plist`

## TestFlight Upload Inputs

The upload script requires operator-provided signing and gateway values:

```sh
export SYNARA_IOS_TEAM_ID="<apple-team-id>"
export SYNARA_IOS_PROVISIONING_PROFILE="<app-store-profile-name>"
export SYNARA_PUSH_GATEWAY_URL="https://push.example.com/_matrix/push/v1/notify"

# Optional if the notification extension uses a separate manual profile:
export SYNARA_IOS_NOTIFICATION_SERVICE_PROVISIONING_PROFILE="<extension-profile-name>"

synara-ios/scripts/upload-testflight-internal.sh
```

For unattended App Store Connect authentication, pass
`SYNARA_ASC_KEY_PATH`, `SYNARA_ASC_KEY_ID`, and `SYNARA_ASC_ISSUER_ID` from a
private shell or secret store outside the repository.

## App Pusher Contract

```text
kind: http
app_id: <apns-topic>
pushkey: <apns-device-token>
data.format: event_id_only
data.url: https://push.example.com/_matrix/push/v1/notify
```

## Agent Approval Actions

For approval prompts, the push gateway must include the native category and room
event metadata in the APNs payload only after matching trusted approval metadata
for the same `room_id` and `event_id`:

```json
{
  "aps": {
    "alert": {
      "title": "Approval Required: Dangerous Command",
      "body": "Security scan requires approval."
    },
    "category": "synara.agent-approval",
    "mutable-content": 1
  },
  "room_id": "!room:matrix.example.com",
  "event_id": "$approval:matrix.example.com",
  "synara": {
    "kind": "agent-approval",
    "room_id": "!room:matrix.example.com",
    "event_id": "$approval:matrix.example.com"
  }
}
```

The app maps native APNs action identifiers to Matrix reactions after timeline
revalidation of the focused approval event:

- `agent-approval.approve-once` -> `✅`
- `agent-approval.deny` -> `❌`

`agent-approval.approve-always` (`♾️`) is in-app only and requires an explicit
confirmation step on the approval card.

## Local Smoke Checklist

1. Launch a signed Release/TestFlight device build.
2. Log in with a disposable test account on the configured homeserver.
3. Enable notification permission.
4. Confirm registration state updates in Settings.
5. Confirm the homeserver receives push registration and returns success.
6. Send a test room message to trigger Matrix gateway flow.
7. Confirm iOS receives a non-blank notification and badge update.
8. Toggle lock-screen previews off and confirm generic notification text remains.
9. Toggle message-content previews on and confirm cleartext and encrypted events
   with locally available keys can be enriched by the Notification Service
   Extension.
10. Tap notification and confirm room/event route or safe fallback.

## Production Gate Notes

- Keep gateway credentials in a deploy-host secret store.
- Redact tokens and device IDs in logs.
- Keep payloads generic and `event_id_only`.
- Verify sandbox and production APNs endpoints are partitioned.
- Keep network ingress restrictions in private operator docs.

## Known External Dependencies

- Production APNs AuthKey installed outside the repository.
- Physical iOS device or TestFlight build for real APNs tokens.
- App Store entitlement/profile with Push Notifications, App Groups, and
  Keychain Sharing.
- Optional later: agent-approval metadata ingest endpoint on the gateway.
- Release remains blocked until a real APNs delivery on a physical device
  proves preview ON/OFF behavior and records peak NSE physical footprint with
  headroom below Apple's extension memory ceiling, including encrypted,
  timeout, and burst/cancellation paths.
