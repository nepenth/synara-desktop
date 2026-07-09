# Push Gateway Staging Runbook

Purpose: prepare the Matrix-to-APNs delivery path for iOS push validation.

Detailed proxy requirements and the implementation handoff template live in
[`../../docs/agent-approval-notification-proxy-spec.md`](../../docs/agent-approval-notification-proxy-spec.md).

## Live Infrastructure (2026-07-09)

| Item | Value |
| --- | --- |
| Public Matrix pusher URL | `https://push.whyland.com/_matrix/push/v1/notify` |
| CTID / hostname / IP | `116` / `synara-push-gateway` / `10.0.10.39/23` |
| Reverse proxy | Caddy on LXC114 (`10.0.10.14`), host `push.whyland.com` |
| Runtime | Rust binary `/usr/local/bin/synara-push-gateway` |
| Systemd unit | `synara-push-gateway.service` (enabled, `onboot=1`) |
| Config | `/etc/synara-push-gateway/env` |
| APNs topic / app_id | `com.whylandcreative.synara` |
| Apple team id | `NK6CM9YJC6` |
| Current APNs mode | **mock** until AuthKey `.p8` is installed |
| APNs environment target | **production** (required for TestFlight) |
| Prometheus | `synara-push-gateway` scrape on `10.0.10.39:9100` |

Verified from Matrix homeserver LXC (`matrix-synapse`):

```text
POST https://push.whyland.com/_matrix/push/v1/notify -> 200 {"rejected":[]}
```

Public `/healthz` remains 404 by design. Backend health/ready stay LAN-only.

Caddy currently source-restricts `push.whyland.com` to Whyland LAN/global
prefix (`10.0.0.0/8`, `2605:59ca::/32`, loopback/link-local). That is correct
for homeservers under Whyland. External homeservers (for example `matrix.org`)
will receive `403` until that allowlist is intentionally widened.

## Current Implementation State

- Client-side Matrix pusher registration includes `app_id`, `pushkey`, and
  gateway URL payload data.
- Release builds read `SynaraPushGatewayURL` from the app bundle and default to
  `https://push.whyland.com/_matrix/push/v1/notify` via `project.yml`.
- Local/debug runs can still override with `SYNARA_PUSH_GATEWAY_URL`.
- Debug builds use `APS_ENVIRONMENT=development` (sandbox APNs tokens). Do **not**
  expect Debug tokens to succeed against the production gateway once real APNs
  mode is enabled. Use Release/TestFlight for the production smoke.
- Push routing and badge handling are fully implemented in app runtime.
- Agent approval notifications register the `synara.agent-approval` APNs
  category and handle `agent-approval.approve-once`,
  `agent-approval.deny` actions by sending Matrix reactions (`✅`, `❌`) after
  revalidating the focused Matrix event. `agent-approval.approve-always` is not
  offered natively and requires in-app confirmation for `♾️`.
- Synara iOS registers pushers with `format: event_id_only`, so the push
  gateway cannot infer approval prompts from encrypted Matrix event content. The
  gateway needs a trusted approval metadata path keyed by `room_id` and
  `event_id` before it can attach approval actions to APNs notifications.
- Gateway source lives on the deploy host at `/home/nepenthe/synara-push-gateway-src`
  (not yet published as a separate public repo).

In addition, unit coverage verifies gateway registration payload shape and endpoint path for both set/delete operations:

- `MatrixPusherPayload` includes required `app_id`, `pushkey`, `kind: http`, and `data.url` / `data.format`.
- `pushers/set` and `pushers/delete` payload shapes are asserted in `PushServiceTests`.

Run the related tests:

```sh
cd synara-ios
xcodebuild -project Synara.xcodeproj -scheme Synara -destination 'platform=iOS Simulator,name=iPhone 16' -only-testing:SynaraTests/PushServiceTests test
```

## GitHub Repository Variables

These non-secret repository variables are set on `nepenth/synara-desktop`:

| Variable | Value |
| --- | --- |
| `SYNARA_PUSH_GATEWAY_URL` | `https://push.whyland.com/_matrix/push/v1/notify` |
| `SYNARA_PUSH_PUBLIC_BASE_URL` | `https://push.whyland.com` |
| `SYNARA_PUSH_APP_ID` | `com.whylandcreative.synara` |
| `SYNARA_APNS_TOPIC` | `com.whylandcreative.synara` |
| `SYNARA_APNS_TEAM_ID` | `NK6CM9YJC6` |

Existing desktop signing secrets remain separate (`APPLE_*`, `TAURI_*`). The
APNs **provider** `.p8` key MUST NOT be stored in GitHub Actions for this path;
it belongs only on LXC116 under `/etc/synara-push-gateway/`.

Optional future CI secrets (not required for the live gateway itself):

| Secret | Purpose |
| --- | --- |
| `SYNARA_ASC_KEY_ID` / `SYNARA_ASC_ISSUER_ID` / `SYNARA_ASC_KEY_BASE64` | App Store Connect API for TestFlight upload automation |
| `SYNARA_IOS_PROVISIONING_PROFILE_BASE64` | Manual/App Store profile if CI archives device builds |

