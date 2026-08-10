# P0.4 — Swift/Rust version provenance

> **Generated evidence artifact for Phase 0 task P0.4.** Machine-readable twin:
> [`swift-rust-version-provenance.json`](swift-rust-version-provenance.json).
>
> Docs-only. Does **not** change iOS package pins, desktop crate pins, or any
> production Matrix code.

| Field | Value |
| ----- | ----- |
| Task ID | **P0.4** |
| Generation date | **2026-07-24** |
| Base integration tip at generation | `feature/matrix-rust-sdk-full-replacement` @ `c76f4ff7cb19e3e6d1536e9b8f7b8d269f374dcb` |
| Work branch at generation | `matrix-rust/p0.4-swift-rust-provenance` |
| Artifact / integration state | `landed` / `merged` |
| Strict acceptance / Phase 0 gate | `open` / `open` — see [`program-status.md`](program-status.md) |

## Goals

1. Determine the Matrix Rust SDK **source revision** embedded in /
   shipped by `matrix-rust-components-swift` release **`26.06.06`**
   (SwiftPM also normalizes as `26.6.6`).
2. Decide and document the **desktop/iOS version-alignment target** for this
   program.
3. Produce reproducible Markdown + JSON evidence.

## Non-goals

- No iOS package version bump (stay on `26.06.06` / `ec3b2161…`).
- No desktop crate pin change (stay on `matrix-sdk` / `matrix-sdk-ui` `=0.18.0` /
  commit `1c44fb66…`).
- No production app code, `Cargo.toml` (src-tauri), `package.json`, or
  `matrix-js-sdk` usage changes.
- No dual-backend, SDK selector, or raw `/_matrix/` HTTP paths.
- No re-opening of completed P0.2 FR rows (7.8–7.11); no re-promotion of
  FR-7.9-011.
- No downloading XCFramework binaries into the repository; no building
  XCFramework from source in this task.
- Alignment is **protocol/behavior version strategy**, not sharing the same FFI
  package: desktop embeds Rust crates directly; iOS stays on Swift FFI
  components.

---

## 1. Local iOS package pins (verified)

| Source | Field | Value |
| ------ | ----- | ----- |
| Xcode package ref | `synara-ios/Synara.xcodeproj/project.pbxproj` | `repositoryURL` = `https://github.com/matrix-org/matrix-rust-components-swift.git`; `kind` = `exactVersion`; `version` = **`26.06.06`** |
| XcodeGen | `synara-ios/project.yml` | `exactVersion: 26.06.06` |
| App Package.resolved | `synara-ios/Synara.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved` | `identity` = `matrix-rust-components-swift`; `version` = **`26.6.6`**; `revision` = **`ec3b2161ba371a13609e7181077d2f3baef188f5`** |
| Spike Package.swift | `synara-ios/spikes/matrix-sdk-probe/Package.swift` | `.package(..., exact: "26.06.06")` |
| Spike Package.resolved | `synara-ios/spikes/matrix-sdk-probe/Package.resolved` | same `26.6.6` / `ec3b2161ba371a13609e7181077d2f3baef188f5` |

Notes:

- SwiftPM **normalizes** CalVer `26.06.06` → resolved version string `26.6.6`.
- App and spike pins are **identical**.

### Validation (local)

```sh
# From repository root
grep -A6 'matrix-rust-components-swift' \
  synara-ios/Synara.xcodeproj/project.pbxproj

python3 -c "
import json
for path in [
  'synara-ios/Synara.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved',
  'synara-ios/spikes/matrix-sdk-probe/Package.resolved',
]:
  d=json.load(open(path))
  print(path, [p for p in d['pins'] if 'matrix-rust' in p['identity']])
"

grep -n '26.06.06\|matrix-rust-components-swift' \
  synara-ios/project.yml \
  synara-ios/spikes/matrix-sdk-probe/Package.swift
```

---

## 2. Upstream Swift package at tag `26.06.06`

