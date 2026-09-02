import assert from 'node:assert/strict';
import test from 'node:test';

import {
  respondPollWithNativeOwner,
  sendPollCommandWithNativeOwner,
  sendPollWithNativeOwner,
} from '../nativePollOwner';

test('native logged-in session is the sole poll-start owner', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const owner = await sendPollWithNativeOwner(
    {
      roomId: '!room:example.org',
      question: 'Deploy now?',
      answers: ['Yes', 'No'],
      maxSelections: 1,
    },
    true,
    async (command, args) => {
      calls.push({ command, args });
      if (command === 'matrix_session_snapshot') {
        return { available: true, value: { status: 'logged_in' } };
      }
      return {
        available: true,
        value: {
          roomId: '!room:example.org',
          eventId: '$poll:example.org',
          status: 'sent',
        },
      };
    }
  );

  assert.equal(owner, 'native');
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_send_poll',
      args: {
        roomId: '!room:example.org',
        question: 'Deploy now?',
        answers: ['Yes', 'No'],
        maxSelections: 1,
        threadRoot: undefined,
        replyTo: undefined,
      },
    },
  ]);
});

test('poll-start owner forwards threadRoot and replyTo to the native command', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const owner = await sendPollWithNativeOwner(
    {
      roomId: '!room:example.org',
      question: 'Deploy now?',
      answers: ['Yes', 'No'],
      maxSelections: 1,
      threadRoot: '$root:example.org',
      replyTo: '$root:example.org',
    },
    true,
    async (command, args) => {
      calls.push({ command, args });
      if (command === 'matrix_session_snapshot') {
        return { available: true, value: { status: 'logged_in' } };
      }
      return {
        available: true,
        value: {
          roomId: '!room:example.org',
          eventId: '$poll:example.org',
          status: 'sent',
        },
      };
    }
  );

  assert.equal(owner, 'native');
  assert.deepEqual(calls[1]?.args, {
    roomId: '!room:example.org',
    question: 'Deploy now?',
    answers: ['Yes', 'No'],
    maxSelections: 1,
    threadRoot: '$root:example.org',
    replyTo: '$root:example.org',
  });
});

test('native logged-in session is the sole poll-response owner', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const owner = await respondPollWithNativeOwner(
    {
      roomId: '!room:example.org',
      pollEventId: '$poll:example.org',
      answerIds: ['a1_x'],
    },
    true,
    async (command, args) => {
      calls.push({ command, args });
      if (command === 'matrix_session_snapshot') {
        return { available: true, value: { status: 'logged_in' } };
      }
      return {
        available: true,
        value: {
          roomId: '!room:example.org',
          eventId: '$vote:example.org',
          status: 'sent',
        },
      };
    }
  );

  assert.equal(owner, 'native');
  assert.equal(calls[1]?.command, 'matrix_poll_respond');
});

test('web and native logged-out sessions retain the legacy poll owner', async () => {
  assert.equal(
    await sendPollWithNativeOwner(
      {
        roomId: '!room:example.org',
        question: 'Q?',
        answers: ['A', 'B'],
        maxSelections: 1,
      },
      false,
      async () => {
        throw new Error('invoke should not be called');
      }
    ),
    'legacy'
  );
});

test('native poll command failure never falls through to legacy sendEvent', async () => {
  await assert.rejects(
    sendPollWithNativeOwner(
      {
        roomId: '!room:example.org',
        question: 'Q?',
        answers: ['A', 'B'],
        maxSelections: 1,
      },
      true,
      async (command) =>
        command === 'matrix_session_snapshot'
          ? { available: true, value: { status: 'logged_in' } }
          : { available: false }
    ),
    /Native Matrix poll send is unavailable/
  );
});

test('slash poll sends one reply/thread snapshot and clears the visible owner only on success', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  let clears = 0;
  const input = {
    roomId: '!room:example.org',
    question: 'Continue?',
    answers: ['Yes', 'No'],
    maxSelections: 1,
    replyTo: '$child:example.org',
    threadRoot: '$root:example.org',
  };
  const owner = await sendPollCommandWithNativeOwner(
    input,
    async () => {
      clears += 1;
    },
    true,
    async (command, args) => {
      calls.push({ command, args });
      return command === 'matrix_session_snapshot'
        ? { available: true, value: { status: 'logged_in' } }
        : {
            available: true,
            value: {
              roomId: input.roomId,
              eventId: '$poll:example.org',
              status: 'sent',
            },
          };
    }
  );

  assert.equal(owner, 'native');
  assert.equal(clears, 1);
  assert.deepEqual(calls[1], {
    command: 'matrix_send_poll',
    args: {
      ...input,
    },
  });

  await assert.rejects(
    sendPollCommandWithNativeOwner(
      input,
      async () => {
        clears += 1;
      },
      true,
      async (command) =>
        command === 'matrix_session_snapshot'
          ? { available: true, value: { status: 'logged_in' } }
          : { available: false }
    ),
    /Native Matrix poll send is unavailable/
  );
  assert.equal(clears, 1, 'failed send must preserve the visible reply draft');

  assert.equal(
    await sendPollCommandWithNativeOwner(
      input,
      async () => {
        clears += 1;
      },
      false,
      async () => {
        throw new Error('desktop invoke must not run');
      }
    ),
    'legacy'
  );
  assert.equal(clears, 1, 'legacy owner must preserve the visible reply draft');
});
