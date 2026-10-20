import assert from 'node:assert/strict';
import test from 'node:test';

import {
  completeAttachmentSendStep,
  hasTrailingAttachmentText,
  makeOrReuseAttachmentSendPlan,
} from '../attachmentSendPlan';
import {
  attachmentTooLargeMessage,
  effectiveNativeAttachmentLimit,
  mediaConfigUploadLimit,
  NATIVE_ATTACHMENT_MAX_BYTES,
} from '../../../utils/nativeMediaLimits';
import { NATIVE_STAGED_MXC, UploadStatus, isNativeStagedUpload } from '../../../state/upload';

test('partial retry preserves the original trailing-text role', () => {
  const initial = makeOrReuseAttachmentSendPlan(undefined, ['one', 'two'], 'Context');
  const afterFirstSuccess = completeAttachmentSendStep(initial, 'one');
  const retry = makeOrReuseAttachmentSendPlan(afterFirstSuccess, ['two'], 'Context');

  assert.equal(retry, afterFirstSuccess);
  assert.equal(retry.textRole, 'trailing');
  assert.deepEqual(retry.remainingTransactionIds, ['two']);
});

test('a new one-file composition uses caption semantics', () => {
  const plan = makeOrReuseAttachmentSendPlan(undefined, ['one'], 'Caption');

  assert.equal(plan.textRole, 'caption');
});

test('editing retained text preserves the original role', () => {
  const initial = makeOrReuseAttachmentSendPlan(undefined, ['one', 'two'], 'Old');
  const afterFirstSuccess = completeAttachmentSendStep(initial, 'one');
  const changed = makeOrReuseAttachmentSendPlan(afterFirstSuccess, ['two'], 'New');

  assert.notEqual(changed, initial);
  assert.equal(changed.body, 'New');
  assert.equal(changed.textRole, 'trailing');
  assert.deepEqual(changed.remainingTransactionIds, ['two']);
});

test('clearing retained text suppresses the trailing text event', () => {
  const initial = makeOrReuseAttachmentSendPlan(undefined, ['one', 'two'], 'Context');
  const afterFirstSuccess = completeAttachmentSendStep(initial, 'one');
  const cleared = makeOrReuseAttachmentSendPlan(afterFirstSuccess, ['two'], '   ');

  assert.equal(cleared.textRole, 'trailing');
  assert.equal(hasTrailingAttachmentText(cleared), false);
});

test('text introduced after a no-text partial send is assigned exactly once', () => {
  const initial = makeOrReuseAttachmentSendPlan(undefined, ['one', 'two', 'three'], '');
  const afterFirstSuccess = completeAttachmentSendStep(initial, 'one');
  const withTrailingText = makeOrReuseAttachmentSendPlan(
    afterFirstSuccess,
    ['two', 'three'],
    'New context'
  );

  assert.equal(withTrailingText.textRole, 'trailing');
  assert.equal(hasTrailingAttachmentText(withTrailingText), true);

  const afterSecondSuccess = completeAttachmentSendStep(withTrailingText, 'two');
  const retry = makeOrReuseAttachmentSendPlan(afterSecondSuccess, ['three'], 'New context');
  assert.equal(retry.textRole, 'trailing');
});

test('text introduced with one attachment remaining becomes its caption', () => {
  const initial = makeOrReuseAttachmentSendPlan(undefined, ['one', 'two'], '');
  const afterFirstSuccess = completeAttachmentSendStep(initial, 'one');
  const retry = makeOrReuseAttachmentSendPlan(afterFirstSuccess, ['two'], 'New caption');

  assert.equal(retry.textRole, 'caption');
  assert.equal(retry.body, 'New caption');
});

test('native attachment preflight never exceeds the 32 MiB owner bound', () => {
  assert.equal(NATIVE_ATTACHMENT_MAX_BYTES, 32 * 1024 * 1024);
  assert.equal(effectiveNativeAttachmentLimit(undefined), NATIVE_ATTACHMENT_MAX_BYTES);
  assert.equal(effectiveNativeAttachmentLimit(64 * 1024 * 1024), NATIVE_ATTACHMENT_MAX_BYTES);
  assert.equal(effectiveNativeAttachmentLimit(8 * 1024 * 1024), 8 * 1024 * 1024);
});

test('the homeserver upload limit is read from the native and the wire key', () => {
  assert.equal(mediaConfigUploadLimit({ maxUploadSizeBytes: 8_000_000 }), 8_000_000);
  assert.equal(mediaConfigUploadLimit({ 'm.upload.size': 5_000_000 }), 5_000_000);
  assert.equal(mediaConfigUploadLimit({}), undefined);
  assert.equal(mediaConfigUploadLimit({ maxUploadSizeBytes: -1 }), undefined);
  assert.equal(
    attachmentTooLargeMessage('clip.mov', 40_000_000, 8_000_000),
    '"clip.mov" is 40.0 MB, over the 8.0 MB upload limit. Remove it to send.'
  );
});

test('a staged composer file is never presented as an uploaded one', () => {
  const file = new File([new Uint8Array([1])], 'a.png', { type: 'image/png' });
  assert.equal(
    isNativeStagedUpload({ file, status: UploadStatus.Success, mxc: NATIVE_STAGED_MXC }),
    true
  );
  assert.equal(
    isNativeStagedUpload({ file, status: UploadStatus.Success, mxc: 'mxc://example.org/a' }),
    false
  );
  assert.equal(isNativeStagedUpload({ file, status: UploadStatus.Idle }), false);
});