## Remaining Owner Action: Install APNs Auth Key

The gateway will stay in mock mode until Apple AuthKey material is installed.

Provide:

1. APNs Auth Key file: `AuthKey_<KEYID>.p8`
2. Key ID (10-character Apple key id)
3. Confirm Team ID is still `NK6CM9YJC6`

Install on LXC116:

```bash
# from a host that can SSH to Proxmox/LXC
KEY_ID=<apple-key-id>
scp AuthKey_${KEY_ID}.p8 nepenthe@10.0.10.3:/tmp/
ssh nepenthe@10.0.10.3 "sudo pct push 116 /tmp/AuthKey_${KEY_ID}.p8 /etc/synara-push-gateway/AuthKey_${KEY_ID}.p8 && sudo pct exec 116 -- bash -lc '
  chown root:synara-push /etc/synara-push-gateway/AuthKey_${KEY_ID}.p8
  chmod 640 /etc/synara-push-gateway/AuthKey_${KEY_ID}.p8
  sed -i \
    -e \"s|^SYNARA_APNS_KEY_ID=.*|SYNARA_APNS_KEY_ID=${KEY_ID}|\" \
    -e \"s|^SYNARA_APNS_KEY_PATH=.*|SYNARA_APNS_KEY_PATH=/etc/synara-push-gateway/AuthKey_${KEY_ID}.p8|\" \
    -e \"s|^SYNARA_APNS_MODE=.*|SYNARA_APNS_MODE=real|\" \
    -e \"s|^SYNARA_APNS_TEAM_ID=.*|SYNARA_APNS_TEAM_ID=NK6CM9YJC6|\" \
    /etc/synara-push-gateway/env
  systemctl restart synara-push-gateway
  systemctl status synara-push-gateway --no-pager
  curl -fsS http://127.0.0.1:8080/readyz
'"
```

After install, `readyz` should report `apns_mode":"real"` (or equivalent real-mode readiness).

## Required Inputs

1. Matrix homeserver with a writable `pushers/set` endpoint.
2. Live Matrix push gateway at `https://push.whyland.com/_matrix/push/v1/notify`.
3. Production APNs key + app bundle credentials bound to `com.whylandcreative.synara`.
   TestFlight uses production APNs; do not use sandbox for the TestFlight smoke.

## Gateway URL Wiring

- Release/TestFlight builds use the bundled `SynaraPushGatewayURL` Info.plist
  value from `SYNARA_PUSH_GATEWAY_URL` in `project.yml`.
- Set `SYNARA_PUSH_GATEWAY_URL` in runtime launch or CI/test environment when a
  local override is needed:

```sh
export SYNARA_PUSH_GATEWAY_URL="https://push.example.internal"
```

Gateway URL is currently resolved from:

- `SYNARA_PUSH_GATEWAY_URL` process environment
- `SynaraPushGatewayURL` in `Synara/App/Info.plist` (`$(SYNARA_PUSH_GATEWAY_URL)`)

## App Pusher Contract

```text
kind: http
app_id: com.whylandcreative.synara
pushkey: APNs device token
data.format: event_id_only
data.url: https://push.whyland.com/_matrix/push/v1/notify
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
      "body": "Room: security scan requires approval"
    },
    "category": "synara.agent-approval"
  },
  "room_id": "!room:matrix.org",
  "event_id": "$approval:matrix.org"
}
```

The app maps native APNs action identifiers to Matrix reactions after timeline
revalidation of the focused approval event:

- `agent-approval.approve-once` -> `✅`
- `agent-approval.deny` -> `❌`

`agent-approval.approve-always` (`♾️`) is in-app only and requires an explicit
confirmation step on the approval card.

## Local Smoke Checklist

1. Launch a signed **Release/TestFlight** device build (not Debug/sandbox).
2. Log in with a test account on a Whyland-reachable homeserver.
3. Trigger notifications permission.
4. Confirm registration state updates in Settings (`Register Push`).
5. Confirm homeserver receives push registration and returns success.
6. Send a test room message to trigger Matrix gateway flow.
7. Confirm iOS device receives notification and badge updates.
8. Tap notification and confirm room/event route or safe fallback.

## Production Gate Notes

- Keep gateway credentials in a secret store / LXC path only.
- Redact tokens and device IDs in logs.
- Keep payloads generic (`event_id_only` where possible).
- Verify sandbox and production APNs endpoints are partitioned.
- Keep Caddy source restriction unless external homeservers are intentionally
  supported.

## Known External Dependencies

- Production APNs AuthKey install on LXC116 (remaining).
- Physical device or TestFlight build for real APNs tokens.
- App Store entitlement/profile with Push Notifications.
- Optional later: agent-approval metadata ingest endpoint on the gateway.

## iOS preview path (NSE)

See [ios-push-preview-nse-handoff.md](./ios-push-preview-nse-handoff.md) for the
Notification Service Extension + lock-screen preview settings work owned by the
iOS app agent. Keep `event_id_only`; do not put message bodies on the APNs path.
