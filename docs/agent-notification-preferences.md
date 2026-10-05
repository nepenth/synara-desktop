# Agent notification preferences

Synara stores the complete version 1 preferences object in Matrix global account data `in.synara.agent_notification_preferences`:

```json
{
  "schemaVersion": 1,
  "agentUserIds": ["@forge:example.org"],
  "notifyToolActivity": true,
  "notifyCommentary": true,
  "notifyFinalResponses": true
}
```

All switches default to enabled. An empty sender list filters nothing. IDs must be exact Matrix user IDs, at most 128 entries; display names cannot confer agent identity. Each save replaces the object, so concurrent edits on different devices follow the Matrix account-data last-write behavior. Other Matrix clients can retain this custom account-data object without understanding it; the notification behavior applies to Synara clients that implement the policy.

The classifier reads the authoritative decrypted SDK event's sender and full plain body before any display truncation. It recognizes the first nonempty line `🛠 Tool activity (N updates)` or `💬 Commentary (N updates)`, including singular `update`, an optional Markdown heading or bold wrapper, and requires a positive numeric count followed by nonempty content. It supports both numbered examples and Hermes' unnumbered emoji tool lines. Quoted/code-fenced headings, substring keywords, malformed counts and all other prose fall into final/other responses. Only explicitly configured senders use this classification.

Approval prompts recognized by shared Core always bypass these category switches. Desktop notification decisions validate approval sender and expiry before applying agent filters. Approval display/actions retain the existing stronger eligibility, freshness, account identity and terminal-decision checks; the preference exemption grants no new action authority.

## Ownership and synchronization

`matrix_agent_notification_preferences_snapshot` fetches fresh account data without creating defaults on the server. `matrix_agent_notification_preferences_set` validates and writes an explicit object, then fetches the effective server readback. Unknown versions and malformed settings produce an editor error; delivery policy falls back to notifying.

The authenticated notification owner serializes editor GET/PUT operations. A successful readback supplies write-through policy until the next actual preference `/sync` event. A revision counter prevents a network response already in flight from overriding a newer sync. Same-value remote resets clear pending state. Desktop emits `matrix-agent-notification-preferences-updated`; Apple owner updates use `agent_notification_preferences`. Clients also refresh on focus/activation and expose manual refresh.

Desktop suppresses the notification candidate, including sound, before the delivery ledger. The iOS foreground API `agent_notification_event_allowed` resolves the exact event in the current shared SDK owner, applies the same classifier, and has no focus/dedup/delivery side effects. Its event-fetch fallback is bounded to two seconds; unresolved events throw so the client can preserve notification delivery.

The narrow iOS NSE resolves/decrypts the event first, then fetches fresh account preferences with a two-second limit and falls back to cached preferences or notifying defaults on error. Approval prompts skip this extra settings fetch. A filtered event returns `p4-s11-nse-agent-policy-filtered` rather than a fetch/decryption failure. Policy checks happen before any message body crosses the NSE FFI boundary.

## iOS remote delivery limitation

An already-delivered remote push can only be fully discarded by an NSE in a deliberately configured build signed with Apple's Notification Service Extension Filtering entitlement. Standard builds can remove the classified preview and use a generic notification fallback; they cannot promise complete background alert suppression. The containing app can suppress foreground presentation directly. Encrypted Matrix push payloads do not expose the body to the homeserver/gateway, so account preferences alone cannot suppress them upstream. Synara installs no broad server body-glob rules: those cannot safely preserve anchored classification and approval precedence.
