# Offline timeline operating path

## Contract

- Goal: after a room has synchronized, terminating and reopening Synara while
  offline renders that room's cached messages.
- Owner route: iOS timeline service -> SharedCore UniFFI -> native timeline
  owner -> Matrix Rust SDK room event cache -> the account's SQLite cache
  store.
- The SDK SQLite event cache is the sole persisted timeline owner. The iOS
  shell must not create a second message cache.
- A timeline is authoritatively empty only after the native view opened
  successfully, projected no displayable rows, and reported backward
  pagination as `exhausted`.
- An open, snapshot, pagination, store, decryption, timeout, or connectivity
  failure is an unavailable/failed outcome. It must never be converted to an
  empty outcome.
- When a transient failure or empty update arrives after a last-good in-process
  snapshot, presentation keeps the last-good rows and exposes a retryable,
  static error message.
- Diagnostic codes and user messages are fixed source strings. Raw SDK error
  text, room contents, user identifiers, homeserver URLs, filesystem paths,
  and credentials do not cross the presentation boundary.

## Decisive proof

1. Build and restore an authenticated client through the production client
   builder, using its separate SQLite state and event-cache paths.
2. Subscribe the SDK event-cache owner and process a sync containing a known
   room event.
3. Drop the complete client/process state.
4. Make the homeserver unavailable.
5. Build and restore a fresh client against the same account store.
6. Open the room through the native timeline owner and read back its initial
   snapshot.
7. Confirm the known event is present without a network request, retry,
   fallback, manual repair, or shell-owned message cache.

An outcome-typing unit test is necessary but does not substitute for this
cold-restart proof.

## Automated clean-run proof

Run from the repository root:

```sh
cargo test -p synara-core --test offline_timeline_cold_restart -- --nocapture
```

`offline_timeline_cold_restart` starts a disposable local Matrix `/sync`, opens
the account through the production client builder's separate encrypted SQLite
state and event-cache paths, subscribes the SDK event-cache owner, and syncs a
unique room event exactly once. It then stops the server, drops the entire
client, rebuilds and restores a fresh client against the same account store,
and opens the room through `NativeTimelineOwner`. The authoritative assertion
is the unique event id and body in the fresh native timeline snapshot.

The test fails if the post-restart timeline owner waits more than two seconds,
cannot open the persisted room, or omits the cached event. It does not retain
the first client or maintain an iOS/shell-owned message cache.
