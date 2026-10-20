import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  RoomNotificationMode,
  getRoomNotificationMode,
  nextRoomsNotificationPreferences,
  type RoomsNotificationPreferences,
} from '../useRoomsNotificationPreferences';

const empty = (): RoomsNotificationPreferences => ({
  mute: new Set(),
  specialMessages: new Set(),
  allMessages: new Set(),
});

test('a failed refresh keeps muted rooms muted and flags the failure', () => {
  const loaded = nextRoomsNotificationPreferences(empty(), {
    ok: true,
    rooms: [
      { roomId: '!muted:example.org', mode: 'mute' },
      { roomId: '!loud:example.org', mode: 'all' },
    ],
  });
  assert.equal(loaded.loadFailed, undefined);

  const failed = nextRoomsNotificationPreferences(loaded, { ok: false });
  assert.equal(failed.loadFailed, true);
  assert.equal(getRoomNotificationMode(failed, '!muted:example.org'), RoomNotificationMode.Mute);
  assert.equal(
    getRoomNotificationMode(failed, '!loud:example.org'),
    RoomNotificationMode.AllMessages
  );

  const recovered = nextRoomsNotificationPreferences(failed, { ok: true, rooms: [] });
  assert.equal(recovered.loadFailed, undefined);
});

test('notification settings show write and load failures where the user acted', () => {
  const switcher = readFileSync('src/app/components/RoomNotificationSwitcher.tsx', 'utf8');
  // The menu closes only after the mode is saved.
  assert.match(switcher, /setMode\(mode\)\.then\(handleClose, \(\) => setFailedMode\(mode\)\)/);
  assert.match(switcher, /role="alert"/);
  assert.match(switcher, /useRoomsNotificationPreferencesLoadFailed/);

  const settings = readFileSync(
    'src/app/features/settings/notifications/Notifications.tsx',
    'utf8'
  );
  assert.doesNotMatch(settings, /\bwrite\(\(\) =>/);
  assert.match(settings, /<SectionError message=\{sectionError\('defaults'\)\} \/>/);
  assert.match(settings, /<SectionError message=\{sectionError\('mentions'\)\} \/>/);
  assert.match(settings, /<SectionError message=\{sectionError\('keywords'\)\} \/>/);
  assert.match(settings, /onClick=\{load\}/);
});
