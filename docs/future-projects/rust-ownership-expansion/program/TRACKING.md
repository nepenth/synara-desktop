# Research tracking and residual status

Status: completed research run plus explicitly authorized remediation pass,
reconciled 2026-09-02.

`Ownership` answers the census question. `Residual` reports product/proof work
that remains. A closed ownership verdict never implies a proven feature.

| IDs       | Memo                                                | Research review                                              | Ownership                                           | Residual                                                                                                                                                                                                  |
| --------- | --------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ROE-01    | [memo](../memos/ROE-01-orchestration-memo.md)       | `ACCEPT_WITH_NITS`, `#1081`, `afba8efb`                      | closed: Core lifecycle owner                        | A11 closed: all-slice Apple generation, all-local graph, and exact promoted iOS unit gate passed 655 + 3 skips out of 658                                                                                 |
| ROE-02    | [memo](../memos/ROE-02-verification-memo.md)        | `ACCEPT`, `#1083`                                            | closed: one Core verification owner                 | A3 harness hardened; live proof still open                                                                                                                                                                |
| ROE-03    | [memo](../memos/ROE-03-timeline-rows-memo.md)       | `ACCEPT`, `#1086`                                            | closed: `TimelineViewRow` owner                     | [A7](A7-TIMELINE-PRODUCT-ACTIONS.md) adapter/action and pinned-SDK/Core sequencing implementation is deterministic-complete; live vote/report/forward/decline and two-client interoperability remain open |
| ROE-04/12 | [memo](../memos/ROE-04-message-format-memo.md)      | `ACCEPT_WITH_NITS`, `#1089`, `91bf0b14`                      | closed: platform output-context rendering           | A6 deterministic presentation/security matrix complete across Core, desktop, and iOS; sequencing proof is tracked and implemented separately under A7                                                     |
| ROE-05    | [memo](../memos/ROE-05-visibility-memo.md)          | `ACCEPT_WITH_NITS`, `#1091`, `25c1ee02`                      | closed: Core writes/platform observation            | A4 Core-only private receipt implementation complete; live proof open                                                                                                                                     |
| ROE-06    | [memo](../memos/ROE-06-room-sort-memo.md)           | `ACCEPT_WITH_NITS`, `#1087`, `eff03f01`                      | closed: facts in Core/chrome native                 | A8 closed: Mentions excludes marked-unread-only rooms                                                                                                                                                     |
| ROE-07    | [memo](../memos/ROE-07-notification-policy-memo.md) | `ACCEPT_WITH_NITS`, `#1085`, `651e36b2`                      | closed: Core settings/native delivery               | [A9](A9-NOTIFICATION-DELIVERY.md) deterministic iOS pusher/NSE lifecycle repair complete; physical APNs/NSE proof open and desktop decision source failed/open                                            |
| ROE-08    | [memo](../memos/ROE-08-agent-approval-memo.md)      | original `ACCEPT`, `#1082`; promotion review reopened        | remediated: one exact-event Core decision authority | A2 protocol metadata and live proof remain open                                                                                                                                                           |
| ROE-09    | [memo](../memos/ROE-09-notes-memo.md)               | `ACCEPT`, `#1084`                                            | closed: one Core account-data writer                | A5 deterministic target/projection integrity complete; concurrent live proof open                                                                                                                         |
| ROE-10    | [memo](../memos/ROE-10-drafts-memo.md)              | original `ACCEPT`, `#1088`; promotion review elevated defect | closed: reply metadata in Core/editor native        | A1 closed: Core-revision compare-and-clear spans every desktop send/cancel route; final upload-scope correction passed focused executable and full type validation                                        |
| ROE-11    | [memo](../memos/ROE-11-media-metadata-memo.md)      | `ACCEPT_WITH_NITS`, `#1090`, `41c3d35b`                      | closed: ADR 0005 handle/channel split               | [A10](A10-MEDIA-MEASUREMENT.md) deterministic bounded-transport measurement complete; real-device performance/cancellation evidence open and cache remains unwired                                        |

The detailed acceptance evidence and risk order live in
[ACTIONS.md](ACTIONS.md). Historical merge chronology is available in git and
the worker PRs; it is intentionally not presented as live coordination state.
The prior A1-A5/A8/A11 remediation's deterministic promotion gates were: Core
833 + 3 ignored out of 836 plus all integration/doc tests, desktop 910/910, iOS
unit 655 + 3 skipped out of 658, iOS UI 59 + 14 skipped out of 73, and the Tauri
compile. Its final Grok 4.6 High review found one attachment-send
revision-scope defect; after correction and targeted re-review, no P0-P2
findings remained. PR `#1092` promoted exact head `ce77bdcc` through 13
applicable successful checks with zero failures. Four scope-inapplicable checks
were skipped and are not counted as evidence. It merged as `e9b5016e`; local
`main` and authoritative `origin/main` both resolved to that commit on
2026-09-02. Those historical gates do not validate
`feature/rust-ownership-follow-ons`; its exact-head validation and review record
is locally green: Rust workspace 1,072 passed with 3 ignored, Tauri 441 passed,
desktop modernization 926 passed, Chromium layout 6 passed, repository delivery
277 passed, and Apple 760 passed with 17 intentional skips out of 777. The
production npm audit is clean and the Rust audits have no finding outside their
reviewed allowlists. Independent review accepted the final iOS remediation and
logout error mapping. The independent whole-change review and final Grok 4.6
High review both returned `ACCEPT` with no actionable P0-P2 findings. Remote PR
CI remains pending; all physical/live evidence gates remain open.
