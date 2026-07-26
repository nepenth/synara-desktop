# P0.5 — Toolchain Compatibility Report

| Field | Value |
|-------|--------|
| **Task ID** | P0.5 |
| **Date** | 2026-07-24 |
| **Base integration tip at generation** | `feature/matrix-rust-sdk-full-replacement` @ `4d318da9fdf0689b9cb085a6387fbf338fd67fd2` |
| **Work branch at generation** | `matrix-rust/p0.5-toolchain-compatibility` (docs + probes only) |
| **Desktop SDK pin (program)** | matrix-sdk / matrix-sdk-ui `0.18.0` (git commit `1c44fb66214667c6d00acaf72ab592493653708b`) |
| **Machine twin** | [`toolchain-compatibility-report.json`](toolchain-compatibility-report.json) |
| **Artifact / integration state** | `landed` / `merged` |
| **Strict acceptance / Phase 0 gate** | `open` / `open` — see [`program-status.md`](program-status.md) |

## Executive verdict

**`pass-with-residuals`**

Rust **1.93**, edition **2024** dependency graphs, **Tauri 2.11**, and
**matrix-sdk / matrix-sdk-ui =0.18.0** coexist successfully in isolated probes.
The production Tauri shell (`src-tauri`) still compiles under host Rust 1.93
**without** matrix-sdk. Platform packaging (universal macOS, Linux, signing /
notarization) is analyzed with explicit evidence levels; full product release
and notarization were **not** re-run with an SDK-linked binary (production does
not yet depend on matrix-sdk — by design until Phase 1).

**Historical task result:** P1.1 implementation was unblocked. This did not
close the Phase 0 strict gate; Linux and full product packaging evidence remain
open under the 2026-07-25 review.

---

## A. Environment inventory

| Item | Value | Evidence |
|------|--------|----------|
| Host OS | macOS arm64 | local |
| Default host triple | `aarch64-apple-darwin` | `rustup show` |
| rustc | `1.93.0 (254b59607 2026-01-19)` | `rustc --version` |
| cargo | `1.93.0 (083ac5135 2025-12-15)` | `cargo --version` |
| Active toolchain | `stable-aarch64-apple-darwin` | `rustup show` |
| Installed targets (at report time) | `aarch64-apple-darwin`, `aarch64-apple-ios-sim`, `x86_64-apple-darwin` (added during P0.5 for universal probe) | `rustup target list --installed` |
| Linux targets on host | none | residual for local Linux |

## B. Production `src-tauri` posture (read-only)

| Item | Value |
|------|--------|
| Path | `src-tauri/Cargo.toml` |
| Package | `synara` `1.2.59` |
| edition | `"2021"` |
| rust-version (MSRV) | `"1.77.2"` |
| tauri | `2.11` (lock resolves `2.11.2` in production lockfile) |
| Plugins | 2.x (`localhost`, `window-state`, `clipboard-manager`, `notification`, `opener`, `global-shortcut`, `updater` 2.10.1, `process`) |
| matrix-sdk / matrix-sdk-ui | **not present** (intentional until Phase 1) |
| P0.5 modifications | **none** — production `src-tauri` was not modified |

**MSRV gap (program prerequisite):** production declares `1.77.2`; matrix-sdk
0.18 probes require **1.93**. Permanent pin is **P1.1**, not this spike.

## C. CI posture (workflow analysis — no workflow edits in P0.5)

