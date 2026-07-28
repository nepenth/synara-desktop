# MiniMax draft — scroll_position / large_room re-review

> Generated via local MiniMax-M3 (2026-07-28), then lightly cleaned by Grok
> (removed chain-of-thought preamble; truncated completion). **Draft only** —
> not authoritative until full human/Grok review against matrix-sdk-ui APIs.
>
> Job type: feature-graph hard-problem checklist (see
> [`../minimax-parallel-work.md`](../minimax-parallel-work.md)).
>
> Nodes: `timeline.read_window`, `composer.send_echo`, `nav.routes`.

## Must-assert behaviors

- Variable-height row changes (image load, expand/collapse) preserve the pixel offset of the anchor event at the top of the viewport.
- Back-pagination prepends older events without shifting the first visible event’s vertical position (pixel-stable anchor).
- Jump-to-latest scrolls to the live tail and pins to bottom during live append unless the user has scrolled up (sticky/unsticky).
- Focused-event open (permalink, search hit) brings the target event to a deterministic viewport position, paginating if needed.
- Local echo appears immediately; server echo replaces it without scroll jump or duplicate rows.
- Resize / zoom / DPI changes re-measure without re-anchoring to the wrong event.
- Rust host is sole Timeline/sync owner; no concurrent js-sdk TimelineWindow or second client.

## Automated tests to require

- Variable-height insertion: change a row height; assert anchor event pixel position unchanged.
- Back-pagination: near-top scroll triggers prepend; assert anchor event screen Y stable.
- Jump-to-latest: command lands at live tail; subsequent live append keeps pin while at-bottom.
- Focused-event: open mid-history event id; assert visible within agreed band (center/top).
- Local echo → server echo: no duplicate item keys; scroll delta within tolerance.
- Concurrent pagination + live append race does not drop anchor or duplicate items.
- Large-room stress: bounded memory / window size with multi-10k synthetic history.
- No product import of `matrix-js-sdk` timeline helpers after cutover (guardrail).

## Anti-patterns to reject

- Reimplementing js-sdk `TimelineWindow` sliding-window ownership in the UI.
- Keying virtualized rows by array index instead of SDK item / event / txn identity.
- Fixed-height assumptions for scroll math in variable-height rooms.
- Dual Matrix clients or dual sync owners for one session.
- Recalculating absolute scrollTop from scratch on every diff without an event anchor.
- Losing position when local echo is replaced by the server event.
- Blocking the UI thread on pagination network/crypto work.
- Silent dual-backend fallback for “hard rooms.”

## Open questions for human/Grok

1. Which matrix-sdk-ui Timeline identity fields are the contract for UI keys (event id, unique id, txn id for echoes)?
2. Focused events outside the loaded window: max auto-pagination depth vs explicit “jump failed” UX?
3. Policy for auto back-pagination threshold (rows from top) vs user-gesture-only?
4. Do read-receipt / fully-read marker updates ever move the viewport, or only badges?
5. Packaged DPI/scale matrix for scroll tests on macOS vs Linux WebView?

## MiniMax run notes

| Metric | Value |
| --- | --- |
| Endpoint | `spark-1.whyland.com:8000` MiniMax-M3 W4A16 GPTQ |
| Wall time (this job) | ~60–75s for ~900–1200 completion tokens |
| Observation | Model often emits internal reasoning before final bullets; prompts must demand “final Markdown only,” and Grok should strip preamble before landing. |
| Concurrency guidance | 2–4 parallel jobs still appropriate for inventory/checklist drafts during CI waits. |
