# Last-read navigation operating paths

Status: implementation in progress; runtime proof not confirmed.

The user's 2026-09-06 contract supersedes the previous missing-marker fallback
that silently opened live. ADRs 0002–0004 retain one shared Core Matrix owner.

| Path                   | Actor and clean starting state                                                                                | First action and owner route                                                    | State transitions and completion                                                                                       | Authoritative readback / acceptance                                                                                            | Disqualifiers                                                                                             |
| ---------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| Restore last read      | Signed-in reader; room closed; existing shared read marker                                                    | Select room → native presenter → Core SDK read-state/timeline → native viewport | Load bounded snapshot → place available visible frontier once; otherwise retain placement and expose Jump to Last Read | SDK stream order identifies visible predecessor; viewport shows anchor or actionable button; later sync does not move viewport | Timestamp ordering, invented tail-as-read, delayed unsolicited relocation                                 |
| Follow latest          | Reader in active focused room with latest tail visible                                                        | Receive new event through Core live stream                                      | Render append → preserve bottom ownership → measured latest tail visible                                               | Actual row geometry and SDK live frontier agree; down arrow absent only at latest bottom                                       | Focused-window bottom treated as live, hidden arrow before layout, history reader moved                   |
| Send and return latest | Reader viewing current or historical room context                                                             | Submit native composer → Core send → explicit latest navigation                 | Successful send/local echo → live provider → bottom layout confirmation                                                | Sent message and latest tail visible without gesture                                                                           | Unrelated room moves, failed send moves view, old window reported latest                                  |
| Sparse history         | Reader opens room whose initial SDK cache has one event or undecryptable placeholder and older history exists | Select room → Core timeline → bounded history bootstrap → presenter             | Initial sparse snapshot → bounded older page → render context; preserve available data on network failure              | More history renders or actionable older-history control remains                                                               | Placeholder implies exhausted history, gesture impossible because list cannot scroll, unlimited traversal |

Side effects: normal navigation may populate SDK caches and, only after existing
privacy/focus/visibility policy permits, write read state. Send proof may create
disposable fixture messages. No secret, crypto, push, production-release, or
personal-account operations belong to this workstream. User-authorized test
accounts are the only live fixtures. Core owns Matrix order, read targets, and
pagination; native clients own focus, geometry, and explicit user intents.

Evidence and clean-rerun results will be appended after implementation. Unit and
browser fixtures establish only their declared boundaries; physical-device and
live Matrix routes require separate execution and authoritative readback.

## Repair and evidence so far

Core now keeps the raw receipt identity separate from the navigation anchor.
The SDK's own receipt projection supplies hidden-event positioning; at most 300
in-memory cache events supplement `m.fully_read` ordering. Neither storage
pagination nor origin/receipt timestamps order markers. A folded edit of an old
message resolves to its nearest visible stream predecessor, which may be a newer
message. Initial missing anchors keep the live provider and explicit target.
A sparse cache requests one bounded SDK history page, shared by both clients.

Desktop restores the selected unread anchor ahead of stored-bottom hints,
retains event/pixel position when late anchor context is prepended, promotes
short unread views through Core before following future appends, and treats
successful latest navigation as a new placement even when provider mode already
was live. Actual wheel/pointer input supersedes programmatic-scroll suppression.
New composer sends issue a room-scoped latest intent; edits do not. iOS retains
missing anchors with a Jump to Last Read action, stops inventing read markers,
and issues latest placement for new local sends. Short timelines expose explicit
older pagination. Latest controls depend on provider identity and actual bottom
geometry.

Validation completed before first review:

- Full frontend TypeScript check and focused ESLint passed. Modernization suite:
  954 passed after updating its existing function-shape assertion for useCallback.
- Fresh Core owner unit suite: 43 passed. The first shared-target invocation
  accidentally reused another worktree's artifact (removed timestamp-test names
  exposed it); that run is excluded. A package-only `cargo clean -p synara-core`
  and rebuild explicitly naming this worktree produced the valid result.
- Real matrix-rust-sdk mock-HTTP owner suite: 9 passed, including hidden edits of
  old rows, newer comparable private receipts, unavailable anchor without a
  `/context` detour, and one cached event producing one `/messages` page.
- Actual shipped React presenter and native controller in Chromium, with mocked
  native IPC: 5 passed. These exercise real virtualization, focus, DOM geometry,
  wheel input and native command invocation. They prove live append/history/edit
  preservation, short-room follow, stored-bottom precedence, late missing-anchor
  retention and latest/send completion after provider return and layout.
- The first browser run exposed a real late-prepend position failure; the repair
  passed a complete clean rerun. A separate short-room failure was a fixture's
  incorrect native argument nesting and is excluded from product diagnosis.

Browser command: `npm --prefix synara run test:browser:native-timeline`.
SDK command: `cargo +1.93 test -p synara-core --test p4_s38_timeline_follow_live`.

Runtime verdict: **Confirmed** for the declared SDK mock-HTTP and Chromium
presenter paths only. Signed iOS compilation/UI execution, actual homeserver
proof with this branch, and physical-device behavior remain **Not confirmed**.
The added iOS route tests await regenerated bindings for the base branch's read
frontier fields. No release or broad device-quality claim follows from these
bounded results.

## Fresh-review corrections

The stable iOS timeline now mounts the same last-read and sparse-history recovery
controls as the legacy viewport; a duplicate legacy overlay was removed. This is
source-level correction pending the signed iOS validation described above.

