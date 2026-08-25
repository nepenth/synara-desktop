# Agent Approval Notification Proxy Spec

Reviewed: 2026-08-25

Status: implementation handoff for the Matrix push gateway and APNs notification
proxy that will support Synara iOS agent approval actions.

## Purpose

Synara needs remote iOS notifications that can surface approval prompts from
agent rooms and let the user approve or deny from the native notification. The
proxy must stay privacy-preserving and must not execute commands. Its job is to
translate Matrix push requests into APNs notifications and, when a trusted
approval metadata record exists for the same Matrix event, attach a provisional
APNs category hint. The iOS extension always removes that hint first and only
restores approval actions after the exact event decrypts, matches the shared
classifier, and is younger than five minutes.

Desktop macOS and Linux notification actions are handled by the running desktop
client. This proxy is required for iOS remote push and may be reused by desktop
only if a future remote-push path is designed.

## Current Client Contract

- iOS bundle identifier / APNs topic: configured by the signed app target.
- Apple development team: provide through private signing environment.
- TestFlight uses production APNs, not sandbox APNs.
- Release gateway URL is provided at archive time through
  `SYNARA_PUSH_GATEWAY_URL`.
- Synara iOS registers Matrix pushers with `format: event_id_only`.
- The pusher `pushkey` is the APNs device token.
- Sparse push payloads must contain enough information for app routing:
  `room_id` and `event_id` when available, or at minimum `event_id`.
- iOS notification preview text starts with the APNs `aps.alert.title` and
  `aps.alert.body` values sent by the proxy. The app Notification Service
  Extension may enrich sparse cleartext event notifications on device when the
  lock-screen preview setting is enabled.
- Agent approval APNs category identifier:
  `synara.agent-approval`.
- Registered iOS notification action identifiers exposed on the native category:
  - `agent-approval.review` (first; opens the exact prompt)
  - `agent-approval.approve-once`
  - `agent-approval.deny`
- `agent-approval.approve-always` is intentionally **not** offered on native OS
  notification actions. Permanent approval requires an explicit in-app
  confirmation path; if the action id is still received, the app opens the
  room/event and does **not** send `♾️`.
- Every alert payload must include `aps.mutable-content = 1`. With the user's
  separate time-sensitive approval setting enabled, the notification extension
  can then decrypt and classify a Hermes prompt locally even when the proxy has
  no trusted approval metadata. Review remains first so constrained surfaces
  never make command execution the default action.
- Client-side safety for native/push approval actions (desktop + iOS):
  - require valid kind/action/room/event identifiers;
  - call the shared `matrix_agent_approval_decide` owner, which resolves the
    exact event and applies the detector and reaction under a dedicated
    per-event decision lock without holding the global timeline registry across
    Matrix network awaits or serializing unrelated approval prompts;
  - enforce Hermes's 300-second timeout from the resolved event timestamp;
  - ignore Hermes's bot-owned ✅/♾️/❌ seed reactions while treating any
    current-account terminal reaction as an already-decided prompt;
  - dedupe by room/event, not by action id, so the same client cannot approve
    and then deny from separate notification callbacks.
- A cold-launched iOS notification action joins the same keyed, single-flight
  Matrix-owner startup as the SwiftUI shell before it calls shared core; push
  registration and notification-permission work are deliberately outside this
  critical path. A callback received before dependency binding is retained and
  replayed after binding. A superseded identity or absent restored session
  fails closed and navigates to review rather than reporting success.
- In-app approval cards show bounded full prompt context (reason, multi-line
  command including heredocs, and reply/reaction instructions).
- In-app UI maps the three reactions on the approval prompt event:
  - `agent-approval.approve-once` -> `✅` (one click)
  - `agent-approval.approve-always` -> `♾️` (in-app only; requires explicit
    confirmation before send on web/desktop and iOS room cards)
  - `agent-approval.deny` -> `❌` (one click)
