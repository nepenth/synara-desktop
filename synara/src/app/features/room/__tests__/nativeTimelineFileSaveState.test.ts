import assert from 'node:assert/strict';
import test from 'node:test';
import type { MatrixMediaSaveEvent } from '../../../matrix/media';
import {
  dismissNativeTimelineFileSave,
  nativeTimelineFileSaveButtonLabel,
  nativeTimelineFileSavePercent,
  nativeTimelineFileSaveState,
  noteNativeTimelineFileSaveRoom,
  resetNativeTimelineFileSavesForTests,
  startNativeTimelineFileSave,
} from '../nativeTimelineFileSaveState';

const HANDLE = `timeline-media-${'ab'.repeat(32)}`;
const OTHER = `timeline-media-${'cd'.repeat(32)}`;

/** A save the test settles by hand, with the progress callback it was given. */
const controlledSave = () => {
  const calls: Array<{
    handleId: string;
    onProgress: (event: MatrixMediaSaveEvent) => void;
    resolve: (filename: string) => void;
    reject: (error: unknown) => void;
  }> = [];
  const save = (
    target: { handleId: string },
    onProgress: (event: MatrixMediaSaveEvent) => void
  ): Promise<string> =>
    new Promise<string>((resolve, reject) => {
      calls.push({ handleId: target.handleId, onProgress, resolve, reject });
    });
  return { calls, save };
};

test.beforeEach(() => resetNativeTimelineFileSavesForTests());

test('clicks on the chip, the viewer and the preview share one save', async () => {
  const { calls, save } = controlledSave();
  const first = startNativeTimelineFileSave({ handleId: HANDLE, filename: 'a.pem' }, { save });
  const second = startNativeTimelineFileSave({ handleId: HANDLE, filename: 'a.pem' }, { save });
  assert.equal(first, second);
  assert.equal(calls.length, 1);
  // Another file saves on its own.
  void startNativeTimelineFileSave({ handleId: OTHER }, { save });
  assert.equal(calls.length, 2);
  calls[0].resolve('a.pem');
  await first;
  assert.deepEqual(nativeTimelineFileSaveState(HANDLE), { kind: 'done', filename: 'a.pem' });
});

test('progress moves the save forward and keeps its start time for remounted rows', async () => {
  const { calls, save } = controlledSave();
  const flight = startNativeTimelineFileSave({ handleId: HANDLE }, { save, now: () => 1000 });
  assert.deepEqual(nativeTimelineFileSaveState(HANDLE), {
    kind: 'downloading',
    startedAt: 1000,
    received: 0,
  });
  const { onProgress } = calls[0];
  onProgress({ event: 'started', data: { total: 400 } });
  onProgress({ event: 'progress', data: { received: 100 } });
  assert.deepEqual(nativeTimelineFileSaveState(HANDLE), {
    kind: 'downloading',
    startedAt: 1000,
    received: 100,
    total: 400,
  });
  onProgress({ event: 'finishing' });
  assert.deepEqual(nativeTimelineFileSaveState(HANDLE), { kind: 'finishing', startedAt: 1000 });
  // A progress event that arrives after Finishing does not move it back.
  onProgress({ event: 'progress', data: { received: 400 } });
  assert.equal(nativeTimelineFileSaveState(HANDLE)?.kind, 'finishing');
  calls[0].resolve('report (1).pdf');
  await flight;
  assert.deepEqual(nativeTimelineFileSaveState(HANDLE), {
    kind: 'done',
    filename: 'report (1).pdf',
  });
  onProgress({ event: 'progress', data: { received: 1 } });
  assert.equal(nativeTimelineFileSaveState(HANDLE)?.kind, 'done');
});

