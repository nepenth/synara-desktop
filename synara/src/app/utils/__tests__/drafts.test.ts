import assert from 'node:assert/strict';
import test from 'node:test';
import { Descendant } from 'slate';
import {
  clearRoomDraft,
  getRoomDraftStorageKey,
  loadRoomDraft,
  normalizeStoredRoomDraft,
  saveRoomDraft,
} from '../drafts';

const createStorage = () => {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => {
      values.set(key, value);
    },
    removeItem: (key: string) => {
      values.delete(key);
    },
  };
};

const draftValue = [{ type: 'paragraph', children: [{ text: 'hello' }] }] as Descendant[];

test('room drafts persist per user and room', () => {
  const storage = createStorage();

  assert.equal(
    saveRoomDraft(storage, '@me:example.org', '!room:example.org', draftValue, 123),
    true
  );
  assert.deepEqual(loadRoomDraft(storage, '@me:example.org', '!room:example.org'), draftValue);
  assert.equal(loadRoomDraft(storage, '@me:example.org', '!other:example.org'), undefined);
});

test('room drafts ignore malformed and oversized payloads', () => {
  const storage = createStorage();
  const key = getRoomDraftStorageKey('@me:example.org', '!room:example.org');
  storage.setItem(key, '{bad json');
  assert.equal(loadRoomDraft(storage, '@me:example.org', '!room:example.org'), undefined);

  storage.setItem(key, 'x'.repeat(70 * 1024));
  assert.equal(loadRoomDraft(storage, '@me:example.org', '!room:example.org'), undefined);

  assert.equal(
    normalizeStoredRoomDraft({ version: 2, updatedAt: 1, value: draftValue }),
    undefined
  );
});

test('clearRoomDraft removes persisted drafts', () => {
  const storage = createStorage();
  saveRoomDraft(storage, '@me:example.org', '!room:example.org', draftValue, 123);
  clearRoomDraft(storage, '@me:example.org', '!room:example.org');
  assert.equal(loadRoomDraft(storage, '@me:example.org', '!room:example.org'), undefined);
});
