# P0.6 — Baseline reliability / performance evidence

| Field | Value |
| --- | --- |
| Task | **P0.6** Baseline reliability/performance evidence |
| Date | 2026-07-24 |
| Work branch | `matrix-rust/p0.6-performance-baseline` |
| Integration tip (base) | `feature/matrix-rust-sdk-full-replacement` @ `a2d288b0762104fb15a6c4829bfe1293e865f5b6` |
| Tip message | `docs(matrix): merge P0.5 toolchain compatibility` (P0.1–P0.5 merged; PR #45 for P0.5) |
| Product under baseline | Current desktop **`matrix-js-sdk` 42.0.0** product (Synara `1.2.59`) |
| Machine twin | [`performance-baseline.json`](performance-baseline.json) |
| Status | **Accepted** by independent review — PR to integration (`merged: false`; `pass-with-residuals`) |

Authoritative program plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md) (Phase 0 P0.6; Phase 13 § budgets).

## Executive status

| Surface | Status | Notes |
| --- | --- | --- |
| Metric catalog + Phase 13 budget mapping | **Complete** | All plan-required metrics defined with start/stop, owner, method, privacy |
| Automated timeline row-mapping harness | **Measured** | 50-iteration p50/p95 on macOS arm64; budget `< 25 ms` holds |
| Live signed-in UX latencies (startup, room switch, pagination, reconnect, E2EE open, media) | **Residual — methodology documented** | No fabricated p50/p95; agent session lacks signed-in multi-run protocol execution |
| Memory / idle CPU / disk growth live scenarios | **Residual — methodology documented** | Standard large-account scenario defined; measurements pending operator |
| Linux live UX baselines | **Residual** | Host is macOS; Linux procedure documented, not executed here |
| Disposable Synapse multi-hour soak | **Out of scope for P0.6 stop condition** | Optional later; not required to close Phase 0 evidence scaffold |

**Honest summary:** Phase 0 now has a **committed, repeatable catalog and an automated proxy baseline** for the virtualization row-mapping layer that feeds the web timeline. End-to-end user-path p50/p95 values are **not** invented; they remain `residual-pending-live` with exact operator steps below. Phase 13 will compare Rust cutover results against this catalog using the same markers and budgets (±10% p95 on key latencies; memory ±15%; idle CPU within Phase 0 noise band).

## Non-goals

- Not a Matrix Rust SDK performance study (no production SDK accepted yet).
- Not a dual-backend A/B framework.
- Not product behavior changes to “improve numbers.”
- Cold `cargo check` / compile times are **not** UX baselines (toolchain cost is P0.5).
- iOS numbers in `synara-ios/docs/performance-report.md` are **reference only**, not desktop Phase 13 baselines.
- No secrets, room IDs, event content, tokens, or homeserver credentials in artifacts.

---

## Environment inventory

| Item | Value |
| --- | --- |
| Host OS | macOS 26.5.2 (Build 25F84) |
| Arch | arm64 (`aarch64-apple-darwin`) |
| CPU / RAM | Apple M2 / 24 GiB |
| Node | v22.22.3 |
| npm | 10.9.8 |
| rustc / cargo | 1.93.0 (254b59607 2026-01-19) / 1.93.0 (083ac5135 2025-12-15) |
| App marketing version | `1.2.59` (root + `synara/` + production package) |
| Matrix product SDK | `matrix-js-sdk@42.0.0` (JS + rust-crypto wasm path) |
| Tauri app launched for live UX metrics? | **No** (process may exist on host; P0.6 agent did **not** execute signed-in multi-run capture) |
| Homeserver used for live metrics | **None** (live residual) |
| Disposable Synapse available | `scripts/synapse-integration.sh` + `integration/synapse/` (not started for this task) |
| Linux host for live metrics | **Not available** on this agent host |

---

## Metric catalog

Privacy default for all metrics: **never** record room IDs, event IDs, user IDs, message text, ciphertext, tokens, recovery keys, or full homeserver URLs in committed artifacts. Prefer counts, enums, and relative timings. See [`../timeline-diagnostics.md`](../timeline-diagnostics.md).

### M-STARTUP-READY

| Field | Definition |
| --- | --- |
| ID | `M-STARTUP-READY` |
| Definition | Wall time from process/webview start (or cold session restore entry) until the signed-in client is **interactive**: crypto boot complete + first usable room list / main shell (sync state past blocking splash). |
| Start marker | App process launch (packaged) or `ClientRoot` session init begin (`initClient` path). |
| Stop marker | Splash dismissed / main client chrome interactive after `startClient` and usable sync state (operator notes exact UI state). |
| Primary owner | Tauri shell cold start + web bootstrap + `matrix-js-sdk` store/`initRustCrypto` + sync |
| Measurement now | **Live manual** (`residual-pending-live`) |
| Phase 13 budget | p95 no worse than **+10%** vs this baseline |
| Privacy | No account identifiers in logs; use anonymous run index only |