| Workflow | Runners | Toolchain action | Targets / notes | Cargo cache |
|----------|---------|------------------|-----------------|-------------|
| `ci.yml` | `ubuntu-22.04` (+ iOS job on `macos-26`) | `dtolnay/rust-toolchain@4cda84d5c5c54efe2404f9d843567869ab1699d4` (# stable 2026-07-22) | host only for desktop shell | **none** (`Swatinem/rust-cache` not used) |
| `desktop-package-smoke.yml` | `ubuntu-22.04`, macOS | same dtolnay pin | Linux deb + macOS app smoke | none |
| `macos-signed-build.yml` | `macos-15` | same + `targets: aarch64-apple-darwin,x86_64-apple-darwin` | `universal-apple-darwin` DMG, codesign, notarytool, stapler, spctl | none |
| `release.yml` | ubuntu-22.04 + macos-15 | same; universal targets on macOS | cargo check/test; signed universal + notarize | none |

**Triggers (coverage residual):** `ci.yml` runs on `push`/`PR` to `main` and
`release/**` only — **not** on the integration branch
`feature/matrix-rust-sdk-full-replacement` by default. Phase 1 work must either
extend triggers or rely on PR-to-main/release paths.

**Linux deps (already present in CI):** `libwebkit2gtk-4.1-dev`,
`libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`.

**Signing / notarization (orthogonal to toolchain if binary builds):**
certificate import → `tauri build --target universal-apple-darwin` →
`codesign --verify` → `xcrun notarytool submit` → `stapler` → `spctl`.
Not re-exercised in this spike with an SDK-linked product binary (no production
SDK dep; no signing secrets in agent).

---

## D. Proof results

### D.1 Isolated matrix-sdk 0.18 probe (edition 2024 / rust-version 1.93)

| Field | Result |
|-------|--------|
| Path | `probes/matrix-rust-sdk-0.18/` |
| Deps | `matrix-sdk = "=0.18.0"`, `matrix-sdk-ui = "=0.18.0"`, `default-features = false` |
| Command | `cargo check --locked` |
| Host rustc | 1.93.0 |
| Result | **PASS** (exit 0, ~1m 09s cold-ish) |
| Note | Confirms prior P0.3 lockfile still checks on current stable 1.93 |

### D.2 Production Tauri shell under Rust 1.93 (no SDK)

| Field | Result |
|-------|--------|
| Path | `src-tauri/` |
| Command | `cargo check --locked` |
| Host rustc | 1.93.0 |
| Result | **PASS** (exit 0, ~1m 05s) |
| Note | Product edition remains 2021 / MSRV 1.77.2; compiles on 1.93 without matrix-sdk |

### D.3 Tauri 2 + matrix-sdk 0.18 coexistence probe

| Field | Result |
|-------|--------|
| Path | `probes/tauri-matrix-sdk-compat/` (**new**, isolated) |
| edition / rust-version | `2024` / `1.93` |
| Deps | `tauri = "2.11"` (defaults; lock `2.11.5`), `matrix-sdk = "=0.18.0"`, `matrix-sdk-ui = "=0.18.0"` (default-features false) |
| Commands | `cargo check --locked`; `cargo test --locked`; `cargo check --locked --target x86_64-apple-darwin` |
| Host aarch64 | **PASS** (check + 1 unit test) |
| x86_64-apple-darwin | **PASS** (`cargo check --locked --target x86_64-apple-darwin`, exit 0) |
| Feature unification | Direct matrix-sdk requests no default features; `matrix-sdk-ui` enables `matrix-sdk/e2e-encryption` transitively (same as API-shape probe). No resolution conflict with Tauri/reqwest/rustls observed. |
| Failures encountered | (1) `AppHandle` needs `Runtime` generic — fixed with `AppHandle<Wry>`. (2) `Wry` gated on `wry` feature — fixed by using Tauri default features (aligned with production defaults). |
| Not a full Tauri app | No frontend, no `tauri-build` app manifest, no product integration |

**Resolved coexistence lock highlights:**

| Crate | Coexistence probe | Production `src-tauri` |
|-------|-------------------|------------------------|
| tauri | 2.11.5 | 2.11.2 |
| matrix-sdk | 0.18.0 | — |
| matrix-sdk-ui | 0.18.0 | — |
| reqwest | 0.13.4 | 0.13.4 |
| wry | 0.55.1 | 0.55.1 |
| tokio | 1.53.1 | 1.52.3 |

Minor transitive version drift between probe and production lockfiles is
expected for isolated crates; Phase 1 will unify inside `src-tauri` when SDK is
pinned there.

### D.4 Disk / compile cost (local observation)

| Tree | Approx. `target/` size after checks |
|------|-------------------------------------|
| `probes/matrix-rust-sdk-0.18/target` | ~408M |
| `probes/tauri-matrix-sdk-compat/target` | ~903M (includes dual-target aarch64 + x86_64 artifacts) |
| `src-tauri/target` | ~1.1G |

**CI impact:** matrix-sdk + crypto/aws-lc + UI stack is heavy; without cargo
cache, Phase 1 CI jobs will recompile much of the graph every run. Recommend
`Swatinem/rust-cache` (or equivalent) with distinct keys for `src-tauri` vs
probes.

---

## E. Platform / packaging coexistence (evidence levels)

| Concern | Status | Evidence level | Notes |
|---------|--------|----------------|-------|
| macOS host aarch64 check/build | **Proven** | Local `cargo check` | Probes + src-tauri |
| macOS universal (x86_64 + aarch64) | **Partial proven** | Local: `rustup target add x86_64-apple-darwin` + coexistence `cargo check --target x86_64-apple-darwin` **PASS**; full `tauri build --target universal-apple-darwin` **not** run | CI already builds universal with dtolnay stable + both targets in `macos-signed-build.yml` / `release.yml` |
| Linux | **Not local** | Workflow analysis only | `ubuntu-22.04` + webkit deps + `cargo check/test` already in CI for shell; must re-prove with Rust 1.93 pin + matrix-sdk in Phase 1 CI |
| Signing / notarization | **Toolchain-orthogonal if binary builds** | Workflow analysis | codesign / notarytool / stapler / spctl present; residual: not re-run with SDK-linked product binary in P0.5 |
| Product edition 2021 vs SDK edition 2024 deps | **OK for coexistence** | Local probe | Cargo allows mixed editions across crates; product crate may stay 2021 until team chooses otherwise; MSRV must rise to 1.93 for SDK |

---

## F. Phase 1 CI / runner / cache recommendations

Concrete checklist for **P1.1** and early Phase 1 (document only — workflows
**not** edited in P0.5):

1. **Permanent toolchain pin (P1.1 owns):**
   - Add root or `src-tauri/rust-toolchain.toml` with `channel = "1.93.0"` (or
     exact patch team selects).
   - Bump production `src-tauri` `rust-version` from `1.77.2` → `1.93` (or
     matching pin).
2. **Workflows:** change `dtolnay/rust-toolchain` from floating `stable` (even
   if currently SHA-pinned) to **explicit** `toolchain: 1.93.0` (or
   rust-toolchain.toml driven) so CI cannot silently float past the program pin.
3. **macOS universal targets:** keep
   `targets: aarch64-apple-darwin,x86_64-apple-darwin` on signed/release jobs.
4. **Cargo cache:** add `Swatinem/rust-cache` (or `actions/cache` on
   `~/.cargo` + `target`) with keys scoped by:
   - OS / runner
   - lockfile hash (`src-tauri/Cargo.lock`, optionally probes)
   - toolchain version
5. **Integration-branch coverage:** extend `ci.yml` (or a dedicated workflow)
   to run on `feature/matrix-rust-sdk-full-replacement` and/or
   `matrix-rust/**` task branches once production SDK lands — currently limited
   to `main` / `release/**`.
6. **Probe CI (optional):** run `cargo check --locked` for
   `probes/matrix-rust-sdk-0.18` and `probes/tauri-matrix-sdk-compat` on PR
   paths that touch probes or SDK pins.
7. **Timeouts / disk:** expect longer first compile after SDK lands; 45–60 min
   job timeouts already used for package jobs are likely still OK with cache;
   watch GitHub runner disk (~14G free typical) when dual-target + release
   artifacts + matrix-sdk debug symbols accumulate.
8. **Linux webkit deps:** retain existing apt packages; re-prove
   `cargo check/test` + package smoke after SDK feature set is chosen
   (sqlite/native-tls/etc. may add system deps).

---

## G. Residual risks

1. **Linux not proven on this host** — only workflow analysis; Phase 1 CI must
   green-light SDK-linked builds on `ubuntu-22.04`.
2. **Full universal product bundle not rebuilt** — x86_64 `cargo check` on the
   coexistence probe passed; full `tauri build --target universal-apple-darwin`
   with SDK remains Phase 1 / release residual.
3. **Signing / notarization not re-exercised** with an SDK-linked binary.
4. **Production MSRV still 1.77.2** — compiles on 1.93 today, but declaring and
   enforcing 1.93 is P1.1; without it, older toolchains will fail once SDK is
   added.
5. **Edition mix** — product crate edition 2021 vs edition 2024 dependency
   crates is fine in Cargo; teams should not assume product must move to 2024
   for SDK compile (optional style choice).
6. **CI trigger gap** — integration and task branches may not run desktop
   `cargo check` until triggers expand or PRs target covered branches.
7. **No cargo cache today** — SDK will amplify cold-compile cost.
8. **Feature set still TBD** — coexistence used minimal matrix-sdk features +
   ui-driven e2e unification; production may enable `sqlite`, media, etc.,
   which can change compile time and native deps (re-check in Phase 1).
9. **Lockfile version drift** between isolated probes and production is normal
   until Phase 1 unifies dependencies in `src-tauri`.

---

## H. Non-goals / explicit statements

- Production `src-tauri/**` was **not** modified (no rust-version bump, no
  matrix-sdk dependency, no Tauri feature changes).
- Production matrix-sdk / matrix-sdk-ui deps were **not** added (plan: after
  P0.5/capability gates; production pin is Phase 1).
- `.github/workflows/**` were **not** edited; required CI changes are
  documented only.
- No dual-backend, SDK selector, or raw `/_matrix/` HTTP introduced in product
  code.
- No FR-7.8–7.11 re-open; no commit/push/PR/branch switch by this writer task.
- Permanent toolchain pin is **P1.1**, not permanentized in this spike beyond
  probe evidence.
- Full release, notarization, and Linux package smoke were **not** run.

---

## I. Validation commands for reviewers

```sh
# Host
rustc --version   # expect 1.93.x
cargo --version

# D.1 — matrix-sdk API-shape probe
cd probes/matrix-rust-sdk-0.18
cargo check --locked

# D.2 — production shell (no SDK) under 1.93
cd ../../src-tauri
cargo check --locked

# D.3 — Tauri + matrix-sdk coexistence
cd ../probes/tauri-matrix-sdk-compat
cargo check --locked
cargo test --locked

# Optional universal arch check (requires target)
rustup target add x86_64-apple-darwin
cargo check --locked --target x86_64-apple-darwin
```

---

## J. Artifact index

| Artifact | Role |
|----------|------|
| This file | Human-readable compatibility report |
| `toolchain-compatibility-report.json` | Machine-readable twin |
| `probes/matrix-rust-sdk-0.18/` | Existing 0.18 API-shape probe |
| `probes/tauri-matrix-sdk-compat/` | New coexistence probe + lockfile |
| `implementation-handoff.md` | Program state (P0.4 merged tip, P0.5 evidence) |
