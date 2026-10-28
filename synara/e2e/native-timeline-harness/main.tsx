// Actual shipped presenter/controller and browser geometry; only native IPC is
// a deterministic fixture. This cannot establish live SDK/Matrix correctness.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import { darkTheme } from '../../src/colors.css';
import { NativeTimelinePresenter } from '../../src/app/features/room/NativeTimelinePresenter';
import { ThreadSidePanel } from '../../src/app/features/room/ThreadSidePanel';
import { normalizeThreadDisplay } from '../../src/app/utils/threadDisplay';
import { setNativeIdentity } from '../../src/app/state/nativeIdentity';
import { requestRoomLatestAfterSend } from '../../src/app/features/room/nativeTimelineNavigation';
import {
  applyNativeTimelineViewDelta,
  type NativeTimelinePosition,
  type NativeTimelineViewDeltaBatch,
  type NativeTimelineViewSnapshot,
} from '../../src/app/features/room/nativeTimelineView';

const room = '!navigation:example.test';
const params = new URLSearchParams(location.search);
const scenario = params.get('scenario') ?? 'live';
const polish = params.has('polish');
const jank = params.has('jank');
// `editable`: Core lets the user edit every message (the in-place editor).
const editable = params.has('editable');
// `failOpen`: the first room open fails the way Core rejects a command.
let failOpen = params.has('failOpen');
const coreOpenFailure = (diagnosticId: string) => ({
  code: 'Unknown',
  message: 'The native Matrix timeline is unavailable.',
  diagnosticId,
});
const FILE_MD_HANDLE = `timeline-media-${'ab'.repeat(32)}`;
const FILE_ZIP_HANDLE = `timeline-media-${'cd'.repeat(32)}`;
const FILE_MD_BYTES = `# Agent notes

Use **bold** for emphasis.

- dash item
* star item

1. ordered

| Name | Role |
| --- | --- |
| Ada | Lead |

- [x] done task
- [ ] open task

See ~~strike~~ and \`inline code\`.
`;
const FILE_MD_HTML = [
  '<h1>Agent notes</h1>',
  '<p>Use <strong>bold</strong> for emphasis.</p>',
  '<ul><li>dash item</li></ul>',
  '<ul><li>star item</li></ul>',
  '<ol><li>ordered</li></ol>',
  '<table><thead><tr><th>Name</th><th>Role</th></tr></thead>',
  '<tbody><tr><td>Ada</td><td>Lead</td></tr></tbody></table>',
  '<ul><li>[x] done task</li><li>[ ] open task</li></ul>',
  '<p>See <del>strike</del> and <code>inline code</code>.</p>',
].join('');
let sequence = polish
  ? 4
  : scenario === 'sparse-missing' || scenario === 'file-md' || scenario === 'file-zip'
    ? 1
    : scenario === 'short'
      ? 2
      : scenario === 'media' || scenario === 'embeds'
        ? 3
        : jank
          ? 180
          : 60;
// `scenario=media`: an image, a video and an image the webview cannot load.
const MEDIA_IMAGE_HANDLE = `timeline-media-${'1a'.repeat(32)}`;
const MEDIA_VIDEO_HANDLE = `timeline-media-${'2b'.repeat(32)}`;
const MEDIA_BROKEN_HANDLE = `timeline-media-${'3c'.repeat(32)}`;
// `eventWindow` serves an unread position from an event-focused window: it
// cannot follow live, and its first forward page reaches the newest event.
// Thread fixtures: `failThreadOpen` rejects the first thread open,
// `failThreadPaginate` the first thread pagination, `threadMore` leaves older
// replies on the server and `holdThreadReopen` holds every later thread open.
let failThreadOpen = params.has('failThreadOpen');
let failThreadPaginate = params.has('failThreadPaginate');
let threadOpenCount = 0;
let releaseThreadOpen: (() => void) | undefined;
let historyIndex = 0;
let stream = 0;
let releaseJump: (() => void) | undefined;
let releaseLastRead: (() => void) | undefined;
let failLastRead = params.has('failLastRead');
let lastCandidateStreamId: string | undefined;
let releaseOperation: (() => void) | undefined;
let operationDeferred = false;
// `nativeEvents`: the views' delta channels, as `desktop_timeline_subscribe`
// and the Rust registry deliver them (room-wide until an open claims a stream).
const channelCallbacks = new Map<number, (message: unknown) => void>();
const deltaSubscriptions = new Map<
  number,
  { roomId: string; channelId: number; nextIndex: number }
