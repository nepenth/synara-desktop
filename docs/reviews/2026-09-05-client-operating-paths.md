# Client session, timeline, and read-state repair

## Scope and intended paths

The user reports failed iOS timeline stability, live delivery and logout, and
macOS automatic read acknowledgement on the released client. These reports
establish failed user experiences; the exact device executions are not yet
available for runtime proof. This work starts from main at `2c93f23b` on
`client-session-timeline-read-fixes`.

| Path | Actor, starting state and first action | Owner route and transitions | Completion and authoritative readback | Acceptance and disqualifiers |
| --- | --- | --- | --- | --- |
| Stable iOS viewport | Signed-in reader opens a room and leaves visible rows stationary | Core timeline projection → Swift session feed → stable UIKit viewport; content updates preserve surviving row positions | Visible row geometry remains stationary after updates settle | Identical content causes no table/scroll mutation; changed content preserves the visible anchor. Repeated repositioning or a manual scroll correction fails the path. |
| iOS live delivery | Signed-in reader opens a normal room and reaches its current live edge | Core SDK timeline owner → typed Swift feed → rendered rows; history placement must retain or correctly enter an actual live provider | A subsequent synced remote event appears through the same owned subscription | No room reopen, app restart, unseen-tail acknowledgement, or presentation-only live label. Historical search remains historical until a verified transition. |
| iOS logout | Reader with a persisted session, including a failed restore, taps Sign out | Logout service → account-bound push/session owners → secure store and local caches → signed-out shell | Secure store reads empty and a fresh launch stays signed out; remote cleanup has a truthful outcome | Remote unavailability must not leave a user trapped in an unusable local session. No unrelated device pusher deletion, hidden surviving credentials, or claim of remote revocation without evidence. |
| Desktop automatic read | Active reader opens an unread room and visibly reaches the current live tail | React viewport → typed Core command → SDK timeline/read receipt owner → room-list projection | SDK/server read state advances to the observed event and unread indication clears | No context-menu action required; background windows and unseen newer events remain unread. A focused provider's local tail alone is not evidence of the room live edge. |

## Authority and proof boundaries

Source edits, the feature PR, merge to main and a new release are explicitly
authorized. Tests must use disposable fixtures/accounts; authorization does not
extend to logging out the user's real account or sending real room messages.
Local logout deletes credentials and account caches; receipt tests change only
fixture read state; release publication changes GitHub and distribution assets.
Each side effect must stay with its owning component.

Validation will distinguish source/fixture execution from physical iOS/macOS
proof. CI success alone cannot establish the latter. Each repaired path needs a
clean run from its declared starting state, with results and remaining limits
recorded here before release.

## Earliest divergences and repairs

1. **Desktop contract defect:** `follow_live_tail` can change placement without
   emitting another row revision. The desktop applied the receipt-response
   stale rule (`revision <= current`) to this placement response and discarded
   success. Follow readbacks now accept equal revisions only for the same room,
   session and captured stream, with a live position. Older responses remain
   rejected. Separately, painted bottom geometry is observed after layout and
   resize, including rooms that cannot scroll.
2. **iOS presentation defect:** the UIKit adapter reapplied identical rows and
   restored a visible anchor by first scrolling it elsewhere. Identical row
   updates now execute any pending command without a table snapshot; visible
   anchors are corrected in place, with a one-device-pixel threshold.
3. **iOS invalidation contract defect:** a one-element buffer subscribed by room
   retained a stream-specific DTO, which the consumer could discard after a
   transient read-marker stream displaced the live invalidation. The buffer
   now retains room invalidations, and the consumer reads its owned snapshot.
4. **Logout ownership/contract defects:** local sign-out depended on remote
   pusher success, and the product's server-revocation method invoked an FFI
   operation that only dropped handles. The remote route now makes an actual
   SDK logout request against the exact loaded account/device, bounded at five
   seconds. Account-bound push writes and cleanup are also bounded and cannot reopen push
   registration while local sign-out proceeds. This is necessary because the
   pinned UniFFI async bridge does not propagate Swift task cancellation;
   otherwise waiting for reconciliation can block logout on an HTTP write.
   Core quiesces sync and stores,
   clears its projection and retained handles, and deletes the account's
   session-material vault entry. Swift then deletes its identity envelope and
   publishes signed out. History and its encryption key are preserved; they
   are not authentication credentials. A local deletion failure remains an
   error, while remote cleanup failure is not represented as remote success.

## Local proof record

- Browser geometry run: 7 tests passed, including a short room with zero scroll
  events, content growth, resize, replacement and observer disposal.
- Frontend suite: 952 tests passed, including equal-revision follow acceptance
  and rejection of stale or different room/session responses.
- Core logout: two tests passed through the real typed owner and SDK. Mock
  homeserver expects exactly one authenticated logout request. Wrong device
  and account requests fail; vault readback proves removal; a fresh Core cannot
  restore the forgotten session. A missing restored client does not block local
  credential removal and unrelated vault entries/storage keys remain intact.
- Focused iOS run: 176 tests, one pre-existing skip, no failures after updating
  the old remote-failure/re-registration expectation to the new local logout
  contract. The UIKit test records zero offset writes for 20 identical updates.
- Full local Core unit run: 872 passed, three existing ignored tests. All four
  SDK follow-live tests passed, including stale-tail and focused-provider
  rejection and continued sync after a valid unread-to-live transition.
- Full local iOS unit run: 709 tests, three existing skips, no failures.
- Exact-head CI and publication evidence are attached to the associated PR
  and the v2.1.27 Release workflow.

Physical iOS/macOS symptom resolution remains **Not confirmed** until exercised
on the affected devices. The fixture runs confirm their scoped owner and
presentation behavior; they do not establish production homeserver, APNs,
network-recovery or physical-device results.
