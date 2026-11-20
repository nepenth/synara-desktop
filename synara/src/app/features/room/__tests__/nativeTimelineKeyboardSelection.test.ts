import assert from 'node:assert/strict';
import test from 'node:test';
import {
  adjacentSelectableRow,
  composerArrowUpLeavesForTimeline,
  focusLastOwnTimelineMessage,
  focusTimelineComposer,
  lastOwnMessageTarget,
  registerTimelineComposerFocus,
  registerTimelineLastOwnMessageFocus,
  timelineMessageLabel,
  timelineSelectionKey,
  type TimelineSelectionRow,
} from '../nativeTimelineKeyboardSelection';

const key = (
  value: string,
  modifiers: Partial<Record<'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey', boolean>> = {}
) => ({ key: value, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...modifiers });

const idle = { empty: true, suggestionsOpen: false, composing: false };
const theirs: TimelineSelectionRow = { selectable: true, ownMessage: false, sending: false };
const mine: TimelineSelectionRow = { selectable: true, ownMessage: true, sending: false };
const sending: TimelineSelectionRow = { selectable: false, ownMessage: true, sending: true };
const failed: TimelineSelectionRow = { selectable: false, ownMessage: true, sending: false };
const marker: TimelineSelectionRow = { selectable: false, ownMessage: false, sending: false };

test('Up leaves the message box only when it is empty and nothing else wants the key', () => {
  assert.equal(composerArrowUpLeavesForTimeline(key('ArrowUp'), idle), true);
  // A draft keeps normal caret movement.
  assert.equal(composerArrowUpLeavesForTimeline(key('ArrowUp'), { ...idle, empty: false }), false);
  assert.equal(
    composerArrowUpLeavesForTimeline(key('ArrowUp'), { ...idle, suggestionsOpen: true }),
    false
  );
  assert.equal(
    composerArrowUpLeavesForTimeline(key('ArrowUp'), { ...idle, composing: true }),
    false
  );
  // Alt+Up is the previous room; Shift+Up extends a selection.
  for (const modifier of ['altKey', 'shiftKey', 'ctrlKey', 'metaKey'] as const) {
    assert.equal(
      composerArrowUpLeavesForTimeline(key('ArrowUp', { [modifier]: true }), idle),
      false
    );
  }
  assert.equal(composerArrowUpLeavesForTimeline(key('ArrowDown'), idle), false);
});

test('a selected message takes plain Up, Down, Escape and E only', () => {
  assert.equal(timelineSelectionKey(key('ArrowUp')), 'previous');
  assert.equal(timelineSelectionKey(key('ArrowDown')), 'next');
  assert.equal(timelineSelectionKey(key('Escape')), 'composer');
  assert.equal(timelineSelectionKey(key('e')), 'edit');
  assert.equal(timelineSelectionKey(key('E')), 'edit');
  // Cmd/Ctrl+E is the composer's code key, and Shift+E is typing.
  assert.equal(timelineSelectionKey(key('e', { metaKey: true })), undefined);
  assert.equal(timelineSelectionKey(key('e', { ctrlKey: true })), undefined);
  assert.equal(timelineSelectionKey(key('E', { shiftKey: true })), undefined);
  assert.equal(timelineSelectionKey(key('ArrowUp', { altKey: true })), undefined);
  assert.equal(timelineSelectionKey(key('r')), undefined);
});

test('Up selects my newest message, never an older one while the newest is still sending', () => {
  assert.deepEqual(lastOwnMessageTarget([mine, theirs, mine, theirs, marker]), {
    kind: 'row',
    index: 2,
  });
  assert.deepEqual(lastOwnMessageTarget([theirs, marker]), { kind: 'none' });
  assert.deepEqual(lastOwnMessageTarget([]), { kind: 'none' });
  // The message just sent has no event yet: wait for it rather than open the one before.
  assert.deepEqual(lastOwnMessageTarget([mine, theirs, sending]), { kind: 'wait' });
  // A message that failed to send does not block the ones that were sent.
  assert.deepEqual(lastOwnMessageTarget([mine, theirs, failed]), { kind: 'row', index: 0 });
});

test('Up and Down step over rows that cannot take focus and stop at the ends', () => {
  const rows = [theirs, marker, mine, failed, theirs];
  assert.equal(adjacentSelectableRow(rows, 2, 'previous'), 0);
  assert.equal(adjacentSelectableRow(rows, 2, 'next'), 4);
  assert.equal(adjacentSelectableRow(rows, 0, 'previous'), undefined);
  assert.equal(adjacentSelectableRow(rows, 4, 'next'), undefined);
});

test('a room and a thread in it each answer only for their own timeline', () => {
  const calls: string[] = [];
  const offRoom = registerTimelineLastOwnMessageFocus('!r', undefined, () => {
    calls.push('room');
    return true;
  });
  const offThread = registerTimelineLastOwnMessageFocus('!r', '$root', () => {
    calls.push('thread');
    return true;
  });
  const offComposer = registerTimelineComposerFocus('!r', '$root', () => calls.push('box'));

  assert.equal(focusLastOwnTimelineMessage('!r', '$root'), true);
  assert.equal(focusLastOwnTimelineMessage('!r', undefined), true);
  assert.equal(focusLastOwnTimelineMessage('!r', '$other'), false);
  assert.equal(focusLastOwnTimelineMessage('!elsewhere', undefined), false);
  assert.equal(focusTimelineComposer('!r', '$root'), true);
  // A room with no message box (read-only): the key keeps its default.
  assert.equal(focusTimelineComposer('!r', undefined), false);
  assert.deepEqual(calls, ['thread', 'room', 'box']);

  offRoom();
  offThread();
  offComposer();
  assert.equal(focusLastOwnTimelineMessage('!r', undefined), false);
  assert.equal(focusTimelineComposer('!r', '$root'), false);
});

test('the newest mounted view answers, and unmounting it hands back to the one before', () => {
  const offFirst = registerTimelineLastOwnMessageFocus('!r', undefined, () => false);
  const offSecond = registerTimelineLastOwnMessageFocus('!r', undefined, () => true);
  assert.equal(focusLastOwnTimelineMessage('!r', undefined), true);
  offSecond();
  assert.equal(focusLastOwnTimelineMessage('!r', undefined), false);
  offFirst();
});

test('a focused message is announced as who wrote what, on one line and not at full length', () => {
  assert.equal(timelineMessageLabel('Ada', 'Ship it\n  today'), 'Ada: Ship it today');
  assert.equal(timelineMessageLabel('Ada', ''), 'Message from Ada');
  assert.equal(timelineMessageLabel('Ada', undefined), 'Message from Ada');
  const long = timelineMessageLabel('Ada', 'word '.repeat(100));
  assert.ok(long.endsWith('…'));
  assert.ok(long.length <= 'Ada: '.length + 161);
});
