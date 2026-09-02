# ROE-07: Notification and Critical-Approval Policy

Prior: **shared policy yes; OS delivery no**.

Core already owns Matrix push-rule settings and can own cross-client
eligibility, privacy level, deduplication, urgency, expiry, and safe action
semantics. APNs, Notification Service Extension execution, macOS/Linux tray
delivery, banners, badges, buttons, and haptics are platform integrations.

## Bounded research question

Which decisions about muted rooms, mentions, foreground suppression, preview
privacy, encrypted-content availability, replacement/deduplication, expiry,
and clock skew are duplicated authority rather than OS capability handling?
Treat critical agent-approval classification and action resolution as ROE-08;
do not build a parallel policy here.

Any proposed extraction must fail closed when content or trust context is
unavailable and must prove policy tables, locked/encrypted cases, duplicates,
expiry, and cross-client parity. NSE memory limits, APNs/TestFlight proof, and
desktop notification actions remain platform-specific validation.