- iOS Settings → Notifications → Local Delivery Diagnostics exposes the most
  recent 48 notification-extension stage codes and timestamps from App Group
  storage. This device-only flight recorder never stores push payloads,
  room/event/user IDs, sender names, message content, tokens, URLs,
  credentials, or raw error text. Its fixed codes distinguish invalid payloads,
  disabled preferences, missing shared session/store state, queued/cancelled
  resolution, core resolution failure, successful preview/approval
  classification, final delivery, and the system deadline.

### External / not complete in this repo

The following remain **outside** this repository and must not be treated as done
by client-only remediations:

- Optional notification proxy trusted approval metadata ingest and provisional
  category hint (`POST /v1/agent-approval-events` + matching Matrix push).
- Production APNs / TestFlight end-to-end validation of approval categories.
- Installed-app updater smoke for desktop release channels.
- Large-history timeline performance instrumentation beyond in-app `perfLog`
  diagnostics.

## Required Architecture

The service should support two input paths:

1. Matrix Push Gateway API:
   `POST /_matrix/push/v1/notify`
2. Trusted approval metadata ingest:
   `POST /v1/agent-approval-events`

The Matrix push request is still the delivery trigger because it carries the
device pushkeys selected by the homeserver. The approval metadata endpoint lets
a trusted agent bridge, homeserver module, or notification classifier tell the
proxy that a specific Matrix event is an approval prompt without exposing full
command text to APNs.

The proxy stores approval metadata in a short-lived cache keyed by:

```text
homeserver + room_id + event_id
```

When a Matrix push arrives for the same key, the proxy may send an APNs payload
with the `synara.agent-approval` category as a routing hint. The notification
extension removes any incoming approval category before resolution; the hint
never grants reaction controls. If no metadata exists, the proxy sends a
generic Synara notification and the extension can still classify it locally.

This split is important: because Synara uses `event_id_only`, a normal Matrix
push gateway cannot inspect encrypted event content. Do not treat proxy metadata
or a category value as authorization, and do not attach approve/deny actions to
every notification as a workaround.

## Approval Metadata Endpoint

Suggested request:

```json
{
  "homeserver": "matrix.example.com",
  "room_id": "!room:matrix.example.com",
  "event_id": "$approval:matrix.example.com",
  "sender": "@agent:matrix.example.com",
  "title": "Approval Required: Dangerous Command",
  "body": "Security scan requires approval.",
  "expires_at": "2026-07-08T16:30:00Z"
}
```

Rules:

- Require authentication, for example `Authorization: Bearer <shared secret>`.
- Require HTTPS in deployed environments.
- Require non-empty `homeserver`, `room_id`, and `event_id`.
- Store metadata for a bounded TTL, default 15 minutes.
- Cap `title` at 120 characters and `body` at 240 characters.
- Do not require or store full command text.
- Reject payloads that include access tokens, APNs tokens, Matrix access tokens,
  recovery keys, or unbounded command bodies.
- Return `202 Accepted` after storing a valid record.

## Matrix Push Gateway Behavior

For `POST /_matrix/push/v1/notify`:

- Accept the Matrix Push Gateway API request shape used by Synara pushers.
- For each device, treat the Matrix pusher `pushkey` as an APNs device token.
- Validate `app_id` against the configured APNs topic before sending APNs.
- Extract `room_id`, `event_id`, badge counts, and any safe route fields.
- Look up approval metadata by homeserver, room_id, and event_id.
- Send one APNs request per target device.
- Return the Matrix push response with invalid APNs tokens in `rejected`.

Expected response shape:

```json
{
  "rejected": []
}
```

If APNs returns invalid-token statuses such as `BadDeviceToken`,
`Unregistered`, or `DeviceTokenNotForTopic`, include that pushkey in
`rejected` so the homeserver can stop using it.

## APNs Payloads

Generic Matrix notification:

