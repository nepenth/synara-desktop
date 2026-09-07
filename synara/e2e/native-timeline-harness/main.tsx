// Actual shipped presenter/controller and browser geometry; only native IPC is
// a deterministic fixture. This cannot establish live SDK/Matrix correctness.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import 'folds/dist/style.css';
import { NativeTimelinePresenter } from '../../src/app/features/room/NativeTimelinePresenter';
import { requestRoomLatestAfterSend } from '../../src/app/features/room/nativeTimelineNavigation';
import type {
  NativeTimelinePosition,
  NativeTimelineViewDeltaBatch,
  NativeTimelineViewSnapshot,
} from '../../src/app/features/room/nativeTimelineView';

const room = '!navigation:example.test';
const params = new URLSearchParams(location.search);
const scenario = params.get('scenario') ?? 'live';
let sequence = scenario === 'sparse-missing' ? 1 : scenario === 'short' ? 2 : 60;
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
  body: `Message ${index}\nNative timeline geometry fixture line two.\nLine three.`,
  edited: false,
  capabilities: {
    react: false,
    reply: false,
    edit: false,
    redact: false,
    report: false,
    pin: false,
    forward: false,
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
        revision: current.revision + 10,
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
    if (command === 'matrix_timeline_open') {
      let selectedPosition = position;
      let lastRead = false;
      if (request?.position?.kind === 'focused') {
        if (!request.position.event_id) throw new Error('Focused open requires event_id');
        selectedPosition = { kind: 'focused', target_event_id: request.position.event_id };
        lastRead = request.position.event_id === '$missing';
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

const api = {
  commands,
  activeStreamCount: () => snapshots.size,
  emitAfterOpen: () => emitCandidateUpdate('Last read changed after adoption'),
  releaseOperation: () => releaseOperation?.(),
  append() {
    rows.push(makeRow(++sequence));
    update();
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
          height: 480,
          width: 700,
          display: 'flex',
          flexDirection: 'column',
          border: '1px solid gray',
        }}
      >
        {mounted && <NativeTimelinePresenter roomId={room} eventId={focusedEventId} />}
      </div>
    </>
  );
}
createRoot(document.getElementById('root')!).render(<App />);
