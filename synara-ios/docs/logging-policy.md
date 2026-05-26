# Synara iOS Logging Policy

Synara iOS uses a small logging wrapper over `OSLog.Logger`. All messages pass
through `LogRedactor` before being emitted or captured by test mocks.

Allowed categories:

- `app`: app lifecycle and non-sensitive shell state
- `auth`: authentication state transitions without credentials or tokens
- `matrix`: Matrix client lifecycle, SDK setup, and non-sensitive failures
- `push`: APNs registration state without device tokens
- `routing`: tab, route, and deep-link handling
- `settings`: local preference changes without account identifiers
- `sync`: sync lifecycle and aggregate status

Never log:

- access tokens, refresh tokens, session exports, or passwords
- APNs device tokens
- full Matrix user IDs, room IDs, event IDs, or aliases from production accounts
- full URLs containing homeserver paths, query strings, or identifiers
- message content, attachment names, profile data, or invite metadata

Debug logs are compiled for `DEBUG` builds only. Release builds keep verbose SDK
logging disabled by default until a support workflow with explicit user consent
exists.
