import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path: string) => readFileSync(path, 'utf8');
const roomInput = read('src/app/features/room/RoomInput.tsx');
const editor = read('src/app/components/editor/Editor.tsx');
const presenter = read('src/app/features/room/NativeTimelinePresenter.tsx');
const uploadBoard = read('src/app/components/upload-board/UploadBoard.tsx');
const uploadCard = read('src/app/components/upload-card/UploadCardRenderer.tsx');
const compactCard = read('src/app/components/upload-card/CompactUploadCardRenderer.tsx');

const submitRoute = roomInput.slice(
  roomInput.indexOf('const submit = useCallback('),
  roomInput.indexOf('const handleSendUpload =')
);

test('the composer clears synchronously when a send is queued, never after the server ack', () => {
  const beforeQueue = submitRoute.slice(0, submitRoute.lastIndexOf('sendQueue.enqueue('));
  // No await may run before the composer is cleared and the send is queued:
  // an await there let a second Enter send twice (D80) and erased text typed
  // during the flight (D67).
  assert.doesNotMatch(
    beforeQueue.replace(/const afterSent = async[\s\S]*?\n {8}\};/, ''),
    /await /
  );
  assert.match(beforeQueue, /cloneComposerContent\(editor\.children\)/);
  assert.match(beforeQueue, /resetEditor\(editor\)/);
  // The success path does not touch the editor or the stored draft (D68).
  const job = submitRoute.slice(submitRoute.lastIndexOf('sendQueue.enqueue('));
  assert.doesNotMatch(job, /resetEditor|clearRoomDraft|setMsgDraft/);
});

test('slash commands are queued and awaited so failures reach the composer', () => {
  assert.match(
    submitRoute,
    /sendQueue\.enqueue\([\s\S]{0,120}commandName[\s\S]{0,120}commandContent\.exe\(plainText\)/
  );
  assert.match(roomInput, /composerCommandErrorMessage\(failed\.commandName, error\)/);
});

test('the legacy send fallback that dropped reply and thread relations is gone', () => {
  assert.doesNotMatch(roomInput, /sendNativeMessage/);
  assert.match(
    roomInput,
    /nativeOwner === 'legacy'\) throw new ComposerUserError\(COMPOSER_NOT_SIGNED_IN\)/
  );
});

test('after a send only the view showing that timeline moves', () => {
  assert.match(roomInput, /requestRoomLatestAfterSend\(roomId, sendRelation\.threadRoot\)/);
  assert.doesNotMatch(roomInput, /if \(!threadRootOverride\) requestRoomLatestAfterSend/);
  assert.match(
    presenter,
    /observeRoomLatestAfterSend\(\s*roomId,\s*threadRootId,\s*threadRootId \? showThreadTail : showLatestAfterSend\s*\)/
  );
});

test('the composer has an accessible name for its room or thread', () => {
  assert.match(
    roomInput,
    /ariaLabel=\{\s*threadRootEventId\s*\?\s*t\('composer\.thread_aria_label', 'Reply in thread'\)/
  );
  assert.match(roomInput, /defaultValue: 'Message \{\{room\}\}'/);
});

test('keys an input method is composing never reach composer shortcuts', () => {
  const handler = editor.slice(
    editor.indexOf('const handleKeydown'),
    editor.indexOf('const renderPlaceholder')
  );
  assert.ok(
    handler.indexOf('if (isComposing(evt)) return;') >= 0 &&
      handler.indexOf('if (isComposing(evt)) return;') < handler.indexOf('onKeyDown?.(evt)'),
    'composition must be checked before Escape/Backspace/Enter handlers run'
  );
});

test('Enter picks an open autocomplete suggestion before it sends', () => {
  assert.match(roomInput, /autocompleteSelectFirst\.current\?\.\(\)\) return;\s*submit\(\);/);
  assert.equal((roomInput.match(/selectFirstRef=\{autocompleteSelectFirst\}/g) ?? []).length, 4);
});

test('the native clipboard fallback reads a snapshot taken during the paste event', () => {
  assert.match(roomInput, /const clipboardText = snapshotClipboardText\(evt\.clipboardData\);/);
  assert.match(roomInput, /insertClipboardData\(editor, clipboardText\)/);
});

test('upload cards and the board report real state and never reject silently', () => {
  assert.match(uploadBoard, /onClick=\{onSend\}/);
  assert.doesNotMatch(uploadBoard, /useImperativeHandle|void handleSend\(\)/);
  for (const card of [uploadCard, compactCard]) {
    assert.match(card, /mediaConfigUploadLimit\(mediaConfig\)/);
    assert.doesNotMatch(card, /mediaConfig\['m\.upload\.size'\]/);
  }
  assert.match(uploadCard, /upload\.status === UploadStatus\.Success && !nativeStaged/);
  // D85: no renderer upload path, so no Retry that could only fail again.
  assert.doesNotMatch(uploadCard, /startUpload|Retry Upload/);
  assert.doesNotMatch(compactCard, /startUpload|isSynaraDesktop/);
});
