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