>();
const streamOwners = new Map<string, number>();
let callbackSequence = 0;
let subscriptionSequence = 0;
const claimStream = (subscriptionId: unknown, streamId: string | undefined) => {
  if (typeof subscriptionId !== 'number' || !streamId) return;
  if (deltaSubscriptions.has(subscriptionId)) streamOwners.set(streamId, subscriptionId);
};
const commands: { command: string; args?: Record<string, unknown> }[] = [];
const makeRow = (index: number) => ({
  kind: 'message' as const,
  itemId: `$${index}`,
  eventId: `$${index}`,
  senderId: `@reader${index % 2}:example.test`,
  senderName: `Reader ${index % 2}`,
  originServerTs: 1_700_000_000_000 + index * 60_000,
  body:
    scenario === 'file-md'
      ? 'notes.md'
      : scenario === 'file-zip'
        ? 'archive.zip'
        : jank
          ? `Message ${index}\n${Array.from(
              { length: 1 + (index % 6) },
              (_, line) => `Native jank fixture line ${line + 2}.`
            ).join('\n')}`
          : `Message ${index}\nNative timeline geometry fixture line two.\nLine three.`,
  edited: false,
  forwardTransport: polish ? ('text' as const) : undefined,
  ...(scenario === 'file-md'
    ? {
        messageType: 'file' as const,
        mediaFilename: 'notes.md',
        media: {
          handleId: FILE_MD_HANDLE,
          mimeType: 'text/markdown',
        },
      }
    : scenario === 'file-zip'
      ? {
          messageType: 'file' as const,
          mediaFilename: 'archive.zip',
          media: {
            handleId: FILE_ZIP_HANDLE,
            mimeType: 'application/zip',
          },
        }
      : {}),
  capabilities: {
    react: polish,
    reply: polish,
    edit: editable,
    redact: polish,
    report: polish,
    pin: polish,
    forward: polish,
    vote: false,
    declineCall: false,
  },
});
let rows = Array.from({ length: sequence }, (_, index) => makeRow(index + 1));
// `scenario=embeds`: the last two messages are a GIF link and an X link; the
// X post comes from the mocked `desktop_x_post` below.
const EMBED_GIF_URL = 'https://media.giphy.com/media/3o7aD2saalBwwftBIY/giphy.gif';
const EMBED_X_URL = 'https://x.com/example/status/1700000000000000001';
const EMBED_X_POST = {
  __typename: 'Tweet',
  id_str: '1700000000000000001',
  text: 'Launch day: the full video is below.',
  user: { name: 'Example Account', screen_name: 'example' },
  mediaDetails: [
    {
      type: 'video',
      original_info: { width: 1280, height: 720 },
      video_info: {
        variants: [
          {
            bitrate: 832000,
            content_type: 'video/mp4',
            url: 'https://video.twimg.com/fixture.mp4',
          },
        ],
      },
    },
  ],
};
if (scenario === 'embeds') {
  rows = rows.map((row, index) => {
    if (index === rows.length - 2) return { ...row, body: EMBED_GIF_URL };
    if (index === rows.length - 1) return { ...row, body: `Worth watching ${EMBED_X_URL}` };
    return row;
  });
}
// `scenario=threads`: the second-to-last message owns a 7-reply thread.
const THREAD_ROOT = `$${Math.max(1, sequence - 1)}`;
const THREAD_REPLY_COUNT = 7;
if (scenario === 'threads') {
  rows = rows.map((row) =>
    row.eventId === THREAD_ROOT
      ? {
          ...row,
          thread: {
            rootEventId: THREAD_ROOT,
            replyCount: THREAD_REPLY_COUNT,
            latestEventId: `$t${THREAD_REPLY_COUNT}`,
          },
        }
      : row
  );
}
if (scenario === 'media') {
  const media = [
    {
      messageType: 'image' as const,
      mediaFilename: 'sunrise.png',
      media: { handleId: MEDIA_IMAGE_HANDLE, mimeType: 'image/png', width: 64, height: 48 },
    },
    {
      messageType: 'video' as const,
      mediaFilename: 'clip.mp4',
      media: { handleId: MEDIA_VIDEO_HANDLE, mimeType: 'video/mp4', width: 64, height: 48 },
    },
    {
      messageType: 'image' as const,
      mediaFilename: 'broken.png',
      media: { handleId: MEDIA_BROKEN_HANDLE, mimeType: 'image/png', width: 64, height: 48 },
    },
  ];
  rows = rows.map(
    (row, index) =>
      ({ ...row, body: media[index].mediaFilename, ...media[index] }) as (typeof rows)[number]
  );
}
const threadRows = () => [
  rows.find((row) => row.eventId === THREAD_ROOT) ?? makeRow(1),
  ...Array.from({ length: THREAD_REPLY_COUNT }, (_, index) => ({
    ...makeRow(index + 1),
    itemId: `$t${index + 1}`,
    eventId: `$t${index + 1}`,
    body: `Thread reply ${index + 1}`,
    // One reply carries Matrix HTML, as Hermes and Element send it.
    ...(index === 5
      ? {
          formattedBody:
            '<p>Thread reply 6 is <strong>formatted</strong> with <code>code</code></p><script>bad()</script>',
        }
      : {}),
    threadRoot: THREAD_ROOT,
  })),
];
/** Thread streams the presenter has opened and not yet closed. */
const openThreadStreams = new Set<string>();
let position: NativeTimelinePosition =
  scenario === 'missing' || scenario === 'sparse-missing'
    ? { kind: 'unread', anchor_event_id: '$missing' }
    : scenario === 'unread' || scenario === 'short'
      ? { kind: 'unread', anchor_event_id: '$2' }
      : { kind: 'live_bottom' };
