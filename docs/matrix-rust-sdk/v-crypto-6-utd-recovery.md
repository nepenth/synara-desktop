# V-CRYPTO.6 — automatic UTD/history recovery

Status: **implemented candidate** in [#235](https://github.com/nepenth/synara-desktop/pull/235) on `matrix-rust/v-crypto-6-utd-recovery`.

## Retained operating path

1. The opened room uses the existing Rust-owned `matrix_sdk_ui::Timeline` in
   `NativeTimelineRegistry`.
2. Each native snapshot classifies SDK UTD causes into privacy-safe `pending`
   or currently `unavailable` state and reconciles the existing P5.10 `UtdIndex` plus P8.7
   `UtdRecoveryCoordinator` inside that registry.
3. Encrypted pagination inserts events through the SDK event cache. The SDK
   redecryptor owns both inserted UTD events and later room-key arrival; Synara
   does not scan or export Megolm session IDs.
4. The existing native timeline snapshot loop observes SDK item replacement and
   returns plaintext without a restart or user retry action. No key, session ID,
   ciphertext, or raw SDK error crosses IPC.
5. The banner reports pending/unavailable/recovered counts and opens the existing
   Devices recovery settings, which retain the native secret-storage and backup
   owners. There is deliberately no Retry button.
6. Pins and inbox rows that previously depended on a JS `Decrypted` listener
   use a bounded event-focused timeline in the same registry and consume the
   same safe projected item. Successful readback becomes a safe text message;
   unavailable state uses the existing `m.bad.encrypted` presentation while the
   live focused timeline keeps checking for later keys. Successful text readback
   stops polling; media URLs/raw content are not projected.

## Deleted superseded ownership

- `room.ts` `decryptAllTimelineEvent` and its `CryptoBackend` import.
- `RoomTimeline` encrypted-pagination JS decrypt call.
- `useRoomEvent` JS `attemptDecryption`, `CryptoBackend`, and `await-to-js`.
- `EncryptedContent.tsx`, its barrel export, and its three wrappers. The
  replacement adapter owns no Matrix JS decryption listener.

The unrelated room-activity and Element Call decrypted-event listeners remain.
Notifications, calls, room-list, and V-CRYPTO.7 device/trust ownership are not
otherwise migrated by this slice.

## Accounting

- Capability owners deleted: one JS bulk retry helper, one JS per-event decrypt
  path, one JS decrypted-event listener component, and one pagination call.
- Generated desktop inventory: **207 production + 11 test import files before
  and after**. The zero global file delta is honest: the deleted JS listener
  importer is replaced by one bounded MatrixEvent presentation adapter, while
  the crypto methods/deep imports/listener are gone.
- Direct desktop-runtime accounting remains **218 files** and drops from
  **275 → 273 import lines**.
- No raw `/_matrix` runtime route or backend selector was added.

## Verification

- `cargo test --lib matrix::timeline::live::tests` — 6 passed.
- `cargo check --lib` — passed.
- `npm --prefix synara run typecheck:modernization` — passed.
- `npm run check:matrix-sdk-usage` — passed.
- Focused Rust tests prove pending → automatic plaintext reconciliation,
  cause-to-current-availability mapping, placeholder safety, and event-readback IPC privacy.

Live two-client Synapse/UI proof is **not confirmed** for this merged vertical. The
existing acceptance scenario remains the runtime gate; compile/unit evidence is
not represented as live recovery proof.