| Field | Value |
| ----- | ----- |
| Package repository | <https://github.com/matrix-org/matrix-rust-components-swift> |
| Release tag | `26.06.06` |
| Release URL | <https://github.com/matrix-org/matrix-rust-components-swift/releases/tag/26.06.06> |
| Tag / package commit | `ec3b2161ba371a13609e7181077d2f3baef188f5` |
| Package commit URL | <https://github.com/matrix-org/matrix-rust-components-swift/commit/ec3b2161ba371a13609e7181077d2f3baef188f5> |
| Published (UTC) | `2026-06-06T07:51:21Z` |
| Binary asset | `MatrixSDKFFI.xcframework.zip` |
| Binary download | <https://github.com/matrix-org/matrix-rust-components-swift/releases/download/26.06.06/MatrixSDKFFI.xcframework.zip> |
| Package.swift checksum | `e7f3a2abe6a3d516fc92ef04f9e04b60e45dfce39af98f36ffc4ce28d26f8a18` |
| Release asset digest | `sha256:e7f3a2abe6a3d516fc92ef04f9e04b60e45dfce39af98f36ffc4ce28d26f8a18` (matches Package.swift) |

### Package.swift (binary target)

At revision `ec3b2161…`, `Package.swift` declares:

- `version = "26.06.06"`
- `url = "…/releases/download/26.06.06/MatrixSDKFFI.xcframework.zip"`
- `checksum = "e7f3a2abe6a3d516fc92ef04f9e04b60e45dfce39af98f36ffc4ce28d26f8a18"`
- Binary target name: `MatrixSDKFFI`; product library: `MatrixRustSDK`

The package is a thin SwiftPM wrapper around a **pre-built** XCFramework; it does
not vendor the Rust source tree.

### Release tooling (how the embedded SHA is recorded)

Upstream `Tools/Release/Sources/Release.swift` builds from a sibling
`matrix-rust-sdk` checkout, captures `git commitHash`, and commits the package
bump with message form:

```text
Bump to version <calver> (matrix-rust-sdk/<branch> <commitHash>)
```

That is the primary provenance channel for CalVer → Rust SDK commit mapping.

---

## 3. Embedded Matrix Rust SDK revision (proven)

| Field | Value |
| ----- | ----- |
| Embedded git commit | **`1c44fb66214667c6d00acaf72ab592493653708b`** |
| Confidence | **High** (multiple independent primary sources) |
| matrix-rust-sdk tree | <https://github.com/matrix-org/matrix-rust-sdk/tree/1c44fb66214667c6d00acaf72ab592493653708b> |
| matrix-rust-sdk commit | <https://github.com/matrix-org/matrix-rust-sdk/commit/1c44fb66214667c6d00acaf72ab592493653708b> |
| Corresponding crates release tag | **`matrix-sdk-0.18.0`** |
| Tag peels to | `1c44fb66214667c6d00acaf72ab592493653708b` (annotated tag object → commit) |
| Crate versions at that tag | `matrix-sdk` / `matrix-sdk-ui` **0.18.0** (see P0.3a) |
| Commit date (UTC) | `2026-06-02T11:29:09Z` (`chore: Release matrix-sdk version 0.18.0`) |
| Swift package CalVer lag | Package published `2026-06-06` (~4 days after the Rust release commit) |

### Primary evidence (ordered)

| # | Claim | Method | Result |
| - | ----- | ------ | ------ |
| E1 | Package bump commit names embedded Rust SHA | `git log -1` / GitHub commit API on `ec3b2161…` | Message: `Bump to version 26.06.06 (matrix-rust-sdk/HEAD 1c44fb66214667c6d00acaf72ab592493653708b)` |
| E2 | GitHub release body is the Rust tree URL | GitHub Releases API `…/releases/tags/26.06.06` | `body` = `https://github.com/matrix-org/matrix-rust-sdk/tree/1c44fb66214667c6d00acaf72ab592493653708b` |
| E3 | Release script embeds `commitHash` in commit message | Inspect `Tools/Release/Sources/Release.swift` at tag `26.06.06` | `git.commitHash` written into bump commit message |
| E4 | `matrix-sdk-0.18.0` is the same commit | GitHub git API: tag → annotated tag → commit | Target commit `1c44fb66214667c6d00acaf72ab592493653708b` |
| E5 | Desktop P0.3a pin matches | [`0.18.0-source-provenance.md`](0.18.0-source-provenance.md) | Desktop pin = `1c44fb66…` / `matrix-sdk-0.18.0` |
| E6 | Local iOS resolved revision matches release tag commit | Package.resolved vs `git rev-parse` at tag `26.06.06` | Both `ec3b2161ba371a13609e7181077d2f3baef188f5` |
| E7 | XCFramework checksum consistency | Package.swift `checksum` vs release asset `digest` | Both `e7f3a2abe6a3d516fc92ef04f9e04b60e45dfce39af98f36ffc4ce28d26f8a18` |