let snapshot: NativeTimelineViewSnapshot;
const snapshots = new Map<string, NativeTimelineViewSnapshot>();
const emitCandidateUpdate = (body: string, skipRevision = false, remove = false) => {
  if (!lastCandidateStreamId) throw new Error('No candidate stream');
  const current = snapshots.get(lastCandidateStreamId);
  if (!current) return;
  const row = { ...current.rows[0], body };
  const batch: NativeTimelineViewDeltaBatch = {
    schemaVersion: 1,
    sessionGeneration: current.sessionGeneration,
    roomId: current.roomId,
    streamId: lastCandidateStreamId,
    revision: current.revision + (skipRevision ? 2 : 1),
    ops: remove ? [{ op: 'remove', index: 0 }] : [{ op: 'set', index: 0, row }],
  };
  snapshots.set(lastCandidateStreamId, {
    ...current,
    revision: batch.revision,
    rows: remove ? current.rows.slice(1) : [row, ...current.rows.slice(1)],
  });
  emitBatch(batch);
};
const emitBatch = (batch: NativeTimelineViewDeltaBatch) => {
  const owner = streamOwners.get(batch.streamId);
  for (const [id, subscription] of deltaSubscriptions) {
    if (subscription.roomId !== batch.roomId || (owner !== undefined && owner !== id)) continue;
    channelCallbacks.get(subscription.channelId)?.({
      message: batch,
      index: subscription.nextIndex++,
    });
  }
};
const update = () => {
  snapshot = {
    schemaVersion: 1,
    sessionGeneration: 1,
    roomId: room,
    revision:
      Math.max(snapshot?.revision ?? 0, ...[...snapshots.values()].map((value) => value.revision)) +
      1,
    position,
    rows: [...rows],
    pagination: { backward: 'available', forward: 'available' },
    readState: {
      visibleTailEventId: rows.at(-1)?.eventId,
      receiptTailEventId: rows.at(-1)?.eventId,
      ownReadEventId: position.kind === 'unread' ? position.anchor_event_id : '$2',
      unreadAnchorEventId: position.kind === 'unread' ? position.anchor_event_id : '$2',
      isMarkedUnread: false,
    },
    capabilities: {
      markRead: true,
      markUnread: true,
      paginateBackward: true,
      paginateForward: true,
      canRedactOwn: true,
      canRedactOther: false,
    },
  };
  for (const [id, current] of snapshots) {
    const hasMissing = current.rows.some((row) => 'eventId' in row && row.eventId === '$missing');
    const nextRows =
      hasMissing && !rows.some((row) => row.eventId === '$missing')
        ? [{ ...makeRow(0), eventId: '$missing', itemId: '$missing' }, ...rows]
        : rows;
    snapshots.set(id, {
      ...snapshot,
      position: current.position,
      rows: [...nextRows],
      readState: { ...snapshot.readState, ownReadEventId: current.readState.ownReadEventId },
    });
  }
};
const openSnapshot = (selectedPosition: NativeTimelinePosition, includeLastRead = false) => {
  const streamId = `fixture-${++stream}`;
  const result = {
    ...snapshot,
    // Core revisions are independent and start at zero for every opened stream.
    revision: 0,
    position: selectedPosition,
    rows:
      includeLastRead && !rows.some((row) => row.eventId === '$missing')
        ? [{ ...makeRow(0), eventId: '$missing', itemId: '$missing' }, ...rows]
        : [...rows],
  };
  snapshots.set(streamId, result);
  return { schemaVersion: 1, streamId, position: selectedPosition, snapshot: result };
};
update();
// Pagination and read-state replies carry stream metadata only, as in Core.
const withoutRows = <S extends { rows: unknown }>(snapshot: S): Omit<S, 'rows'> => {
  const { rows: _rows, ...metadata } = snapshot;
  return metadata;
};
window.__SYNARA_DESKTOP__ = {
  platform: 'tauri',
  invoke: async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
    commands.push({ command, args });
    const request = args?.request as
      | {
          streamId?: string;
          position?: { kind: string; event_id?: string };
          observedLiveTailEventId?: string;
        }
      | undefined;
    const delayedCommand = {
      read: 'matrix_timeline_set_read_state',
      paginate: 'matrix_timeline_paginate',
      follow: 'matrix_timeline_follow_live',
      poll: 'matrix_timeline_snapshot',
    }[params.get('delayOperation') ?? ''];
    if (params.has('delayOperation') && command === delayedCommand && !operationDeferred) {
      operationDeferred = true;
      const current = snapshots.get(request?.streamId ?? (args?.streamId as string));
      if (!current) throw new Error('Unknown delayed source stream');
      const oldSnapshot = {
        ...current,
        revision: current.revision + (params.has('sameStream') ? 0 : 10),
        position:
          command === 'matrix_timeline_follow_live'
            ? ({ kind: 'live_bottom' } as const)
            : current.position,
      };
      await new Promise<void>((resolve) => {
        releaseOperation = resolve;
      });
      if (params.get('operationResult') === 'reject')
        throw new Error('Superseded operation rejected');
      if (params.get('operationResult') === 'unavailable') return undefined as T;
      if (command === 'matrix_timeline_set_read_state')
        return { view: withoutRows(oldSnapshot), receiptSent: true } as T;
      if (command === 'matrix_timeline_paginate') return withoutRows(oldSnapshot) as T;
      return oldSnapshot as T;
    }
    if (command === 'matrix_session_snapshot') return { status: 'logged_in' } as T;
    if (command === 'matrix_room_members_snapshot') {
      const memberRoomId = String(args?.roomId ?? '');
      return {
        sessionGeneration: 1,
        roomId: memberRoomId,
        members: ['@alice:example.org', '@bob:example.org'].map((userId) => ({
          roomId: memberRoomId,
          userId,
          displayName: userId === '@alice:example.org' ? 'Alice Liddell' : 'Bob Builder',
          membership: 'join',
          powerLevel: 0,
        })),
      } as T;
    }
    if (command === 'desktop_x_post') return JSON.stringify(EMBED_X_POST) as T;
    if (command === 'matrix_edit_message') {
      return {
        roomId: room,
        eventId: args?.eventId,
        localTxnId: 'edit-txn',
        status: 'sent',
      } as T;
    }
    if (
      command === 'matrix_timeline_open' &&
      request?.position?.kind === 'focused' &&
      request.position.event_id === '$broken'
    ) {
      // Core could not open the routed event at all (the bridge reports it
      // as an invalid request).
      throw coreOpenFailure('d0.3-timeline-invalid-room-id');
    }
    if (command === 'matrix_timeline_open' && failOpen && request?.position?.kind !== 'focused') {
      failOpen = false;
      throw coreOpenFailure('d0.3-timeline-open-failed');
    }
    if (command === 'matrix_timeline_timestamp_to_event') {
      return {
        roomId: room,
        eventId: '$history-jump',
        originServerTs: Number(args?.timestampMs) || 1_600_000_000_000,
      } as T;
    }
    if (command === 'matrix_timeline_open' && request?.position?.kind === 'thread') {
      threadOpenCount += 1;
      if (failThreadOpen) {
        failThreadOpen = false;
        throw coreOpenFailure('v-timeline-thread-open-failed');
      }
      if (params.has('holdThreadReopen') && threadOpenCount > 1) {
        await new Promise<void>((resolve) => {
          releaseThreadOpen = resolve;
        });
      }
      const threadPosition = {
        kind: 'thread',
        root_event_id: (request.position as { root_event_id?: string }).root_event_id ?? '',
      } as NativeTimelinePosition;
      const opened = openSnapshot(threadPosition);
      const threadSnapshot = {
        ...opened.snapshot,
        rows: threadRows(),
        pagination: {
          backward: params.has('threadMore') ? 'available' : 'exhausted',
          forward: 'exhausted',
        } as const,
      };
      snapshots.set(opened.streamId, threadSnapshot);
      openThreadStreams.add(opened.streamId);
      return { ...opened, snapshot: threadSnapshot } as T;
    }
    if (
      command === 'matrix_timeline_open' &&
      request?.position?.kind === 'focused' &&
      (request.position.event_id === '$gone' ||
        (scenario === 'threads' && request.position.event_id?.startsWith('$t')))
    ) {
      // Core resolves a routed event before choosing a provider: a thread
      // reply opens the live room at its root, a deleted one the live bottom.
      const reply = request.position.event_id !== '$gone';
      const opened = openSnapshot(
        reply ? { kind: 'focused', target_event_id: THREAD_ROOT } : { kind: 'live_bottom' }
      );
      return {
        ...opened,
        focusRoute: reply ? { kind: 'thread', root_event_id: THREAD_ROOT } : { kind: 'missing' },
      } as T;
    }
    if (command === 'matrix_timeline_open') {
      let selectedPosition = position;
      let lastRead = false;
      if (request?.position?.kind === 'focused') {
        if (!request.position.event_id) throw new Error('Focused open requires event_id');
        selectedPosition = { kind: 'focused', target_event_id: request.position.event_id };
        lastRead = request.position.event_id === '$missing';
        if (
          request.position.event_id === '$history-jump' &&
          !rows.some((row) => row.eventId === '$history-jump')
        ) {
          rows = [
            {
              ...makeRow(0),
              itemId: '$history-jump',
              eventId: '$history-jump',
              originServerTs: 1_600_000_000_000,
              body: 'History jump target',
            },
            ...rows,
          ];
          update();
        }
      }
      if (lastRead && failLastRead) {
        failLastRead = false;
        throw new Error('Last-read context is temporarily unavailable');
      }
      const opened = openSnapshot(selectedPosition, lastRead && !params.has('omitLastRead'));
      if (lastRead || params.get('earlyOpen') === 'initial') {
        lastCandidateStreamId = opened.streamId;
        if (params.has('earlyDeltas')) {
          const count = params.get('earlyDeltas') === 'overflow' ? 65 : 1;
          for (let index = 0; index < count; index += 1) {
            emitCandidateUpdate(
              'Last read changed during open',
              params.get('earlyDeltas') === 'gap',
              params.get('earlyDeltas') === 'remove'
            );
          }
        }
      }
      if (lastRead && params.has('delayLastRead')) {
        await new Promise<void>((resolve) => {
          releaseLastRead = resolve;
        });
      }
      return opened as T;
    }
    if (command === 'matrix_timeline_snapshot') {
      // Event-route tests cannot pass by repairing a lost batch with a later poll.
      return (
        params.has('nativeEvents') ? undefined : snapshots.get(args?.streamId as string)
      ) as T;
    }
    if (command === 'matrix_timeline_follow_live') {
      // An event-focused window cannot be promoted in place.
      if (params.has('eventWindow')) throw new Error('v-timeline-follow-live-tail-not-loaded');
      if (args?.observedLiveTailEventId !== rows.at(-1)?.eventId) throw new Error('Unseen tail');
      const current = snapshots.get(args?.streamId as string);
      if (!current) throw new Error('Unknown stream');
      const next = {
        ...current,
        revision: current.revision + 1,
        position: { kind: 'live_bottom' } as const,
      };
      snapshots.set(args?.streamId as string, next);
      return next as T;
    }
    if (command === 'matrix_timeline_set_read_state') {
      const current = snapshots.get(request?.streamId ?? '');
      if (!current) throw new Error('Unknown stream');
      // Core publishes the new read state as a delta before it replies; the
      // reply carries metadata only. Without a delta channel the fixture
      // leaves the stream as it was, as a dropped batch would.
      if (deltaSubscriptions.size === 0)
        return { view: withoutRows(current), receiptSent: true } as T;
      const next = {
        ...current,
        revision: current.revision + 1,
        readState: { ...current.readState, ownReadEventId: request?.observedLiveTailEventId },
      };
      snapshots.set(request?.streamId ?? '', next);
      emitBatch({
        schemaVersion: 1,
        sessionGeneration: next.sessionGeneration,
        roomId: next.roomId,
        streamId: request?.streamId ?? '',
        revision: next.revision,
        ops: [],
        readState: next.readState,
      });
      return { view: withoutRows(next), receiptSent: true } as T;
    }
    if (command === 'matrix_timeline_jump_latest') {
      const opened = openSnapshot({ kind: 'live_bottom' });
      if (params.get('earlyOpen') === 'latest') {
        lastCandidateStreamId = opened.streamId;
        emitCandidateUpdate('Changed during latest');
      }
      if (params.has('delayJump'))
        await new Promise<void>((resolve) => {
          releaseJump = resolve;
        });
      snapshots.delete(request?.streamId ?? '');
      return opened as T;
    }
    if (command === 'matrix_timeline_close') {
      snapshots.delete(request?.streamId ?? '');
      openThreadStreams.delete(request?.streamId ?? '');
      return undefined as T;
    }
    if (command === 'matrix_timeline_paginate') {
      if (failThreadPaginate && openThreadStreams.has(request?.streamId ?? '')) {
        failThreadPaginate = false;
        throw new Error('Thread history is temporarily unavailable');
      }
      const current = snapshots.get(request?.streamId ?? '');
      const direction = (args?.request as { direction?: string } | undefined)?.direction;
      if (current && params.has('eventWindow') && direction === 'forwards') {
        // The event-focused window reached the room's newest event; Core
        // publishes the page state as its own batch before replying.
        const next = {
          ...current,
          revision: current.revision + 1,
          pagination: { ...current.pagination, forward: 'exhausted' as const },
        };
        snapshots.set(request?.streamId ?? '', next);
        emitBatch({
          schemaVersion: 1,
          sessionGeneration: next.sessionGeneration,
          roomId: next.roomId,
          streamId: request?.streamId ?? '',
          revision: next.revision,
          ops: [],
          pagination: next.pagination,
        });
        return withoutRows(next) as T;
      }
      return (current ? withoutRows(current) : undefined) as T;
    }
    if (command === 'matrix_media_text_preview') {
      return { text: FILE_MD_BYTES } as T;
    }
    if (command === 'desktop_render_markdown') {
      // Core renders markdown (Rust `render_composer_markdown` tests pin the
      // rules); the harness returns the HTML Core produces for this fixture.
      return (args?.source === FILE_MD_BYTES ? FILE_MD_HTML : null) as T;
    }
    if (command === 'matrix_media_save') {
      const filename = typeof args?.filename === 'string' ? args.filename : 'download';
      return { filename } as T;
    }
    if (command === 'matrix_timeline_redact') {
      // Core's readback for a completed redaction of the requested event.
      const redact = args?.request as { roomId: string; eventId: string };
      return {
        schemaVersion: 1,
        action: 'redact',
        roomId: redact.roomId,
        eventId: redact.eventId,
        status: 'redacted',
      } as T;
    }
    return undefined as T;
  },
};