```json
{
  "aps": {
    "alert": {
      "title": "Synara",
      "body": "New activity"
    },
    "badge": 3,
    "sound": "default",
    "mutable-content": 1
  },
  "room_id": "!room:matrix.example.com",
  "event_id": "$event:matrix.example.com",
  "synara": {
    "kind": "matrix-event"
  }
}
```

The proxy should always send a non-empty `aps.alert` for user-visible pushes.
If decrypted/safe event content is unavailable because the pusher uses
`event_id_only` or the room is encrypted, use a generic but explicit fallback
such as title `Synara` and body `New activity`. An APNs payload with no
`aps.alert.body`, an empty string body, or only `content-available` can deliver
without useful preview text in Notification Center.

Agent approval notification (the category is provisional and is locally
removed/revalidated by the extension):

```json
{
  "aps": {
    "alert": {
      "title": "Approval Required: Dangerous Command",
      "body": "Security scan requires approval."
    },
    "category": "synara.agent-approval",
    "badge": 3,
    "sound": "default",
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

APNs action identifiers are not sent in the payload. They are registered in the
iOS app as part of the `synara.agent-approval` notification category.

## Configuration Requirements

Minimum environment variables:

```text
SYNARA_PUSH_BIND=127.0.0.1:8080
SYNARA_PUBLIC_BASE_URL=https://push.example.com
SYNARA_ALLOWED_HOMESERVERS=matrix.example.com
SYNARA_AGENT_APPROVAL_WEBHOOK_TOKEN=<redacted>
SYNARA_APNS_MODE=mock|sandbox|production
SYNARA_APNS_TEAM_ID=<apple-team-id>
SYNARA_APNS_KEY_ID=<apple-key-id>
SYNARA_APNS_KEY_PATH=/run/secrets/synara-apns-auth-key.p8
SYNARA_APNS_TOPIC=<apns-topic>
SYNARA_LOG_LEVEL=info
```

Local development can use `SYNARA_APNS_MODE=mock`, which should validate inputs
and log the APNs request envelope without contacting Apple. A physical debug
device needs `sandbox`; TestFlight and App Store builds need `production`.

## Security Requirements

- Never execute command text.
- Never approve or deny on the server side.
- Never send Matrix access tokens, APNs tokens, recovery keys, full command
  bodies, or decrypted private message content in APNs payloads.
- Keep APNs `.p8` keys out of the repository and mounted from a secret store.
- Redact pushkeys and tokens in logs. Hash them if correlation is needed.
- Restrict metadata ingest to trusted callers.
- Restrict Matrix notify ingress by network policy, reverse proxy allowlist, or
  deployment-specific authentication where possible.
- Cap request bodies and reject malformed JSON.
- Deduplicate by `pushkey + room_id + event_id + notification kind` for a short
  TTL to avoid repeated APNs sends during homeserver retries.
- Use APNs `apns-topic` from `SYNARA_APNS_TOPIC`.
- Use APNs `apns-push-type: alert`.
- Use an APNs collapse id derived from the Matrix event, for example a bounded
  hash of `synara:<room_id>:<event_id>`.

## Observability Requirements

Expose:

- `GET /healthz` returning process health.
- `GET /readyz` returning APNs configuration readiness and cache availability.
- Structured logs with request id, notification kind, APNs status, APNs reason,
  redacted pushkey hash, room/event hashes, and elapsed time.
- Counters for Matrix notify requests, APNs successes, APNs failures, rejected
  pushkeys, approval metadata ingests, approval cache hits, and approval cache
  misses.

Logs must be useful enough to debug a failed notification without revealing the
full Matrix event body or APNs token.

## Acceptance Tests

The implementation should include automated tests for:

- Matrix `event_id_only` payload without metadata sends a generic APNs payload.
- Generic APNs payloads include non-empty `aps.alert.title` and
  `aps.alert.body` preview fields.
- Approval metadata followed by a matching Matrix push sends
  `aps.category = synara.agent-approval`.
- Matrix push before metadata either sends generic notification or waits only for
  a documented bounded grace period.
- Unknown `app_id` is ignored or rejected without contacting APNs.
- Invalid APNs token responses are returned in Matrix `rejected`.
- Mock APNs mode records the exact payload envelope.
- Payload validation rejects missing `room_id` or `event_id` on metadata ingest.
- Payload validation caps title/body length.
- APNs payloads do not contain command text, access tokens, or raw APNs tokens.
- Duplicate Matrix retries do not create duplicate APNs sends inside the dedupe
  TTL.

## Local Smoke Test

1. Start the proxy in mock APNs mode.
2. `POST /v1/agent-approval-events` with a known `room_id` and `event_id`.
3. `POST /_matrix/push/v1/notify` with the same event and a fake APNs token.
4. Confirm the logged APNs envelope contains:
   - `aps.category = synara.agent-approval`
   - the same `room_id`
   - the same `event_id`
   - no command body
5. Repeat with a non-approval event and confirm no `aps.category` is present.
6. Switch to APNs sandbox with a physical debug device token and confirm Apple
   returns success.
7. Switch to APNs production for TestFlight validation.

## Information Needed At Handoff

When the proxy is implemented or locally deployed, return:

- Repository/path and commit SHA for the proxy implementation.
- Runtime command or service unit used to start it.
- Local URL and deployed URL.
- Health and readiness endpoint output.
- Redacted environment summary: APNs mode, topic, team id, key id, gateway URL,
  allowed homeservers, and metadata TTL.
- Example approval metadata request.
- Example Matrix notify request.
- Example generic APNs mock envelope.
- Example agent approval APNs mock envelope.
- One successful APNs sandbox or production response, with token redacted.
- One invalid-token response proving `rejected` mapping works.
- Logs for one generic notification and one agent approval notification.
- Test command and passing test output.
- Known limitations and any manual setup still required.

## LLM Handoff Template

Use this prompt when handing the proxy work to another implementation agent:

```text
You are implementing the Synara notification proxy for Matrix-to-APNs delivery.

