# Device verification local proof

This proof exercises the production `NativeVerificationOwner` twice against a
real Matrix test account. It sends one request to one exact peer device, accepts
the request, starts and accepts SAS, compares the projected values, confirms on
both clients, reaches `Done`, and reads the exact peer back as `Verified` through
both the Matrix SDK and Synara device projection.

## Core proof

Provide the three test-account variables without committing or printing them:

```sh
export SYNARA_LIVE_HOMESERVER='https://matrix.example.test'
export SYNARA_LIVE_USERNAME='test-user'
export SYNARA_LIVE_PASSWORD='...'
scripts/run-device-verification-proof.sh
```

The command deliberately uses `cargo test --lib`. Omitting `--lib` compiles all
integration-test binaries and adds unrelated disk/time failure modes.

Set `SYNARA_VERIFICATION_DIAGNOSTICS=1` on an iOS UI-test launch to record the
same privacy-safe transition trace. Each line includes only an event name, a
12-character SHA-256 flow correlation tag, and coarse request/SAS states. It
never records credentials, user/device IDs, public or secret keys, MACs, SAS
emoji/decimals, recovery material, or raw SDK errors.

## Required clean-route result

The trace must contain one correlated route in this order (duplicate state
notifications are allowed; duplicate actions are not). Event names and their
state fields are shown exactly as emitted:

`request_sent(request=created) → incoming_event → incoming_registered → request_state(request=ready) → request_state(request=transitioned) → sas_state(sas=accepted) → sas_state(sas=keys_exchanged) → sas_confirming(sas=keys_exchanged) → sas_state(sas=confirmed) → request_state(request=done)`

The paired iOS UI smoke additionally requires identical user-visible SAS values,
confirmation on both simulators, process termination/relaunch, and the exact
coordinated peer session labelled `Verified` in Account settings. A retry,
second request, app restart before completion, store edit, or fallback trust
flag invalidates the proof. `SYNARA_LIVE_VERIFICATION_REUSE_SESSION=1` also
invalidates a clean-route durability claim because trust may predate the run.

Both simulator runners must authenticate as the same Matrix user and create
distinct fresh device sessions. The smoke exchanges the SDK-backed account user
and device identities through temporary coordination files, fails before
request creation when the users differ or device IDs match, and never prints
those values. A second test account is appropriate for room/message tests, but
cannot be substituted for the same-account session that this verification entry
point targets.

Account Settings observes the shared-core `devices` wakeup and reloads through
the homeserver-backed device snapshot. The smoke also invokes the explicit
Refresh Sessions action while waiting for the exact coordinated peer; repeatedly
scrolling an old snapshot is not a discovery proof.

A complete dark-mode visual proof contains six screenshots: painted SAS values,
terminal completion, and post-relaunch durable peer trust for both initiator and
responder. The two SAS captures are serialized because concurrently active
Simulator instances can race the host compositor. If one simulator still
captures placeholder cards, repeat the same unchanged route on a clean simulator
instance; the run is visually acceptable only when both images paint all values.
Each exact peer-trust row must be hittable and at least eight points above the
floating tab bar before capture.

Generated `.xctestrun` files contain test credentials. Keep role-specific copies
beside the build products so Xcode's `__TESTROOT__` remains valid, isolate logs
and result bundles in a temporary directory, and delete the configs, logs,
result bundles, coordination files, and screenshots immediately after review.
