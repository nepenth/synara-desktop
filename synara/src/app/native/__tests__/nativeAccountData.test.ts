import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../utils/desktop';
import {
  getCachedAccountData,
  loadAllNativeAccountData,
  loadNativeAccountData,
  setNativeAccountData,
  setNativeAccountDataInvokeForTests,
} from '../nativeAccountData';
import { getAccountData } from '../../utils/room';
import { AccountDataEvent } from '../../../types/matrix/accountData';
import { nextRecentEmoji } from '../../plugins/recent-emoji';
import { makeSynaraSpacesContent } from '../../hooks/useSidebarItems';

const ok = (value: unknown): DesktopInvokeResult<unknown> => ({ available: true, value });

const store = () => {
  const data = new Map<string, Record<string, unknown>>();
  const writes: Array<Record<string, unknown> | undefined> = [];
  setNativeAccountDataInvokeForTests(async (command, args) => {
    const key = `${(args?.roomId as string) ?? ''}|${args?.eventType as string}`;
    if (command === 'matrix_account_data_get') {
      return ok({ eventType: args?.eventType, content: data.get(key) ?? null });
    }
    if (command === 'matrix_account_data_set') {
      writes.push(args);
      data.set(key, args?.content as Record<string, unknown>);
      return ok({ eventType: args?.eventType, content: args?.content });
    }
    if (command === 'matrix_account_data_types') {
      const prefix = `${(args?.roomId as string) ?? ''}|`;
      return ok({
        types: [...data.keys()]
          .filter((k) => k.startsWith(prefix))
          .map((k) => k.slice(prefix.length)),
      });
    }
    return { available: false };
  });
  return { data, writes };
};

test('getAccountData starts a load, then returns cached content as an event reading', async () => {
  const { data } = store();
  data.set('|in.synara.spaces', { sidebar: ['!s:x'] });
  assert.equal(getAccountData(AccountDataEvent.SynaraSpaces), undefined);
  await loadNativeAccountData(AccountDataEvent.SynaraSpaces);
  const reading = getAccountData(AccountDataEvent.SynaraSpaces);
  assert.equal(reading?.getType(), 'in.synara.spaces');
  assert.deepEqual(reading?.getContent(), { sidebar: ['!s:x'] });
});

test('writes update the cache at once and keep unrelated keys of the object', async () => {
  const { writes } = store();
  await setNativeAccountData(AccountDataEvent.SynaraSpaces, { shortcut: ['!old:x'], extra: 1 });
  const content = makeSynaraSpacesContent(['!a:x']);
  assert.deepEqual(content, { shortcut: ['!old:x'], extra: 1, sidebar: ['!a:x'] });
  await setNativeAccountData(AccountDataEvent.SynaraSpaces, content);
  assert.deepEqual(getCachedAccountData(AccountDataEvent.SynaraSpaces), content);
  assert.equal(writes.length, 2);
});

test('room account data is listed and loaded per room', async () => {
  const { data } = store();
  data.set('!r:x|org.example.pref', { color: 'blue' });
  data.set('|org.example.global', { on: true });
  const roomData = await loadAllNativeAccountData('!r:x');
  assert.deepEqual([...roomData.entries()], [['org.example.pref', { color: 'blue' }]]);
});

test('recent emoji moves a used emoji to the front, counts it and drops malformed entries', () => {
  assert.deepEqual(nextRecentEmoji(undefined, '😀'), [['😀', 1]]);
  assert.deepEqual(nextRecentEmoji([['👍', 3], ['😀', 2], ['bad']], '😀'), [
    ['😀', 3],
    ['👍', 3],
  ]);
  const many = Array.from({ length: 120 }, (_, i) => [`e${i}`, 1]);
  assert.equal(nextRecentEmoji(many, 'new').length, 100);
});
