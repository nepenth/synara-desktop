# ROE-02: Device Verification and Trust State Machines

Hypothesis: request discovery, SAS phases, trust transitions, cancellation,
timeouts, and completion semantics should be one explicit Rust state machine.

Preserve platform ownership of verification screens, emoji/decimal display,
accessibility, user confirmation, navigation, and OS lifecycle reactions.

Investigate:

- matrix-rust-sdk verification APIs and authoritative transition semantics;
- incoming and outgoing requests, concurrent sessions, duplicate events,
  resume/restart, cancellation, timeout, and remote-device disappearance;
- `KeysExchanged`, presentability, comparison, confirmation, trust propagation,
  and cross-signing prerequisites;
- typed states/actions required by React and SwiftUI without exposing SDK
  internals or allowing impossible actions;
- diagnostics that explain the earliest failed transition without leaking
  secrets.

Minimum proof: exhaustive transition-table tests, model/property tests for
illegal transitions, two-device encrypted Synapse proof, process-restart
recovery, desktop+iOS presentation-contract tests, and physical-device
verification before acceptance.
