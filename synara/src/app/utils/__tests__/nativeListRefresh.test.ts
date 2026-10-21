import assert from 'node:assert/strict';
import test from 'node:test';
import type { DesktopInvokeResult } from '../desktop';
import {
  createCoalescedRefresh,
  createSharedNativeRead,
  keepIfSameJson,
} from '../nativeListRefresh';

const deferred = () => {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => {
    resolve = done;
  });
  return { promise, resolve };
};

test('D8: a wake during a refresh runs one more refresh after it', async () => {
  const gates = [deferred(), deferred()];
  let runs = 0;
  const refresh = createCoalescedRefresh(async () => {
    const gate = gates[runs];
    runs += 1;
    await gate?.promise;
  });

  const first = refresh();
  // A burst of wakes while the first read is in flight.
  const second = refresh();
  const third = refresh();
  assert.equal(runs, 1, 'never two reads at once');
  assert.equal(second, first, 'callers share the run that will observe them');
  assert.equal(third, first);

  gates[0].resolve();
  await new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
  assert.equal(runs, 2, 'the dropped wake is read once, not once per wake');
  gates[1].resolve();
  await first;
  assert.equal(runs, 2);

  await refresh();
  assert.equal(runs, 3, 'an idle refresh runs at once');
});

test('D8: a failing run still re-runs for a pending wake, and disposal stops it', async () => {
  let runs = 0;
  let disposed = false;
  const gate = deferred();
  const refresh = createCoalescedRefresh(
    async () => {
      runs += 1;
      if (runs === 1) {
        await gate.promise;
        throw new Error('transient');
      }
    },
    () => disposed
  );
  const first = refresh();
  void refresh();
  gate.resolve();
  await first;
  assert.equal(runs, 2);

  const blocked = deferred();
  const again = createCoalescedRefresh(
    async () => {
      runs += 1;
      await blocked.promise;
    },
    () => disposed
  );
  const pending = again();
  void again();
  disposed = true;
  blocked.resolve();
  await pending;
  assert.equal(runs, 3, 'no re-run after disposal');
  await again();
  assert.equal(runs, 3);
});

test('D10: concurrent session reads share one IPC call', async () => {
  const calls: string[] = [];
  const gate = deferred();
  const read = createSharedNativeRead(async (command) => {
    calls.push(command);
    await gate.promise;
    return { available: true, value: calls.length } as DesktopInvokeResult<unknown>;
  });
  const reads = [
    read('matrix_session_snapshot'),
    read('matrix_session_snapshot'),
    read('matrix_sync_status'),
  ];
  gate.resolve();
  const results = await Promise.all(reads);
  assert.deepEqual(calls, ['matrix_session_snapshot', 'matrix_sync_status']);
  assert.equal(results[0], results[1]);

  await read('matrix_session_snapshot');
  assert.equal(calls.length, 3, 'a later read is a fresh call');
});

test('D10: an equal readback keeps the current value', () => {
  const current = { invites: [{ roomId: '!a' }] };
  assert.equal(keepIfSameJson(current, { invites: [{ roomId: '!a' }] }), current);
  const changed = { invites: [{ roomId: '!b' }] };
  assert.equal(keepIfSameJson(current, changed), changed);
});
