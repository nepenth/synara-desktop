# iOS Security Review

Reviewed: 2026-05-27

Status: initial Phase 6 review complete for the native MVP. External TestFlight remains gated by production signing, privacy, legal, and final E2EE integration.

## Reviewed Surfaces

- Auth/session: password login persists Matrix sessions through Keychain-backed secure storage.
- Local stores: logout wipe stops Matrix sync, resets SDK stores, clears room cache, clears push registration state, deletes secure session data, and resets navigation.
- Push: push payload parsing routes only to internal app destinations and badge counts; payloads must stay generic on the APNs path.
- Notification previews: the Notification Service Extension reads the app-group
  preview setting and shared Keychain session, then performs a bounded on-device
  Matrix event lookup for cleartext events only. It leaves encrypted events,
  agent approval prompts, disabled previews, missing sessions, and timeouts on
  the gateway-provided fallback text.
- Deep links: app routing now accepts only `synara://` links and `https://synara.app/r/...` universal links, and rejects unsafe schemes/hosts.
- Agent actions: action kinds are allow-listed; unsafe URLs and malformed payloads are rejected before rendering or execution.
- Media: external media descriptions are sanitized; authenticated Matrix media stays behind app service handling.
- Logging: logger redacts bearer tokens, token query fields, JSON token fields, APNs tokens, Matrix IDs, event IDs, and URLs.

## Validation Evidence

- `LogRedactorTests` cover token, Matrix ID, event ID, URL, and APNs redaction.
- `AppRouteTests` cover valid custom/universal links and unsafe link rejection.
- `AgentActionResolverTests` cover allow-listed actions and unsafe URL rejection.
- `LocalWipeServiceTests` cover wipe calls across session, Matrix lifecycle, room cache, push state, and secure storage.
- `NotificationPreviewSupportTests` cover preview payload parsing, preference
  defaults, cleartext preview clamping, and encrypted-event fallback.
- Gated live agent approval smoke validates real Matrix approval event submission with disposable test data.

## Release-Blocking Items

- Production E2EE must use the Matrix Rust SDK crypto path before claiming encrypted-room support for TestFlight/App Store.
- Apple Developer organization enrollment, signing, App Groups, Keychain Sharing,
  and APNs key handling must be completed outside the repository.
- AGPL/App Store distribution review must be completed before external distribution.
- External TestFlight must wait for approved support and privacy URLs.

## Non-Blocking Follow-Ups

- Run dependency advisory checks during CI hardening.
- Add signed-device logout wipe inspection once a physical device profile exists.
- Capture a final log sample from a live session and verify no token, room, event, or URL leakage appears.
