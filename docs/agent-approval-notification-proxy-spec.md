# Agent Approval Notification Proxy Spec

Reviewed: 2026-10-04

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
- Every event alert must carry both `room_id` and `event_id` as strings,
  consistently across any repeated root/nested fields. The NSE cannot resolve
  an exact event without both. Event-only references can help foreground tap
  routing, but cannot provide extension previews or approval actions.
- Matrix may send unread-count-only notifications without either ID. Translate
  these to badge-only APNs updates; do not add `aps.alert` or `mutable-content`
  or manufacture a visible “New activity” notification. Include `aps.badge: 0`
  when the authoritative aggregate is zero so the OS can clear a stale badge.
- The proxy sends generic `aps.alert` fallback text. The Notification Service
  Extension fetches/decrypts the exact event on device and composes content only
  when the message-preview preference is enabled. Gateway text and approval
  hints never bypass local validation.
- Agent approval APNs category identifier:
  `synara.agent-approval`.
- Registered iOS notification action identifiers exposed on the native category:
  - `agent-approval.approve-once`
  - `agent-approval.deny`
  - `agent-approval.review` (opens the exact prompt)
- `agent-approval.approve-always` is intentionally **not** offered on native OS
  notification actions. Permanent approval requires an explicit in-app
  confirmation path; if the action id is still received, the app opens the
  room/event and does **not** send `♾️`.
- Every alert payload must include `aps.mutable-content = 1`. With the user's
  separate time-sensitive approval setting enabled, the notification extension
  can then decrypt and classify a Hermes prompt locally even when the proxy has
  no trusted approval metadata. Tapping the notification body reviews the exact
  prompt. Approve once and Deny require authentication and foreground the app.
- The extension's generic and deadline fallbacks remove all provisional
  approval presentation and gateway-supplied message text. Locally resolved,
  fresh approval prompts use Critical level and a critical sound only when OS
  Critical Alert authorization is enabled; otherwise they use Time Sensitive.
  Critical Alerts require Apple-approved signing entitlements and user permission.
  The proxy must not set critical urgency or sound as a substitute for local
  classification. See the [Apple notification review and signing configuration](reviews/2026-10-04-notification-previews-and-critical-approvals.md).
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
  recent bounded notification-extension stage codes and timestamps from App Group
  storage. This device-only flight recorder never stores push payloads,
  room/event/user IDs, sender names, message content, tokens, URLs,
  credentials, or raw error text. Its fixed codes distinguish invalid payloads,
  disabled preferences, missing shared session/store state, queued/cancelled
  resolution, core resolution failure, successful preview/approval
  classification, final delivery, and the system deadline. This branch adds
  Copy/Share Diagnostic Report for up to 256 entries, plus fixed reasons for
  missing/ambiguous payload references and typed Core failure categories.

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
- If both IDs are absent and the request only updates unread counts, send a
  badge-only update with no visible alert. Reject/log malformed event requests
  separately; do not guess room/event IDs or substitute an unrelated event.
- Look up approval metadata by homeserver, room_id, and event_id.
- Send one APNs request per target device.
- Return the Matrix push response with invalid APNs tokens in `rejected`.

### Confirmed counts only gateway repair

The October 4 infrastructure evidence identifies the deployed source as
`/home/nepenthe/synara-push-gateway-src` on `hermes-agent`, commit
`906b9a01b4c438a84e639d4d802aff0373641ebd`. The deployed binary matched that
source at the time of capture; the maintainer subsequently reported the gateway
repair completed before preparation of the client release.
At 20:12:11 EDT, Synapse sent a legitimate counts-only update, with no
`room_id` or `event_id`, legacy `id: ""`, `sender: ""` and `type: null`.
`build_apns_message` in `src/main.rs` incorrectly emitted a visible alert,
sound, mutable content and `synara.kind: matrix-event`. The iOS extension
correctly could not resolve an event from that push. This section is a repair
handoff for the infrastructure owner; this client repository has not changed
or deployed that server.

Classify the request before approval metadata lookup or alert composition:

