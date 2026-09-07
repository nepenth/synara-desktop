# Visible read frontier repair

Status: implementation prepared; native and live validation pending.
Base: `0695da32c77bc1a56ca22c9ec383f8fd006e6a24` (v2.1.28).
Branch: `fix/read-visible-frontier-2026-09-06`.

## Intended operating path

- Goal: a focused, active client displaying the live bottom clears room unread state after new messages and folded edits/reactions, without a manual Mark as Read action.
- Actor and start: an authenticated dedicated test user with an open live room, private automatic receipts enabled under the existing activity setting, and the latest remote row visibly pinned.
- First action: receive a second test user's message, then edits/reactions to that message while continuing to view the bottom.
- Owner route: SwiftUI/UIKit or React viewport observation → typed Core timeline adapter → shared Core read owner → matrix-rust-sdk → Matrix read-marker endpoint → server readback and room-list projection.
- Transitions: paired visible/receipt frontier delivered with timeline state → confirmed visible tail → delayed automatic acknowledgement → exact SDK-tail comparison → private receipt and fully-read marker → cleared unread state.
- Side effects: disposable test room messages, edits, reactions and private read markers; test API sessions/room membership are cleaned up. Fixture clients never write receipts.
- Authority: Core owns event/receipt semantics and Matrix writes; clients own visibility, focus and lifecycle. Focused historical providers cannot authorize full live-room acknowledgement.
- Completion/readback: actual `m.fully_read`, `m.read.private`, notification count and room-list state, together with the same rendered row and no corrective gesture.
- Acceptance: edits/reactions arriving after a prior successful acknowledgement re-arm the receipt path; new offscreen/background messages remain unread; a newer SDK message invalidates an older visibility token.
- Disqualifiers: manual Mark as Read, fixture-written receipts, treating an edit as a new displayed message, acknowledging an unseen new message, or substituting a source/test success label for live server evidence.

## Cause and repair

Both clients submitted the newest **displayed row's event ID** as the exact SDK live-tail observation. In matrix-sdk-ui 0.18, `Timeline::latest_event_id` delegates to the controller's `all_remote_events` tail, explicitly including events hidden or folded into another item. An edit/reaction advances that SDK tail while leaving the displayed message ID unchanged. Core's correct exact-target check therefore returns a no-op indefinitely, while manual Mark as Read succeeds because it resolves the SDK tail directly. Client deduplication by displayed event ID also suppressed edits after a successful prior acknowledgement.

This is a shared contract defect. The Core read-state projection now carries two optional event IDs: `visibleTailEventId` and `receiptTailEventId`. The latter is captured **before** reading the current displayed tail. Clients submit the receipt ID only when its paired visible ID matches their current rendered remote tail. This is presentation observation, not a second Matrix owner.

The existing Core exact-tail write comparison remains unchanged. If a new message arrives between the two projection reads, the receipt token is older than the SDK tail and fails closed at execution. If metadata is delivered before the corresponding new row, the client rejects the mismatching pair. A new message arriving after projection likewise invalidates the token. Folded edit/reaction targets produce new deduplication identities without changing row identity.

Desktop consumes the pair from snapshot/read-state deltas, including follow-live promotion. iOS attaches the paired receipt target to the matching tail `TimelineItem`. A token-only change changes that row's value and triggers the existing stable viewport apply/confirmed-bottom callback. The receipt task stores the transport identity for deduplication, while the unread divider retains the displayed event identity. Provider-generation, foreground, pinned-visibility, interaction and privacy checks remain in place.

ADR 0003 and ADR 0004 ownership remains unchanged. No client-side parsing of Matrix edits/reactions, protocol writes, or trust bypass was added.

## Validation and limits

- TypeScript modernization tests: 954 passed, including folded edit/reaction identities and metadata-before-row rejection.
- Full frontend and modernization TypeScript checks passed.
- Rust formatting, Matrix boundaries and whitespace checks passed.
- Added a real matrix-rust-sdk mock-server integration case: initial message acknowledgement → folded edit acknowledgement → folded reaction acknowledgement → old token rejected after a new message. It inspects the actual read-marker HTTP bodies for exact private targets and absence of public receipts.
- Added an iOS regression that a receipt-token-only change reconfigures the same stable row and preserves the token through row copies.
- Extended the existing gated live iOS proof to verify edit and reaction server markers/private receipts after the initial message was already read, with no gesture, retaining the background/offscreen negative cases.
- Native compilation, SDK integration execution, signed simulator execution and physical-device evidence are **Not confirmed** in this packet. Local native builds were held because available disk space fell to about 1.3 GiB; remote CI/native proof must complete before acceptance.

The reported user path is Failed because it requires manual acknowledgement. Source evidence identifies a concrete defect that matches agent-edited rooms; it does not establish that every reported unread case shares this cause.

## Remaining timeline work, outside this branch

1. iOS normal open ignores a marker outside the live rows; `SharedCoreReadMarkers` invents the newest row if no marker exists. Core opening can fall back to live, while the user now requests preserving position with Jump to Last Read. Effective-frontier projection also prefers `m.fully_read` over a potentially newer receipt. Resolve this through one Core navigation/read contract and update the superseded unavailable-marker rule.
2. iOS and desktop composer success paths do not issue an explicit latest-view intent when sending from history. Add room-scoped send intent through the existing viewport/navigation owner.
3. Desktop initial placement lets saved `atBottom` override a selected unread anchor. Geometric bottom observation and following-live intent can diverge after short unread-room promotion. Jump-to-latest sets bottom optimistically before Core/layout confirmation.
4. iOS bootstrap stops after any displayable row, including an undecryptable placeholder. A one-row non-scrollable cache cannot satisfy the stable viewport's pagination condition requiring content larger than its viewport. Validate provider underfill independently from missing encryption keys.
5. Both clients intentionally suppress automatic receipts under Hide Typing & Read Receipts, as their existing UI wording states. This branch preserves that policy; any change needs an explicit product decision and matching explanation.

## Independent review correction

The first review identified a debounce race: observation B replaced A's pending receipt ID, but the already-installed task still captured A's displayed ID for the unread divider. The queue now stores and drains one immutable `(visibleEventID, receiptEventID)` observation. The completed write uses both fields from that drained observation; later queued observations cannot change it. A regression covers A → B replacement before dequeue and a further arrival while B's pair is retained.

Full branch whitespace validation against the v2.1.28 base and exact `cargo +1.93.0 fmt --all --check` passed in both workspace roots. Native execution remains pending the orchestrator's build slot.

For the subsequent last-read navigation work, keep the raw receipt frontier separate from its displayed anchor. Existing read markers may name folded events. Core must resolve them to the nearest displayed event at or before their **stream position**, which may differ from the edited target when an edit updates an older row. Do not attempt to repair existing transport markers by moving `m.fully_read` backwards; the SDK may reject that regression. Missing resolution must preserve the user's navigation intent and expose the requested last-read action.
