import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ImagePack } from '../../../plugins/custom-emoji/ImagePack';
import {
  getGlobalImagePacksWithNativeOwner,
  getRoomImagePacksWithNativeOwner,
  getUserImagePackWithNativeOwner,
  imagePackFromNativeDto,
  isNativePackReadSession,
  setGlobalImagePacksWithNativeOwner,
  setUserImagePackWithNativeOwner,
  type NativeInvoke,
} from '../nativeImagePackOwner';

const loggedInInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  if (command === 'matrix_get_user_image_pack') {
    return {
      available: true,
      value: {
        sessionGeneration: 1,
        pack: {
          id: '@u:example.org',
          content: {
            pack: { display_name: 'Me' },
            images: { smile: { url: 'mxc://example.org/a' } },
          },
        },
      },
    };
  }
  if (command === 'matrix_get_room_image_packs') {
    return {
      available: true,
      value: {
        sessionGeneration: 1,
        roomId: '!r:example.org',
        packs: [
          {
            id: '$e1',
            roomId: '!r:example.org',
            stateKey: '',
            content: { pack: { display_name: 'Room' }, images: {} },
          },
        ],
      },
    };
  }
  if (command === 'matrix_get_global_image_packs') {
    return {
      available: true,
      value: {
        sessionGeneration: 1,
        packs: [
          {
            id: '$g1',
            roomId: '!r:example.org',
            stateKey: 'global',
            content: { pack: { display_name: 'Global' }, images: {} },
          },
        ],
      },
    };
  }
  if (command === 'matrix_set_user_image_pack') {
    return { available: true, value: { status: 'ok' } };
  }
  if (command === 'matrix_set_global_image_packs') {
    return { available: true, value: { status: 'ok' } };
  }
  return { available: false };
};

const failClosedInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  return { available: false };
};

test('legacy when not desktop for pack read', async () => {
  assert.equal(await isNativePackReadSession(false, loggedInInvoke), false);
  assert.equal(await getUserImagePackWithNativeOwner(false, loggedInInvoke), 'legacy');
  assert.equal(await getGlobalImagePacksWithNativeOwner(false, loggedInInvoke), 'legacy');
  assert.equal(
    await getRoomImagePacksWithNativeOwner('!r:example.org', false, loggedInInvoke),
    'legacy'
  );
});

test('native user pack read', async () => {
  const pack = await getUserImagePackWithNativeOwner(true, loggedInInvoke);
  assert.ok(pack instanceof ImagePack);
  assert.equal((pack as ImagePack).id, '@u:example.org');
  assert.equal((pack as ImagePack).meta.name, 'Me');
});

test('native room and global pack read', async () => {
  const room = await getRoomImagePacksWithNativeOwner('!r:example.org', true, loggedInInvoke);
  assert.ok(Array.isArray(room));
  assert.equal((room as ImagePack[]).length, 1);
  assert.equal((room as ImagePack[])[0].meta.name, 'Room');

  const global = await getGlobalImagePacksWithNativeOwner(true, loggedInInvoke);
  assert.ok(Array.isArray(global));
  assert.equal((global as ImagePack[]).length, 1);
  assert.equal((global as ImagePack[])[0].address?.stateKey, 'global');
});

test('pack read fail-closed when command missing', async () => {
  await assert.rejects(
    () => getUserImagePackWithNativeOwner(true, failClosedInvoke),
    /unavailable/i
  );
  await assert.rejects(
    () => getGlobalImagePacksWithNativeOwner(true, failClosedInvoke),
    /unavailable/i
  );
});

test('user pack write legacy when not desktop', async () => {
  assert.equal(
    await setUserImagePackWithNativeOwner({ pack: {}, images: {} }, false, loggedInInvoke),
    'legacy'
  );
});

test('user pack write native ok', async () => {
  const result = await setUserImagePackWithNativeOwner(
    { pack: { display_name: 'Me' }, images: {} },
    true,
    loggedInInvoke
  );
  assert.equal(result, 'native');
});

test('user pack write fail-closed when command missing', async () => {
  await assert.rejects(
    () =>
      setUserImagePackWithNativeOwner(
        { pack: { display_name: 'Me' }, images: {} },
        true,
        failClosedInvoke
      ),
    /unavailable/i
  );
});

test('global pack write legacy when not desktop', async () => {
  assert.equal(
    await setGlobalImagePacksWithNativeOwner({ rooms: {} }, false, loggedInInvoke),
    'legacy'
  );
});

test('global pack write native ok', async () => {
  const result = await setGlobalImagePacksWithNativeOwner(
    { rooms: { '!r:example.org': { '': {} } } },
    true,
    loggedInInvoke
  );
  assert.equal(result, 'native');
});

test('global pack write fail-closed when command missing', async () => {
  await assert.rejects(
    () => setGlobalImagePacksWithNativeOwner({ rooms: {} }, true, failClosedInvoke),
    /unavailable/i
  );
});

test('dto to ImagePack address', () => {
  const pack = imagePackFromNativeDto({
    id: '$e',
    roomId: '!r:example.org',
    stateKey: 'key',
    content: { pack: { display_name: 'X' }, images: {} },
  });
  assert.equal(pack.address?.roomId, '!r:example.org');
  assert.equal(pack.address?.stateKey, 'key');
});