### M-ROOM-SWITCH-STABLE

| Field | Definition |
| --- | --- |
| ID | `M-ROOM-SWITCH-STABLE` |
| Definition | Time from room selection (nav click) to **first stable timeline render** suitable for reading/scrolling. |
| Start marker | Room select / route change into room. |
| Stop marker | Privacy-safe `room-timeline.first-stable-bottom` (or equivalent stable open for focused-event mode) with `elapsedMs`. |
| Related records | `room-timeline.open`, `room-timeline.render-window`, `room-timeline.first-stable-bottom` |
| Primary owner | Web timeline (`RoomTimeline`) + JS SDK room/timeline load |
| Measurement now | **Live manual** via timeline diagnostics (`residual-pending-live`) |
| Phase 13 budget | p95 no worse than **+10%** |
| Privacy | Diagnostics must not include room/event IDs (existing design) |

### M-TIMELINE-OPEN-INITIAL

| Field | Definition |
| --- | --- |
| ID | `M-TIMELINE-OPEN-INITIAL` |
| Definition | Initial open of a previously visited large room at live bottom (cleartext or already-decrypted history). Distinct from room-switch when cold room data must load. |
| Start / stop | Same family as room-switch; report separately when room was not the previously selected room and timeline must load. |
| Primary owner | JS SDK timeline window + virtualization |
| Measurement now | **Live manual** (`residual-pending-live`) |
| Phase 13 budget | p95 no worse than **+10%** (groups with room-switch / initial timeline plan language) |
| Privacy | Counts + timings only |

### M-TIMELINE-OPEN-ENCRYPTED

| Field | Definition |
| --- | --- |
| ID | `M-TIMELINE-OPEN-ENCRYPTED` |
| Definition | Open encrypted room until first stable readable render (decrypted placeholders resolved or stable UTD placeholders shown — operator notes which). |
| Start / stop | Room select → first stable render with crypto path exercised |
| Primary owner | rust-crypto wasm + timeline decrypt path + UI |
| Measurement now | **Live manual** (`residual-pending-live`) |
| Phase 13 budget | **initial encrypted timeline p95** no worse than **+10%** |
| Privacy | Never log plaintext or ciphertext payloads |

### M-PAGINATION-BACKWARD

| Field | Definition |
| --- | --- |
| ID | `M-PAGINATION-BACKWARD` |
| Definition | One backward history page: from pagination start to complete with anchor preserved. |
| Start / stop | `room-timeline.pagination-start` → `room-timeline.pagination-complete` (error path: `pagination-error`) |
| Primary owner | JS SDK pagination + virtual timeline anchor restore |
| Measurement now | **Live manual** (`residual-pending-live`) |
| Phase 13 budget | p95 no worse than **+10%** |
| Privacy | Range counts only; no event bodies |

### M-RECONNECT-SETTLE

| Field | Definition |
| --- | --- |
| ID | `M-RECONNECT-SETTLE` |
| Definition | Offline → online: time until sync returns to healthy settled state and UI stops error/reconnect thrash. |
| Start marker | Network offline (OS or forced) while signed in and previously SYNCING. |
| Stop marker | Sync healthy + room list/timeline consistent (no dual sync owners). |
| Primary owner | JS SDK sync + network stack + product reconnect UI |
| Measurement now | **Live manual** (`residual-pending-live`) |
| Phase 13 budget | Plan lists reconnect in P0.6 measure set; compare under P13.1 soak/reconnect. Use same start/stop; apply **±10% p95** as the latency budget unless Phase 13 runbook tightens soak-only criteria. |
| Privacy | No tokens / device IDs in committed tables |

### M-MEDIA-OPEN

| Field | Definition |
| --- | --- |
| ID | `M-MEDIA-OPEN` |
| Definition | Time from user open of an image thumbnail or attachment until first paint of media content (or explicit failure UI). |
| Start / stop | Click open → first visible decode/paint |
| Primary owner | Media fetch/cache + webview decode |
| Measurement now | **Live manual** (`residual-pending-live`) |
| Phase 13 budget | No worse than **+10% p95** (media called out in P0.6; treat as key latency) |
| Privacy | No MXC URIs or filenames that encode room secrets; use media class labels only |

