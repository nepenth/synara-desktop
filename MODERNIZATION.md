# Desktop Modernization

This desktop branch is the native half of the Synara modernization stack. The
web client owns Matrix state and feature UI; this wrapper owns native shell
surfaces and exposes a small typed bridge to the web app.

## Native Scope

- Tray/status-bar menu for Show Synara, Later, Notifications, and Quit.
- Close-to-tray behavior on desktop platforms.
- Native notification permissions and click activation.
- Dock/taskbar badge count updates from unread, notification, and Later
  summaries supplied by the web client.
- Configurable global shortcuts for Show, Later, and Notifications.
- macOS camera/microphone permission descriptions for calls.
- Structured Hermes/agent action bridge for trusted backend integrations.
- Signed auto-update support is intentionally disabled in this branch until a
  stable release metadata endpoint is configured and tested for this fork.

## Validation

```sh
cargo test --manifest-path src-tauri/Cargo.toml
```

Full package builds require the paired web client assets in the nested `synara/`
checkout:

```sh
cd synara
npm ci
npm run build
cd ..
npm ci
npm run tauri build
```

For local macOS smoke builds without a Developer ID certificate, use an ad-hoc
signing identity so LaunchServices receives a fully sealed bundle:

```sh
APPLE_SIGNING_IDENTITY=- npm run tauri build -- --bundles app
```

Release builds should continue to use the normal certificate/notarization
environment. The ad-hoc identity is only for private local validation.

Packaged builds expose their identity in the tray menu and About dialog as
`Build <version> <branch>@<short-sha>`. Use that value to confirm the running
app matches the PR head you intended to test.

When replacing an existing app bundle locally, move the old bundle aside before
copying the new one; `cp -R source.app /Applications/Synara.app` can copy the new
bundle inside the old bundle instead of replacing it.

```sh
mv /Applications/Synara.app "/Applications/Synara.app.$(date +%Y%m%d%H%M%S).previous"
cp -R src-tauri/target/release/bundle/macos/Synara.app /Applications/Synara.app
```

Local `cargo check` and `cargo test` can run with the placeholder app shell, but
runtime smoke testing should use a real web `dist/` bundle.

## Runtime Smoke Test

Before marking the desktop PR ready for broader review, capture screenshots or
short clips for these flows:

1. Tray/status-bar menu opens Show, Later, and Notifications.
2. Closing the main window hides it to the tray and Quit exits the app.
3. A native notification click focuses Synara and deep-links to a room/event or
   thread anchor supplied by the web client.
4. Badge counts update when unread/Later/notification summaries change.
5. Configurable global shortcuts route to Show, Later, and Notifications.
6. A Hermes/agent action is copied, opened, or emitted through the
   `synara://agent-action` event with sanitized payload data.

Use test Matrix accounts for preview builds and demos. A PR preview of a Matrix
client is untrusted client code until merged and served from a trusted release
channel.

## Bridge Security

- The injected `window.__SYNARA_DESKTOP__` object only advertises capabilities
  and forwards explicit Tauri command names.
- Agent action payloads are sanitized in Rust before any local handling:
  IDs/titles/prompts are trimmed and capped, markdown is capped, action kinds
  are allow-listed, and URLs must use HTTPS.
- Local handling is intentionally narrow: copy actions write bounded text to the
  clipboard, open actions launch HTTPS URLs through the OS opener, and all other
  supported actions are emitted as `synara://agent-action` events for a backend
  integration to handle.
- The wrapper does not keep a duplicate copy of Matrix room state, Later items,
  favorites, folders, notification settings, or threads.

## Release Notes

- Releases are signed with the existing Ed25519 release key documented in the
  README.
- macOS notarization and hardened runtime settings should be verified as part
  of the normal Tauri release workflow before distributing outside private fork
  testing.
- Linux package behavior should be checked on the target packaging format
  because tray and notification behavior varies by desktop environment.

## Related Docs

- [Desktop integration contract](docs/desktop-modernization.md)
