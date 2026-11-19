import assert from 'node:assert/strict';
import test from 'node:test';
import {
  createRoomFolderWithNativeOwner,
  deleteRoomFolderWithNativeOwner,
  isRoomFolderReadOnlyError,
  isValidRoomFolderName,
  moveRoomFolderWithNativeOwner,
  normalizeRoomFolderName,
  renameRoomFolderWithNativeOwner,
  roomFolderDiagnosticId,
  roomFolderErrorMessage,
  setRoomFolderWithNativeOwner,
  snapshotRoomFoldersWithNativeOwner,
  type NativeRoomFolderInvoke,
} from '../nativeRoomFolderOwner';

const snapshot = (readOnly = false) => ({
  sessionGeneration: 1,
  version: readOnly ? 2 : 1,
  readOnly,
  folders: [{ id: 'f1', name: 'Work', roomIds: ['!a:example.test'] }],
});

const recorder = (value: unknown = snapshot()) => {
  const calls: Array<[string, Record<string, unknown> | undefined]> = [];
  const invoke: NativeRoomFolderInvoke = async (command, args) => {
    calls.push([command, args]);
    return { available: true, value };
  };
  return { calls, invoke };
};

test('each mutation is one native command with camelCase arguments', async () => {
  const { calls, invoke } = recorder();
  await snapshotRoomFoldersWithNativeOwner(invoke);
  await renameRoomFolderWithNativeOwner('f1', '  Side   projects ', invoke);
  await deleteRoomFolderWithNativeOwner('f1', invoke);
  await moveRoomFolderWithNativeOwner('f1', 2, invoke);
  await setRoomFolderWithNativeOwner('!a:example.test', 'f1', invoke);
  await setRoomFolderWithNativeOwner('!a:example.test', null, invoke);
  assert.deepEqual(calls, [
    ['matrix_room_folders_snapshot', undefined],
    // The name is normalized as Core does before it is sent.
    ['matrix_room_folders_rename', { folderId: 'f1', name: 'Side projects' }],
    ['matrix_room_folders_delete', { folderId: 'f1' }],
    ['matrix_room_folders_move', { folderId: 'f1', toIndex: 2 }],
    ['matrix_room_folders_set_room', { roomId: '!a:example.test', folderId: 'f1' }],
    // Unfiling sends an explicit null, not a missing field.
    ['matrix_room_folders_set_room', { roomId: '!a:example.test', folderId: null }],
  ]);
});

test('create files the room in the same command and returns the new id', async () => {
  const { calls, invoke } = recorder({ folderId: 'new', snapshot: snapshot() });
  const result = await createRoomFolderWithNativeOwner('Work', '!a:example.test', invoke);
  assert.equal(result.folderId, 'new');
  assert.equal(result.snapshot.folders[0].name, 'Work');
  await createRoomFolderWithNativeOwner('Empty', undefined, invoke);
  assert.deepEqual(calls, [
    ['matrix_room_folders_create', { name: 'Work', roomId: '!a:example.test' }],
    ['matrix_room_folders_create', { name: 'Empty' }],
  ]);
});

test('an unavailable shell or a malformed reply fails closed', async () => {
  const unavailable: NativeRoomFolderInvoke = async () => ({ available: false });
  await assert.rejects(snapshotRoomFoldersWithNativeOwner(unavailable), /unavailable/);
  await assert.rejects(
    setRoomFolderWithNativeOwner('!a:example.test', 'f1', recorder({ folders: 'no' }).invoke),
    /unavailable/
  );
  await assert.rejects(
    createRoomFolderWithNativeOwner('Work', undefined, recorder(snapshot()).invoke),
    /unavailable/
  );
  await assert.rejects(moveRoomFolderWithNativeOwner('f1', -1, recorder().invoke), /position/);
});

test('native rejections pass through unchanged so their diagnostic id can be read', async () => {
  const rejection = {
    code: 'InvalidRequest',
    message: 'raw',
    diagnosticId: 'v-room-folders.too-many-folders',
  };
  const invoke: NativeRoomFolderInvoke = async () => {
    throw rejection;
  };
  await assert.rejects(createRoomFolderWithNativeOwner('Work', undefined, invoke), (error) => {
    assert.equal(error, rejection);
    return true;
  });
});

test('every folder error id maps to plain sentence-case copy, never the raw message', () => {
  const ids = [
    'invalid-name',
    'too-many-folders',
    'content-too-large',
    'unsupported-version',
    'folder-not-found',
    'room-is-direct',
    'room-is-space',
    'room-not-joined',
  ];
  const messages = ids.map((id) =>
    roomFolderErrorMessage({ diagnosticId: `v-room-folders.${id}`, message: 'secret detail' })
  );
  assert.equal(new Set(messages).size, ids.length);
  for (const message of messages) {
    assert.match(message, /^[A-Z]/);
    // Sentence case: no Title Case Words beyond the first (Synara is a name).
    assert.equal(/[a-z,] (?!Synara\b)[A-Z]/.test(message), false, message);
    assert.ok(message.endsWith('.'), message);
    assert.equal(message.includes('secret'), false);
  }
  assert.equal(
    roomFolderErrorMessage({ diagnosticId: 'v-room-folders.too-many-folders' }),
    'You can have up to 100 folders. Delete one to add another.'
  );
  assert.equal(
    roomFolderErrorMessage({ diagnosticId: 'v-room-folders.invalid-name' }),
    'Folder names need 1 to 64 characters.'
  );
  assert.equal(roomFolderErrorMessage(new Error('boom')), "Couldn't save your folders. Try again.");
  assert.equal(roomFolderDiagnosticId({ diagnosticId: 'v-send.other' }), undefined);
  assert.equal(
    isRoomFolderReadOnlyError({ diagnosticId: 'v-room-folders.unsupported-version' }),
    true
  );
  assert.equal(isRoomFolderReadOnlyError({ diagnosticId: 'v-room-folders.invalid-name' }), false);
});

test('names follow Core: trimmed, whitespace collapsed, 1 to 64 characters', () => {
  assert.equal(normalizeRoomFolderName('  a \n  b  '), 'a b');
  assert.equal(isValidRoomFolderName('   '), false);
  assert.equal(isValidRoomFolderName('x'), true);
  assert.equal(isValidRoomFolderName('x'.repeat(64)), true);
  assert.equal(isValidRoomFolderName('x'.repeat(65)), false);
  // Characters, not UTF-16 units: 64 emoji fit.
  assert.equal(isValidRoomFolderName('🙂'.repeat(64)), true);
  assert.equal(isValidRoomFolderName(` ${'x'.repeat(64)}  `), true);
});