**No XCFramework binary download or string-scan was required.** The embedded
commit is published explicitly in the package commit message and release body.
If those ever disagreed, a secondary method would be temp-dir download of the
zip and careful inspection of build metadata inside the XCFramework — not done
here because primary evidence already converges.

### Validation (upstream, re-runnable)

```sh
# 1) Clone the Swift package at the iOS-resolved tag
git clone --depth 1 --branch 26.06.06 \
  https://github.com/matrix-org/matrix-rust-components-swift components-swift
cd components-swift
git rev-parse HEAD
# expect: ec3b2161ba371a13609e7181077d2f3baef188f5

git log -1 --format=%B
# expect message containing:
#   1c44fb66214667c6d00acaf72ab592493653708b

# 2) Release metadata
curl -sL \
  https://api.github.com/repos/matrix-org/matrix-rust-components-swift/releases/tags/26.06.06 \
  | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['body']); print(d['published_at'])"
# expect body tree URL ending in 1c44fb66214667c6d00acaf72ab592493653708b

# 3) Confirm matrix-sdk-0.18.0 peels to the same commit
git clone --depth 1 --branch matrix-sdk-0.18.0 \
  https://github.com/matrix-org/matrix-rust-sdk matrix-rust-sdk
cd matrix-rust-sdk && git rev-parse HEAD
# expect: 1c44fb66214667c6d00acaf72ab592493653708b
```

---

## 4. Desktop pin recap (P0.3a — locked)

Authoritative artifact: [`0.18.0-source-provenance.md`](0.18.0-source-provenance.md)
/ [`0.18.0-source-provenance.json`](0.18.0-source-provenance.json).

| Field | Value |
| ----- | ----- |
| Repository | <https://github.com/matrix-org/matrix-rust-sdk> |
| Release tag | `matrix-sdk-0.18.0` |
| Pinned commit | `1c44fb66214667c6d00acaf72ab592493653708b` |
| Crates | `matrix-sdk` `=0.18.0`, `matrix-sdk-ui` `=0.18.0` |
| Packaging form | crates.io / Cargo (desktop embeds crates **directly**, not uniffi) |
| Upstream MSRV / edition | Rust `1.93` / edition `2024` |
| Probe path | `probes/matrix-rust-sdk-0.18/` (evidence harness only) |

Program constraint (unchanged by P0.4): desktop production remains pinned to
exact crates `matrix-sdk` / `matrix-sdk-ui` **`=0.18.0`** at commit
`1c44fb66…` for Phase 1+ as already decided by the plan and P0.3.

---

## 5. Side-by-side comparison

| Dimension | iOS (shipped today) | Desktop (program pin) | Match? |
| --------- | ------------------- | --------------------- | ------ |
| Distribution | `matrix-rust-components-swift` CalVer **`26.06.06`** | crates.io **`matrix-sdk` / `matrix-sdk-ui` `0.18.0`** | Different packaging by design |
| Package / lock revision | Swift package commit **`ec3b2161…`** | crates.io package checksums (see P0.3a lockfile) | N/A (different systems) |
| **Embedded / source Matrix Rust SDK git commit** | **`1c44fb66214667c6d00acaf72ab592493653708b`** | **`1c44fb66214667c6d00acaf72ab592493653708b`** | **Yes — identical** |
| Crate series at that commit | `matrix-sdk` / `matrix-sdk-ui` **0.18.0** (release commit) | **0.18.0** | **Yes** |
| FFI / UniFFI | Yes (XCFramework + generated Swift bindings) | No (native Rust crates in Tauri host) | Different binding layer |
| Dual-backend | No (iOS already SDK-primary via Swift FFI) | Forbidden (full replacement of `matrix-js-sdk`) | Program-consistent |
| Element X reference context | Element X iOS has used newer components (e.g. **`26.07.15`**, see `docs/desktop-foundation-reference-audit.md`) | N/A | **Context only — not a Synara pin** |

### Date relationship

| Event | UTC date |
| ----- | -------- |
| Rust release commit `1c44fb66…` (`matrix-sdk-0.18.0`) | 2026-06-02 |
| Swift components release `26.06.06` / package commit `ec3b2161…` | 2026-06-06 |
| Element X iOS reference note (`26.07.15`) | ~2026-07 (audit snapshot 2026-07-21) — **not adopted** |