### M-IDLE-CPU

| Field | Definition |
| --- | --- |
| ID | `M-IDLE-CPU` |
| Definition | Process CPU % after settled sync, no user input, steady-state for ≥60s. |
| Primary owner | Sync long-poll/idle + web timers + shell |
| Measurement now | **Live manual** Activity Monitor / `top` (`residual-pending-live`) |
| Phase 13 budget | No regression beyond **measurement noise agreed in Phase 0** — provisional noise band: **±2 percentage points absolute** on the same host class, or document a tighter band when live samples exist. |
| Privacy | Aggregates only |

### M-MEMORY-LARGE-ACCOUNT

| Field | Definition |
| --- | --- |
| ID | `M-MEMORY-LARGE-ACCOUNT` |
| Definition | JS heap (where exposed) + process RSS after **standard large-account scenario** (below). |
| Standard scenario (definition even if residual) | Signed-in account with ≥200 joined rooms if available (else max available; record N); open 5 rooms including ≥1 encrypted large room (≥1k loaded events if available); leave app settled 2 minutes; sample memory. |
| Primary owner | JS SDK stores + DOM/virtualizer + crypto |
| Measurement now | **Live manual** (`residual-pending-live`); overlay heap when `synara.performance.debug=true` |
| Phase 13 budget | Memory no worse than **+15%**; no unbounded growth |
| Privacy | No room names in committed tables — use counts |

### M-DISK-GROWTH

| Field | Definition |
| --- | --- |
| ID | `M-DISK-GROWTH` |
| Definition | Growth of app data / IndexedDB / crypto store over a fixed scenario. |
| Fixed scenario | Cold launch → login or restore → idle 10 min with light room opens (5) → measure store directory size delta. Prefer disposable Synapse test account. |
| Paths (operator) | Browser/webview IndexedDB for current product; Tauri app data directory on desktop — record path **class**, not account-derived names, in public notes. |
| Primary owner | IndexedDB + rust-crypto store (today); native encrypted SQLite post-cutover |
| Measurement now | **Live manual** (`residual-pending-live`) |
| Phase 13 budget | No unbounded store growth; compare absolute delta within scenario |
| Privacy | Never commit store dumps |

### M-TIMELINE-MAP-10K / M-TIMELINE-MAP-50K (automated proxy)

| Field | Definition |
| --- | --- |
| IDs | `M-TIMELINE-MAP-10K`, `M-TIMELINE-MAP-50K` |
| Definition | Synthetic timeline **row key/index map** construction for a **rendered window of 200** events drawn from a virtual 10k / 50k event history. |
| Start / stop | Single `performance.now()` span around `makeRows` + Map fill (same as product harness). |
| Primary owner | Web timeline virtualization mapping layer (not SDK sync) |
| Measurement now | **Automated** — `npm run test:timeline-performance` + multi-iteration aggregator |
| Budget (harness) | `durationMs < 25` per scenario (hard assert) |
| Phase 13 role | Regression canary for mapping layer; **does not replace** live room-switch / pagination UX budgets |
| Privacy | Fully synthetic keys (`$synthetic-N`) |

---

## Automated results (executed)

### Product harness (single-shot smoke)

```sh
cd synara && npm run test:timeline-performance
```

Observed on this host (illustrative single run; multi-iteration is authoritative):

| events | rendered_events | rows | duration_ms |
| ---: | ---: | ---: | ---: |
| 10000 | 200 | 202 | ~0.09 |
| 50000 | 200 | 202 | ~0.05 |

Hard budget: each scenario `< 25 ms`. **PASS.**

### Multi-iteration p50/p95 (authoritative automated baseline)

```sh
# from repo root
node scripts/matrix-rust-p0.6-baseline-harness.mjs --iterations 50
node scripts/matrix-rust-p0.6-baseline-harness.mjs --iterations 50 --json
node --test scripts/__tests__/matrix-rust-p0.6-baseline-harness.test.mjs
```

| Metric | n | min_ms | p50_ms | p95_ms | max_ms | mean_ms | budget_ms | result |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `M-TIMELINE-MAP-10K` | 50 | 0.0243 | **0.0384** | **0.1611** | 0.2793 | 0.0517 | 25 | **PASS** |
| `M-TIMELINE-MAP-50K` | 50 | 0.0244 | **0.0255** | **0.0869** | 0.1165 | 0.0299 | 25 | **PASS** |

- Captured at: `2026-07-24T23:28:12.781Z`
- Host: macOS arm64, Node v22.22.3, Apple M2
- Percentile method: nearest-rank on sorted samples
- Note: 50k is often *faster* than 10k here because both scenarios only materialize the same ~200-event rendered window; the eventCount parameter only shifts the synthetic index range.