Desktop latest placement now requires both adoption of the returned native
provider and the same navigation input that requested it. A focus request in the
same room supersedes an outstanding latest completion. Sparse-history and
last-read actions share a vertical layout so both remain separately clickable.

Two Chromium regression cases run the shipped presenter/controller against
mocked native IPC. Against the pre-fix product source at `fec30148`, the delayed
latest result displaced the focused viewport from `$30` to `$55`, and recovery
controls overlapped (older action bottom 70 px, last-read action top 30 px).
With the correction, all seven browser cases passed, including actual focused-row
viewport visibility, preserved event/pixel geometry, separate recovery-control
bounds, and each action's corresponding native command. Full frontend TypeScript
checking, 954 modernization tests, and focused ESLint passed; the repository ESLint configuration does not
include the harness TSX file. This does not establish live Matrix or iOS runtime
behavior.

## Missing-marker and live-follow corrections after Grok review

An unavailable initial marker now preserves a saved history event/pixel location,
or starts at the live tail when there is no usable saved anchor. The recovery
target survives passive follow-live promotion and later snapshots. A pointer
press alone no longer relinquishes live following; actual scrolling still
updates ownership from viewport geometry.

Jump to Last Read keeps the current Core provider while a new focused provider
opens. The controller adopts that provider only when its matching account
generation/room snapshot contains the requested event. Failed or missing-target
responses retain the old rows and recovery button; superseded responses close
their unused streams. Successful placement clears the recovery target, as does
an explicit latest intent. Poll responses from replaced streams are ignored.
All Matrix context and provider ownership remain in Core; no Core or Swift source
changed in this correction.

The Chromium fixture now retains distinct snapshots per Core stream and allows
successful follow-live in the sparse missing-marker case. Seven new regressions
failed against unchanged `c4379c1f` product source: initial missing-marker entry
was 3808 px from the tail; saved history re-entry moved from `$38` to `$1`;
successful sparse promotion lost the recovery action; clicking at the bottom
left the next message outside the viewport; and failed, absent-target, and
superseded focused opens lost recovery. The first repaired run also caught a
36 px saved-offset error caused by canceling a layout adjustment when the button
appeared. That correction passed the focused case and the full clean rerun.

Final current-source validation: all 15 Chromium presenter/controller cases
passed, including explicit-latest dismissal and discarded-stream cleanup;
954 modernization tests, full frontend TypeScript, and focused ESLint passed.
The harness TSX remains outside the repository ESLint include pattern. This is
mocked-native browser proof only; it does not extend the separate live Matrix,
signed iOS, physical-device, or release evidence.

## Stream-adoption corrections after independent review

Core stream revisions restart for each open. The controller now retains bounded
candidate updates while the open RPC is pending, replays only the returned
stream's updates, and validates the last-read target against the reconciled
snapshot. Initial and latest opens use the same replay boundary. The buffer is
scoped to the room and known session generation, limited to 64 batches and 2048
operations/rows/pin IDs, and released on adoption, failure, supersession, or
cleanup. A gap, overflow, or removed target rejects the last-read candidate while
preserving the existing provider and recovery control. Only streams returned by
this controller's own opens are eligible for discarded-candidate closure.

Pagination, read-state, follow-live, and snapshot-poll responses now check the
captured stream, navigation revision, and navigation request before updating the
view. Superseded command rejections cannot reach presenter error handlers; poll
rejections are ignored. These guards prevent independently numbered snapshots
from an old stream from replacing a newly adopted provider.

Before the correction, eight of nine new Chromium cases failed against
`6d778914`: candidate revision 1 was lost before adoption, gaps and buffer
oversizing silently adopted the candidate, delayed pagination success/failure
replaced or errored the new view, and delayed read success/unavailability did the
same. Stale read rejection was already benign. The harness now models revision
zero for every Core open and routes updates through the actual Tauri event
listener bridge. Polling is disabled in event-route cases so it cannot heal a
lost update and conceal the failure.

Final correction validation: all 33 Chromium presenter/controller cases passed
in a clean run, including candidate replay/removal/gap/overflow, cancellation
with queued updates, initial/latest replay, and delayed success/unavailability/
rejection for pagination, read-state, follow-live, and polling. Full frontend
TypeScript, all 954 modernization tests, focused ESLint, and diff whitespace
checks passed. The harness TSX remains outside the ESLint include pattern.
No Rust or Swift build was run, and no live Matrix, iOS/device, or release claim
is added by these mocked-native browser results.

## Same-stream readback correction after Grok v2

A pagination response copied before an already-applied delta now leaves that
newer snapshot intact. Pagination uses the existing readback-staleness check only
after confirming stream and navigation ownership. Unavailable pagination and
read-state actions throw through the existing action handlers while retaining
the ready provider; they no longer replace working rows with a synchronization
error. Invalid non-stale snapshots still fail the synchronization check. Follow
and poll already ignored same-stream lag and required no additional source change.

Twelve new Chromium cases defer a command on the current provider, deliver a
real Tauri event delta, and then release success, unavailability, or rejection for
pagination/read-state/follow-live/poll. They assert the delta's rendered text,
retained recovery control, one active provider, event/pixel geometry, and no
unhandled error; pagination cases also exercise a retry. Against unchanged
`ddda4f3f` product source, three failed: stale pagination success and unavailable
pagination/read-state erased the rows. The other nine passed.

Final validation: all 45 Chromium cases passed in a clean run, with full frontend
TypeScript, 954 modernization tests, focused ESLint, and diff whitespace checks
also passing. This correction changes Desktop code only and adds no native/Core
build, live Matrix, signed iOS/device, or release evidence.
