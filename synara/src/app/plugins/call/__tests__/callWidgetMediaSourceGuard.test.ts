import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const driverSource = readFileSync(
  join(process.cwd(), 'src/app/plugins/call/CallWidgetDriver.ts'),
  'utf8'
);

const mediaMethods = [
  driverSource.slice(
    driverSource.indexOf('public async getMediaConfig()'),
    driverSource.indexOf('public async uploadFile(')
  ),
  driverSource.slice(
    driverSource.indexOf('public async downloadFile('),
    driverSource.indexOf('public getKnownRooms(')
  ),
].join('\n');

test('CallWidgetDriver media methods have no JS media ownership or fail-closed stubs', () => {
  for (const forbidden of [
    'this.mx.getMediaConfig',
    'mxcUrlToHttp',
    'downloadMedia',
    'fetch(',
    'throwNativeCallWidgetCapabilityUnavailable',
    'void contentUri',
    'Legacy',
    'return legacy',
    'isNative ? rust : js',
  ]) {
    assert.equal(mediaMethods.includes(forbidden), false, forbidden);
  }
  assert.equal(mediaMethods.includes('getMediaConfig(): Promise<IGetMediaConfigResult>'), true);
  assert.equal(mediaMethods.includes('nativeCallWidgetMediaOwner'), false);
  assert.equal(mediaMethods.includes('getMediaConfigWithNativeOwner'), true);
  assert.equal(mediaMethods.includes('downloadFileWithNativeOwner'), true);
});