---

## 6. Version-alignment target (decision)

### Decision

**Classification: (A) exact same git commit now.**

| Key | Value |
| --- | ----- |
| `decision` | `exact_same_git_commit` |
| `same_commit` | **`true`** |
| `same_crate_series` | **`true`** (`matrix-sdk` / `matrix-sdk-ui` **0.18.0**) |
| Shared commit | `1c44fb66214667c6d00acaf72ab592493653708b` |
| Desktop pin (locked) | crates `=0.18.0` @ `1c44fb66…` — **unchanged** |
| iOS pin (locked for this task) | components-swift **`26.06.06`** @ `ec3b2161…` — **unchanged** |

### Rationale

1. **Primary-source proof** shows iOS `26.06.06` was built from the same
   `matrix-rust-sdk` commit that tags as `matrix-sdk-0.18.0`, which is already
   the desktop program pin (P0.3a / plan).
2. Alignment for this program is **source-revision / protocol-behavior parity**,
   not identical packaging: desktop embeds crates; iOS consumes the UniFFI
   XCFramework. That split is intentional and **does not** create a dual-backend
   on desktop.
3. **Do not bump** iOS to follow Element X (`26.07.15` etc.) as part of P0.4.
   Staying on proven `26.06.06` preserves the exact-commit alignment and avoids
   unvetted binding churn during Phase 0/1 foundation work.
4. If a later approved iOS components bump moves ahead of desktop `0.18.0`, the
   program would reclassify toward intentional divergence with a re-run of this
   provenance task and shared semantic fixtures as interim parity method — that
   is a **future** gate, not the current state.

### Residual risks / follow-up gates

| ID | Risk / gate | Mitigation |
| -- | ----------- | ---------- |
| R1 | Future iOS CalVer bump embeds a newer Rust commit than desktop `0.18.0` | Re-run P0.4-style provenance before accepting any iOS components bump; decide desktop crate bump vs intentional temporary divergence |
| R2 | Element X / ecosystem pressure to adopt `26.07.x+` bindings | Context only until a scoped iOS upgrade task with binding-diff review (see iOS alignment audit 26.5.13→26.6.6 style) |
| R3 | FFI surface vs crates public API drift even at same commit | Same source commit minimizes semantic drift; still treat UniFFI and crates as distinct integration surfaces in cutover tests |
| R4 | XCFramework binary not re-hashed byte-for-byte in this task | Checksum match Package.swift ↔ GitHub asset digest is recorded; optional offline zip verify is a reviewer convenience, not a missing primary proof |
| G1 | Keep desktop `=0.18.0` until a program-approved crate bump | Enforced by plan + P0.3 pins |
| G2 | Keep iOS `26.06.06` until an approved iOS package upgrade task | Enforced by this decision; P0.4 does not bump |
| G3 | Cross-platform parity tests should prefer shared semantic fixtures over assuming identical FFI APIs | Especially if platforms ever diverge in CalVer vs crate pin |

---

## 7. Relationship to other Phase 0 work

| Task | Relationship |
| ---- | ------------ |
| P0.3 / P0.3a | Desktop commit `1c44fb66…` already locked; this task proves iOS embeds **the same** commit |
| P0.2 FR rows | Unchanged; FR-7.8-009 iOS pusher path remains on current Swift bindings |
| P0.5 | Toolchain follow-up at generation time; subsequently landed with residuals |
| P0.6 / P0.7 | Perf baseline / migration UX — no pin changes here |

---

## 8. Reviewer checklist

- [ ] Local pbxproj / Package.resolved / spike pins match tables in §1.
- [ ] Upstream tag `26.06.06` HEAD is `ec3b2161…`.
- [ ] Bump commit message and release body both cite `1c44fb66…`.
- [ ] `matrix-sdk-0.18.0` peels to `1c44fb66…`.
- [ ] Alignment decision is **exact same commit**; no pin files changed.
- [ ] Diff is docs-only under `docs/matrix-rust-sdk/`.
- [ ] JSON twin stays synchronized with this Markdown.

---

## Document control

| Item | Value |
| ---- | ----- |
| Authors | Bounded writer session (P0.4) |
| Machine twin | `docs/matrix-rust-sdk/swift-rust-version-provenance.json` |
| Related | `0.18.0-source-provenance.*`, `README.md`, plan §P0.4 |
