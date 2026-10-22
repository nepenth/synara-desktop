import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { pickFileDropZone, windowPasteTargetsComposer } from '../useFileDrop';

type FakeElement = {
  name: string;
  parent?: FakeElement;
  region?: boolean;
  contains: (other: FakeElement | null) => boolean;
  closest: (selector: string) => FakeElement | null;
};

const element = (name: string, parent?: FakeElement, region = false): FakeElement => {
  const self: FakeElement = {
    name,
    parent,
    region,
    contains: (other) => {
      for (let node = other ?? undefined; node; node = node.parent) {
        if (node === self) return true;
      }
      return false;
    },
    closest: () => {
      for (let node: FakeElement | undefined = self; node; node = node.parent) {
        if (node.region) return node;
      }
      return null;
    },
  };
  return self;
};

const app = element('app');
const roomRegion = element('room-region', app, true);
const timeline = element('timeline', roomRegion);
const threadRegion = element('thread-region', app, true);
const threadMessage = element('thread-message', threadRegion);
const sidebar = element('sidebar', app);

const roomZone = { name: 'room', region: () => roomRegion as unknown as Element, primary: true };
const threadZone = {
  name: 'thread',
  region: () => threadRegion as unknown as Element,
  primary: false,
};

test('a drop reaches only the composer whose pane is under the pointer', () => {
  const zones = [threadZone, roomZone];
  assert.equal(pickFileDropZone(zones, threadMessage as unknown as Element)?.name, 'thread');
  assert.equal(pickFileDropZone(zones, timeline as unknown as Element)?.name, 'room');
  // Outside both panes (sidebar, missing hit): the room composer.
  assert.equal(pickFileDropZone(zones, sidebar as unknown as Element)?.name, 'room');
  assert.equal(pickFileDropZone(zones, null)?.name, 'room');
  assert.equal(pickFileDropZone([], null), undefined);
});

test('a window paste goes to the composer of the focused pane only', () => {
  const room = roomRegion as unknown as Element;
  const thread = threadRegion as unknown as Element;
  const focusInThread = threadMessage as unknown as Element;
  const focusInRoom = timeline as unknown as Element;
  assert.equal(windowPasteTargetsComposer(thread, focusInThread, false), true);
  assert.equal(windowPasteTargetsComposer(room, focusInThread, true), false);
  assert.equal(windowPasteTargetsComposer(room, focusInRoom, true), true);
  assert.equal(windowPasteTargetsComposer(thread, focusInRoom, false), false);
  // Focus outside every pane (body, sidebar): the room composer.
  assert.equal(windowPasteTargetsComposer(room, sidebar as unknown as Element, true), true);
  assert.equal(windowPasteTargetsComposer(thread, null, false), false);
});

test('native drops arrive once, on one channel, and are size-checked before any read', () => {
  const hook = readFileSync('src/app/hooks/useFileDrop.ts', 'utf8');
  assert.ok(!hook.includes("addEventListener('synara-native-file-drop'"));
  assert.equal(hook.match(/listen<PlatformNativeFileDropPayload>/g)?.length, 1);
  assert.ok(hook.includes('readPlatformDroppedFiles(detail.paths, limitRef.current)'));
  const roomInput = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  assert.ok(roomInput.includes('maxFileBytes: attachmentLimit'));
  assert.ok(roomInput.includes('primary: !threadRootOverride'));
  const room = readFileSync('src/app/features/room/Room.tsx', 'utf8');
  assert.ok(room.includes('data-file-drop-region="room"'));
  const pane = readFileSync('src/app/features/room/ThreadSidePanel.tsx', 'utf8');
  assert.ok(pane.includes('data-file-drop-region="thread"'));
});