if (params.has('nativeEvents')) {
  // The real `@tauri-apps/api` Channel runs on these internals: it orders
  // messages by index and unregisters itself on the end message.
  const desktop = window.__SYNARA_DESKTOP__;
  const desktopInvoke = desktop?.invoke;
  if (!desktop || !desktopInvoke) throw new Error('The harness bridge is missing');
  desktop.invoke = async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
    if (command === 'desktop_timeline_subscribe') {
      commands.push({ command, args });
      const id = ++subscriptionSequence;
      deltaSubscriptions.set(id, {
        roomId: args?.roomId as string,
        channelId: (args?.channel as { id: number }).id,
        nextIndex: 0,
      });
      return id as T;
    }
    if (command === 'desktop_timeline_unsubscribe') {
      commands.push({ command, args });
      const id = args?.subscriptionId as number;
      const subscription = deltaSubscriptions.get(id);
      deltaSubscriptions.delete(id);
      for (const [streamId, owner] of streamOwners) {
        if (owner === id) streamOwners.delete(streamId);
      }
      // Rust drops the channel, which tells the webview no more will come.
      if (subscription) {
        channelCallbacks.get(subscription.channelId)?.({
          end: true,
          index: subscription.nextIndex,
        });
      }
      return (subscription !== undefined) as T;
    }
    const result = await desktopInvoke<T>(command, args);
    const request = args?.request as { streamId?: string } | undefined;
    if (command === 'matrix_timeline_open' || command === 'matrix_timeline_jump_latest') {
      claimStream(args?.subscriptionId, (result as { streamId?: string } | undefined)?.streamId);
    }
    if (command === 'matrix_timeline_close' && request?.streamId) {
      streamOwners.delete(request.streamId);
    }
    return result;
  };
  Object.assign(window, {
    __TAURI_INTERNALS__: {
      transformCallback: (handler: (message: unknown) => void) => {
        const id = ++callbackSequence;
        channelCallbacks.set(id, handler);
        return id;
      },
      unregisterCallback: (id: number) => {
        channelCallbacks.delete(id);
      },
      invoke: async (command: string, args: Record<string, unknown>) => {
        // Other views still listen to small Tauri events; none fire here.
        if (command === 'plugin:event|listen') return ++callbackSequence;
        if (command === 'plugin:event|unlisten') return undefined;
        return window.__SYNARA_DESKTOP__?.invoke?.(command, args);
      },
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => undefined },
  });
}

