# iOS Handoff: Push Previews via NSE + Settings Toggle

Audience: iOS implementation agent for `nepenth/synara-desktop` / `synara-ios`.
Owner: Synara iOS implementation.
Date: 2026-07-09
Status: implementation in local validation

## Goal

Implement **privacy-preserving lock-screen previews** for Synara iOS:

1. Keep Matrix pusher format **`event_id_only`** (do **not** put message bodies on the APNs path).
2. Add a **Notification Service Extension (NSE)** that enriches sparse APNs payloads on-device.
3. Add Settings: **“Show message content in notifications”**, off by default.

The push gateway must:

- Sends non-blank generic alerts.
- Includes `room_id`, `event_id`, badge, `synara.kind`.
- Sets **`aps.mutable-content = 1`** so NSE can run.
- Supports agent-approval category when approval metadata is present.

This handoff is **iOS-only**. Do not change the push gateway contract without
coordinating the proxy implementation.

---

## Architecture (source of truth)

```text
Homeserver  --event_id_only-->  push gateway
Gateway     --APNs alert (generic or approval) + room_id/event_id + mutable-content-->  device
NSE         --get-only Keychain callback + one-shot Matrix resolve-->  rewrite title/body (if allowed)
User        sees rich banner when possible; generic fallback otherwise
```

### Privacy rules (MUST)

- **DO NOT** switch default pusher format off `event_id_only`.
- **DO NOT** send full Matrix event bodies through APNs as the primary path.
- NSE may fetch event content **only on device**, only when preview setting is enabled.
- Fail closed: if resolve fails / times out / locked Keychain → leave gateway text.
- Never log access tokens, full message bodies, or APNs tokens.

---

## Phase A — Notification Service Extension (#2)

### A1. Xcode / XcodeGen targets

In `synara-ios/project.yml` (and regenerated `Synara.xcodeproj`):

1. Add app extension target, e.g.:
   - name: `SynaraNotificationService`
   - bundle id: `com.whylandcreative.synara.NotificationService`
   - deployment target: iOS 16.0 (match app)
2. Embed extension in main app target.
3. Shared App Group (recommended):
   - `group.com.whylandcreative.synara`
4. Keychain access group if needed for session item sharing (must match main app access group policy).

Reference already exists in enrollment docs:

```text
com.whylandcreative.synara.NotificationService
```

### A2. Entitlements / capabilities

Main app + NSE:

- Push Notifications (already on app).
- **App Groups** for shared preference + optional small cache.
- Keychain sharing / access group consistent with `SecureSessionStore`.
- Background Modes: `remote-notification` remains on app.

NSE Info.plist:

- `NSExtension` → `NSExtensionPointIdentifier` = `com.apple.usernotifications.service`
- Principal class / extension entrypoint for `UNNotificationServiceExtension`.

### A3. Payload contract from gateway (already live)

Expect `userInfo` roughly like:

```json
{
  "aps": {
    "alert": { "title": "Synara", "body": "New activity" },
    "badge": 3,
    "sound": "default",
    "content-available": 1,
    "mutable-content": 1
  },
  "room_id": "!room:matrix.example.com",
  "event_id": "$event:matrix.example.com",
  "notification_summary": { "appBadgeCount": 3 },
  "synara": {
    "kind": "matrix-event",
    "room_id": "!room:matrix.example.com",
    "event_id": "$event:matrix.example.com"
  }
}
```

Agent approval variant may include:

```json
"aps": { "category": "synara.agent-approval", "mutable-content": 1, "...": "..." },
"synara": { "kind": "agent-approval", "room_id": "...", "event_id": "..." }
```

Reuse existing parsers:

- `NotificationPushRouteParser` / flatten helpers in `PushService.swift`
- Prefer keys: `room_id` / `event_id`, fallback nested `synara.*`

### A4. NSE runtime behavior

The extension links `SynaraNseCore`, never the full `SynaraCore` package. Its
UniFFI surface is one narrow `NsePreviewRequest` with async `resolve()` and
idempotent `cancel()`, plus a get-only secret callback. Cancellation selects
against and drops the Rust Matrix future promptly; Rust owns client creation
and cleanup on success, error, cancellation, and timeout. Swift must not retain
a Matrix client or expose vault writes from the extension process.

Implement `UNNotificationServiceExtension`:

```text
didReceive(_:withContentHandler:)
serviceExtensionTimeWillExpire()
```

Algorithm:

1. Copy best-attempt content immediately.
2. Read **preview preference** (App Group UserDefaults). If disabled → call contentHandler with original.
3. If `synara.kind == agent-approval` and title/body already specific → optional pass-through (still may soft-validate).
4. Parse `room_id` + `event_id`. Missing either → original.
5. Load Matrix session from Keychain (**same store as main app**, App Group aware).
6. Bounded resolve (20s product deadline, below the system deadline):
   - open SDK client if possible, or lightweight authenticated request
   - fetch event / use notification client / existing sparse route resolver patterns
7. Build preview:
   - title: room name if available else sender display name else "Synara"
   - body: `sender: truncated body` or generic “sent a message”
   - clamp title ≤ 120, body ≤ 240
8. On any failure / timeout → original content.
9. Always call `contentHandler` exactly once.

### A5. Keychain session access (critical)

Current session lives in main app Keychain via `SecureSessionStore`.