### Non-UX proxies (explicitly labeled — not Phase 13 UX baselines)

| Proxy | Status | Label |
| --- | --- | --- |
| Production `src-tauri` `cargo check --locked` | Proven in P0.5 on Rust 1.93 | **build proxy only** |
| Isolated matrix-sdk coexistence probe compile | Proven in P0.5 | **build proxy only** |
| iOS fixture wall times | Historical reference in iOS docs | **not desktop baseline** |

---

## Live scenario protocol (macOS)

Even when numbers are residual, Phase 13 operators must use this protocol so baselines remain comparable.

### Preconditions

1. Build type recorded: `dev` (`npm run tauri dev`) **or** local package (`npm run tauri build -- --bundles app` / local release script). Prefer **local release-like** for Phase 13; dev is acceptable for Phase 0 methodology dry-runs if labeled.
2. Prefer **disposable Synapse** (`scripts/synapse-integration.sh`) over production homeservers for scripted traffic.
3. Signed-in test account; no production secrets in repo.
4. Commit SHA recorded (`git rev-parse HEAD`).
5. Quit other heavy apps; note power mode (plugged in).

### Enable instrumentation

In the webview DevTools console:

```js
localStorage.setItem('synara.performance.debug', 'true');
location.reload();
```

Timeline diagnostics (privacy-safe) always append via `desktop_append_log` on desktop. Resolve native log path:

```js
window.__SYNARA_DESKTOP__?.invoke('desktop_log_path').then(console.info);
```

Optional foreground capture:

```bash
"/Applications/Synara.app/Contents/MacOS/synara" 2>&1 \
  | tee "$HOME/Desktop/synara-timeline-$(date +%Y%m%d-%H%M%S).log"
```

Disable:

```js
localStorage.removeItem('synara.performance.debug');
```

Reference: [`../../synara/docs/synara-performance.md`](../../synara/docs/synara-performance.md), [`../timeline-diagnostics.md`](../timeline-diagnostics.md), [`../production-smoke-checklist.md`](../production-smoke-checklist.md).

### Extract timings without secrets

1. Filter native log / console for `room-timeline.*` and `[synara:perf]`.
2. Export only: event name, `elapsedMs` / duration, counts, booleans, openMode enums.
3. Redact any accidental identifiers before sharing.
4. Do **not** commit raw logs to the repository.

### Sample sizes

| Metric family | Recommended n | Notes |
| --- | ---: | --- |
| Room switch / timeline open / pagination | ≥20 | Warm up 3 discard runs |
| Encrypted open | ≥20 | Same room class each run |
| Startup-to-ready | ≥10 (cold) | Fully quit between cold runs |
| Reconnect | ≥10 | Fixed offline duration (e.g. 15s) |
| Media open | ≥20 | Same attachment class |
| Idle CPU / memory / disk | ≥5 samples after settle | Document settle time |

Report **p50 and p95** with nearest-rank; include min/max/mean.

### Scenario scripts (operator)

1. **Startup-to-ready:** cold launch → time to interactive shell; repeat.
2. **Room switch:** from room A → room B (large cleartext); capture `first-stable-bottom.elapsedMs`.
3. **Initial timeline:** process restart, open large room first.
4. **Encrypted open:** open encrypted room with history; note decrypt vs UTD-stable.
5. **Pagination:** at top of loaded window, trigger backward page; duration start→complete.
6. **Reconnect:** disable network 15s while SYNCING; re-enable; time to healthy settle.
7. **Media:** open image in-room; time to first paint.
8. **Idle CPU:** after settle, sample CPU 60s.
9. **Memory large-account:** execute standard scenario; record RSS + heap.
10. **Disk growth:** measure store size before/after fixed scenario.

### Blockers for this agent session

| Blocker | Impact |
| --- | --- |
| No automated signed-in UI driver in task scope | All live UX metrics residual |
| Stop condition forbids multi-hour Synapse soaks / large UI automation frameworks | Live multi-run not executed |
| Linux workstation not this host | Linux live residual |
| Secrets / multi-account test matrix not injected into agent | Cannot honestly fill p50/p95 |

---

## Live scenario protocol (Linux)

Mirror macOS with these differences:

