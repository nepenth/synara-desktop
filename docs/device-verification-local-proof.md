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
