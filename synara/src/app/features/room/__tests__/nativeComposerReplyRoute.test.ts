import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';

import type { GifResult } from '../../../utils/gifProvider';
import { nativeComposerSendRelation } from '../nativeComposerDraftOwner';
import { sendAttachmentWithNativeOwner } from '../nativeSendAttachmentOwner';
import { sendGifWithNativeOwner } from '../nativeSendGifOwner';
import { sendPollWithNativeOwner } from '../nativePollOwner';
import { sendTextWithNativeOwner } from '../nativeSendTextOwner';

test('one Core reply draft supplies the exact relation to every composer send owner', async () => {
  const relation = nativeComposerSendRelation({
    draftRevision: 1,
    eventId: '$selected:example.org',
    senderId: '@alice:example.org',
    body: 'selected message',
    threadRootEventId: '$root:example.org',
  });
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke = async (command: string, args?: Record<string, unknown>) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') {
      return { available: true as const, value: { status: 'logged_in' } };
    }
    if (command === 'matrix_send_text') {
      return {
        available: true as const,
        value: {
          roomId: '!room:example.org',
          eventId: '$text:example.org',
          localTxnId: 'text-txn',
          status: 'sent',
        },
      };
    }
    if (command === 'matrix_send_poll') {
      return {
        available: true as const,
        value: {
          roomId: '!room:example.org',
          eventId: '$poll:example.org',
          status: 'sent',
        },
      };
    }
    return {
      available: true as const,
      value: {
        roomId: '!room:example.org',
        eventId: '$media:example.org',
        localTxnId: 'media-txn',
        status: 'sent',
      },
    };
  };

  await sendTextWithNativeOwner(
    { roomId: '!room:example.org', body: 'text', ...relation },
    true,
    invoke
  );
  await sendAttachmentWithNativeOwner(
    {
      roomId: '!room:example.org',
      transactionId: 'attachment-txn',
      file: { filename: 'note.txt', mimeType: 'text/plain', bytes: [1] },
      ...relation,
    },
    true,
    invoke
  );
  await sendPollWithNativeOwner(
    {
      roomId: '!room:example.org',
      question: 'Continue?',
      answers: ['Yes', 'No'],
      maxSelections: 1,
      ...relation,
    },
    true,
    invoke
  );
  const gif: GifResult = {
    id: 'gif-1',
    title: 'wave',
    url: 'https://cdn.example.org/wave.gif',
    previewUrl: 'https://cdn.example.org/wave-preview.gif',
    width: 100,
    height: 100,
    provider: 'custom',
  };
  await sendGifWithNativeOwner(
    {
      roomId: '!room:example.org',
      gif,
      transactionId: 'gif-txn',
      ...relation,
    },
    true,
    invoke,
    async () => ({
      blob: new Blob([new Uint8Array([0x47, 0x49, 0x46])], { type: 'image/gif' }),
      fileName: 'wave.gif',
    })
  );

  const sends = calls.filter(({ command }) => command.startsWith('matrix_send_'));
  assert.deepEqual(
    sends.map(({ command, args }) => ({
      command,
      replyTo: args?.replyTo,
      threadRoot: args?.threadRoot,
    })),
    [
      {
        command: 'matrix_send_text',
        replyTo: '$selected:example.org',
        threadRoot: '$root:example.org',
      },
      {
        command: 'matrix_send_attachment',
        replyTo: '$selected:example.org',
        threadRoot: '$root:example.org',
      },
      {
        command: 'matrix_send_poll',
        replyTo: '$selected:example.org',
        threadRoot: '$root:example.org',
      },
      {
        command: 'matrix_send_attachment',
        replyTo: '$selected:example.org',
        threadRoot: '$root:example.org',
      },
    ]
  );
});

test('preview, cancellation, and successful sends consume only the Core reply owner', () => {
  const roomInput = readFileSync(
    resolve(process.cwd(), 'src/app/features/room/RoomInput.tsx'),
    'utf8'
  );
  const timelinePresenter = readFileSync(
    resolve(process.cwd(), 'src/app/features/room/NativeTimelinePresenter.tsx'),
    'utf8'
  );
  const localDraftAtoms = readFileSync(
    resolve(process.cwd(), 'src/app/state/room/roomInputDrafts.ts'),
    'utf8'
  );

  assert.match(roomInput, /const replyDraft = useNativeComposerReplyDraft\(roomId\)/);
  assert.match(roomInput, /clearNativeComposerReplyDraft\(\{ roomId, expectedDraftRevision \}\)/);
  assert.match(roomInput, /clearReplyDraftAfterSend\(sendRelation\.draftRevision, \(\) =>/);
  const uploadHandler = roomInput.slice(
    roomInput.indexOf('const handleSendUpload ='),
    roomInput.indexOf('const submit =')
  );
  assert.match(
    uploadHandler,
    /const \{ draftRevision, replyTo, threadRoot \} = nativeComposerSendRelation\(replyDraft\)/,
    'the upload route must declare its clear revision from the same relation snapshot it sends'
  );
  assert.match(
    uploadHandler,
    /clearReplyDraftAfterSend\(draftRevision, \(\) =>/,
    'the upload route must clear the revision declared in its own lexical scope'
  );
  assert.doesNotMatch(
    roomInput,
    /clearReplyDraftAfterSend\((?:sendRelation\.)?replyTo,/,
    'no asynchronous send route may clear by Matrix event id'
  );
  assert.equal(
    (
      roomInput.match(
        /clearReplyDraftAfterSend\((?:sendRelation\.draftRevision|draftRevision),/g
      ) ?? []
    ).length,
    6,
    'slash poll, attachment plan, attachment-only, text, poll dialog, and GIF routes clear their snapshotted Core revision'
  );
  assert.ok(
    (roomInput.match(/clearReplyDraft\(replyDraft\.draftRevision\)/g) ?? []).length >= 2,
    'keyboard and button cancellation must compare the displayed Core revision'
  );
  assert.match(roomInput, /nativeComposerSendRelation\(replyDraft\)/);
  assert.match(roomInput, /sendPollCommandWithNativeDesktopOwner/);
  assert.match(roomInput, /useCommands\([\s\S]*sendSlashPoll[\s\S]*\)/);
  assert.ok(
    (roomInput.match(/clearReplyDraftAfterSend\(/g) ?? []).length >= 4,
    'text, attachment, poll, and GIF success paths must clear the same Core owner'
  );
  assert.doesNotMatch(roomInput, /roomIdToReplyDraftAtomFamily/);
  assert.doesNotMatch(localDraftAtoms, /roomIdToReplyDraftAtomFamily/);
  assert.doesNotMatch(
    timelinePresenter,
    /useNativeComposerReplyDraft/,
    'timeline actions set the Core draft; only the composer should render its projection'
  );
});
