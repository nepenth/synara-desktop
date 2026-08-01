# V-ROOMS.R-INVITE-REASON — invite reason SDK gap after #375

| Field   | Value                                                                                                                           |
| ------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Status  | **Open residual — docs only**                                                                                                   |
| Tip SHA | `c0d5ec40` (`#375`, native room moderation writes)                                                                              |
| Base    | `feature/matrix-rust-sdk-full-replacement`                                                                                      |
| SDK pin | `matrix-sdk = 0.18.0`, `matrix-sdk-ui = 0.18.0`                                                                                 |
| Policy  | Native desktop is fail-closed; `dual_backend` is forbidden                                                                      |
| Related | [V-ROOMS members moderation writes](v-rooms-members-moderation-write-scope.md), [full vertical policy](full-vertical-policy.md) |

> **Scope guard.** This is a documentation inventory. It does not change the
> invite UI, the Rust owner, the SDK pin, `product.rs`, `#375`, or `#39`.

## 1. The gap

The Matrix Client-Server API permits an optional `reason` in the direct invite
request body. The homeserver copies that value to the subsequent
`m.room.member` invite event. See the [invite endpoint in the Matrix
specification](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3roomsroomidinvite).

The pinned Matrix Rust SDK does not expose that request field on the high-level
invite method. In `matrix-sdk 0.18.0`,
[`Room::invite_user_by_id`](https://docs.rs/matrix-sdk/0.18.0/matrix_sdk/room/struct.Room.html#method.invite_user_by_id)
accepts only the invitee's `UserId`.

This is therefore a protocol capability that the current SDK surface cannot
reach. It is not a homeserver limitation and it is not evidence that the
native moderation vertical is incomplete in its other reason-bearing paths:
the native kick and ban methods already accept an optional reason.

## 2. What #375 owns today

The invite path is single-owner on desktop:

```text
InviteUserPrompt / UserModeration / /invite -r
  -> inviteUserWithNativeOwner
  -> matrix_room_invite { roomId, userId, reason? }
  -> normalize_moderation_reason(reason)
  -> Room::invite_user_by_id(user_id)
  -> homeserver invite request without reason
```

The current implementation deliberately keeps the optional argument at the
IPC and owner boundaries for API stability, but binds it to `_reason` before
calling the SDK method. The resulting behavior is:

| Surface            | Current behavior                                      | Honest status                       |
| ------------------ | ----------------------------------------------------- | ----------------------------------- |
| Invite user prompt | Collects and forwards an optional reason              | Input is accepted, not delivered    |
| `/invite ... -r`   | Forwards the optional reason through the native owner | Input is accepted, not delivered    |
| Native IPC         | Validates the room/user and receives `reason`         | Reason is normalized then discarded |
| SDK request        | Calls `Room::invite_user_by_id`                       | No reason field can be supplied     |
| JS fallback        | Not attempted                                         | Correct under fail-closed policy    |
| Kick / ban         | Pass optional reason to the SDK method                | Separate reason paths are supported |

The invite reason must not be described as delivered while the final native
call is `invite_user_by_id(&user_id)`.

## 3. Closure options

These are alternatives for one native owner, not backends to select at
runtime.

### Option A — consume a released SDK API with reason support (preferred)

Upgrade the pinned `matrix-sdk` / compatible `matrix-sdk-ui` release once the
high-level room invite API exposes the optional reason. Keep the invite on the
same native owner and pass the normalized reason to the SDK method.

Required work:

- confirm the released API and compatible SDK/UI versions before changing the
  exact Cargo pin;
- preserve the existing input validation and session requirement;
- remove the intentional discard and pass `reason.as_deref()` (or the released
  API's equivalent) to the SDK;
- add a request/event-level test proving the reason reaches the homeserver and
  appears on the invite membership event;
- update the lockfile and SDK provenance documents together.

This is the cleanest closure because the supported high-level SDK method stays
the sole Matrix mutation owner. The tradeoff is release timing and the normal
SDK upgrade compatibility work.

### Option B — contribute the missing capability upstream, then consume it

If no released SDK API is available, propose the smallest upstream Matrix Rust
SDK change: extend `Room::invite_user_by_id` (or add a clearly named reason-
aware companion), map the field into the Ruma invite request, and add SDK
coverage for the serialized request. After that change is released, close the
gap using Option A.

This keeps the behavior reusable for other Matrix Rust SDK clients and avoids
a local protocol fork. It is not closed by merely opening an upstream issue or
by depending on an unreleased commit; the application still needs a released,
pinned API and a live proof.

### Option C — use a typed low-level request through the same native SDK client

If product delivery cannot wait for a high-level SDK release, evaluate a
native low-level Ruma invite request sent by the existing authenticated
`matrix_sdk::Client`. This would remain one Rust/native owner, not a JS
fallback or a second backend.

This option is acceptable only if the pinned SDK exposes the required typed
request/send path and the implementation preserves the existing lifecycle,
error mapping, rate limiting, and sync/cache semantics. It must not construct a
raw REST URL or manually send a fabricated membership state event.

Before selecting it, prove:

- the typed request serializes `user_id` and `reason` exactly as the Matrix
  invite endpoint requires;
- the response/error behavior is equivalent to the current room method;
- subsequent sync produces the expected invite event and reason;
- no concurrent room mutation or retry path can create duplicate or
  contradictory ownership;
- the code remains inside the native moderation owner and has no JS fallback.

This is a viable native escape hatch, but it creates a lower-level SDK
maintenance surface. It should not be implemented speculatively in this docs
PR.

### Option D — remove the unsupported product promise

If invite reasons are not a required product capability, remove the reason
input and `-r` flag from the invite surfaces, remove the IPC argument, and
document that invite is ID-only. This closes the honesty gap by dropping the
unsupported behavior rather than delivering it.

This is a product decision, not an SDK replacement. It should be selected only
with explicit product approval; this PR does not make that decision.

## 4. Options that do not close the gap

- Calling `mx.invite` when the native path cannot carry a reason. That creates
  dual Matrix mutation ownership and violates the fail-closed desktop policy.
- Sending a separate room message containing the reason. A message is not the
  `reason` field on the invite membership event and changes the product
  semantics.
- Writing `m.room.member` directly with `sendStateEvent`. Membership
  transitions must use the invite API; this also bypasses the SDK's room
  mutation semantics.
- Keeping the current `_reason` discard and marking the residual complete.
  The command is native, but the requested protocol field is still absent.

## 5. Closure proof

V-ROOMS.R-INVITE-REASON is closed only when all of the following are true:

1. The single native invite owner sends the optional reason through a released,
   pinned SDK/Ruma path, or the product input has been explicitly removed.
2. No desktop invite path falls back to `mx.invite` or another Matrix writer
   when the native path is unavailable; failure remains terminal.
3. A focused unit/contract test proves the reason is retained through input
   normalization and native request construction.
4. An authenticated two-user proof verifies that the invited user's
   `m.room.member` invite event contains the requested reason. A command
   invocation test alone is insufficient.
5. The invite projection and any existing invite-reason display are checked
   for the delivered event value, with no claim that an absent value was
   delivered.
6. SDK pin/provenance and the #375 scope note no longer describe
   `invite_user_by_id` as discarding the reason.

Until then, the honest status is **native invite mutation wired; invite reason
SDK residual open**.
