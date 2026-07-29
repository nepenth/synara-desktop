# V-CRYPTO.7 — native device list, trust, and actions

| Field      | Value                                                                            |
| ---------- | -------------------------------------------------------------------------------- |
| Date       | 2026-07-29                                                                       |
| Base       | integration commit `05e3f64dc9b6e1b38dcc181abe2090370e85a5e3` (V-CRYPTO.6, #235) |
| Candidate  | PR [#236](https://github.com/nepenth/synara-desktop/pull/236); reviewed code head `7df8abe` |
| Next slice | V-AUTH.1                                                                         |

## Retained operating path

The settings UI now has one owner for this capability:

```text
device settings
  → SDK-neutral Tauri commands and bounded DTOs
  → managed native session
  → matrix-sdk Client::devices / Encryption::get_user_devices
```

`Client::devices` is the authoritative list. Trust is joined by device ID from
`Encryption::get_user_devices` and
`Device::is_verified_with_cross_signing`. The current device sorts first; other
devices sort by `lastSeenTs` descending with a device-ID tiebreaker. The bounded
DTO retains display name, `lastSeenIp`, `lastSeenTs`, trust, and whether the row
is the current device. It deliberately contains no device keys, tokens, recovery
material, raw SDK errors, or raw UIAA objects. The former Ed25519-key display was
removed because device keys are outside this IPC contract.

Reads are demand-driven on view mount/refocus. The session-owned supported
`Encryption::devices_stream` emits a generation-scoped refresh trigger only
when the current user's documented `new` or `changed` maps contain entries. It
does not poll, inspect raw sync, reach into `OlmMachine`, or treat undocumented
empty/deletion wakeups as authoritative. Rename and delete return an
authoritative server readback, which the UI installs directly.

## Rename and other-device deletion

Rename accepts the device ID selected from the authoritative rendered snapshot,
lets the SDK/server validate it, and returns a fresh authoritative snapshot
after the native rename. It does not add a redundant pre-action network read.

Delete is purpose-specific to selected non-current devices. Rust validates the
selection against an authoritative snapshot and owns a pending operation with a
mandatory operation ID, session generation, selected device IDs, and opaque UIAA
session. Password input crosses IPC once, is immediately wrapped in a zeroizing
buffer, and is never retained in pending state. Cancel, password continuation,
and SSO continuation carry the challenge's session generation. Rust checks that
generation against both the active session and pending operation before checking
the operation ID, so an old-session callback cannot affect a reused operation ID.
Leaving the device surface during UIAA sends a best-effort cancellation for the
retained operation ID, so an opaque auth session is not left natively pending
without a reachable UI owner.

For multi-stage UIAA, Rust considers only flows whose remaining stages are all
Password or SSO. It exposes the current stage for one viable method, preferring
Password across alternative flows, and advances using the server's authoritative
`completed` stages. The challenge exposes authentication as one scalar method,
not an array. The WebView receives no raw UIAA parameters, session, or error. The
one exception is the SDK/Ruma-generated SSO fallback URL, which
necessarily embeds the opaque session. The SSO popup completion path accepts
`authDone` only from the exact fallback origin and exact child window; the manual
Continue action uses the same native acknowledgement command. UIAA authentication
failure remains distinct from terminal delete failure.

Delete succeeds only after the SDK call succeeds and a fresh `Client::devices`
readback proves every selected device absent. The OIDC account-dashboard
`sessionEnd` route and current-device native logout/verification routes remain
unchanged.

The frontend device query is keyed by the existing native bootstrap session
generation. A request that resolves after a native session transition therefore
can update only its old generation's cache entry and cannot become the next
account's visible device list.

## Superseded owners deleted

- `ActionUIA.tsx`, its device-delete-only Password and SSO stages, and the dead
  device-delete `useUIAMatrixError` path. Registration/reset UIA remains.
- `DeviceVerificationStatus.ts`, `useDeviceVerificationStatus.ts`, and
  `useUserTrustStatusChange.ts`.
- `platform/device.ts` and its repair-only platform tests/exports.
- The old Rust `DeviceIndex` harness (`devices/index.rs`, `devices/error.rs`, and
  `devices/tests.rs`).
- Device-page `CryptoApi`, Matrix device model, device listener, and polling
  ownership. Device settings now consume only the native DTO boundary.

## Accounting and evidence

- Direct desktop-runtime inventory: **218 files / 273 import declarations →
  212 / 265**.
- Production importer files: **207 → 201**.
- Repository-wide importer files: **221 → 215**.
- Focused validation passes: `cargo fmt`, one-job `cargo check`,
  `cargo test matrix::devices` (2 passed), scoped Prettier/ESLint, frontend and
  modernization typechecks, inventory validation, Matrix Rust guardrails, JSON
  parse, and `git diff --check`.
- Live multi-session/UI proof is not claimed by this active PR.

This PR candidate closes the V-CRYPTO device-list/trust/action owner. It does not
claim a global crypto phase gate or replace generic registration/reset UIA.
