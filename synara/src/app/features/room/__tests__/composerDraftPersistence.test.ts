import assert from 'node:assert/strict';
import test from 'node:test';

import {
  createCoalescingScheduler,
  DRAFT_SAVE_DEBOUNCE_MS,
  DRAFT_SAVE_MAX_WAIT_MS,
  type SchedulerClock,
} from '../coalescingScheduler';
import { createComposerDraftPersistence, type DraftTarget } from '../composerDraftPersistence';

/** A manual clock: timers run only when the test advances time. */
const fakeClock = () => {
  let now = 0;
  let nextId = 1;
  const timers = new Map<number, { at: number; run: () => void }>();
  const clock: SchedulerClock = {
    now: () => now,
    setTimeout: (run, ms) => {
      const id = nextId;
      nextId += 1;
      timers.set(id, { at: now + ms, run });
      return id;
    },
    clearTimeout: (handle) => {
      timers.delete(handle as number);
    },
  };
  const advance = (ms: number) => {
    const end = now + ms;
    for (;;) {
      let due: [number, { at: number; run: () => void }] | undefined;
      timers.forEach((timer, id) => {
        if (timer.at <= end && (!due || timer.at < due[1].at)) due = [id, timer];
      });
      if (!due) break;
      timers.delete(due[0]);
      now = due[1].at;
      due[1].run();
    }
    now = end;
  };
  return { clock, advance, pendingTimers: () => timers.size };
};

type Write =
  | { kind: 'save'; draftKey: string; text: string; urgent: boolean; at: number }
  | { kind: 'clear'; draftKey: string; at: number };

/** A composer value; held by reference like Slate's tree. */
type Value = { text: string };

const target = (roomId: string, thread?: string): DraftTarget => ({
  draftKey: thread ? `${roomId}#thread:${thread}` : roomId,
  roomId,
  slot: thread ? `thread:${thread}` : 'main',
});

const setup = () => {
  const { clock, advance, pendingTimers } = fakeClock();
  const writes: Write[] = [];
  const persistence = createComposerDraftPersistence<Value>({
    clock,
    writer: {
      save: (t, value, urgent) =>
        writes.push({
          kind: 'save',
          draftKey: t.draftKey,
          text: value.text,
          urgent,
          at: clock.now(),
        }),
      clear: (t) => writes.push({ kind: 'clear', draftKey: t.draftKey, at: clock.now() }),
    },
  });
  /** Type `text` one character per `gapMs`, as separate changes. */
  const type = (into: DraftTarget, text: string, gapMs: number, from = '') => {
    let value = from;
    for (const char of text) {
      value += char;
      persistence.change(into, { text: value });
      advance(gapMs);
    }
    return value;
  };
  return { persistence, writes, advance, pendingTimers, type };
};

const A = target('!a:example.org');
const B = target('!b:example.org');

test('the debounce sits between keystrokes and a pause, with a max-wait above it', () => {
  assert.equal(DRAFT_SAVE_DEBOUNCE_MS, 400);
  assert.ok(DRAFT_SAVE_MAX_WAIT_MS > DRAFT_SAVE_DEBOUNCE_MS);
  assert.ok(DRAFT_SAVE_MAX_WAIT_MS <= 5_000);
});

test('rapid typing writes once, after the pause, with the last text', () => {
  const { writes, advance, type, pendingTimers } = setup();
  type(A, 'hello', 100);
  assert.equal(writes.length, 0, 'nothing is written per keystroke');
  advance(DRAFT_SAVE_DEBOUNCE_MS);
  assert.equal(writes.length, 1);
  assert.deepEqual(
    { ...writes[0], at: undefined },
    { kind: 'save', draftKey: A.draftKey, text: 'hello', urgent: false, at: undefined }
  );
  // 'o' landed at 400 ms; the write follows 400 ms of quiet.
  assert.equal(writes[0].at, 400 + DRAFT_SAVE_DEBOUNCE_MS);
  assert.equal(pendingTimers(), 0);
  advance(5_000);
  assert.equal(writes.length, 1, 'no further writes without changes');
});

test('the scheduler arms one timer per burst, not one per change', () => {
  const { clock, pendingTimers } = fakeClock();
  let armed = 0;
  const counting: SchedulerClock = {
    ...clock,
    setTimeout: (run, ms) => {
      armed += 1;
      return clock.setTimeout(run, ms);
    },
  };
  const scheduler = createCoalescingScheduler<string>({
    delayMs: 400,
    clock: counting,
    write: () => undefined,
  });
  for (let i = 0; i < 50; i += 1) scheduler.schedule('k', `v${i}`);
  assert.equal(armed, 1);
  assert.equal(pendingTimers(), 1);
});

test('continuous typing still writes at the max-wait', () => {
  const { writes, advance, type } = setup();
  // 6 s of steady typing, never pausing for the debounce.
  type(A, 'x'.repeat(60), 100);
  const saves = writes.filter((w) => w.kind === 'save');
  assert.ok(saves.length >= 2, `expected periodic saves, got ${saves.length}`);
  saves.forEach((save, i) => {
    const previous = i === 0 ? 0 : saves[i - 1].at;
    assert.ok(save.at - previous <= DRAFT_SAVE_MAX_WAIT_MS, `gap ${save.at - previous}`);
  });
  advance(DRAFT_SAVE_DEBOUNCE_MS);
  const last = writes[writes.length - 1];
  assert.equal(last.kind === 'save' && last.text, 'x'.repeat(60));
});

