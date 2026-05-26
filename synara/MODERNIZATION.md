# Synara Modernization

This branch stack captures the Slack/Discord parity and Hermes-agent workflow
slice across the app runtime and the paired desktop wrapper. It is intentionally
reviewable as a modernization batch, while individual features can still be
split into smaller upstream PRs.

## Feature Scope

- Message/media polish: GIF search with Matrix media upload, media replies,
  quote/forward flows, polls, rich Hermes cards, and reaction moderation.
- Triage workflows: room/message unread markers, Later with reminders,
  snooze/edit/complete, notification inbox, direct room/event/thread jumps, and
  desktop badge summaries.
- Navigation and organization: advanced search filters, expanded command
  palette actions, persisted drafts, favorites/folders, and constrained accent
  customization.
- Desktop bridge: tray/status actions, native notifications, badge counts,
  configurable global shortcuts, media permission copy, updater entry points,
  and structured agent actions.

## Review Evidence

Attach or update PR descriptions with screenshots or short clips for these
flows before requesting broad upstream review:

1. Hermes/agent card with Copy as Markdown, Copy JSON, Copy Links, syntax
   highlighting, and quick actions.
2. Command palette action mode showing Later, notifications, message search,
   create/join, and desktop shortcut actions.
3. Notification inbox with per-room controls, batch mark-read, and direct
   room/event navigation.
4. Poll creation, timeline voting, and result visualization.
5. Desktop tray/status menu, badge count, native notification click, and Later
   or notification deep-link.

Review builds of a Matrix client must be treated as untrusted client code.
Use test Matrix accounts for screenshots and demos.

### Capturing Demo Assets

Use an authenticated test Matrix account and a non-production homeserver or test
rooms. Do not capture access tokens, server admin pages, private room names,
private DM content, or decrypted production-room messages.

Recommended assets:

- Static screenshots: `Cmd+Shift+4` on macOS or the desktop environment
  screenshot tool on Linux. Capture the full app window when showing layout
  changes and crop narrowly only for focused controls.
- Short clips: 10-20 seconds per flow is enough. Show one action path per clip:
  command palette, Later snooze/edit, notification deep-link, Hermes action,
  poll vote, and desktop tray/badge behavior.
- Performance clips: enable
  `localStorage.setItem('synara.performance.debug', 'true')`, reload, then capture
  a slow scroll through a long room with the overlay visible.
- PR attachment notes: include the branch/commit shown in the desktop About or
  tray build label so reviewers can match assets to the exact build.

## Validation

Focused app-runtime modernization tests:

```sh
npm ci
npm run test:modernization
npm run test:timeline-performance
git diff --check
```

Desktop wrapper tests:

```sh
cd /path/to/synara-desktop
cargo test --manifest-path src-tauri/Cargo.toml
```

Linux CI is the source of truth for the packaged app-runtime build while the local
macOS Rollup native package issue remains environment-specific.

## Known Build Notes

- `npm run build` is expected to pass in GitHub Linux CI for this branch stack.
  Some macOS hosts can fail before Vite runs when the optional native Rollup
  package is blocked by local code-signature/quarantine state. Reinstalling
  dependencies in a clean checkout or relying on Linux CI is the current
  workaround.
- Repo-wide `npm run typecheck` still includes pre-existing Matrix SDK/Jotai
  failures outside the modernization files. The modernization gate uses
  `npm run typecheck:modernization` against `tsconfig.modernization.json` for a
  scoped signal.
- The desktop repo can run `cargo test` without the full bundled app runtime. A
  full runtime smoke test still requires the runtime `dist/` assets to be present
  in the desktop wrapper checkout.
- macOS-specific refresh-rate overrides are intentionally out of scope for this
  branch. Performance work is focused on bounded timeline rendering, scroll
  anchoring, instrumentation, and compositor-friendly UI behavior.

## Security Notes

- GIF provider requests and selected GIF downloads use no credentials and no
  referrer. Selected GIFs are uploaded to Matrix media and sent as `mxc://`
  attachments, including encrypted attachment metadata in encrypted rooms.
- URL preview cards are intentionally disabled in this branch. Plain links
  remain clickable, but the client does not request homeserver URL-preview
  fetches or render preview cards.
- Later, unread anchors, drafts, and sidebar metadata never store decrypted
  message bodies in Matrix account data.
- The desktop bridge uses explicit command names for badge, shortcut, and agent
  actions. Agent action payloads are bounded, kind-allow-listed, and sanitized
  again in the Tauri wrapper before local handling or `synara://agent-action`
  event emission.

## Performance Notes

- GIF search is debounced and guards against stale result replacement.
- Message search filters are applied as bounded utility predicates; expensive
  UI lists should stay virtualized through the existing room/search rendering
  patterns.
- The main room timeline uses TanStack variable-height virtualization with
  event-ID keyed row anchors, bounded overscan, and scroll restoration when
  Matrix pagination or media/card resizing changes row height.
- `npm run test:timeline-performance` generates 10k and 50k synthetic timeline
  rows and fails if row mapping regresses beyond the current budget.
- `localStorage.setItem('synara.performance.debug', 'true')` enables a small
  in-app overlay with FPS, long-task count, heap usage where available, and
  rendered timeline row count.
- Hermes cards cap per-section counts and block sizes before render.
- Notification and Later badge counts are summarized before crossing the
  desktop IPC boundary.

## Related Docs

- [Compatibility namespaces](docs/synara-namespaces.md)
- [Modernization roadmap](docs/synara-modernization-roadmap.md)
- [Performance notes](docs/synara-performance.md)