NSE requirements:

- Session credentials must be readable by NSE process.
- Prefer App Group container + Keychain access group used by both targets.
- If current store is app-only accessibility, update to a shared accessibility suitable for background unlock constraints (`afterFirstUnlock` is usually required for NSE).
- Document any security tradeoff in `docs/security-review.md` / privacy inventory.

**DO NOT** put access tokens in UserDefaults.

### A6. Matrix resolve strategy (bounded)

Preferred order:

1. Reuse the Matrix Rust SDK notification client with a distinct cross-process
   store-lock identity, `RoomLoadSettings::One(room_id)`, and a bounded
   end-to-end restore/decryption deadline.
2. For encrypted events without locally available keys, leave the generic body.

Must remain within NSE time budget. No long sync loops.

Existing hooks to study:

- `SparsePushRouteResolving` / `MatrixSparsePushRouteResolver`
- `PushService.swift` notification action planning
- `SecureSessionStore.swift`
- `SynaraApp.swift` notification delegate handlers

### A7. Tests

Add unit tests for pure helpers (no device required):

- payload parse: room/event from flat + nested `synara`
- preview composition + clamping
- preference gate (disabled → no enrichment plan)
- approval category pass-through rules

Optional simulator tests only if existing harness supports extension targets.

---

## Phase B — Settings toggle (#3)

### B1. UI

In `SettingsView` (Notifications section near existing Push registration):

- Toggle: **“Show message content in notifications”**
- Default: **OFF**
- Helper text:
  “When enabled, Synara can show sender and message text on the lock screen after secure on-device lookup. Message content is not sent through Apple’s push servers.”

### B2. Storage

- Store boolean in App Group UserDefaults, e.g.:
  - suite: `group.com.whylandcreative.synara`
  - key: `synara.settings.lockScreenMessagePreviews`
- Main app writes; NSE reads only.
- Mirror into app settings model if one exists.

### B3. Behavior matrix

| Setting | NSE |
| --- | --- |
| OFF | Do not fetch/decrypt; keep gateway generic/approval text |
| ON | Attempt bounded resolve and rewrite title/body |

Approval actions remain governed by existing revalidation/TTL in `SynaraNotificationActionContract` — preview setting does not authorize actions.

---

## Out of scope for this iOS handoff

- Push gateway approval-metadata ingest (infrastructure-owned).
- Changing `event_id_only`.
- Desktop notification paths.
- Full command text in APNs.

---

## Acceptance criteria

### NSE (#2)

- [ ] Extension target builds and embeds in Release/Debug.
- [ ] APNs with `mutable-content` invokes NSE on device/TestFlight.
- [ ] With session available + cleartext event + previews ON: banner shows useful title/body.
- [ ] With previews OFF / no session / timeout: non-blank generic fallback remains.
- [ ] No tokens/bodies in logs.
- [ ] Existing agent-approval action handling still works (`approve-once` / `deny`).
- [ ] Release `.appex` exports no `_uniffi_synara_core_` symbols.
- [ ] Release archive checker passes on the final embedded `.appex` and records
      its device slice, linked images, and executable size before upload.
- [ ] Physical-device peak `phys_footprint` stays below Apple's NSE memory
      ceiling with measured headroom for both cleartext and encrypted events.

### Settings (#3)

- [ ] Toggle visible and persists across relaunch.
- [ ] NSE observes the same value via App Group.
- [ ] Privacy copy present and accurate.

### Manual smoke (device / TestFlight)

1. Release/TestFlight build (production APNs).
2. Register push; confirm pusher URL from `SYNARA_PUSH_GATEWAY_URL` and
   `event_id_only`.
3. Previews ON → send cleartext room message → lock screen shows preview (or best-effort).
4. Previews OFF → same path shows generic non-blank text.
5. Tap notification routes to room/event.
6. Agent-approval notification (when gateway metadata present) shows category actions.
7. Capture `preview memory peak_footprint_kb` from device logs for cleartext,
   encrypted, timeout, generic-fallback, and burst/cancellation cases.
   Simulator and Mach-O file size are diagnostic only and cannot satisfy this
   release gate.

---

## Files likely to touch

```text
synara-ios/project.yml
synara-ios/Synara/Resources/Synara.entitlements
synara-ios/SynaraNotificationService/  (new)
synara-ios/Synara/Services/PushService.swift
synara-ios/Synara/Services/SecureSessionStore.swift
synara-ios/Synara/Features/SettingsView.swift
synara-ios/Synara/App/SynaraApp.swift
synara-ios/SynaraTests/...
synara-ios/docs/privacy-data-inventory.md
synara-ios/docs/push-gateway-staging.md
synara-ios/docs/security-review.md
```

---

## Coordination notes for iOS agent

- Gateway base: private operator configuration.
- Notify path: `/_matrix/push/v1/notify` (public Matrix path)
- Approval ingest: `/v1/agent-approval-events` (trusted bearer; not for the iOS app)
- App id / topic: configured by the signed app target.
- Apple team id: private signing environment.
- Use the repository's normal commit authorship configuration.

When done, report:

1. Extension target + bundle id
2. App Group / Keychain sharing choices
3. Preference key + default
4. Resolve strategy used
5. Test evidence
6. Remaining blockers (signing/profile/App Group capability in Apple Developer if needed)
