# Synara Settings Compatibility Contract

Reviewed: 2026-05-25

Status: initial shared contract with runtime split logic in
`src/app/state/settings.ts` and schema fixtures under `docs/contracts/`.

## Purpose

Settings must remain portable where they affect shared Synara behavior, while
desktop-only shortcuts and future iOS-only preferences stay in platform
settings. This prevents iOS, macOS, and Linux from overwriting channel-specific
state.

## Machine-Readable Artifacts

- [synara-shared-settings.schema.json](./contracts/synara-shared-settings.schema.json)
- [synara-desktop-platform-settings.schema.json](./contracts/synara-desktop-platform-settings.schema.json)
- [synara-settings.json fixtures](./contracts/fixtures/synara-settings.json)

## Storage Keys

| Key                | Scope           | Purpose                                             |
| ------------------ | --------------- | --------------------------------------------------- |
| `settings`         | Shared runtime  | Platform-neutral Synara UI and workflow settings.   |
| `platformSettings` | Desktop runtime | Desktop shortcut settings and desktop-only options. |

Legacy merged `settings` blobs are still read so existing desktop users keep
their shortcuts after migration. New writes split shared and desktop platform
settings.

## Shared Settings

Shared settings are platform-neutral controls for theme, composer behavior,
timeline display, notification preferences, date/time formatting, GIF opt-in,
and developer tools visibility.

Desktop-only shortcut and diagnostic fields are not shared settings:

- `desktopShortcutShow`
- `desktopShortcutLater`
- `desktopShortcutNotifications`
- `desktopDiagnosticsEnabled`
- `desktopDiagnosticsPerformance`
- `desktopDiagnosticsSession`
- `desktopDiagnosticsRoomState`
- `desktopDiagnosticsOverlay`

## iOS Notes

- iOS should read only shared settings that have native meaning.
- iOS platform settings should use its own storage key or native settings
  domain, not the desktop `platformSettings` shortcut schema.
- New shared settings must be added to this document and schema before iOS
  consumes them.
- `themeBaseColor` is a shared _schema_ field (full `#rrggbb` only). Desktop
  persists it in the `settings` localStorage blob. iOS persists the same key
  in app-group UserDefaults. Those stores are not synced; they will drift
  until a real shared settings transport exists. Do not treat the matching
  key name as a shared runtime.

## Acceptance Criteria

- Shared settings fixtures validate without desktop shortcut fields.
- Desktop platform settings fixtures validate with only desktop shortcut and diagnostic fields.
- Runtime tests prove legacy merged shortcut settings migrate into platform
  settings while new writes split shared and platform state.
