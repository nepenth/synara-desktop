# Grok4.6 High source review

I'll review this as a third-party source check of the test-only observer and fixture correction, starting with the review skill and then the exact head, base, and scoped files.The review skill would spawn a subagent; I'll do this myself with read-only inspection of the exact head, the provided delta, and the surrounding test/observer code.I'll read the live test, observer helper, fixture, and focus-policy implementation next so the verdict is based on those paths, not the diff alone.Checking one more cleanup interaction: class-level `continueAfterFailure` versus `XCTFail` inside the leave `defer`.**REQUEST_CHANGES**

Test-only delta. Parent navigation production source is **not** approved. Recorded 255/5/`45.013s` is author-scoped evidence, not physical-app proof from this review. Worktree contents match the supplied diff; SHA `2feb53f` was not independently hashed (read-only Read/Grep, no git/web/subagents).

## Blocking

**Cleanup can stop after the first failed leave, so it does not attempt every helper leave/logout.**

- `synara-ios/SynaraUITests/SynaraUITests.swift:7`
- `synara-ios/SynaraUITests/SynaraUITests.swift:1171-1176`
- also `1161-1168`

`setUpWithError` sets `continueAfterFailure = false`. The leave `defer` `XCTFail`s on writer leave, then tries reader leave. Under that XCTest setting, `XCTFail` aborts the test method. Remaining statements in that `defer` do not run, so `reader.leaveRoom` is skipped. If XCTest unwinds with `fatalError`/NSException rather than a Swift error, the logout `defer`s can be skipped too.

That breaks the delta’s own contract: fail loudly **and** attempt every helper leave/logout.

The previous `try?` swallowed errors but always attempted all four calls. This is louder and **narrower** on the failure path.

**Fix:** do all four operations first, collect failures, then one `XCTFail`:

```swift
defer {
    var failed: [String] = []
    do { try writer.leaveRoom(roomID: roomID) } catch { failed.append("writer leave") }
    do { try reader.leaveRoom(roomID: roomID) } catch { failed.append("reader leave") }
    do { try writer.logout() } catch { failed.append("writer helper logout") }
    do { try reader.logout() } catch { failed.append("reader helper logout") }
    if !failed.isEmpty {
        XCTFail("Live read cleanup failed: \(failed.joined(separator: ", "))")
    }
}
```

Keep the comment at `1183-1185`: helper leave/logout is not all-device revocation; app login on the dedicated simulator stays.

## Other axes — no further blocking issues

**Observer false positives.** Exact-target private receipt plus `notificationCount == 0` is not weaker than the old combined Bool. `hasExpectedPrivateReceipt` requires `m.read.private` for this helper `userID` on the **current** event id (`3188-3206`, asserted at `1264-1265`). The documented stale replay (same `next_batch`, previous receipt, count 0) fails that check because later phases use different event ids (edit → reaction → background → offscreen). `repeatsPreviousBatch` / `containsPreviousExpectedReceipt` / `roomPresent` are logged and not asserted; that is residual, not a false-pass for a newer event. Missing `next_batch` throws (`3194-3196`); no silent fallback to initial `/sync`. No receipt/count retries; `waitForFullyRead` is still the owner-completion wait, then one `readState`.

**Cursor / session isolation.** `readObservationBatch` lives on the helper instance (`2945-2946`), starts nil, and is only advanced in `readState`. `fullyReadEventID` uses account-data GET, not `/sync` (`3122-3129`), so it does not steal the observer cursor. Writer never calls `readState`. App login is a separate UI session (`2482-2496`) after local `SYNARA_RESET_SESSION_ON_LAUNCH`; that flag wipes local store/credentials, not `/logout/all`. Helper `/logout` is per-token. Fresh room id each run, so the filter string is not reused across runs.

**Missing fields.** Nil `notification_count` fails `XCTAssertEqual(..., 0)` (`1265`, `3181`). Missing `next_batch` throws. Empty/missing ephemeral yields `hasExpectedPrivateReceipt == false`. Diagnostic print at `1262-1263` includes both receipt and count before asserts (needed because `continueAfterFailure = false` would skip the count assert after a receipt failure).

**Assertions not weakened.** Same four proof sites as before (edit/reaction/foreground/reopened). Split asserts replace one Bool. Reopened still requires count 0. No expected-receipt change, no extra gesture, no helper retry.

**Fixture correction.** `focusPolicyItems` is four rows `0..3` (`2610-2627`). Caught-up now puts the receipt on tail index `3` (`575`) → `.live`. Former input is kept as `testRoomTimelineZeroUnreadCountPreservesNewerComparableReceiptBehindTail` (`581-590`) expecting `.unread(markerEventID: "$synthetic-2:matrix.org")`. That matches “zero count is not proof of a current receipt.”

**Live path coverage in source (not runtime proof).** Visible initial/live rows; folded edit/reaction on the same hittable row; background negative (`1226-1227`); foreground ack + room-list “unread” clear; offscreen negative (`1243-1244`); then exact private receipt + zero count.

**Docs evidence scope.** `docs/reviews/2026-09-06-live-read-navigation-followup.md` is consistent with the source (no product files in this delta; helper `since`; split asserts; loud cleanup intent; simulator session retained). Paths like `/private/tmp/synara-live-read-confirmation.xcresult`, “255 selected Swift”, and `45.013s` are author records. This review did not read those artifacts and does not treat them as independent device proof. Passing navigation UI cases on this branch is not parent-nav sign-off.

## Non-claims

- No credentials, `.env`, or Keychain contents inspected beyond the already-public reset-on-launch flag.
- Synapse cache links in the follow-up doc were not fetched.
- Parent `RoomTimelineFocusPolicy` / desktop nav corrections were used only to check the fixture, not reviewed for acceptance.
