# Device verification local proof

The production goal is **current-device verification**, not merely local trust
between two device rows. The actor starts on an unverified Synara session while
at least one already cross-signed session is online. Settings reads
`Encryption::verification_state()` and
`Encryption::has_devices_to_verify_against()` through the shared Core snapshot,
enables **Verify This Device** only when the SDK reports an eligible authority,
and starts an own-identity request by passing no device ID. The SDK own identity
selects the eligible authority. Rust owns request acceptance, SAS start/accept,
and the cryptographic state machine. The UI must present one coherent SDK SAS
snapshot and the human owns the match/mismatch decision; Rust sends confirmation
only after that explicit decision. Completion is reached only when a new device
snapshot reads `own_verification == Verified`.

An explicit Account session-row device ID is a different, supported direct-peer
operation. It is useful for exercising SAS transport, but its peer row becoming
`Verified` is not proof that the current device was cross-signed.

## Automated current-device authority proof

The ignored Rust harness exercises the complete product-owner route against an
authorized test account:

```sh
cargo test -p synara-core --lib \
  live_own_device_verification_is_authoritative_and_durable \
  -- --ignored --nocapture
```

It persists one responder crypto store under
`target/live-own-device-verification/<opaque-account-tag>` (override the parent
directory with `SYNARA_LIVE_VERIFICATION_STORE_ROOT`). On its first run, and
only when the account has no published cross-signing identity, it completes the
SDK password-UIA bootstrap using `SYNARA_LIVE_PASSWORD`. Later runs log the same
device back into that store and reuse its private cross-signing identity. The
only emitted checkpoints are coarse, static labels; credentials, Matrix IDs,
device IDs, SAS values, tokens, keys, and raw SDK errors are never printed.

If the account already has a published identity but the persisted store lacks
its matching private identity, the harness fails. It deliberately will not
replace the server identity, because rotating an existing authority would be a
destructive shortcut rather than a verification proof.

Every run creates a fresh initiator store, proves it is initially `Unverified`
and sees an eligible authority, starts `NativeVerificationOwner::start(None)`,
compares the two SDK SAS projections, confirms both sides, waits for `Done`, and
requires `Encryption::verification_state() == Verified`. It then drops the
initiator client, rebuilds it over the same crypto store, restores the exact
Matrix session, syncs, and requires both the SDK state and product device
snapshot to remain `Verified`. The disposable initiator logs out and its store
is removed; the responder store remains the reusable authority fixture.

## Core SAS transport diagnostic

Provide the three test-account variables without committing or printing them:

```sh
export SYNARA_LIVE_HOMESERVER='https://matrix.example.test'
export SYNARA_LIVE_USERNAME='test-user'
export SYNARA_LIVE_PASSWORD='...'
scripts/run-device-verification-proof.sh
```

The command deliberately uses `cargo test --lib`. It creates two fresh sessions
and therefore exercises the explicit direct-peer route only. Fresh sessions are
not eligible own-identity authorities. This diagnostic must not be cited as the
current-device proof. Omitting `--lib` compiles all integration-test binaries
and adds unrelated disk/time failure modes.

Set `SYNARA_VERIFICATION_DIAGNOSTICS=1` on an iOS UI-test launch to record the
same privacy-safe transition trace. Each line includes only an event name, a
12-character SHA-256 flow correlation tag, and coarse request/SAS states. It
never records credentials, user/device IDs, public or secret keys, MACs, SAS
emoji/decimals, recovery material, or raw SDK errors.

## Required current-device clean-route result

The trace must contain one correlated route in this order (duplicate state
notifications are allowed; duplicate actions are not). Event names and their
state fields are shown exactly as emitted:

The common prefix is:

`request_sent(request=created) → incoming_event → incoming_registered → request_state(request=ready)`

If the initiator starts SAS, its route then contains:

`sas_started → request_state(request=transitioned) → sas_owner_accept`

If the authority starts first (including when `Ready` is coalesced away), the
initiator instead contains:

`request_state(request=transitioned) → sas_owner_accept`

Both routes must then reach:

`sas_state(sas=accepted) → sas_state(sas=keys_exchanged) → sas_confirming(sas=keys_exchanged) → sas_state(sas=confirmed) → request_state(request=done)`

`sas_confirming` is permitted only after the UI displayed the coherent
`keys_exchanged` SAS snapshot and the user explicitly confirmed a match.

The initiating Settings action must call the nil-target own-identity route. The
authority session must already be cross-signed before the run. After `Done`,
terminate and relaunch the initiating app, refresh Security, and require the
shared Core device snapshot to read `own_verification == Verified`. A locally
verified peer row, a direct session-row request, or an arbitrary/trusted-peer
fallback disqualifies the run even when SAS reaches `Done`.

The paired iOS UI smoke additionally requires identical user-visible SAS values,
confirmation on both simulators, and process termination/relaunch. The new
initiating session must start `Unverified`; the authority session is intentionally
pre-existing and cross-signed. A retry, second request, app restart before
completion, store edit, or fallback trust flag invalidates the proof.
The Security row exposes this state as a dedicated accessibility value. The
runner requires exact `Unverified` and `Verified` values; a label substring match
is not admissible because `Unverified` contains `Verified`.

Both participants must authenticate as the same Matrix user and have distinct
device IDs. Only the initiating device is fresh. The authority's pre-run
`Verified` state and the initiator's pre-run `Unverified` state are mandatory
starting-state evidence. A second test account cannot be substituted for the
same-account session that this verification entry point targets.

For the paired iOS UI test, provision the responder simulator first: sign in,
complete account verification from an already trusted client, confirm its
Security row says `Verified`, and preserve that simulator's app data. Set
`SYNARA_LIVE_VERIFICATION_REUSE_SESSION=1` for the paired run. The test requires
that persisted responder session and fails if it reaches login. The initiator
always resets and signs in fresh even when that shared environment flag is set;
it must read `Unverified` before **Verify This Device** becomes enabled. Do not
set or use an explicit target-device variable for this proof.

Account Settings coordination reads the two current user/device identities only;
it neither selects an exact authority nor invokes **Refresh Sessions**. Authority
eligibility is read through the shared Core
`Encryption::has_devices_to_verify_against()` projection before the nil-target
request. A manual session refresh is not part of this clean route and cannot
substitute for that SDK-owned eligibility readback.

A complete dark-mode visual proof contains the painted SAS values and terminal
completion for both participants, plus pre-run and post-relaunch current-device
state for the initiator and pre-run authority state for the responder. The two
SAS captures use a two-way role barrier and an explicit
background/foreground repaint because concurrently active Simulator instances
can race the host compositor. Persist each role's seven accessibility values
before capture and compare the two files byte-for-byte. If one simulator still
captures blank or placeholder glyphs while those exact values and the protocol
route pass, record it separately as a raster/compositor failure: it neither
invalidates the functional SAS proof nor satisfies the visual proof. Repeat the
same unchanged route on a clean simulator instance; the run is visually
acceptable only when both images paint all values. The post-relaunch initiating
Security row's dedicated accessibility value must exactly equal `Verified`; the
runner also requires that row to be hittable at least eight points above the
floating tab bar before capture.

Generated `.xctestrun` files contain test credentials. Keep role-specific copies
beside the build products so Xcode's `__TESTROOT__` remains valid, isolate logs
and result bundles in a temporary directory, and delete the configs, logs,
result bundles, coordination files, and screenshots immediately after review.
