# Desktop room navigation, visual polish, and agent approvals

Branch: `feature/desktop-polish-approvals-2026-09-13`

Base: `6779e35b` (latest main, Synara 2.1.35)

## Changes

1. **Disappearing room rows.** Home, Direct, and Space virtualizers now measure the list's offset inside their shared scroll container. Preceding actions, Favorites, and headings no longer advance the virtual range past still-visible rooms. Stable room keys preserve measurements when lists reorder. Resizing and collapsing sections update the offset.
2. **Inbox Notifications error.** The old route called a native facade HTTP stub returning `{}` without contacting the homeserver. A typed native `matrix_inbox_notifications` command now uses the authenticated Rust SDK request, preserves opaque pagination tokens and highlight filters, and keeps valid empty results distinct from malformed or failed requests. Credentials remain native.
3. **Message menu.** The native and legacy message menus use the shared floating surface. Normal rows have quiet backgrounds. Redact and Report form a separate final moderation group below a divider, with critical text instead of solid red resting rows.
4. **Composer.** A matte fill, narrow edge light, soft shadow, and focus border replace the broad gradient. The tool area uses a subtle separator. Light/dark and increased-contrast styling remain supported.
5. **Approvals center.** A dedicated shield icon in the sticky navigation rail opens `/approvals/`. Its badge counts actionable requests across joined rooms, independently of notification preferences. The page shows agent identity, room, reason, command, remaining time, and a source-message link. Search never changes the global count. Pending requests sort by earliest expiry; Recent separates requests already decided by this account or expired. Permanent approval retains an explicit confirmation and now places keyboard focus on the confirmation, returning it on cancellation. Decisions use the existing Rust authority and are never implemented as reaction toggles.
6. **Avatar reloads on room return.** Immutable MXC media reuses session-scoped object URLs and concurrent downloads. Warm sources are available on the first render, with commit-phase leases and external-store snapshots that also handle React StrictMode and cache pressure. Idle retention is bounded by 128 entries, 16 MiB, and five minutes; active URLs stay leased. Logout invalidates pending work and revokes URLs. Mutable native timeline handles retain only active leases so reopening revalidates their native ownership. Client caches have weak registry references and are released when their session facade is collected.

## Approval discovery and scope

The Core owner discovers approvals from SDK timelines in joined rooms, including rooms the renderer has never opened. It turns off read-marker/receipt tracking and has no send side effects. Core classification, current-account reaction ownership, redactions, decryption, expiry, and the existing local decision registry determine status. Hermes' seeded reactions do not count as decisions.

Discovery is bounded and reports loading or incomplete coverage rather than silently displaying an authoritative zero. The rail uses a partial-count indication, and the page explains incomplete coverage. The renderer refreshes every five seconds and on focus; expiry updates locally each second. Coverage follows the currently subscribed SDK vector, including resets and the SDK’s initial 20-item window. Slow history retries run concurrently with live updates, so they cannot delay redactions or decisions. Observers release and rebuild their timeline lease every ten minutes or after 512 additional visible items, allowing unattended SDK caches to shrink. Limits are 512 joined rooms, four concurrent history readers, and 500 projected requests; exceeding a limit reports partial coverage.

History is recently observed requests (up to one hour), not an unlimited archive. The existing five-minute action window remains unchanged. Hermes remains the final authority on whether a request is authentic and actionable.

## Verification

- Full TypeScript check and production frontend build passed.
- Frontend modernization suite: 1,013 tests passed.
- New room-scroll and avatar navigation regressions: 12 passed in Chromium and WebKit, including StrictMode/cache-pressure room return and recovery after a shared download fails.
- Approval presenter/provider regressions: 24 passed in Chromium and WebKit, covering decisions, permanent confirmation/cancellation, source navigation, search, expiry, remote decisions, failures, partial coverage, unreadable requests, native session replacement, late callbacks, and narrow/light layouts.
- Existing native timeline navigation suite passed (46 existing cases).
- Rust tests cover the authenticated notifications endpoint, request/response validation, command registration and no-session behavior.
- Approval Rust tests: seven policy/lifecycle tests and five SDK integration tests passed. These exercise unopened-room discovery, seeded/own reactions, redaction, room departure, reset recovery, cache shrinking/reopening, and absence of message/read side effects. A delayed-history test proves live redaction processes within 500 ms while pagination stalls for two seconds. A 400-newer-message fixture proves initially incomplete discovery immediately continues beyond its first bounded pass and finds the approval within two seconds.
- Desktop `cargo check` passed. Full frontend ESLint and Prettier checks passed. Matrix boundary and repository quality gates passed.
- Computer control inspected actual production components in local deterministic fixtures: scrolled from Favorites into Rooms, revisited avatars without a second download, opened the message menu, inspected permanent confirmation, and typed into the matte composer in dark/light layouts.

The browser fixtures substitute native IPC with deterministic data. They prove frontend behavior and geometry, not a live Matrix account or installed Linux desktop session. Rust integration fixtures exercise the real SDK against a disposable HTTP test server. The installed application and release version were not replaced.

## Independent review

All review requests use Grok 4.6 with `xhigh` reasoning and source packets supplied directly to the backend.

- Notifications: no actionable findings.
- Room virtualization and composer: no actionable findings. Avatar review identified a warm-URL eviction race and strong cache registry retention; commit-phase leases, deferred eviction, external-store snapshots, and weak registry ownership address them. A counterfactual test restoring synchronous eviction fails on the missing warm source. The second Grok review accepted those fixes and identified re-leasing after another avatar retries a failed download; the recommended snapshot dependency is implemented, reviewed by the root agent, and covered by the two-engine recovery regression.
- A menu-divider finding was rejected with source evidence: the primary action group always contains Save for later, so the reported divider-only state is unreachable.
- Approval review identified command visibility, session-generation isolation, blocked live updates during history retries, and ambiguous partial-coverage labels. Unparsed requests now show raw source without decision buttons; native generation replacement clears optimistic overlays and rejects late callbacks; retries remain concurrent with the live stream; settled partial coverage has its own label and warning badge.
- Additional root review found stale completeness after SDK resets and unbounded observer lifetimes. Coverage now follows the live subscribed vector and observer leases rotate to release caches; SDK integration tests cover both. The second backend Grok review accepted cancellation, live updates, rebasing, and lifecycle behavior; its final scheduling finding is fixed by immediately retrying an initially incomplete bootstrap, with the busy-room regression and root review confirming the correction.

The second frontend Grok review confirmed the original command visibility, generation isolation, partial-coverage, and disclosure fixes. Its remaining findings—stale resolved announcements after session/client replacement and missing accessible submission-error status—are corrected with session-scoped announcement clearing and an alert role. Existing browser regressions now assert both behaviors. Root review confirmed these final small corrections.

All six requested items have completed Grok 4.6 `xhigh` review, actionable findings are resolved, and no review findings remain open. The original menu-divider report is the sole rejected finding, for the source-evidenced reason above.
