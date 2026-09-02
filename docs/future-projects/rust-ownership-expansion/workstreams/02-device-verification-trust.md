# ROE-02: Device Verification and Trust State Machines

Prior: **already Core-owned; investigate only a bounded iOS continuity
remainder**.

Core already owns verification request discovery, SAS phases, allowed actions,
trust transitions, cancellation, timeout, and completion semantics. React and
SwiftUI should display those states and report user/lifecycle observations.

## Bounded research question

Does iOS omit or lose a device-key, request identity, or app-lifecycle input
needed to resume the existing Core state machine, especially around
`KeysExchanged`, presentability, comparison, confirmation, and trust
propagation? Distinguish missing state-machine authority from a binding,
persistence, navigation, or rendering defect.

## Keep closed

- Emoji/decimal layout, accessibility, confirmation screens, navigation, and
  OS lifecycle reactions remain platform-owned.
- Do not create a Swift verification engine or a second Rust state machine.
- Do not expose SDK internals or permit impossible actions through DTOs.

A memo should map the failing transition from current diagnostics and source.
Only a proven missing Core input or transition can justify planning. Any later
implementation requires transition/property tests, two-device encrypted
Synapse proof, restart recovery, both presenter contracts, and physical-device
verification.
