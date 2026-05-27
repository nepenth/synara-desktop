# iOS Accessibility Checklist

Reviewed: 2026-05-27

Status: initial Phase 6 audit complete for the native MVP surface.

## Audited Flows

- Homeserver entry, validation error, and login form.
- Signed-in tab shell, room list, invite accept/reject, and room opening.
- Timeline reading, message composer, attachment affordance, and send action.
- Agent card summary and approve/reject controls.
- Later list navigation to a room anchor.
- Settings, notification controls, About, Licenses, Privacy, Support, and logout confirmation.

## Implemented Fixes

- Room rows expose a combined VoiceOver summary with room name, preview, unread count, and highlight state.
- Timeline rows expose sender-aware summaries for text, media, redacted, encrypted, unsupported, and agent-card events.
- Composer controls have explicit labels and hints for attach, message entry, and send.
- Agent action buttons use regular control sizing and explicit hints.
- Invite and logout destructive paths include explicit hints and confirmation coverage.
- Phase 6.5 keeps icon controls at 44 points, adds visible search and product
  headers, and preserves deterministic UI coverage after the visual redesign.

## Manual Review Notes

- Dynamic Type: primary controls use system `Text`, `TextField`, `Button`, `List`, and `Form` surfaces; multiline message, privacy, support, and license copy wraps rather than clipping.
- Tap targets: icon-only composer controls are framed at 44x44 points; agent action controls use regular control size.
- VoiceOver completion path: login, open room, read timeline summaries, send a message, open settings, and logout are represented by deterministic UI tests and accessible identifiers.

## Remaining Release Gates

- Run a human VoiceOver pass on a physical iPhone before external TestFlight.
- Capture large Dynamic Type screenshots for login, room timeline, composer, and settings.
- Re-audit after real encrypted timeline rendering replaces the current encrypted placeholder path.
