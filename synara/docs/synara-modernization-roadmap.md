# Synara Modernization Roadmap

This roadmap captures the Slack/Discord parity work around the modernization branches, plus the paired desktop wrapper polish.

## Captured In The Current Stacked PR

- GIF discoverability: config-gated user setting, composer onboarding, disabled-by-default provider behavior, private provider fetches, and Matrix media upload before send.
- Hermes/agent cards: configured content keys, explicit activation only, payload limits, artifact/action URL filtering, generic quick-action buttons, collapsible sections, syntax-highlighted code/diffs, section counts, Copy as Markdown, Copy JSON, and Copy Links actions.
- Later inbox: anchor-only account data, completed state, snooze/edit due dates, clear completed, direct reminder notification jumps, active/done views, and overdue/today badge summaries.
- Forwarding: sanitized forwarding, encrypted-to-unencrypted confirmation, quote-with-context forwarding, and contiguous multi-message forwarding selection.
- Unread anchors: private `in.synara.unread_anchor` markers plus a distinct timeline divider treatment.
- Drafts: local-only per-room composer drafts in `localStorage`, never Matrix account data.
- Polls: Matrix poll creation UI, `/poll Question | Option A | Option B`, timeline rendering, voting, and results visualization.
- Search and keyboard navigation: existing `Cmd/Ctrl+K` room/space/DM quick switcher preserved; `>` action mode opens Later, notifications, invites, message search, create/join, and explore; message search now has sender autocomplete, date range filters, and type filters for media, files, audio, links, and polls.
- Notification center polish: notification inbox has a visible-room batch mark-read action, direct room/event notification jumps, desktop badge integration, and per-room notification mode controls.
- Threading UX: thread root metadata is preserved across notification/search timeline cards, notification deep links can target thread anchors, and thread-aware message rendering is kept distinct from room-level unread markers.
- Spaces/folders/favorites: existing drag-and-drop space folders are preserved on the same `in.synara.spaces` account-data model, and favorites/folder organization remains Matrix/client state instead of desktop-local state.
- Desktop integration: app runtime exposes the `window.__SYNARA_DESKTOP__` adapter, active Later/unread badge counts, native notification click deep links, and structured Hermes/agent actions for the paired desktop wrapper.
- Theme customization: constrained accent color picker for primary controls and links.
- General hardening: i18n coverage for new UI strings, ARIA labels for new controls, focused modernization tests, and namespace documentation.

## Follow-Up PRs

1. Agent backend implementations

   - Connect the structured `desktop_agent_action` bridge to concrete backend services for Regenerate, Continue, structured task forms, export/session logging, and branch/fork conversation flows.

2. Native distribution hardening

   - Keep macOS/Linux package signing, updater release metadata, and store-specific notification/media permission copy aligned with the desktop wrapper.

3. Theme customization
   - Add more curated themes before considering advanced CSS injection.