1. Package/install path: Arch/`pacman` or local bundle per project packaging docs; record DE (GNOME/KDE/etc.).
2. WebView: WebKitGTK — use inspector if available; still prefer native `desktop_log_path` diagnostics.
3. CPU/memory: `ps`, `top`, or desktop system monitor on the `synara` process.
4. Network offline: `nmcli` / interface down for reconnect tests (restorable).
5. Disk: app data under XDG paths — record path **class** only.
6. Preflight: same `npm run check:production-smoke` family from [`../production-smoke-checklist.md`](../production-smoke-checklist.md) Linux section.

**Status on this task:** procedure only — **not executed**.

---

## Phase 13 budget linkage

From plan Phase 13 (required performance budgets relative to **this** Phase 0 baseline):

| Plan budget | Metric IDs | Rule |
| --- | --- | --- |
| startup-to-ready p95 | `M-STARTUP-READY` | ≤ baseline p95 × **1.10** |
| room-switch-to-first-stable-render p95 | `M-ROOM-SWITCH-STABLE` | ≤ baseline p95 × **1.10** |
| initial encrypted timeline p95 | `M-TIMELINE-OPEN-ENCRYPTED` | ≤ baseline p95 × **1.10** |
| pagination p95 | `M-PAGINATION-BACKWARD` | ≤ baseline p95 × **1.10** |
| idle CPU after settled sync | `M-IDLE-CPU` | within Phase 0 noise band (provisional ±2 pp until live samples refine) |
| memory after standard large-account | `M-MEMORY-LARGE-ACCOUNT` | ≤ baseline × **1.15** |
| no unbounded growth | `M-DISK-GROWTH`, memory, channels, media cache | qualitative + scenario deltas |
| timeline scroll/anchor acceptance | diagnostics + smoke | at least as good as baseline |

Plan also measures in P0.6 (mapped here): reconnect → `M-RECONNECT-SETTLE`; media → `M-MEDIA-OPEN`; disk → `M-DISK-GROWTH`.

Any exceeded budget at Phase 13 requires written explanation and explicit approval; mean improvement does not excuse severe p95/p99 regression.

---

## Residuals checklist

| ID | State | Blocker |
| --- | --- | --- |
| `M-STARTUP-READY` | `residual-pending-live` | signed-in multi-run not executed |
| `M-ROOM-SWITCH-STABLE` | `residual-pending-live` | same |
| `M-TIMELINE-OPEN-INITIAL` | `residual-pending-live` | same |
| `M-TIMELINE-OPEN-ENCRYPTED` | `residual-pending-live` | same |
| `M-PAGINATION-BACKWARD` | `residual-pending-live` | same |
| `M-RECONNECT-SETTLE` | `residual-pending-live` | same |
| `M-MEDIA-OPEN` | `residual-pending-live` | same |
| `M-IDLE-CPU` | `residual-pending-live` | same |
| `M-MEMORY-LARGE-ACCOUNT` | `residual-pending-live` | same |
| `M-DISK-GROWTH` | `residual-pending-live` | same |
| Linux all live metrics | `residual-pending-live` | no Linux host this session |
| `M-TIMELINE-MAP-10K` / `50K` | **measured** | — |

---

## Reviewer re-run commands

```sh
git rev-parse HEAD
# expect work on matrix-rust/p0.6-performance-baseline based on a2d288b…

cd synara && npm run test:timeline-performance

cd ..
node scripts/matrix-rust-p0.6-baseline-harness.mjs --iterations 50
node --test scripts/__tests__/matrix-rust-p0.6-baseline-harness.test.mjs

# confirm docs twin
test -f docs/matrix-rust-sdk/performance-baseline.md
test -f docs/matrix-rust-sdk/performance-baseline.json
```

Reviewer should verify:

1. No product code / production dependency changes.
2. No fabricated live p50/p95 in MD/JSON.
3. FR-7.8–7.11 findings untouched; no re-promotion of FR-7.9-011.
4. JSON `merged: false` until integration merge.
5. Automated numbers re-run within the same budget class (sub-ms–low-ms mapping; all samples ≪ 25 ms).

---

## Related assets

- [`../../synara/docs/synara-performance.md`](../../synara/docs/synara-performance.md) — timeline strategy + debug overlay
- [`../../synara/scripts/run-timeline-performance-harness.mjs`](../../synara/scripts/run-timeline-performance-harness.mjs) — product harness
- [`../../scripts/matrix-rust-p0.6-baseline-harness.mjs`](../../scripts/matrix-rust-p0.6-baseline-harness.mjs) — multi-iteration aggregator
- [`../timeline-diagnostics.md`](../timeline-diagnostics.md) — privacy-safe timeline records
- [`../production-smoke-checklist.md`](../production-smoke-checklist.md) — human smoke surface
- [`implementation-handoff.md`](implementation-handoff.md) — execution handoff
