# ROE-08: Agent Approval Detection and Action Resolution

Hypothesis: recognition, authorization options, expiry, and response mapping
for Hermes-style approvals should be one typed Rust policy shared by clients.

Preserve platform ownership of cards, sheets, notification buttons, haptics,
and emoji presentation.

Investigate:

- the authoritative Hermes event/message contract and versioning;
- structured events versus formatted/plaintext fallback detection;
- approve, deny, approve-session, approve-always, expiry, revocation, and
  duplicate-action semantics;
- room/sender trust, spoofing resistance, power/identity checks, and audit
  records;
- response encoding and idempotent delivery across room UI and notifications.

Minimum proof: real contract fixtures, spoof/adversarial corpus, expiry and
duplicate property tests, end-to-end Hermes/Matrix integration, notification
quick-action proof, desktop/iOS parity, and audit-log redaction checks.
