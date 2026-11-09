import assert from 'node:assert/strict';
import test from 'node:test';
import {
  ComposerCommandRefusal,
  composerCommandSuggestions,
  resolveTypedMentions,
  runComposerCommand,
  setNativeComposerCommandsInvokeForTests,
} from '../nativeComposerCommands';
import { composerCommandErrorMessage } from '../composerSendError';

test('a refused command keeps Core copy as the composer error', async () => {
  const calls: unknown[] = [];
  setNativeComposerCommandsInvokeForTests(async (command, args) => {
    calls.push([command, args]);
    return {
      available: true,
      value: {
        outcome: null,
        error: {
          kind: 'unknown_command',
          message: 'Synara does not know the command /rainbow.',
          diagnosticId: null,
        },
      },
    };
  });
  await assert.rejects(
    runComposerCommand('!r:x', '/rainbow hi'),
    (error: unknown) =>
      error instanceof ComposerCommandRefusal &&
      composerCommandErrorMessage('rainbow', error) === 'Synara does not know the command /rainbow.'
  );
  assert.deepEqual(calls, [
    ['matrix_composer_run_command', { roomId: '!r:x', text: '/rainbow hi', confirmed: false }],
  ]);
});

test('outcomes, suggestions and mentions pass through', async () => {
  setNativeComposerCommandsInvokeForTests(async (command) => {
    if (command === 'matrix_composer_run_command') {
      return {
        available: true,
        value: { outcome: { kind: 'done', message: 'Invited @a:x.' }, error: null },
      };
    }
    if (command === 'matrix_composer_command_suggestions') {
      return {
        available: true,
        value: [{ name: 'ban', usage: '/ban <@user:server>', description: 'Ban' }, { bogus: 1 }],
      };
    }
    return {
      available: true,
      value: [{ start: 0, end: 5, userId: '@puck:x', text: '@Puck' }],
    };
  });
  assert.deepEqual(await runComposerCommand('!r:x', '/invite @a:x', true), {
    kind: 'done',
    message: 'Invited @a:x.',
  });
  assert.equal((await composerCommandSuggestions('!r:x', 'ba')).length, 1);
  const members = [{ userId: '@puck:x', displayName: 'Puck' }];
  assert.equal((await resolveTypedMentions('@Puck', members, '@me:x')).length, 1);
  // Nothing to resolve: no round trip.
  assert.deepEqual(await resolveTypedMentions('no mention', members, '@me:x'), []);
});

test('without a native session the command is refused, not sent', async () => {
  setNativeComposerCommandsInvokeForTests(async () => ({ available: false }));
  await assert.rejects(runComposerCommand('!r:x', '/me waves'), /not signed in/);
});
