# Research tracking and residual status

Status: completed research run plus explicitly authorized remediation pass,
reconciled 2026-09-02.

`Ownership` answers the census question. `Residual` reports product/proof work
that remains. A closed ownership verdict never implies a proven feature.

| IDs | Memo | Research review | Ownership | Residual |
| --- | --- | --- | --- | --- |
| ROE-01 | [memo](../memos/ROE-01-orchestration-memo.md) | `ACCEPT_WITH_NITS`, `#1081`, `afba8efb` | closed: Core lifecycle owner | A11 closed: all-slice Apple generation, all-local graph, and exact promoted iOS unit gate passed 655 + 3 skips out of 658 |
| ROE-02 | [memo](../memos/ROE-02-verification-memo.md) | `ACCEPT`, `#1083` | closed: one Core verification owner | A3 harness hardened; live proof still open |
| ROE-03 | [memo](../memos/ROE-03-timeline-rows-memo.md) | `ACCEPT`, `#1086` | closed: `TimelineViewRow` owner | A7 DTO/adapter and message/poll/sticker relation parity implemented; new action UI follows separately |
| ROE-04/12 | [memo](../memos/ROE-04-message-format-memo.md) | `ACCEPT_WITH_NITS`, `#1089`, `91bf0b14` | closed: platform output-context rendering | A6 shared corpus, harness integrations, and send-bound enforcement implemented; full matrix remains open |
| ROE-05 | [memo](../memos/ROE-05-visibility-memo.md) | `ACCEPT_WITH_NITS`, `#1091`, `25c1ee02` | closed: Core writes/platform observation | A4 Core-only private receipt implementation complete; live proof open |
| ROE-06 | [memo](../memos/ROE-06-room-sort-memo.md) | `ACCEPT_WITH_NITS`, `#1087`, `eff03f01` | closed: facts in Core/chrome native | A8 closed: Mentions excludes marked-unread-only rooms |
| ROE-07 | [memo](../memos/ROE-07-notification-policy-memo.md) | `ACCEPT_WITH_NITS`, `#1085`, `651e36b2` | closed: Core settings/native delivery | A9 open delivery evidence |
| ROE-08 | [memo](../memos/ROE-08-agent-approval-memo.md) | original `ACCEPT`, `#1082`; promotion review reopened | remediated: one exact-event Core decision authority | A2 protocol metadata and live proof remain open |
| ROE-09 | [memo](../memos/ROE-09-notes-memo.md) | `ACCEPT`, `#1084` | closed: one Core account-data writer | A5 deterministic target/projection integrity complete; concurrent live proof open |
| ROE-10 | [memo](../memos/ROE-10-drafts-memo.md) | original `ACCEPT`, `#1088`; promotion review elevated defect | closed: reply metadata in Core/editor native | A1 closed: Core-revision compare-and-clear spans every desktop send/cancel route; final upload-scope correction passed focused executable and full type validation |
| ROE-11 | [memo](../memos/ROE-11-media-metadata-memo.md) | `ACCEPT_WITH_NITS`, `#1090`, `41c3d35b` | closed: ADR 0005 handle/channel split | A10 open performance evidence |

The detailed acceptance evidence and risk order live in
[ACTIONS.md](ACTIONS.md). Historical merge chronology is available in git and
the worker PRs; it is intentionally not presented as live coordination state.
Final deterministic promotion gates: Core 833 + 3 ignored out of 836 plus all
integration/doc tests, desktop 910/910, iOS unit 655 + 3 skipped out of 658,
iOS UI 59 + 14 skipped out of 73, and the Tauri compile are green. Final Grok
4.6 High review found one attachment-send revision-scope defect; after
correction and targeted re-review, no P0-P2 findings remain. PR `#1092`
promoted exact head `ce77bdcc` through 13 applicable successful checks with
zero failures. Four scope-inapplicable checks were skipped and are not counted
as evidence. It merged as `e9b5016e`; local `main` and authoritative
`origin/main` both resolved to that commit on 2026-09-02.
