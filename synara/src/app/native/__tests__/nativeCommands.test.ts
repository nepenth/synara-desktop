import assert from 'node:assert/strict';
import { afterEach, test } from 'node:test';
import type { DesktopInvokeResult } from '../../utils/desktop';
import { setNativeIdentity } from '../../state/nativeIdentity';
import {
  fetchNativeRoomEvent,
  getNativeCryptoStatus,
  getNativeMediaConfig,
  getNativeProfileInfo,
  mxcUrlToNative,
  redactNativeEvent,
  searchNativeUserDirectory,
  sendNativeEvent,
  sendNativeMessage,
  sendNativeStateEvent,
  setNativeAvatarUrl,
  setNativeCommandInvokeForTests,
  setNativeDisplayName,
  uploadNativeMedia,
} from '../nativeCommands';
import type { NativeInvoke } from '../nativeWire';

const ok = (value: unknown): DesktopInvokeResult<unknown> => ({ available: true, value });
const unavailable: DesktopInvokeResult<unknown> = { available: false };

const invokingWith = (routes: Record<string, unknown>) => {
  const callLog: string[] = [];
  const invoke: NativeInvoke = async (command) => {
    callLog.push(command);
    if (Object.prototype.hasOwnProperty.call(routes, command)) return ok(routes[command]);
    return unavailable;
  };
  return { invoke, callLog };
};

afterEach(() => {
  setNativeCommandInvokeForTests(async () => unavailable);
  setNativeIdentity({});
});

test('mxcUrlToNative turns a requested size into a native thumbnail', () => {
  assert.equal(mxcUrlToNative('mxc://example.org/a'), 'mxc://example.org/a');
  assert.equal(
    mxcUrlToNative('mxc://example.org/a', 48, 48, 'crop'),
    'thumbnail/48x48/crop/mxc://example.org/a'
  );
  assert.equal(
    mxcUrlToNative('mxc://example.org/a', 320, 240, 'scale'),
    'thumbnail/320x240/scale/mxc://example.org/a'
  );
  assert.equal(mxcUrlToNative('https://example.org/a', 48, 48, 'crop'), null);
});

test('profile writes call the native profile commands', async () => {
  const { invoke } = invokingWith({
    matrix_set_own_display_name: { status: 'ok' },
    matrix_set_own_avatar: { status: 'ok' },
  });
  setNativeCommandInvokeForTests(invoke);
  assert.deepEqual(await setNativeDisplayName('Alice'), { status: 'ok' });
  assert.deepEqual(await setNativeAvatarUrl('mxc://example.org/a'), { status: 'ok' });
});

test('fetchNativeRoomEvent reads matrix_timeline_event_readback', async () => {
  const { invoke } = invokingWith({
    matrix_timeline_event_readback: {
      sessionGeneration: 8,
      roomId: '!r:example.org',
      eventId: '$evt1',
      item: {
        itemId: 'i1',
        eventId: '$evt1',
        sender: '@alice:example.org',
        type: 'm.room.message',
        body: 'hello',
        originServerTs: 999,
      },
    },
  });
  setNativeCommandInvokeForTests(invoke);
  const evt = await fetchNativeRoomEvent('!r:example.org', '$evt1');
  assert.equal(evt?.eventId, '$evt1');
  assert.equal(evt?.sender, '@alice:example.org');
  assert.equal(evt?.type, 'm.room.message');
  assert.equal(evt?.body, 'hello');
});

test('fetchNativeRoomEvent returns null on unavailable command', async () => {
  setNativeCommandInvokeForTests(async () => unavailable);
  assert.equal(await fetchNativeRoomEvent('!r:example.org', '$evt1'), null);
});

test('sendNativeMessage sends through matrix_send_text', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_send_text: {
      roomId: '!r:example.org',
      eventId: '$e1',
      localTxnId: 't1',
      status: 'sent',
    },
  });
  setNativeCommandInvokeForTests(invoke);
  const sent = await sendNativeMessage('!r:example.org', { body: 'hello' });
  assert.equal(sent?.eventId, '$e1');
  assert.deepEqual(callLog, ['matrix_send_text']);
});

test('sendNativeMessage fails closed when the command is unavailable', async () => {
  setNativeCommandInvokeForTests(async () => unavailable);
  assert.equal(await sendNativeMessage('!r:example.org', { body: 'x' }), null);
});

test('sendNativeEvent sends m.room.message and resolves null for other types', async () => {
  const { invoke } = invokingWith({
    matrix_send_text: {
      roomId: '!r:example.org',
      eventId: '$msg',
      localTxnId: 't2',
      status: 'sent',
    },
  });
  setNativeCommandInvokeForTests(invoke);
  const msg = await sendNativeEvent('!r:example.org', 'm.room.message', { body: 'hi' });
  assert.equal(msg?.eventId, '$msg');
  const gap = await sendNativeEvent('!r:example.org', 'm.custom.type', {});
  assert.equal(gap, null);
});

test('sendNativeStateEvent maps covered room-state types and leftover native send', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_set_room_name: { status: 'ok', roomId: '!r:example.org', sessionGeneration: 8 },
    matrix_send_state_event: { status: 'ok' },
  });
  setNativeCommandInvokeForTests(invoke);
  const name = await sendNativeStateEvent('!r:example.org', 'm.room.name', { name: 'New' });
  assert.equal(name?.status, 'ok');
  const leftover = await sendNativeStateEvent('!r:example.org', 'm.room.canonical_alias', {
    alias: '#r:example.org',
  });
  assert.equal(leftover?.status, 'ok');
  assert.deepEqual(callLog, ['matrix_set_room_name', 'matrix_send_state_event']);
});