1. Both valid, nonempty room and event IDs: construct an event alert.
2. Both IDs absent and a valid counts object present: construct a badge update.
   Legacy empty `id` and `sender` fields do not make this an event.
3. Partial or invalid event references, or neither counts nor event references:
   report a malformed request without inventing an alert or guessing IDs.
   Payload validation failures must not mark valid device tokens as rejected.

The badge branch must preserve the existing authoritative aggregate convention
and serialize its value even when zero. Matrix permits zero-valued count members
to be omitted: a present `counts: {}` must follow that convention, rather than
being mistaken for an event or absent counts. Reject booleans, negative,
fractional or out-of-range counts. Do not turn an absent counts object on an
unrelated malformed request into a badge clear.

Use this badge payload, replacing zero with the authoritative nonzero count
when appropriate:

```json
{
  "aps": { "badge": 0 }
}
```

Retain `content-available: 1` only if the existing background bookkeeping needs
it. Omit `alert`, `sound`, `mutable-content`, `category`, `interruption-level`,
approval metadata and Matrix event routing hints. Send `apns-push-type: alert`,
the configured app topic and `apns-priority: 5`. Apple defines this push type to
include badge updates; it does not imply a visible banner. A background push
type is for a payload without alert, sound or badge, and is not the direct
badge-update route. Missing Matrix `prio` must not add sound or priority 10 to
the badge branch. See [Apple APNs request headers](https://developer.apple.com/documentation/usernotifications/sending-notification-requests-to-apns)
and [background updates](https://developer.apple.com/documentation/usernotifications/pushing-background-updates-to-your-app).

The event branch retains a nonempty generic alert, `mutable-content: 1` and exact
room/event IDs. Preserve low-priority events as priority 5 without sound;
normal/high events use the existing priority 10 policy. Only trusted, matched
approval metadata may promote an event at the gateway; the client still verifies
the actual event before adding approval actions or urgency.
Every branch with authoritative counts, including event alerts, must emit
`aps.badge: 0` when the aggregate is zero. The badge-only branch is not the only
place where the current `badge > 0` condition must change.

Use a stable, bounded collapse identifier derived from the full event identity
and a separate namespace for badge snapshots. A domain-separated SHA-256 hex
digest over a canonical or length-prefixed homeserver, room, event and kind
tuple fits APNs's 64-byte limit. Avoid stripping punctuation from room IDs,
which can collide, and avoid one collapse ID for all events in a room, which can
replace a pending approval with a later unrelated message. Badge snapshots can
coalesce by recipient/topic/account identity where available. Event retry
deduplication belongs to exact event identity and must record successful APNs
submission; counts-only updates are idempotent and need no event deduplication.
Scope event deduplication by recipient device, topic and APNs environment so one
device's successful submission does not suppress another device's delivery.
This follows the [Matrix Push Gateway contract](https://spec.matrix.org/latest/push-gateway-api/).

Required gateway regression coverage:

- Counts-only zero, nonzero and omitted-zero members; exact payload and headers;
  absent `prio`; legacy empty fields; no alert, sound or NSE invocation.
- Partial/invalid IDs and invalid counts produce no invented alert.
- Low-priority valid event retains exact IDs, no sound and priority 5; normal
  events retain their alert behavior. Both paths serialize authoritative zero
  counts as `aps.badge: 0`.
- Distinct approvals in the same room have distinct collapse IDs; punctuation
  variants do not collide; retrying one event retains its ID; badge and event
  namespaces do not replace one another.
- Invalid APNs tokens still populate `rejected`; other failures do not reject
  otherwise valid tokens or record a successful event delivery.

After a reviewed deployment, verify count zero clears the physical app badge
without a Lock Screen/Watch alert, then verify a new event still produces a
preview. The separate 20:11 client fetch failure remains open: its valid event
was accepted by APNs after the 20:09:23 request and delivered about 116 seconds
later. Neither the counts-only repair nor submission acceptance proves that
preview failure resolved.

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
- Count-only Matrix requests produce badge-only APNs updates with no alert or
  notification-service invocation.
- Conflicting repeated room/event IDs are rejected rather than arbitrarily chosen.
- Generic event APNs payloads include non-empty `aps.alert.title` and
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
