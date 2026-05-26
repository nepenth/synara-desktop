# Synara Route Contract

Reviewed: 2026-05-25

Status: initial shared contract with runtime normalization in
`src/app/routes/synaraRoutes.ts`. The canonical app-relative route schema and
fixtures live under `docs/contracts/`.

## Purpose

Synara routes are internal app destinations used by room navigation, system
notifications, Later reminders, agent approval jumps, and future iOS deep-link
handling. The contract is deliberately smaller than the current React Router
implementation: it names the destinations that native app channels must
preserve, while allowing each platform to present them with native navigation.

The desktop runtime currently serializes routes as app-relative paths beginning
with `/`. iOS may wrap the same destination in a custom URL scheme or Universal
Link transport, but the app should normalize that transport back into one of
the destinations below before routing.

## Rules

Machine-readable artifacts:

- [synara-route.schema.json](./contracts/synara-route.schema.json)
- [synara-route.json fixtures](./contracts/fixtures/synara-route.json)

The JSON Schema defines canonical path shapes. Runtime readers still decode
segments and reject malformed percent encoding according to
`src/app/routes/synaraRoutes.ts`.

- Routes must be app-relative paths. They must start with `/` and must not start
  with `//`.
- Notification routes are capped at 2,048 characters by
  `SystemNotificationRequest`.
- Matrix room IDs, room aliases, event IDs, and space IDs must be percent
  encoded when embedded in path segments.
- Routes must not contain access tokens, refresh tokens, device tokens,
  homeserver credentials, APNs tokens, decrypted message bodies, or private
  preview text.
- Remote HTTPS URLs are not routes. Agent artifacts and external links must go
  through the platform external-URL safety policy instead.
- Unknown, malformed, unauthorized, or inaccessible destinations should fall
  back to the safest containing surface: inbox, room list, or settings.

## Destination Model

```ts
type SynaraRouteDestination =
  | { kind: 'home' }
  | { kind: 'direct' }
  | { kind: 'room'; roomIdOrAlias: string; eventId?: string; parentSpaceIdOrAlias?: string }
  | { kind: 'space'; spaceIdOrAlias: string }
  | { kind: 'spaceLobby'; spaceIdOrAlias: string }
  | { kind: 'inbox'; section?: 'notifications' | 'invites' | 'later' }
  | { kind: 'create' }
  | { kind: 'explore'; server?: string }
  | { kind: 'settings'; section?: string };
```

The runtime route parser now uses this shape as its destination model. Future
Swift types should preserve these fields and validation rules.

## Current Desktop Paths

| Destination      | Current path shape                           | Notes                                         |
| ---------------- | -------------------------------------------- | --------------------------------------------- |
| Home room list   | `/home/`                                     | Default joined-room surface.                  |
| Home room        | `/home/:roomIdOrAlias/:eventId?/`            | General room destination.                     |
| Direct room list | `/direct/`                                   | DM surface.                                   |
| Direct room      | `/direct/:roomIdOrAlias/:eventId?/`          | DM destination.                               |
| Space            | `/:spaceIdOrAlias/`                          | Space root surface.                           |
| Space lobby      | `/:spaceIdOrAlias/lobby/`                    | Space discovery/lobby surface.                |
| Space room       | `/:spaceIdOrAlias/:roomIdOrAlias/:eventId?/` | Room destination scoped under a parent space. |
| Inbox            | `/inbox/`                                    | Container for notification-oriented work.     |
| Notifications    | `/inbox/notifications/`                      | Notification center.                          |
| Invites          | `/inbox/invites/`                            | Invite inbox.                                 |
| Later            | `/inbox/later/`                              | Later inbox.                                  |
| Create           | `/create`                                    | Creation entry point.                         |
| Explore          | `/explore/`                                  | Public-room discovery entry point.            |
| Explore server   | `/explore/:server/`                          | Server-scoped public-room discovery.          |
| Settings         | `/settings/`                                 | General settings and desktop diagnostics.     |

Auth, registration, reset-password, room settings, and space settings paths are
runtime routes, but they are not notification/deep-link targets for the iOS MVP
unless a task explicitly adds them to this contract.

## Notification Routing

`SystemNotificationRequest.route` uses the current desktop path string as the
portable route payload. Senders should prefer the narrowest safe route:

- Message notification: room route with `eventId` when the event is safe to
  reveal as an opaque ID.
- Thread notification: room route with the thread root or event anchor, then let
  the target app resolve the thread context after local sync.
- Later reminder: `/inbox/later/` unless a safe event anchor is available.
- Invite notification: `/inbox/invites/`.
- Agent approval: room/event anchor when available; otherwise
  `/inbox/notifications/`.

Notification payloads must not include decrypted message bodies for routing.
The target app should recompute context after opening and syncing.

## iOS Transport Mapping

The proposed iOS transport forms are:

- Custom scheme: `synara://route/<percent-encoded-app-route>`
- Universal Link: `https://synara.app/r/<percent-encoded-app-route>`

These transports are placeholders until iOS Phase 0 chooses bundle IDs,
associated domains, and route parser ownership. The normalized destination must
still satisfy the app-relative route rules above.

## Acceptance Criteria

- Desktop notification delivery continues to accept only app-relative paths.
- New iOS route parsing tests use this document as the fixture source.
- Route payloads remain opaque identifiers only; no token or message-preview
  leakage is introduced.
- Unknown destinations fall back safely instead of crashing or opening external
  URLs.
