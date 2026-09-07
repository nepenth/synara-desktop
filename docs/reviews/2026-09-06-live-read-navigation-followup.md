# Live read/navigation follow-up operating paths

Before changes, investigate two failures on the reviewed navigation branch:

1. A returning reader with authoritative current read state opens a room. The
   room should restore a comparable last-read position when it is behind the
   live tail; a caught-up reader should follow the live tail. Swift observes the
   room and timeline projections; Core owns receipts and ordering. The exact
   first action is room entry. Acceptance is the correct initial viewport mode
   and continued following behavior, without treating a delayed zero unread
   count as proof of a current receipt.
2. A reader at the visible live tail receives new messages, a folded edit and
   reaction, then backgrounds and foregrounds. The app's observed visible
   frontier should produce the private receipt and fully-read marker through
   Core. The server owns the final receipt/count readback; the UI must clear the
   room unread indication. Side effects are the existing receipt writes and
   disposable test fixture messages. Authority is limited to the configured test
   accounts and dedicated simulator. No manual mark-read, changed privacy
   preference, unseen-event acknowledgement or UI retry can establish success.

The initial failed live run reached the background message and advanced its
fully-read marker, then failed the combined private-receipt/count assertion.
The earliest divergence within that combined assertion is not yet known. Do not
change the expected receipt or add retries without distinguishing the actual
receipt, notification count and owner completion boundary.

Initial runtime verdict: **Failed** for the combined acceptance test;
classification/root cause remains **Not confirmed** pending owner readback.

Source findings before runtime instrumentation:

- The caught-up unit fixture contains four ordered remote messages, a read
  receipt at index 2 and a fully-read marker at index 1. Its false unread-count
  flag contradicts a newer visible message at index 3. The requested navigation
  contract explicitly restores the comparable read marker even when room-list
  counts lag. True caught-up coverage must establish that the read frontier is
  at the actual live tail; a zero count alone cannot establish that condition.
- The live test already passed the initial/live/edit/reaction acknowledgements,
  its background negative check, the foreground fully-read marker, and the
  cleared room UI. The combined final helper separately fetches `/sync` without
  a cursor and conflates missing private receipt with nonzero notification
  count. A bounded readback split is required before changing product behavior.

Diagnostic preparation (product unchanged):

- Corrected the caught-up fixture to put its receipt on the actual last row.
  Preserved the former input as an explicit zero-count/newer-receipt-behind-tail
  regression, so neither contract is lost.
- Separated private receipt and notification count assertions. Readback reports
  room presence, count, expected receipt presence, number of own private receipt
  targets and whether the server batch repeats the previous observation. Logs
  exclude account, room and event identifiers, tokens, and message contents.
- Helper session logout and both room leaves now fail the test on error while
  still attempting every cleanup. The app-created session is retained on the
  dedicated simulator; this is not all-device session cleanup.

Candidate observer divergence, not yet a runtime conclusion: Synapse caches
sync responses by a key containing the literal filter string and `since`.
The helper repeatedly requests an initial sync without advancing a cursor.
The server's initial-sync response cache can therefore replay an earlier
read state. See upstream [sync request key](https://github.com/element-hq/synapse/blob/develop/synapse/rest/client/sync.py)
and [sync response cache](https://github.com/element-hq/synapse/blob/develop/synapse/handlers/sync.py).
No manual receipt, extra UI acknowledgement gesture or retry was added.

Diagnostic result: **Failed**. The fresh diagnostic run at
`/private/tmp/synara-live-read-diagnostic.xcresult` passed the actual app route
through foreground and reopening, including visible rows, fully-read markers,
negative background/offscreen checks and cleared room UI. Edit, reaction and
foreground observations each had the expected private receipt and count zero.
The reopened readback had count zero but the wrong receipt, the exact previous
`next_batch`, and the previous foreground receipt. This confirms an evidence
boundary defect in the helper's repeated initial sync; no product read-path
failure was observed. Helper cleanup produced no failures.

Repair at the observer boundary: carry `next_batch` into the next single `/sync`
request's `since` parameter, using stable filter serialization. Reject a missing
batch token instead of silently reverting to initial sync. Preserve separate
private receipt and count assertions. The diagnostic itself is not successful
proof; the entire navigation/live path must pass again with fresh helper
sessions, a fresh room and an app session reset after this repair.

Clean post-repair result: **Confirmed** within the configured live-account and
signed dedicated-simulator scope. `/private/tmp/synara-live-read-confirmation.xcresult`
and `/tmp/synara-live-read-confirmation.log` record 255 selected Swift unit tests
(one pre-existing opt-in test skipped, zero failures) and all five selected UI
tests passing. The actual live automatic-read path passed in 45.013 seconds.
Each edit, reaction, foreground and reopened observer sample reported the
expected private receipt, notification count zero and a new server batch. All
original fully-read checks, visible-row checks, negative background/offscreen
checks and cleared room-list UI passed. All helper leave/logout operations
completed without a cleanup failure. No retry, manual receipt or extra read
acknowledgement gesture was introduced.

The four independent navigation UI cases also passed: unread-position
restoration, explicit recovery for a missing marker, sending from history back
to latest, and stable three-message/jump-to-latest/5,000-event boundedness.
The corrected caught-up fixture and zero-count regression passed among 125
TimelineServiceTests. The repair contains no product changes. The preserved
boundary mechanism is the existing single-observation proof, now reading the
server's advancing stream and asserting receipt/count separately.