test('a room switch writes the pending draft to the room it was typed in', () => {
  const { persistence, writes, advance, type } = setup();
  const typed = type(A, 'for a', 50);
  // The composer leaves A (its cleanup), then opens B and B's draft loads.
  persistence.leave(A, { text: typed });
  persistence.change(B, { text: 'b draft' });
  assert.deepEqual(
    writes.map((w) => ({ ...w, at: undefined })),
    [{ kind: 'save', draftKey: A.draftKey, text: 'for a', urgent: true, at: undefined }]
  );
  advance(DRAFT_SAVE_DEBOUNCE_MS);
  assert.deepEqual(
    writes.map((w) => w.draftKey),
    [A.draftKey, B.draftKey]
  );
  assert.equal(writes.filter((w) => w.draftKey === A.draftKey).length, 1);
});

test('a change for another key writes the pending one to its own key first', () => {
  const { persistence, writes } = setup();
  const thread = target('!a:example.org', '$root');
  persistence.change(A, { text: 'room' });
  persistence.change(thread, { text: 'thread' });
  assert.deepEqual(
    writes.map((w) => w.kind === 'save' && [w.draftKey, w.text]),
    [[A.draftKey, 'room']]
  );
});

test('send clears at once and a pending save never brings the text back', () => {
  const { persistence, writes, advance, type } = setup();
  type(A, 'secret', 50);
  // Send: the editor is emptied while the save is still pending.
  persistence.change(A, undefined);
  assert.deepEqual(
    writes.map((w) => w.kind),
    ['clear']
  );
  advance(DRAFT_SAVE_MAX_WAIT_MS * 3);
  assert.deepEqual(
    writes.map((w) => w.kind),
    ['clear'],
    'the old text is not saved after the clear'
  );
  // Leaving the room afterwards keeps it cleared.
  persistence.leave(A, undefined);
  assert.ok(writes.every((w) => w.kind === 'clear'));
});

test('a repeated empty change does not clear again', () => {
  const { persistence, writes } = setup();
  persistence.change(A, undefined);
  persistence.change(A, undefined);
  assert.equal(writes.length, 1);
  // Text typed and erased before it was written: still nothing to clear.
  persistence.change(A, { text: 'x' });
  persistence.change(A, undefined);
  assert.equal(writes.length, 1);
});

test('typing after a send saves the new text normally', () => {
  const { persistence, writes, advance, type } = setup();
  type(A, 'one', 50);
  persistence.change(A, undefined);
  type(A, 'two', 50);
  advance(DRAFT_SAVE_DEBOUNCE_MS);
  assert.deepEqual(
    writes.map((w) => (w.kind === 'save' ? w.text : 'clear')),
    ['clear', 'two']
  );
});

test('unmount writes the pending draft at once', () => {
  const { persistence, writes, advance, type } = setup();
  const typed = type(A, 'unsent', 50);
  persistence.leave(A, { text: typed });
  assert.equal(writes.length, 1);
  assert.equal(writes[0].kind === 'save' && writes[0].urgent, true);
  advance(DRAFT_SAVE_MAX_WAIT_MS);
  assert.equal(writes.length, 1, 'nothing is written twice');
});

test('leaving writes a change the editor made without a change event', () => {
  const { persistence, writes, advance } = setup();
  const value = { text: 'abc' };
  persistence.change(A, value);
  advance(DRAFT_SAVE_DEBOUNCE_MS);
  assert.equal(writes.length, 1);
  // The same tree as written: leaving writes nothing.
  persistence.leave(A, value);
  assert.equal(writes.length, 1);
  // Content the change handler never saw is written on leave.
  persistence.leave(A, { text: 'abcd' });
  assert.equal(writes.length, 2);
});

test('leaving an emptied draft clears it', () => {
  const { persistence, writes, type } = setup();
  type(A, 'gone', 50);
  persistence.leave(A, undefined);
  assert.deepEqual(
    writes.map((w) => w.kind),
    ['clear']
  );
});

test('window blur or hide writes what is pending', () => {
  const { persistence, writes, type } = setup();
  type(A, 'blur', 50);
  persistence.flush();
  assert.equal(writes.length, 1);
  assert.equal(writes[0].kind === 'save' && writes[0].urgent, true);
  persistence.flush();
  assert.equal(writes.length, 1);
});

test('a disposed scheduler writes later changes at once', () => {
  const { clock } = fakeClock();
  const written: string[] = [];
  const scheduler = createCoalescingScheduler<string>({
    delayMs: 400,
    clock,
    write: (_key, value) => written.push(value),
  });
  scheduler.schedule('k', 'pending');
  scheduler.dispose();
  assert.deepEqual(written, ['pending']);
  scheduler.schedule('k', 'late');
  assert.deepEqual(written, ['pending', 'late']);
});
