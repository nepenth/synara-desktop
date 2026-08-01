import assert from 'node:assert/strict';
import { ImagePack } from '../../../plugins/custom-emoji/ImagePack';
import {
  getGlobalImagePacksWithNativeOwner,
  getRoomImagePacksWithNativeOwner,
  getUserImagePackWithNativeOwner,
  imagePackFromNativeDto,
  isNativePackReadSession,
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
  return { available: false };
};

const failClosedInvoke: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  return { available: false };
};

export async function testLegacyWhenNotDesktop() {
  assert.equal(await isNativePackReadSession(false, loggedInInvoke), false);
  assert.equal(await getUserImagePackWithNativeOwner(false, loggedInInvoke), 'legacy');
  assert.equal(await getGlobalImagePacksWithNativeOwner(false, loggedInInvoke), 'legacy');
  assert.equal(
    await getRoomImagePacksWithNativeOwner('!r:example.org', false, loggedInInvoke),
    'legacy',
  );
}

export async function testNativeUserPack() {
  const pack = await getUserImagePackWithNativeOwner(true, loggedInInvoke);
  assert.ok(pack instanceof ImagePack);
  assert.equal((pack as ImagePack).id, '@u:example.org');
  assert.equal((pack as ImagePack).meta.name, 'Me');
}

export async function testNativeRoomAndGlobalPacks() {
  const room = await getRoomImagePacksWithNativeOwner('!r:example.org', true, loggedInInvoke);
  assert.ok(Array.isArray(room));
  assert.equal((room as ImagePack[]).length, 1);
  assert.equal((room as ImagePack[])[0].meta.name, 'Room');

  const global = await getGlobalImagePacksWithNativeOwner(true, loggedInInvoke);
  assert.ok(Array.isArray(global));
  assert.equal((global as ImagePack[]).length, 1);
  assert.equal((global as ImagePack[])[0].address?.stateKey, 'global');
}

export async function testFailClosedWhenCommandMissing() {
  await assert.rejects(
    () => getUserImagePackWithNativeOwner(true, failClosedInvoke),
    /unavailable/i,
  );
  await assert.rejects(
    () => getGlobalImagePacksWithNativeOwner(true, failClosedInvoke),
    /unavailable/i,
  );
}

export function testDtoToImagePackAddress() {
  const pack = imagePackFromNativeDto({
    id: '$e',
    roomId: '!r:example.org',
    stateKey: 'key',
    content: { pack: { display_name: 'X' }, images: {} },
  });
  assert.equal(pack.address?.roomId, '!r:example.org');
  assert.equal(pack.address?.stateKey, 'key');
}
