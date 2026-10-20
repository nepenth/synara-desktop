import assert from 'node:assert/strict';
import test from 'node:test';
import {
  COMPOSER_NOT_SIGNED_IN,
  ComposerUserError,
  SEND_FAILED_FALLBACK,
  commandFailureError,
  composerCommandErrorMessage,
  composerSendErrorMessage,
  coreDiagnosticId,
} from '../composerSendError';
import { BulkActionError } from '../../../utils/bulkActions';

// The shape Tauri rejects with (`MatrixAuthCommandError`, camelCase).
const coreRejection = (diagnosticId: string) => ({
  code: 'Unknown',
  message: 'The native Matrix message could not be sent.',
  diagnosticId,
});

test('Core send rejections become specific copy instead of a doubled generic line', () => {
  const tooLong = composerSendErrorMessage(coreRejection('d0.4-send-text-payload-too-large'));
  assert.match(tooLong, /too long to send/);

  const wedged = composerSendErrorMessage(coreRejection('d0.4-send-queue-wedged'));
  assert.match(wedged, /refused this message/);
  assert.match(wedged, /discard the failed copy/);

  assert.equal(
    composerSendErrorMessage(coreRejection('d0.4-send-requires-session')),
    COMPOSER_NOT_SIGNED_IN
  );
  assert.match(
    composerSendErrorMessage(coreRejection('d0.4-send-sdk-http-rate-limited')),
    /Wait a moment/
  );
  assert.match(composerSendErrorMessage(coreRejection('d0.4-send-queue-cancelled')), /discarded/);

  for (const id of ['d0.4-send-queue-wedged', 'd0.4-send-sdk-http-forbidden']) {
    const message = composerSendErrorMessage(coreRejection(id));
    assert.doesNotMatch(message, /Could not send message: Could not send message/);
    assert.doesNotMatch(message, /native/i);
  }
});

test('unknown Core failures keep their diagnostic id for reporting', () => {
  assert.equal(
    composerSendErrorMessage(coreRejection('d0.4-send-sdk-concurrent-request-failed')),
    `${SEND_FAILED_FALLBACK} (Error code: d0.4-send-sdk-concurrent-request-failed)`
  );
  assert.equal(coreDiagnosticId({ diagnostic_id: 'x-1' }), 'x-1');
  assert.equal(coreDiagnosticId(new Error('plain')), undefined);
});

test('generic diagnostic endings map for non-send commands', () => {
  assert.match(
    composerCommandErrorMessage('invite', coreRejection('v-rooms.invite-forbidden')),
    /^\/invite did not finish: You do not have permission/
  );
  assert.match(
    composerCommandErrorMessage('join', coreRejection('v-rooms.join-network-failed')),
    /could not reach your homeserver/
  );
});

test('user copy passes through; internal "Native …" copy and bare objects do not', () => {
  assert.equal(
    composerSendErrorMessage(new ComposerUserError('Remove "a.mov" to send.')),
    'Remove "a.mov" to send.'
  );
  assert.equal(
    composerSendErrorMessage(new Error('Native Matrix text send is unavailable.')),
    SEND_FAILED_FALLBACK
  );
  assert.equal(composerSendErrorMessage({}), SEND_FAILED_FALLBACK);
  assert.equal(composerSendErrorMessage(undefined), SEND_FAILED_FALLBACK);
  assert.equal(composerSendErrorMessage('Server said no.'), 'Server said no.');
  assert.equal(
    composerCommandErrorMessage('join', new ComposerUserError('Add a room address.')),
    'Add a room address.'
  );
});

test('per-user commands report who failed and why', () => {
  const copy = { done: 'Invited', verb: 'invite', noun: 'people' };
  assert.equal(commandFailureError(['@a:x'], [], copy), undefined);

  const error = commandFailureError(
    ['@a:x', '@b:x'],
    [{ item: '@b:x', error: coreRejection('v-rooms.invite-forbidden') }],
    copy
  );
  assert.ok(error instanceof BulkActionError);
  assert.equal(
    error.message,
    "Invited 1 of 2 people. Couldn't invite @b:x. Try again. You do not have permission to do that in this room."
  );
  // The composer shows it without a second "/invite did not finish" prefix.
  assert.equal(composerCommandErrorMessage('invite', error), error.message);

  const unexplained = commandFailureError(
    ['@a:x'],
    [{ item: '@a:x', error: new Error('Native Matrix room moderation is unavailable.') }],
    copy
  );
  assert.equal(unexplained?.message, "Couldn't invite @a:x. Try again.");
});
