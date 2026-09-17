import assert from 'node:assert/strict';
import test from 'node:test';
import {
  isNativeVerificationTerminal,
  MAX_QR_IMAGE_DATA_URL_CHARS,
  nativeVerificationErrorMessage,
  NativeVerificationRequest,
  parseNativeVerificationQr,
  sanitizeNativeVerificationRequest,
  selectNativeVerificationRequest,
  verificationRequestCanFallbackToSas,
  verificationRequestHasQr,
  verificationRequestHasSasCodes,
  verificationRequestNeedsSasStart,
} from '../nativeVerification';

const request = (
  direction: NativeVerificationRequest['direction'],
  phase: NativeVerificationRequest['phase']
): NativeVerificationRequest => ({
  flowId: 'flow',
  otherUserId: '@alice:example.org',
  direction,
  phase,
});

test('SAS start projection follows Matrix request ownership', () => {
  assert.equal(verificationRequestNeedsSasStart(request('outgoing', 'ready')), true);
  assert.equal(verificationRequestNeedsSasStart(request('incoming', 'started')), false);
  assert.equal(verificationRequestNeedsSasStart(request('incoming', 'ready')), false);
  assert.equal(verificationRequestNeedsSasStart(request('outgoing', 'requested')), false);
  assert.equal(verificationRequestNeedsSasStart(request('outgoing', 'sas_ready')), false);
  assert.equal(
    verificationRequestNeedsSasStart({
      ...request('outgoing', 'ready'),
      qr: {
        imageDataUrl: "data:image/svg+xml;charset=utf-8,<svg xmlns='http://www.w3.org/2000/svg'/>",
        scanned: false,
      },
    }),
    false
  );
  assert.equal(
    verificationRequestNeedsSasStart({
      ...request('outgoing', 'started'),
      qr: {
        imageDataUrl: "data:image/svg+xml;charset=utf-8,<svg xmlns='http://www.w3.org/2000/svg'/>",
        scanned: false,
      },
    }),
    false
  );
  assert.equal(
    verificationRequestNeedsSasStart(request('outgoing', 'started')),
    false,
    'desktop auto-start is Ready-only; Core falls back to SAS when a QR image cannot be shown'
  );
  assert.equal(
    verificationRequestNeedsSasStart({
      ...request('outgoing', 'sas_ready'),
      sas: { emoji: [{ symbol: '🐶', description: 'Dog' }] },
    }),
    false
  );
});

test('SAS compare requires emoji or decimal codes before confirm', () => {
  assert.equal(verificationRequestHasSasCodes(request('outgoing', 'sas_ready')), false);
  assert.equal(
    verificationRequestHasSasCodes({
      ...request('outgoing', 'sas_ready'),
      sas: { emoji: [{ symbol: '🐶', description: 'Dog' }] },
    }),
    true
  );
  assert.equal(
    verificationRequestHasSasCodes({
      ...request('incoming', 'sas_ready'),
      sas: { decimals: [11, 22, 33] },
    }),
    true
  );
  assert.equal(
    verificationRequestHasSasCodes({
      ...request('outgoing', 'sas_ready'),
      sas: { decimals: [11, 22] as unknown as [number, number, number] },
    }),
    false
  );
});

test('native verification failures use a fixed privacy-safe message', () => {
  const message = nativeVerificationErrorMessage().toLowerCase();
  for (const forbidden of ['token', 'key', 'mac', 'secret', 'ciphertext', 'recovery']) {
    assert.equal(message.includes(forbidden), false);
  }
});

test('inbox keeps the in-progress flow instead of always taking requests[0]', () => {
  const incoming = request('incoming', 'requested');
  incoming.flowId = 'incoming';
  const sas = {
    ...request('outgoing', 'sas_ready'),
    flowId: 'sas',
    sas: { emoji: [{ symbol: '🐶', description: 'Dog' }] },
  };
  const done = { ...request('outgoing', 'done'), flowId: 'done' };
  assert.equal(isNativeVerificationTerminal('done'), true);
  assert.equal(isNativeVerificationTerminal('failed'), true);
  assert.equal(isNativeVerificationTerminal('sas_ready'), false);
  assert.equal(selectNativeVerificationRequest([incoming, sas])?.flowId, 'incoming');
  assert.equal(selectNativeVerificationRequest([incoming, sas], 'sas')?.flowId, 'sas');
  assert.equal(selectNativeVerificationRequest([done, incoming])?.flowId, 'incoming');
  assert.equal(selectNativeVerificationRequest([done])?.flowId, 'done');
  assert.equal(selectNativeVerificationRequest([]), undefined);
});

test('QR parser accepts a bounded SVG data URL and rejects huge payloads', () => {
  const svg = "data:image/svg+xml;charset=utf-8,<svg xmlns='http://www.w3.org/2000/svg'/>";
  assert.deepEqual(parseNativeVerificationQr({ imageDataUrl: svg, scanned: false }), {
    imageDataUrl: svg,
    scanned: false,
  });
  assert.equal(parseNativeVerificationQr({ imageDataUrl: svg, scanned: true })?.scanned, true);
  assert.equal(
    parseNativeVerificationQr({ imageDataUrl: 'not-an-image', scanned: false }),
    undefined
  );
  assert.equal(
    parseNativeVerificationQr({
      imageDataUrl: `data:image/svg+xml;charset=utf-8,${'x'.repeat(MAX_QR_IMAGE_DATA_URL_CHARS)}`,
      scanned: false,
    }),
    undefined
  );
  const dropped = sanitizeNativeVerificationRequest({
    ...request('outgoing', 'started'),
    qr: { imageDataUrl: 'https://evil.example/qr.png', scanned: false },
  });
  assert.equal(dropped.qr, undefined);
  assert.equal(
    verificationRequestHasQr({
      ...request('outgoing', 'started'),
      qr: { imageDataUrl: svg, scanned: false },
    }),
    true
  );
  assert.equal(
    verificationRequestCanFallbackToSas({
      ...request('outgoing', 'started'),
      qr: { imageDataUrl: svg, scanned: false },
    }),
    true
  );
  assert.equal(
    verificationRequestCanFallbackToSas({
      ...request('outgoing', 'sas_ready'),
      qr: { imageDataUrl: svg, scanned: false },
      sas: { emoji: [{ symbol: '🐶', description: 'Dog' }] },
    }),
    false
  );
});