test('uploadNativeMedia uploads through matrix_upload_media', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_upload_media: { mxc: 'mxc://example.org/up1' },
  });
  setNativeCommandInvokeForTests(invoke);
  const uploaded = await uploadNativeMedia({ mimeType: 'image/png', bytes: [1, 2, 3] });
  assert.equal(uploaded?.mxc, 'mxc://example.org/up1');
  assert.deepEqual(callLog, ['matrix_upload_media']);
});

test('uploadNativeMedia throws when unavailable', async () => {
  setNativeCommandInvokeForTests(async () => unavailable);
  await assert.rejects(() => uploadNativeMedia({ mimeType: 'x', bytes: [1] }));
});

test('getNativeMediaConfig reads m.upload.size wire key', async () => {
  const { invoke } = invokingWith({
    matrix_media_config: { 'm.upload.size': 10485760 },
  });
  setNativeCommandInvokeForTests(invoke);
  assert.deepEqual(await getNativeMediaConfig(), { maxUploadSizeBytes: 10485760 });
});

test('getNativeMediaConfig fails closed (empty) when unavailable', async () => {
  setNativeCommandInvokeForTests(async () => unavailable);
  assert.deepEqual(await getNativeMediaConfig(), {});
});

test('getNativeProfileInfo loads own profile from the native owner', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: '@alice:example.org',
      device_id: 'DEV',
      homeserver_url: 'https://matrix.example.org',
      sessionGeneration: 5,
    },
    matrix_get_own_profile: {
      userId: '@alice:example.org',
      displayName: 'Alice',
      avatarUrl: 'mxc://example.org/avatar',
    },
  });
  setNativeCommandInvokeForTests(invoke);
  setNativeIdentity({ userId: '@alice:example.org' });
  const profile = await getNativeProfileInfo('@alice:example.org');
  assert.equal(profile.avatar_url, 'mxc://example.org/avatar');
  assert.equal(profile.displayname, 'Alice');
  assert.equal(callLog.includes('matrix_get_own_profile'), true);
});

test('getNativeProfileInfo rejects non-mxc avatars', async () => {
  const { invoke } = invokingWith({
    matrix_session_snapshot: {
      status: 'logged_in',
      user_id: '@alice:example.org',
      device_id: 'DEV',
      homeserver_url: 'https://matrix.example.org',
      sessionGeneration: 5,
    },
    matrix_get_own_profile: {
      userId: '@alice:example.org',
      displayName: 'Alice',
      avatarUrl: 'data:image/png;base64,AAAA',
    },
  });
  setNativeCommandInvokeForTests(invoke);
  setNativeIdentity({ userId: '@alice:example.org' });
  const profile = await getNativeProfileInfo('@alice:example.org');
  assert.equal(profile.avatar_url, undefined);
  assert.equal(profile.displayname, 'Alice');
});
test('getNativeCryptoStatus reads matrix_crypto_status (no keys, D1C)', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_crypto_status: {
      sessionGeneration: 7,
      encryptionEnabled: true,
      crossSigningState: 'Ready',
    },
  });
  setNativeCommandInvokeForTests(invoke);
  const crypto = await getNativeCryptoStatus();
  assert.equal(crypto?.encryptionEnabled, true);
  assert.equal(crypto?.crossSigningState, 'Ready');
  assert.equal(callLog[0], 'matrix_crypto_status');
});

test('redactNativeEvent redacts through matrix_timeline_redact', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_timeline_redact: { event_id: '$evt1', room_id: '!r:example.org' },
  });
  setNativeCommandInvokeForTests(invoke);
  const redacted = await redactNativeEvent('!r:example.org', '$evt1', 'spam');
  assert.equal(redacted?.event_id, '$evt1');
  assert.equal(callLog[0], 'matrix_timeline_redact');
});

test('redactNativeEvent fails closed when unavailable', async () => {
  setNativeCommandInvokeForTests(async () => unavailable);
  assert.equal(await redactNativeEvent('!r:example.org', '$evt1'), null);
});

test('searchNativeUserDirectory maps native user-directory hits and omits non-mxc avatars', async () => {
  const { invoke, callLog } = invokingWith({
    matrix_user_directory_search: {
      limited: true,
      results: [
        {
          userId: '@bob:example.org',
          displayName: 'Bob',
          avatarUrl: 'mxc://example.org/abc',
        },
        {
          userId: '@eve:example.org',
          displayName: 'Eve',
          avatarUrl: 'data:image/png;base64,AAAA',
        },
      ],
    },
  });
  setNativeCommandInvokeForTests(invoke);
  const listed = {
    limited: true,
    results: [
      {
        user_id: '@bob:example.org',
        display_name: 'Bob',
        avatar_url: 'mxc://example.org/abc',
      },
      {
        user_id: '@eve:example.org',
        display_name: 'Eve',
        avatar_url: undefined,
      },
    ],
  };
  assert.deepEqual(await searchNativeUserDirectory({ term: 'bo', limit: 10 }), listed);
  assert.equal(callLog[0], 'matrix_user_directory_search');
  assert.deepEqual(await searchNativeUserDirectory({ term: 'bo' }), listed);
  assert.equal(callLog[1], 'matrix_user_directory_search');
});

test('getNativeCryptoStatus fails closed when unavailable', async () => {
  setNativeCommandInvokeForTests(async () => unavailable);
  assert.equal(await getNativeCryptoStatus(), null);
});

test('searchNativeUserDirectory is empty when the command is unavailable', async () => {
  setNativeCommandInvokeForTests(async () => unavailable);
  assert.deepEqual(await searchNativeUserDirectory({ term: 'term' }), {
    limited: false,
    results: [],
  });
});
