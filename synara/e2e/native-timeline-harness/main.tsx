// Actual shipped presenter/controller and browser geometry; only native IPC is
// a deterministic fixture. This cannot establish live SDK/Matrix correctness.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import { darkTheme } from '../../src/colors.css';
import { NativeTimelinePresenter } from '../../src/app/features/room/NativeTimelinePresenter';
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
const FILE_MD_HANDLE = `timeline-media-${'ab'.repeat(32)}`;
const FILE_ZIP_HANDLE = `timeline-media-${'cd'.repeat(32)}`;
const FILE_MD_BYTES = '# Agent notes\n\nUse **bold** for emphasis.\n';
let sequence = polish
  ? 4
  : scenario === 'sparse-missing' || scenario === 'file-md' || scenario === 'file-zip'
  ? 1
  : scenario === 'short'
  ? 2
  : jank
  ? 180
  : 60;
let historyIndex = 0;
let stream = 0;
let releaseJump: (() => void) | undefined;
let releaseLastRead: (() => void) | undefined;
let failLastRead = params.has('failLastRead');
let lastCandidateStreamId: string | undefined;
let releaseOperation: (() => void) | undefined;
let operationDeferred = false;
const eventCallbacks = new Map<number, (event: unknown) => void>();
const eventListeners = new Map<number, { event: string; handler: number }>();
let callbackSequence = 0;
let eventSequence = 0;
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
    edit: false,
    redact: polish,
    report: polish,
    pin: polish,
    forward: polish,
    vote: false,
    declineCall: false,
  },
});
let rows = Array.from({ length: sequence }, (_, index) => makeRow(index + 1));
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
  for (const [id, listener] of eventListeners) {
    if (listener.event === 'matrix-timeline-view-updated') {
      eventCallbacks.get(listener.handler)?.({ event: listener.event, id, payload: batch });
    }
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
      return (
        command === 'matrix_timeline_set_read_state'
          ? { snapshot: oldSnapshot, receiptSent: true }
          : oldSnapshot
      ) as T;
    }
    if (command === 'matrix_timeline_timestamp_to_event') {
      return {
        roomId: room,
        eventId: '$history-jump',
        originServerTs: Number(args?.timestampMs) || 1_600_000_000_000,
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
      const next = {
        ...current,
        revision: current.revision + 1,
        readState: { ...current.readState, ownReadEventId: request?.observedLiveTailEventId },
      };
      snapshots.set(request?.streamId ?? '', next);
      return { snapshot: next, receiptSent: true } as T;
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
      return undefined as T;
    }
    if (command === 'matrix_timeline_paginate') return snapshots.get(request?.streamId ?? '') as T;
    if (command === 'matrix_media_download') {
      const contentUri = args?.contentUri;
      if (contentUri === FILE_ZIP_HANDLE) {
        return { bytes: [80, 75, 3, 4] } as T;
      }
      return { bytes: Array.from(new TextEncoder().encode(FILE_MD_BYTES)) } as T;
    }
    if (command === 'desktop_save_file') {
      const filename = (args?.payload as { filename?: string } | undefined)?.filename;
      return `/tmp/synara-e2e-${filename || 'download'}` as T;
    }
    return undefined as T;
  },
};

if (params.has('nativeEvents')) {
  Object.assign(window, {
    __TAURI_INTERNALS__: {
      transformCallback: (handler: (event: unknown) => void) => {
        const id = ++callbackSequence;
        eventCallbacks.set(id, handler);
        return id;
      },
      invoke: async (command: string, args: Record<string, unknown>) => {
        if (command === 'plugin:event|listen') {
          const id = ++eventSequence;
          eventListeners.set(id, { event: args.event as string, handler: args.handler as number });
          return id;
        }
        if (command === 'plugin:event|unlisten') {
          eventListeners.delete(args.eventId as number);
          return undefined;
        }
        return window.__SYNARA_DESKTOP__?.invoke?.(command, args);
      },
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener: (_event: string, id: number) => {
        const listener = eventListeners.get(id);
        if (listener) eventCallbacks.delete(listener.handler);
        eventListeners.delete(id);
      },
    },
  });
}

const dispatchBatch = (batch: NativeTimelineViewDeltaBatch) => {
  for (const [id, listener] of eventListeners) {
    if (listener.event === 'matrix-timeline-view-updated') {
      eventCallbacks.get(listener.handler)?.({ event: listener.event, id, payload: batch });
    }
  }
};

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
    if (eventListeners.size === 0 || snapshots.size === 0) {
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
              reactions: [...(row.reactions ?? []), { key: '✅', count: 1, own: true }],
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
  send(roomId = room) {
    requestRoomLatestAfterSend(roomId);
  },
};
Object.assign(window, { nativeTimelineFixture: api });
// The shipped app applies the folds theme to <body> (src/index.tsx). Overlay
// offsets like `config.space.S300` compile to CSS variables that only exist
// under these classes; without them absolute controls collapse to the origin.
// Color tokens (`color.SurfaceVariant.*`) need a theme class. Await index.css
// before paint so screenshot/polish captures are not racing the stylesheet.

function App() {
  const [mounted, setMounted] = useState(true);
  const [focusedEventId, setFocusedEventId] = useState<string>();
  return (
    <>
      <button onClick={() => setMounted((value) => !value)}>Toggle room</button>
      <button onClick={() => setFocusedEventId('$30')}>Focus middle</button>
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
            roomCreatedTs={
              params.has('roomCreated')
                ? Number(params.get('roomCreated')) || 1_600_000_000_000
                : undefined
            }
          />
        )}
      </div>
    </>
  );
}

async function bootHarness() {
  document.body.classList.add(configClass, varsClass);
  if (polish || params.has('theme')) {
    await import('../../src/index.css');
    document.body.classList.add(darkTheme, 'dark-theme');
    document.body.style.backgroundColor = '#161719';
    document.body.style.color = '#ededed';
  }
  createRoot(document.getElementById('root')!).render(<App />);
}

void bootHarness();