if (scenario === 'media' || params.has('surface')) {
  // Core serves media on its own protocol; tests route this origin path instead.
  Object.assign(window, {
    __TAURI_INTERNALS__: {
      ...(window as { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__,
      convertFileSrc: (handle: string) =>
        `${location.origin}/__synara-media/${encodeURIComponent(handle)}`,
    },
  });
}

const dispatchBatch = emitBatch;

const emitActiveBatch = (
  build: (current: NativeTimelineViewSnapshot, streamId: string) => NativeTimelineViewDeltaBatch
) => {
  for (const [streamId, current] of snapshots) {
    const batch = build(current, streamId);
    const next = applyNativeTimelineViewDelta(current, batch);
    if (!next) continue;
    snapshots.set(streamId, next);
    snapshot = next;
    dispatchBatch(batch);
  }
};

const api = {
  commands,
  activeStreamCount: () => snapshots.size,
  emitAfterOpen: () => emitCandidateUpdate('Last read changed after adoption'),
  releaseOperation: () => releaseOperation?.(),
  append() {
    rows.push(makeRow(++sequence));
    update();
  },
  appendLive() {
    const row = makeRow(++sequence);
    rows.push(row);
    emitActiveBatch((current, streamId) => ({
      schemaVersion: 1,
      sessionGeneration: current.sessionGeneration,
      roomId: current.roomId,
      streamId,
      revision: current.revision + 1,
      ops: [{ op: 'push_back', row }],
    }));
  },
  prependHistory(count = 40) {
    const newRows = Array.from({ length: count }, () => makeRow(--historyIndex)).reverse();
    rows = [...newRows, ...rows];
    emitActiveBatch((current, streamId) => ({
      schemaVersion: 1,
      sessionGeneration: current.sessionGeneration,
      roomId: current.roomId,
      streamId,
      revision: current.revision + 1,
      ops: [{ op: 'reset', rows: [...newRows, ...current.rows] }],
    }));
  },
  prependMediaWithoutInfo(count = 8) {
    const newRows = Array.from({ length: count }, () => {
      const row = makeRow(--historyIndex);
      return {
        ...row,
        messageType: 'image' as const,
        media: { handleId: `missing-media:${row.itemId}` },
      };
    }).reverse();
    rows = [...newRows, ...rows];
    emitActiveBatch((current, streamId) => ({
      schemaVersion: 1,
      sessionGeneration: current.sessionGeneration,
      roomId: current.roomId,
      streamId,
      revision: current.revision + 1,
      ops: [{ op: 'reset', rows: [...newRows, ...current.rows] }],
    }));
  },
  metadataPulse() {
    emitActiveBatch((current, streamId) => ({
      schemaVersion: 1,
      sessionGeneration: current.sessionGeneration,
      roomId: current.roomId,
      streamId,
      revision: current.revision + 1,
      ops: [],
      readState: { ...current.readState },
    }));
  },
  emitDeltaFlood(count = 130) {
    for (let index = 0; index < count; index += 1) {
      emitActiveBatch((current, streamId) => ({
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 1,
        ops: [],
        readState: { ...current.readState },
      }));
    }
  },
  outOfOrderLiveAppends() {
    const rowA = makeRow(++sequence);
    const rowB = makeRow(++sequence);
    rows.push(rowA, rowB);
    for (const [streamId, current] of snapshots) {
      const first: NativeTimelineViewDeltaBatch = {
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 1,
        ops: [{ op: 'push_back', row: rowA }],
      };
      const second: NativeTimelineViewDeltaBatch = {
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 2,
        ops: [{ op: 'push_back', row: rowB }],
      };
      const withFirst = applyNativeTimelineViewDelta(current, first);
      const withSecond = withFirst ? applyNativeTimelineViewDelta(withFirst, second) : undefined;
      if (!withSecond) continue;
      snapshots.set(streamId, withSecond);
      snapshot = withSecond;
      dispatchBatch(second);
      dispatchBatch(first);
    }
  },
  emitRevisionGap() {
    if (deltaSubscriptions.size === 0 || snapshots.size === 0) {
      throw new Error('emitRevisionGap requires an open native timeline stream');
    }
    const row = makeRow(++sequence);
    rows.push(row);
    for (const [streamId, current] of snapshots) {
      // Skip more than one revision so a concurrent follow-live increment
      // (N -> N+1) cannot turn this batch into a sequential apply.
      dispatchBatch({
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 8,
        ops: [{ op: 'push_back', row }],
      });
    }
  },
  metadataThenOps() {
    const row = makeRow(++sequence);
    rows.push(row);
    for (const [streamId, current] of snapshots) {
      const meta: NativeTimelineViewDeltaBatch = {
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 1,
        ops: [],
        readState: { ...current.readState, isMarkedUnread: false },
      };
      const ops: NativeTimelineViewDeltaBatch = {
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 2,
        ops: [{ op: 'push_back', row }],
      };
      const withMeta = applyNativeTimelineViewDelta(current, meta);
      const withOps = withMeta ? applyNativeTimelineViewDelta(withMeta, ops) : undefined;
      if (!withOps) continue;
      snapshots.set(streamId, withOps);
      snapshot = withOps;
      dispatchBatch(meta);
      dispatchBatch(ops);
    }
  },
  growEdit(eventId = '$50') {
    emitActiveBatch((current, streamId) => {
      const index = current.rows.findIndex(
        (row) => 'eventId' in row && row.eventId === eventId && row.kind === 'message'
      );
      const row = index >= 0 ? current.rows[index] : undefined;
      if (!row || row.kind !== 'message') {
        return {
          schemaVersion: 1,
          sessionGeneration: current.sessionGeneration,
          roomId: current.roomId,
          streamId,
          revision: current.revision + 1,
          ops: [],
          readState: { ...current.readState },
        };
      }
      return {
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 1,
        ops: [
          {
            op: 'set',
            index,
            row: {
              ...row,
              edited: true,
              body: `${row.body}\n${Array.from(
                { length: 12 },
                (_, line) => `Grown edit ${line}.`
              ).join('\n')}`,
            },
          },
        ],
      };
    });
  },
  hermesActivityEdit(updates = 54, eventId = '$1') {
    emitActiveBatch((current, streamId) => {
      const index = current.rows.findIndex((row) => 'eventId' in row && row.eventId === eventId);
      const row = current.rows[index];
      if (!row || row.kind !== 'message') throw new Error('Hermes edit fixture row missing');
      const entries = Array.from({ length: updates }, (_, line) =>
        line % 7 === 3
          ? `<li>❌ Background task failed: <code>ssh -o BatchMode=yes -o ConnectTimeout=25 agent@example.test</code>. Last output:<pre><code>File &quot;/src/tensorfold/families/glm5_next/cuda/nvfp4_admit.py&quot;, line 180\n  require_reserve(available(), steps[layer], total_reserve)\n  raise NoHostReserve()</code></pre></li>`
          : `<li>📖 Reading agent notes ${
              line + 1
            }. The comparison set is ready and the server is off. Check the available memory before loading weights and running the same questions.</li>`
      ).join('');
      return {
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 1,
        ops: [
          {
            op: 'set',
            index,
            row: {
              ...row,
              edited: true,
              body: `🛠 Tool activity (${updates} updates)\n${Array.from(
                { length: updates },
                (_, line) => `${line + 1}. Reading agent notes.`
              ).join('\n')}`,
              formattedBody: `<p><strong>🛠 Tool activity (${updates} updates)</strong></p><ol>${entries}</ol>`,
            },
          },
        ],
      };
    });
  },
  addReaction(eventId = '$50') {
    emitActiveBatch((current, streamId) => {
      const index = current.rows.findIndex(
        (row) => 'eventId' in row && row.eventId === eventId && row.kind === 'message'
      );
      const row = index >= 0 ? current.rows[index] : undefined;
      if (!row || row.kind !== 'message') {
        return {
          schemaVersion: 1,
          sessionGeneration: current.sessionGeneration,
          roomId: current.roomId,
          streamId,
          revision: current.revision + 1,
          ops: [],
          readState: { ...current.readState },
        };
      }
      return {
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 1,
        ops: [
          {
            op: 'set',
            index,
            row: {
              ...row,
              reactions: [
                ...(row.reactions ?? []),
                {
                  key: '✅',
                  count: 1,
                  own: true,
                  senders: [
                    {
                      userId: '@alice:example.org',
                      reactionEventId: '$alice-reaction:example.org',
                    },
                  ],
                },
              ],
            },
          },
        ],
      };
    });
  },
  insertUngroupedHeader(eventId = '$50') {
    emitActiveBatch((current, streamId) => {
      const index = current.rows.findIndex(
        (row) => 'eventId' in row && row.eventId === eventId && row.kind === 'message'
      );
      if (index < 0) {
        return {
          schemaVersion: 1,
          sessionGeneration: current.sessionGeneration,
          roomId: current.roomId,
          streamId,
          revision: current.revision + 1,
          ops: [],
          readState: { ...current.readState },
        };
      }
      const divider = {
        kind: 'date_separator' as const,
        itemId: `$divider-${current.revision}`,
        timestampMs: 1_700_000_000_000,
      };
      return {
        schemaVersion: 1,
        sessionGeneration: current.sessionGeneration,
        roomId: current.roomId,
        streamId,
        revision: current.revision + 1,
        ops: [{ op: 'insert', index, row: divider }],
      };
    });
  },
  scrollEventIntoView(eventId: string) {
    document
      .querySelector<HTMLElement>(`[data-native-timeline-event-id="${eventId}"]`)
      ?.scrollIntoView({ block: 'center', inline: 'nearest' });
  },
  resizeTimeline(height: number) {
    const el = document.getElementById('native-timeline');
    if (el) el.style.height = `${height}px`;
  },
  edit() {
    rows = rows.map((row, index) =>
      index === 0 ? { ...row, body: `${row.body}\nEdited without navigation.` } : row
    );
    update();
  },
  prependMissing() {
    rows.unshift({ ...makeRow(0), eventId: '$missing', itemId: '$missing' });
    update();
  },
  unread() {
    position = { kind: 'unread', anchor_event_id: '$2' };
    update();
  },
  missing() {
    position = { kind: 'unread', anchor_event_id: '$missing' };
    update();
  },
  releaseLastRead() {
    releaseLastRead?.();
  },
  releaseJump() {
    releaseJump?.();
  },
  /** A composer send into the room, or into one of its threads. */
  send(roomId = room, threadRootId?: string) {
    requestRoomLatestAfterSend(roomId, threadRootId);
  },
  /** Root of the `scenario=threads` thread. */
  threadRoot: THREAD_ROOT,
  openThreadStreams() {
    return openThreadStreams.size;
  },
  /** Open delta channels, and which channel each open stream is claimed by. */
  deltaChannels() {
    return {
      subscriptions: [...deltaSubscriptions.keys()],
      threadOwners: [...openThreadStreams].map((streamId) => streamOwners.get(streamId)),
      roomOwners: [...snapshots.keys()]
        .filter((streamId) => !openThreadStreams.has(streamId))
        .map((streamId) => streamOwners.get(streamId)),
    };
  },
  threadOpenCount() {
    return threadOpenCount;
  },
  releaseThreadOpen() {
    releaseThreadOpen?.();
  },
};
Object.assign(window, { nativeTimelineFixture: api });
setNativeIdentity({ userId: '@reader0:example.test' });
// The shipped app applies the folds theme to <body> (src/index.tsx). Overlay
// offsets like `config.space.S300` compile to CSS variables that only exist
// under these classes; without them absolute controls collapse to the origin.
// Color tokens (`color.SurfaceVariant.*`) need a theme class. Await index.css
// before paint so screenshot/polish captures are not racing the stylesheet.

const threadDisplay = normalizeThreadDisplay(params.get('threadDisplay'));

function App() {
  const [mounted, setMounted] = useState(true);
  const [focusedEventId, setFocusedEventId] = useState<string>();
  const [paneRoot, setPaneRoot] = useState<string>();
  const [paneFocus, setPaneFocus] = useState<string>();
  const [paneWidth, setPaneWidth] = useState(360);
  return (
    <>
      <button onClick={() => setMounted((value) => !value)}>Toggle room</button>
      <button onClick={() => setFocusedEventId('$30')}>Focus middle</button>
      <button onClick={() => setFocusedEventId('$t3')}>Focus thread reply</button>
      <button onClick={() => setFocusedEventId('$gone')}>Focus deleted</button>
      <button onClick={() => setFocusedEventId('$broken')}>Focus broken</button>
      <div style={{ display: 'flex', alignItems: 'stretch' }}>
        <div
          id="native-timeline"
          style={{
            height: polish ? 640 : 480,
            width: polish ? 900 : 700,
            display: 'flex',
            flexDirection: 'column',
            border: '1px solid gray',
          }}
        >
          {mounted && (
            <NativeTimelinePresenter
              roomId={room}
              eventId={focusedEventId}
              threadDisplay={threadDisplay}
              onOpenThreadPane={
                threadDisplay === 'full'
                  ? undefined
                  : (rootEventId, _latestEventId, focusEventId) => {
                      setPaneRoot(rootEventId);
                      setPaneFocus(focusEventId);
                    }
              }
            />
          )}
        </div>
        {paneRoot && (
          <div style={{ height: polish ? 640 : 480, display: 'flex' }}>
            <ThreadSidePanel
              roomId={room}
              rootEventId={paneRoot}
              focusEventId={paneFocus}
              width={paneWidth}
              availableWidth={1600}
              onWidthChange={setPaneWidth}
              onClose={() => setPaneRoot(undefined)}
            />
          </div>
        )}
      </div>
    </>
  );
}

async function bootHarness() {
  document.body.classList.add(configClass, varsClass);
  if (polish || params.has('theme')) {
    await import('../../src/index.css');
    if (params.get('theme') === 'light') {
      document.body.style.backgroundColor = '#ffffff';
      document.body.style.color = '#161719';
    } else {
      document.body.classList.add(darkTheme, 'dark-theme');
      document.body.style.backgroundColor = '#161719';
      document.body.style.color = '#ededed';
    }
  }
  if (params.has('surface')) {
    // Message text outside the timeline (inbox, pins, search, topics).
    const { mountRichTextSurfaces } = await import('./surfaces');
    mountRichTextSurfaces(document.getElementById('root')!);
    return;
  }
  createRoot(document.getElementById('root')!).render(<App />);
}

void bootHarness();
