# A7 timeline product actions

Status: deterministic implementation complete; live Matrix interoperability
proof **Not confirmed**.

## Boundary

Core and the pinned Matrix SDK remain the sole semantic and write authority.
Desktop and iOS render native controls only when the row carries the matching
Core capability and, for forwarding, a closed typed transport. Presenters do
not infer policy from event text, MIME types, paths, media bytes, or raw MXC
URIs. Every success is accepted only after an exact schema/action/room/status
readback; source mutations also bind the original event, while created events
must return a non-empty new event ID.

Action exclusion is keyed by room, event, and stable action class. It lives
beyond transient desktop popouts and iOS room/thread views, so dismissal,
navigation, or changed form payload cannot create a concurrent duplicate.
Poll controls remain pending after send readback until the Core timeline
projection reports the exact selected answer set.

## Independent slices

| Slice        | Core authority and validation                                                                                                                                    | Native presenter                                                                                                                                                               | Deterministic status   | Live status                                                  |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------- | ------------------------------------------------------------ |
| Vote         | Open Core poll capability; exact visible poll options, closed state, maximum selections, uniqueness, and shared poll-wire bounds are checked before the SDK send | Accessible single/multi-select answers, clear vote, bounded selection, pending state                                                                                           | Implemented and tested | **Not confirmed**                                            |
| Report       | Remote-event Core capability; optional reason is trimmed and capped at 512 Unicode scalar values; exact source-event readback                                    | Deliberate destructive confirmation, optional bounded reason, administrator disclosure                                                                                         | Implemented and tested | **Not confirmed**                                            |
| Forward      | Core projects only `text` for text/notice/emote and `media` for image/file/audio/video/sticker; every other semantic message fails closed; the write owner re-reads both rooms' encryption state and requires explicit downgrade authorization | Searchable joined-room target picker; quote only for text; forwarding consumes the required tri-state Core room projection, blocks Unknown/error, and asks for encrypted-to-cleartext confirmation | Implemented and tested | **Not confirmed**, including encrypted-to-encrypted readback |
| Decline call | Remote active-call Core capability; SDK prepares and sends the decline event; exact new-event readback                                                           | Accessible pending decline control on call rows                                                                                                                                | Implemented and tested | **Not confirmed**                                            |

Desktop also consumes applicable Core action capabilities on membership, state,
call, redacted, undecryptable, and fallback rows instead of silently dropping
them. Generic reactions share the same per-event/action exclusion and approval
prompts remain outside that generic route.

Forward encryption downgrade protection is enforced at two boundaries. Each
native presenter consumes the required tri-state Core room-list projection;
SDK Unknown/read error stays `unknown`, never legacy `false`. Presenters block
Unknown/error and request deliberate confirmation for encrypted-to-cleartext
forwarding. The Core write owner then re-reads both rooms through the SDK,
rejects Unknown/error, and rejects encrypted-to-cleartext unless the request
carries the presenter's explicit downgrade authorization. The request never
carries platform-derived encryption facts, so Core remains authoritative and a
direct service caller cannot bypass the policy or race a stale room-list fact.

## Deterministic evidence

- Core: `cargo test -p synara-core app::timeline` exercises capability and
  transport projection, poll semantic/wire validation, bounded reasons,
  tri-state forward policy, and typed action readback helpers.
- Desktop: `npm run typecheck` and `npm run test:modernization` exercise poll
  policy, exact readback rejection, closed forward transport, encryption
  fail-closed behavior, persistent action locks, and accessible action source.
- iOS: `EventActionServiceTests`, `SynaraCoreBindingsTests`, and
  `OutgoingSendServiceTests` exercise capability mapping, exact readbacks,
  stable action keys, the session-lifetime duplicate coordinator, closed
  forward transport, poll policy, and VoiceOver child containment.

## Explicitly open evidence

No deterministic presenter or projection test proves a homeserver accepted and
aggregated each action, an encrypted forward decrypted on a second device, or a
decline interoperated with another call client. Those require live two-client
evidence before promotion from **Not confirmed**.

## Core timeline sequencing gates

`crates/synara-core/tests/p4_s37_timeline_sequencing.rs` drives the pinned
`matrix-sdk-ui` timeline against a mocked homeserver and sends the SDK's real,
ordered `VectorDiff` batches through Synara's identity/delta projection helper.
The first four cases use `project_timeline_diffs` specifically to prove stable
identity and ordered replacement semantics; that helper is not the live
capability-authority or media projection path. The executable cases prove:

- `redaction_replaces_the_existing_projected_row_without_duplicate_identity`:
  the SDK's in-place redaction `Set` becomes one redacted Core row at the same
  index and stable item identity;
- `late_decryption_replaces_utd_with_plaintext_at_the_same_projected_identity`:
  an imported room key replaces the UTD row with decrypted text at the same
  index and item identity;
- `pagination_overlap_keeps_one_projected_identity_per_event`: an event shared
  by live sync and `/messages` may be moved by remove/reinsert diffs, but keeps
  its stable item identity and appears only once in the final Core projection;
- `relation_before_parent_replaces_fallback_reply_with_ready_preview_in_place`:
  an unavailable reply target is replaced by ordered pending and ready `Set`
  operations without changing the reply row identity; and
- `room_power_grant_and_revoke_reset_existing_row_capabilities_and_redact_preflight`:
  unlike the four identity-only cases, this opens `NativeTimelineOwner` and
  exercises the live authority-aware projector; room-power grants and
  revocations reset historical row capabilities and the redact command
  independently enforces the same current authority.

These are deterministic SDK/Core adapter proofs. They are not evidence of live
homeserver acceptance, cross-device decryption, or two-client interoperability;
those live gates remain **Not confirmed**.
