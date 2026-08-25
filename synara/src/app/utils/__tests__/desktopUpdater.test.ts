import test from 'node:test';
import assert from 'node:assert/strict';
import {
  checkDesktopUpdate,
  compareVersions,
  isUpdaterUnavailableError,
  reduceDownloadProgress,
  shouldPromptForUpdate,
  updateErrorMessage,
  type MacosUpdateHandle,
} from '../desktopUpdater';

const jsonFetch =
  (body: unknown, status = 200) =>
  async () =>
    ({
      ok: status >= 200 && status < 300,
      status,
      json: async () => body,
    } as Response);

test('desktop updater compares semantic version strings', () => {
  assert.equal(compareVersions('1.2.22', '1.2.21'), 1);
  assert.equal(compareVersions('v1.2.21', '1.2.21'), 0);
  assert.equal(compareVersions('1.2.20', '1.2.21'), -1);
});

test('desktop updater reports macos update availability', async () => {
  const update: MacosUpdateHandle = {
    currentVersion: '1.2.21',
    version: '1.2.22',
    date: '2026-07-02T12:00:00.000Z',
    body: 'Release notes',
    downloadAndInstall: async () => undefined,
  };

  const result = await checkDesktopUpdate({
    currentVersion: '1.2.21',
    getPlatform: async () => 'darwin',
    supportsUpdater: () => true,
    macosCheck: async () => update,
  });

  assert.equal(result.status, 'available');
  assert.equal(result.platform, 'macos');
  assert.equal(result.version, '1.2.22');
  assert.equal(result.macosUpdate, update);
});

test('desktop updater reports macos no-update result', async () => {
  const result = await checkDesktopUpdate({
    currentVersion: '1.2.21',
    getPlatform: async () => 'macos',
    supportsUpdater: () => true,
    macosCheck: async () => null,
  });

  assert.deepEqual(result, {
    status: 'up-to-date',
    platform: 'macos',
    currentVersion: '1.2.21',
    version: '1.2.21',
    releaseUrl: 'https://github.com/nepenth/synara-desktop/releases/tag/v1.2.21',
  });
});

test('desktop updater gracefully reports missing signed updater config', async () => {
  const result = await checkDesktopUpdate({
    currentVersion: '1.2.21',
    getPlatform: async () => 'darwin',
    supportsUpdater: () => false,
  });

  assert.equal(result.status, 'unavailable');
  assert.equal(result.platform, 'macos');
  assert.match(result.message, /signed release builds/i);
});

test('desktop updater detects unavailable updater plugin errors', () => {
  assert.equal(isUpdaterUnavailableError(new Error('unknown command plugin:updater|check')), true);
  assert.equal(isUpdaterUnavailableError('permission denied for plugin:updater|check'), true);
  assert.equal(isUpdaterUnavailableError(new Error('network request failed')), false);
});

test('desktop updater preserves non-Error updater failure messages', () => {
  assert.equal(
    updateErrorMessage('the platform `darwin-aarch64` was not found'),
    'the platform `darwin-aarch64` was not found'
  );
  assert.equal(
    updateErrorMessage({ message: 'signature verification failed' }),
    'signature verification failed'
  );
  assert.equal(updateErrorMessage({ code: 'updater-failed' }), '{"code":"updater-failed"}');
  assert.equal(updateErrorMessage(undefined), 'Unknown updater error.');
});

test('desktop updater reports linux release availability without self-install', async () => {
  const result = await checkDesktopUpdate({
    currentVersion: '1.2.21',
    getPlatform: async () => 'linux',
    fetchImpl: jsonFetch({ version: '1.2.22', pub_date: '2026-07-02T12:00:00.000Z' }),
  });

  assert.equal(result.status, 'available');
  assert.equal(result.platform, 'linux');
  assert.equal(result.version, '1.2.22');
  assert.match(result.packageManagerHint ?? '', /apt upgrade/);
  assert.match(result.packageManagerHint ?? '', /paru -Syu/);
});

test('desktop updater reports linux up-to-date releases', async () => {
  const result = await checkDesktopUpdate({
    currentVersion: '1.2.21',
    getPlatform: async () => 'linux',
    fetchImpl: jsonFetch({ version: '1.2.21' }),
  });

  assert.equal(result.status, 'up-to-date');
  assert.equal(result.platform, 'linux');
  assert.equal(result.version, '1.2.21');
});

test('desktop updater suppresses dismissed background prompts only', () => {
  assert.equal(
    shouldPromptForUpdate({
      source: 'background',
      version: '1.2.22',
      dismissedVersion: '1.2.22',
    }),
    false
  );
  assert.equal(
    shouldPromptForUpdate({
      source: 'manual',
      version: '1.2.22',
      dismissedVersion: '1.2.22',
    }),
    true
  );
});

test('desktop updater accumulates download progress', () => {
  const started = reduceDownloadProgress(
    { downloadedBytes: 100, finished: true },
    { event: 'Started', data: { contentLength: 1000 } }
  );
  const progressed = reduceDownloadProgress(started, {
    event: 'Progress',
    data: { chunkLength: 250 },
  });
  const finished = reduceDownloadProgress(progressed, { event: 'Finished' });

  assert.deepEqual(started, {
    contentLength: 1000,
    downloadedBytes: 0,
    finished: false,
  });
  assert.deepEqual(progressed, {
    contentLength: 1000,
    downloadedBytes: 250,
    finished: false,
  });
  assert.deepEqual(finished, {
    contentLength: 1000,
    downloadedBytes: 250,
    finished: true,
  });
});
