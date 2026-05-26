# iOS Release Checklist

Reviewed: 2026-05-26

Status: draft release gate checklist.

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
- Privacy policy URL approved before external TestFlight.
- App Privacy labels drafted from actual data flows before App Store review.

## App Store Account Gates

- LLC Apple Developer Program enrollment complete.
- `app.synara.ios` App ID created.
- Required capabilities enabled and documented.
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