test('an unknown total stays unknown', () => {
  const { calls, save } = controlledSave();
  void startNativeTimelineFileSave({ handleId: HANDLE }, { save, now: () => 5 });
  calls[0].onProgress({ event: 'started', data: { total: null } });
  calls[0].onProgress({ event: 'progress', data: { received: 64 } });
  assert.deepEqual(nativeTimelineFileSaveState(HANDLE), {
    kind: 'downloading',
    startedAt: 5,
    received: 64,
    total: undefined,
  });
});

test('a failure stays on the file, retries as a new save, and a too-large file is final', async () => {
  const { calls, save } = controlledSave();
  const flight = startNativeTimelineFileSave({ handleId: HANDLE }, { save });
  calls[0].reject({ diagnosticId: 'v-send.r-media-download-sdk-failed' });
  await flight;
  const failed = nativeTimelineFileSaveState(HANDLE);
  assert.equal(failed?.kind, 'failed');
  assert.equal(failed?.kind === 'failed' && failed.retryable, true);
  assert.match(
    failed?.kind === 'failed' ? failed.message : '',
    /^Synara could not download this file\. Try again\./
  );

  const retry = startNativeTimelineFileSave({ handleId: HANDLE }, { save });
  assert.equal(calls.length, 2);
  assert.equal(nativeTimelineFileSaveState(HANDLE)?.kind, 'downloading');
  calls[1].reject({ diagnosticId: 'v-send.r-media-download-too-large' });
  await retry;
  assert.deepEqual(nativeTimelineFileSaveState(HANDLE), {
    kind: 'failed',
    message: 'This file is larger than Synara can save (300 MB).',
    retryable: false,
  });
});

test('dismiss clears a finished line but never a running save', async () => {
  const { calls, save } = controlledSave();
  const flight = startNativeTimelineFileSave({ handleId: HANDLE }, { save });
  dismissNativeTimelineFileSave(HANDLE);
  assert.equal(nativeTimelineFileSaveState(HANDLE)?.kind, 'downloading');
  calls[0].resolve('a.pem');
  await flight;
  dismissNativeTimelineFileSave(HANDLE);
  assert.equal(nativeTimelineFileSaveState(HANDLE), undefined);
});

test('changing room clears finished lines and keeps running saves', async () => {
  const { calls, save } = controlledSave();
  noteNativeTimelineFileSaveRoom('!a:example.test');
  const done = startNativeTimelineFileSave({ handleId: HANDLE }, { save });
  void startNativeTimelineFileSave({ handleId: OTHER }, { save });
  calls[0].resolve('a.pem');
  await done;
  // A second view of the same room (a thread pane) keeps them.
  noteNativeTimelineFileSaveRoom('!a:example.test');
  assert.equal(nativeTimelineFileSaveState(HANDLE)?.kind, 'done');
  noteNativeTimelineFileSaveRoom('!b:example.test');
  assert.equal(nativeTimelineFileSaveState(HANDLE), undefined);
  assert.equal(nativeTimelineFileSaveState(OTHER)?.kind, 'downloading');
});

test('percent and button copy', () => {
  assert.equal(nativeTimelineFileSavePercent(0, 0), 0);
  assert.equal(nativeTimelineFileSavePercent(1, 3), 33);
  assert.equal(nativeTimelineFileSavePercent(5, 4), 100);
  const downloading = { kind: 'downloading', startedAt: 0, received: 1, total: 4 } as const;
  assert.equal(nativeTimelineFileSaveButtonLabel(downloading, false, 'Save'), 'Save');
  assert.equal(nativeTimelineFileSaveButtonLabel(downloading, true, 'Save'), 'Downloading 25%');
  assert.equal(
    nativeTimelineFileSaveButtonLabel({ ...downloading, total: undefined }, true, 'Save'),
    'Downloading…'
  );
  assert.equal(
    nativeTimelineFileSaveButtonLabel({ kind: 'finishing', startedAt: 0 }, true, 'Save'),
    'Finishing…'
  );
  assert.equal(
    nativeTimelineFileSaveButtonLabel({ kind: 'done', filename: 'a' }, true, 'Save'),
    'Save'
  );
});
