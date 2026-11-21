import test from 'node:test';
import assert from 'node:assert/strict';
import { directChatResolver } from '../directChat';

const BOB = '@bob:example.org';

test('Message opens the existing direct room and creates nothing', async () => {
  const created: string[] = [];
  const resolve = directChatResolver({
    findExisting: (userId) => (userId === BOB ? '!existing:example.org' : undefined),
    create: async (userId) => {
      created.push(userId);
      return '!new:example.org';
    },
  });

  assert.equal(await resolve(BOB), '!existing:example.org');
  assert.deepEqual(created, []);
});

test('Message creates a direct room when there is none', async () => {
  const created: string[] = [];
  const resolve = directChatResolver({
    findExisting: () => undefined,
    create: async (userId) => {
      created.push(userId);
      return '!new:example.org';
    },
  });

  assert.equal(await resolve(BOB), '!new:example.org');
  assert.deepEqual(created, [BOB]);
});

test('a second click while the room is being created does not make a second room', async () => {
  let finish: (roomId: string) => void = () => undefined;
  let creations = 0;
  const resolve = directChatResolver({
    findExisting: () => undefined,
    create: () => {
      creations += 1;
      return new Promise<string>((done) => {
        finish = done;
      });
    },
  });

  const first = resolve(BOB);
  const second = resolve(BOB);
  finish('!new:example.org');

  assert.equal(await first, '!new:example.org');
  assert.equal(await second, '!new:example.org');
  assert.equal(creations, 1);
});

test('a failed creation can be tried again', async () => {
  let attempts = 0;
  const resolve = directChatResolver({
    findExisting: () => undefined,
    create: async () => {
      attempts += 1;
      if (attempts === 1) throw new Error('offline');
      return '!new:example.org';
    },
  });

  await assert.rejects(resolve(BOB), /offline/);
  assert.equal(await resolve(BOB), '!new:example.org');
  assert.equal(attempts, 2);
});
