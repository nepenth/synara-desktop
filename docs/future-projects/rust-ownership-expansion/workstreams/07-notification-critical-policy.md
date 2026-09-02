# ROE-07: Notification and Critical-Approval Policy

Hypothesis: eligibility, privacy level, deduplication, urgency, and action
semantics should be shared policy, while APNs/NSE and desktop notification
delivery remain platform-owned.

Investigate:

- Matrix push rules, muted rooms, mentions, foreground suppression, previews,
  encrypted content availability, badges, and notification replacement;
- critical agent-approval classification, expiration, escalation, and safe
  quick-action availability;
- privacy settings for sender-only, content preview, or hidden content;
- replay/deduplication across sync and push paths;
- platform capability differences and fail-closed behavior when content or
  trust context is unavailable.

Minimum proof: policy table/unit tests, encrypted/locked-device cases,
duplicate-delivery tests, expiry and clock-skew tests, NSE memory-budget tests,
APNs/TestFlight proof, and macOS/Linux notification-action proof.