Context:
- Synara iOS bundle id / APNs topic is supplied by the signed app target.
- TestFlight uses production APNs.
- Synara registers Matrix pushers with format event_id_only.
- The Matrix pusher pushkey is the APNs device token.
- The iOS app registers category synara.agent-approval with native actions:
  agent-approval.review,
  agent-approval.approve-once, agent-approval.deny.
- Approve-always is in-app only; do not expect native ♾️ from notification actions.
- iOS maps approve-once/deny notification actions to Matrix reactions ✅ / ❌
  only through the shared-core event readback, classifier, current-account
  terminal-state check, and 300-second TTL gate.

Build:
1. POST /_matrix/push/v1/notify for Matrix push gateway delivery.
2. POST /v1/agent-approval-events for trusted short-lived approval metadata.
3. GET /healthz and GET /readyz.
4. APNs mock, sandbox, and production modes.
5. Redacted structured logging and invalid-token rejected mapping.

Important:
- Do not execute commands.
- Do not approve or deny on the server.
- Do not attach approve/deny buttons unless approval metadata matches the
  Matrix event.
- Do not include command text, access tokens, or APNs tokens in APNs payloads.
- Do include safe, bounded fallback preview text in `aps.alert`; the iOS
  extension can enrich cleartext events only when the device has a session,
  previews are enabled, and the lookup completes within the extension budget.

Acceptance:
- Generic event_id_only Matrix push sends a generic APNs payload.
- Matching approval metadata plus Matrix push sends aps.category
  synara.agent-approval and includes room_id/event_id.
- Invalid APNs tokens are returned as Matrix rejected pushkeys.
- Tests cover validation, dedupe, mock APNs payloads, and secret redaction.

Return when done:
- Repo/path, commit SHA, run command, local/deployed URLs, env summary with
  secrets redacted, health/ready output, sample requests, sample APNs mock
  envelopes, APNs success/failure evidence, test output, and known limitations.
```
