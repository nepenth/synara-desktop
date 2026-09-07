// Actual shipped presenter/controller and browser geometry; only native IPC is
// a deterministic fixture. This cannot establish live SDK/Matrix correctness.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import 'folds/dist/style.css';
import { NativeTimelinePresenter } from '../../src/app/features/room/NativeTimelinePresenter';
import { requestRoomLatestAfterSend } from '../../src/app/features/room/nativeTimelineNavigation';
import type {
  NativeTimelinePosition,
  NativeTimelineViewSnapshot,
} from '../../src/app/features/room/nativeTimelineView';

const room = '!navigation:example.test';
const params = new URLSearchParams(location.search);
const scenario = params.get('scenario') ?? 'live';
let sequence = scenario === 'sparse-missing' ? 1 : scenario === 'short' ? 2 : 60;
let stream = 0;
let releaseJump: (() => void) | undefined;
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
const update = () => {
  snapshot = {
    schemaVersion: 1,
    sessionGeneration: 1,
    roomId: room,
    revision: (snapshot?.revision ?? 0) + 1,
    position,
    rows: [...rows],
    pagination: { backward: 'available', forward: 'available' },
    readState: {
      visibleTailEventId: rows.at(-1)?.eventId,
      receiptTailEventId: rows.at(-1)?.eventId,
      ownReadEventId: '$2',
      unreadAnchorEventId: '$2',
      isMarkedUnread: false,
    },
    capabilities: {
      markRead: true,
      markUnread: true,
      paginateBackward: true,
      paginateForward: true,
    },
  };
};
update();
window.__SYNARA_DESKTOP__ = {
  platform: 'tauri',
  invoke: async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
    commands.push({ command, args });
    const request = args?.request as
      | { position?: { kind: string; event_id?: string }; observedLiveTailEventId?: string }
      | undefined;
    if (command === 'matrix_timeline_open') {
      stream += 1;
      if (request?.position?.kind === 'focused') {
        if (!request.position.event_id) throw new Error('Focused open requires event_id');
        position = { kind: 'focused', target_event_id: request.position.event_id };
      }
      update();
      return { schemaVersion: 1, streamId: `fixture-${stream}`, position, snapshot } as T;
    }
    if (command === 'matrix_timeline_snapshot') return snapshot as T;
    if (command === 'matrix_timeline_follow_live') {
      if (scenario === 'sparse-missing') throw new Error('Loaded window is not the live tail');
      if (args?.observedLiveTailEventId !== rows.at(-1)?.eventId) throw new Error('Unseen tail');
      position = { kind: 'live_bottom' };
      update();
      return snapshot as T;
    }
    if (command === 'matrix_timeline_set_read_state') {
      update();
      snapshot.readState.ownReadEventId = request?.observedLiveTailEventId;
      return { snapshot, receiptSent: true } as T;
    }
    if (command === 'matrix_timeline_jump_latest') {
      const originalStream = stream;
      if (params.has('delayJump'))
        await new Promise<void>((resolve) => {
          releaseJump = resolve;
        });
      if (stream !== originalStream) {
        // A delayed result belongs to its original native provider. It must not
        // mutate the newer focused provider served by subsequent snapshot polls.
        const latestPosition = { kind: 'live_bottom' } as const;
        return {
          schemaVersion: 1,
          streamId: `fixture-${originalStream}-latest`,
          position: latestPosition,
          snapshot: { ...snapshot, position: latestPosition },
        } as T;
      }
      position = { kind: 'live_bottom' };
      update();
      return { schemaVersion: 1, streamId: `fixture-${stream}`, position, snapshot } as T;
    }
    if (command === 'matrix_timeline_close') return undefined as T;
    if (command === 'matrix_timeline_paginate') return snapshot as T;
    return undefined as T;
  },
};

const api = {
  commands,
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
