import assert from 'node:assert/strict';
import test from 'node:test';
import {
  completeRoomTodoItem,
  createManualRoomNoteItem,
  getRoomNoteItems,
  getRoomNotesSummary,
  moveRoomTodoItem,
  normalizeRoomNotesContent,
  putRoomNoteItem,
  rankRoomNoteItem,
  removeRoomNoteItem,
} from '../roomNotes';

test('room notes normalize malformed account data', () => {
  const content = normalizeRoomNotesContent({
    rooms: {
      '!room:example.org': {
        items: {
          good: {
            id: 'good',
            kind: 'note',
            roomId: '!room:example.org',
            body: 'keep this',
            createdAt: 1,
            updatedAt: 2,
          },
          wrongRoom: {
            id: 'wrongRoom',
            kind: 'todo',
            roomId: '!other:example.org',
            body: 'drop this',
            createdAt: 1,
            updatedAt: 2,
          },
          empty: {
            id: 'empty',
            kind: 'note',
            roomId: '!room:example.org',
            body: '',
            createdAt: 1,
            updatedAt: 2,
          },
        },
      },
    },
  });

  assert.deepEqual(Object.keys(content.rooms?.['!room:example.org']?.items ?? {}), ['good']);
});

test('room notes reorder ToDo items within active and completed groups', () => {
  const first = createManualRoomNoteItem('!room:example.org', 'todo', 'first', 100);
  const second = createManualRoomNoteItem('!room:example.org', 'todo', 'second', 200);
  const third = createManualRoomNoteItem('!room:example.org', 'todo', 'third', 300);
  assert.ok(first);
  assert.ok(second);
  assert.ok(third);

  const content = putRoomNoteItem(
    putRoomNoteItem(putRoomNoteItem(undefined, first), second),
    third
  );
  assert.deepEqual(
    getRoomNoteItems(content, '!room:example.org').map((item) => item.body),
    ['third', 'second', 'first']
  );

  const moved = moveRoomTodoItem(content, '!room:example.org', first.id, 'up', 400);
  assert.deepEqual(
    getRoomNoteItems(moved, '!room:example.org').map((item) => item.body),
    ['third', 'first', 'second']
  );

  const completed = completeRoomTodoItem(moved, '!room:example.org', first.id, true, 500);
  const movedCompleted = moveRoomTodoItem(completed, '!room:example.org', first.id, 'up', 600);
  assert.deepEqual(
    getRoomNoteItems(movedCompleted, '!room:example.org').map((item) => item.body),
    ['third', 'second', 'first']
  );
});

test('room notes assign a stable fractional rank when moving an ordinary note', () => {
  const first = createManualRoomNoteItem('!room:example.org', 'note', 'first', 100);
  const second = createManualRoomNoteItem('!room:example.org', 'note', 'second', 200);
  const third = createManualRoomNoteItem('!room:example.org', 'note', 'third', 300);
  assert.ok(first);
  assert.ok(second);
  assert.ok(third);

  const roomItems = [third, second, first];
  const ranked = rankRoomNoteItem(roomItems, first.id, 'up');
  assert.ok(ranked);
  assert.equal(ranked.updatedAt, first.updatedAt);
  assert.equal(ranked.order, 250);

  const content = putRoomNoteItem(
    putRoomNoteItem(putRoomNoteItem(undefined, third), second),
    ranked
  );
  assert.deepEqual(
    getRoomNoteItems(content, '!room:example.org').map((item) => item.body),
    ['third', 'first', 'second']
  );
});

test('room notes add, sort, complete, and remove items per room', () => {
  const note = createManualRoomNoteItem('!room:example.org', 'note', 'phrase to remember', 100);
  const todo = createManualRoomNoteItem('!room:example.org', 'todo', 'follow up', 200);
  assert.ok(note);
  assert.ok(todo);

  const content = putRoomNoteItem(putRoomNoteItem(undefined, note), todo);
  assert.deepEqual(
    getRoomNoteItems(content, '!room:example.org').map((item) => item.id),
    [todo.id, note.id]
  );
  assert.deepEqual(getRoomNotesSummary(content, '!room:example.org'), {
    total: 2,
    activeTodos: 1,
  });

  const completed = completeRoomTodoItem(content, '!room:example.org', todo.id, true, 300);
  assert.equal(completed.rooms?.['!room:example.org']?.items?.[todo.id]?.completedAt, 300);
  assert.equal(getRoomNotesSummary(completed, '!room:example.org').activeTodos, 0);

  const removed = removeRoomNoteItem(completed, '!room:example.org', note.id);
  assert.deepEqual(
    getRoomNoteItems(removed, '!room:example.org').map((item) => item.id),
    [todo.id]
  );
});
