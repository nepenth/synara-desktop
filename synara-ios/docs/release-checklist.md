# iOS Release Checklist

Reviewed: 2026-08-17

Status: draft release gate checklist.

The 2026-08-17 local proof closed the previously pending simulator execution
gate: the exact CI script passed 436 unit tests and 49 UI tests with only
intentional gated skips, and a signed live suite passed 7 of 7 Matrix scenarios
without retries. This does not close physical-device, APNs, legal, privacy URL,
or external App Store release gates.

## Phase 0 Gates

- Repository layout ADR accepted.
- Apple Developer enrollment checklist created.
- License inventory created and legal blocker recorded.
- Shared contract inventory complete.
- Tauri iOS feasibility spike completed with native SwiftUI still recommended.
- Native Matrix SDK feasibility spike completed at package/import level; real
  test-account login remains for Phase 2.
- Architecture ADR accepted.
- Phase 1 native iOS skeleton can start.

## Phase 1 Simulator Gates

- `Synara.xcodeproj` exists under `synara-ios/`.
- App target, unit test target, and UI test target exist.
- Unsigned simulator build succeeds without Apple credentials.
- Unit and UI test targets compile with `build-for-testing`.
- Unit tests run locally after CoreSimulator is repaired.
- UI test launches the app shell after CoreSimulator is repaired.
- DerivedData, build products, and local signing artifacts are ignored.

## Security And Privacy Gates

- Keychain-backed session store implemented before real login persistence.
- Logging redaction tests cover tokens, Matrix IDs, event IDs, URLs with
  credentials, APNs tokens, and recovery-key-like strings.
- Logout wipes Keychain entries, SDK stores, caches, drafts, and pending push
  registration state.
- Push payloads default to generic content.
- Privacy manifest included in the app target.
- Privacy data inventory drafted.
- Initial security review drafted.
- Accessibility and performance hardening reports drafted.
- Privacy policy URL approved before external TestFlight.
- App Privacy labels drafted from actual data flows before App Store review.

## App Store Account Gates

- LLC Apple Developer Program enrollment complete.
- `com.whylandcreative.synara` App ID created.
- Notification service extension App ID/profile created if rich push previews
  are enabled.
- Required capabilities enabled and documented.
- App Groups and Keychain Sharing enabled for the app and notification service
  extension.
- App Store Connect app record created.
- CI signing secret storage approved.
- APNs sandbox and production key handling approved.
- Export compliance answers reviewed.

## Legal Gates

- AGPL/App Store distribution review complete.
- Third-party dependency notices generated.
- In-app licenses/about screen implemented.
- Source availability and attribution plan approved.
- No external TestFlight or App Store submission before this section is closed.

## Desktop Continuity Gates

- macOS desktop smoke validation remains passing.
- Linux desktop smoke validation remains passing.
- Shared contract changes include TypeScript/runtime validation.
- iOS contract conformance tests use the same JSON fixtures as desktop.
- Desktop package CI remains green after iOS project files are added.
