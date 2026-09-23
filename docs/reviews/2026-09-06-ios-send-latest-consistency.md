# iOS send/latest consistency

## Operating path defined before changes

- Goal/actor: an iOS user sends an attachment, optionally with its caption, and sees the successful send at the live end.
- Starting state: a loaded live or historical room timeline and a staged attachment; failed upload must retain its draft and existing viewport intent.
- First action: tap the normal composer Send button once.
- Owner route: composer send plan → media uploader → Core/SDK send → acknowledged local echo → timeline owner snapshot/updates → exact server-event reconciliation → viewport confirmation.
- Transitions: staged → upload in progress → acknowledged → live viewport request → authoritative echo. A failed upload stops before successful-send navigation.
- Side effects/authority: local simulator fixtures and signed test build. No account messages or live credentials are used by the planned native UI proof. The normal production path sends only through Core.
- Completion/readback: existing UI assertions find the attachment/caption after the send, and reconciler tests show retention across an older snapshot and replacement by the same server event exactly once.
- Acceptance: live sends do not reopen the provider; history successful sends return live; failed uploads do not trigger successful-send navigation; acknowledged attachments survive a lagging snapshot and reconcile by exact identity.
- Disqualifiers: changing expected rows, retries, manufacturing a mock timeline echo, moving history before a failed upload, or treating HTTP acknowledgement as synchronized timeline completion.

## Diagnosis

CI 34073106648 at c4379c1f fails three final post-send row assertions: accessibility caption, image, and file. Earlier staging and Send interactions pass. The file failure accessibility hierarchy contains the old fixture timeline, empty composer, and no staged or uploaded attachment. Corresponding cases passed on runs 34073004381 and 34072678672 before the navigation change.

All three use `ComposerAttachmentSendPlan`'s single-attachment route (the accessibility text is a caption). `onUploaded` appended an ordinary stable row to view state, then the new unconditional `jumpToLatestStable` reopened the provider and applied a snapshot. Only local pending rows survive that replacement. Mock upload does not mutate the mock timeline, so the acknowledged attachment disappears deterministically. Production Core returns the `Room::send_attachment` HTTP event ID, without waiting for the timeline to observe it; the same snapshot boundary is therefore not an authoritative echo guarantee.

The earliest divergence is the UI treating an acknowledged upload as an authoritative timeline row while reopening the timeline to request scrolling. The repair belongs to the existing local-echo reconciliation and viewport request owners.

## Validation

The signed unmodified local baseline reproduced the exact file failure: the normal staging and send steps succeeded, then the existing attachment assertion failed (21.262s). Evidence: `/tmp/synara-send-latest-baseline.log`, `/private/tmp/synara-send-latest-baseline.xcresult`.

The repair requests scrolling without reopening an already-live provider. Historical provider transitions remain after successful upload. Uploaded rows enter the existing local-echo mechanism with `.sent` and their acknowledged server ID; an older snapshot retains the row, and the exact authoritative event replaces it without a duplicate. Exact identity takes precedence over sender/presentation normalization or text similarity. A response already present when the callback arrives is deduplicated at insertion. No second attachment store, mock timeline echo, upload retry, or pre-send navigation was added.

Native validation uses the matched full-Core/NSE artifact and binding pair from the accepted read worktree, cloned into this branch, with shared DerivedData and the dedicated iPhone 17 simulator. No Rust, generated binding, Core API, credential, or live-account changes. Initial validation builds caught required arguments missing from new test fixtures and the legacy scroll call; these were corrected before runtime proof.

The repaired run is `/tmp/synara-send-latest-confirmation-3.log` and `/private/tmp/synara-send-latest-confirmation-3.xcresult`. Unit coverage currently reports 158 passed and the one known ancestor caught-up fixture failure already repaired by accepted read commit 2feb53f3/13578624. That fixture is deliberately unchanged here. New exact-identity tests, attachment send-plan/failure tests, and media tests pass. Final UI result pending.

Live-server timing is not claimed as reproduced by the fixture proof.
