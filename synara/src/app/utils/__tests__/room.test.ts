import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

import {
  canEditEvent,
  getAllVersionsRoomCreator,
  getDirectRoomAvatarUrl,
  getEditedEvent,
  getEventEdits,
  getMemberAvatarMxc,
  getMemberDisplayName,
  getMemberSearchStr,
  getMentionContent,
  getNotificationType,
  getRoomAvatarUrl,
  getStateEvent,
  getStateEvents,
  getThreadRootEventId,
  getUnreadInfo,
  guessPerfectParent,
  isNotificationEvent,
  isRoom,
  isSpace,
  isUnsupportedRoom,
  type MatrixClientReading,
  type MatrixEventReading,
  type RoomReading,
  type EventTimelineSetReading,
  type MemberReading,
} from '../room';
import { MessageEvent, NotificationType, StateEvent } from '../../../types/matrix/room';
import {
  clearNativeRoomStateProjections,
  publishNativeRoomCreatorsProjection,
} from '../../features/matrix-dto/nativeRoomStateProjection';
import {
  resetSessionBootstrapForTests,
  setSessionBootstrapResult,
} from '../../state/sessionBootstrap';

const makeEvent = (
  type: string,
  content: Record<string, unknown>,
  opts: {
    sender?: string;
    id?: string;
    ts?: number;
    redacted?: boolean;
    relation?: { rel_type?: string; event_id?: string };
    stateKey?: string;
    prevContent?: Record<string, unknown>;
    raw?: Record<string, unknown>;
  } = {}
): MatrixEventReading =>
  ({
    getContent: () => content,
    getPrevContent: () => opts.prevContent ?? {},
    getSender: () => opts.sender,
    getType: () => type,
    getStateKey: () => opts.stateKey,
    getTs: () => opts.ts ?? 0,
    getId: () => opts.id,
    getRoomId: () => undefined,
    isRedacted: () => opts.redacted ?? false,
    isSending: () => false,
    getRelation: () => opts.relation ?? null,
    event: { sender: opts.sender, ...opts.raw },
  } as MatrixEventReading);

const makeRoom = (
  stateEvents: Record<string, MatrixEventReading | MatrixEventReading[]>,
  opts: {
    roomId?: string;
    members?: MemberReading[];
    avatarUrl?: string | null;
    unreadTotal?: number;
    unreadHighlight?: number;
    readUpTo?: string;
    markedUnread?: boolean;
    membership?: string;
    isSpaceRoom?: boolean;
  } = {}
): RoomReading => {
  const roomState = {
    getStateEvents: (eventType: string, stateKey?: string) => {
      if (stateKey !== undefined) {
        const hit = stateEvents[eventType];
        if (Array.isArray(hit))
          return hit.find((event) => event.getStateKey() === stateKey) ?? null;
        return hit ?? null;
      }
      return stateEvents[eventType] ?? [];
    },
  };
  return {
    roomId: opts.roomId ?? '!room:example.org',
    currentState: roomState,
    getLiveTimeline: () => ({
      getState: () => roomState,
      getEvents: () => [],
    }),
    getMember: (userId: string) => opts.members?.find((member) => member.userId === userId) ?? null,
    getMembers: () => opts.members ?? [],
    getMxcAvatarUrl: () => opts.avatarUrl ?? null,
    getAvatarFallbackMember: () => undefined,
    getUnreadNotificationCount: (type?: string) =>
      type === 'highlight' ? opts.unreadHighlight ?? 0 : opts.unreadTotal ?? 0,
    getEventReadUpTo: () => opts.readUpTo ?? null,
    accountData: {
      get: () =>
        opts.markedUnread
          ? ({ getContent: () => ({ unread: true }) } as MatrixEventReading)
          : undefined,
    },
    getMyMembership: () => opts.membership ?? 'join',
    isSpaceRoom: () => opts.isSpaceRoom ?? false,
    hasMembershipState: () => false,
  } as unknown as RoomReading;
};

const makeClient = (overrides: Partial<MatrixClientReading> = {}): MatrixClientReading =>
  ({
    getAccountData: () => undefined,
    getRoomPushRule: () => undefined,
    getUserId: () => '@alice:example.org',
    getRooms: () => [],
    getRoom: () => null,
    mxcUrlToHttp: () => null,
    ...overrides,
  } as MatrixClientReading);

test('getStateEvent reads the indexed state projection and falls back to undefined', () => {
  const topic = makeEvent('m.room.topic', { topic: 'Hello' }, { stateKey: '' });
  const room = makeRoom({ 'm.room.topic': topic });
  assert.equal(getStateEvent(room, StateEvent.RoomTopic)?.getContent().topic, 'Hello');
  assert.equal(getStateEvent(room, StateEvent.RoomName), undefined);
});

test('getStateEvents returns all matching events', () => {
  const a = makeEvent('m.space.parent', { via: [] }, { stateKey: '!a:example.org' });
  const b = makeEvent('m.space.parent', { via: [] }, { stateKey: '!b:example.org' });
  const room = makeRoom({ 'm.space.parent': [a, b] });
  assert.deepEqual(
    getStateEvents(room, StateEvent.SpaceParent).map((event) => event.getStateKey()),
    ['!a:example.org', '!b:example.org']
  );
});

test('isSpace/isRoom/isUnsupportedRoom classify by room create type', () => {
  const space = makeRoom({ 'm.room.create': makeEvent('m.room.create', { type: 'm.space' }) });
  const room = makeRoom({ 'm.room.create': makeEvent('m.room.create', {}) });
  const noCreate = makeRoom({});
  const weird = makeRoom({
    'm.room.create': makeEvent('m.room.create', { type: 'org.example.unknown' }),
  });
  // Native room-list projections intentionally do not fabricate m.room.create.
  const nativeSpace = makeRoom({}, { isSpaceRoom: true });

  assert.equal(isSpace(space), true);
  assert.equal(isRoom(space), false);
  assert.equal(isSpace(room), false);
  assert.equal(isRoom(room), true);
  assert.equal(isSpace(nativeSpace), true);
  assert.equal(isRoom(nativeSpace), false);
  assert.equal(isUnsupportedRoom(room), false);
  assert.equal(isUnsupportedRoom(noCreate), true);
  assert.equal(isUnsupportedRoom(weird), true);
  assert.equal(isSpace(null), false);
  assert.equal(isRoom(null), false);
  assert.equal(isUnsupportedRoom(null), false);
});

test('getUnreadInfo reports the max of highlight and total', () => {
  const room = makeRoom({}, { unreadTotal: 3, unreadHighlight: 5, roomId: '!r:example.org' });
  assert.deepEqual(getUnreadInfo(room), { roomId: '!r:example.org', highlight: 5, total: 5 });
});

test('member display/avatar helpers resolve from the room member projection', () => {
  const members: MemberReading[] = [
    {
      userId: '@bob:example.org',
      rawDisplayName: 'Bob',
      getMxcAvatarUrl: () => 'mxc://avatar/bob',
      events: { member: undefined },
    },
    {
      userId: '@alone:example.org',
      rawDisplayName: '@alone:example.org',
      getMxcAvatarUrl: () => 'mxc://avatar/alone',
      events: { member: undefined },
    },
  ];
  const room = makeRoom({}, { members });
  assert.equal(getMemberDisplayName(room, '@bob:example.org'), 'Bob');
  assert.equal(getMemberDisplayName(room, '@alone:example.org'), undefined);
  assert.equal(getMemberAvatarMxc(room, '@bob:example.org'), 'mxc://avatar/bob');
});

test('getMemberSearchStr supports SDK and native member shapes', () => {
  const mxIdToName = (mxId: string) => mxId;
  assert.deepEqual(
    getMemberSearchStr(
      { userId: '@b:example.org', rawDisplayName: 'B', getMxcAvatarUrl: () => undefined },
      'B',
      mxIdToName
    ),
    ['B', '@b:example.org']
  );
  assert.deepEqual(
    getMemberSearchStr(
      { userId: '@b:example.org', displayName: 'Native B' } as never,
      'B',
      (id) => id
    ),
    ['Native B', '@b:example.org']
  );
});

test('canEditEvent requires own text/emote/notice messages without non-thread relations', () => {
  const client = makeClient();
  const own = makeEvent(
    MessageEvent.RoomMessage,
    { msgtype: 'm.text' },
    { sender: '@alice:example.org' }
  );
  const reply = makeEvent(
    MessageEvent.RoomMessage,
    { msgtype: 'm.text', 'm.relates_to': { rel_type: 'm.annotation', event_id: '$x' } },
    { sender: '@alice:example.org' }
  );
  const thread = makeEvent(
    MessageEvent.RoomMessage,
    { msgtype: 'm.text', 'm.relates_to': { rel_type: 'm.thread', event_id: '$root' } },
    { sender: '@alice:example.org' }
  );
  const other = makeEvent(
    MessageEvent.RoomMessage,
    { msgtype: 'm.text' },
    { sender: '@bob:example.org' }
  );

  assert.equal(canEditEvent(client, own), true);
  assert.equal(canEditEvent(client, reply), false);
  assert.equal(canEditEvent(client, thread), true);
  assert.equal(canEditEvent(client, other), false);
});

test('edit helpers resolve the latest replacement from a timeline set relations container', () => {
  const editA = makeEvent(
    MessageEvent.RoomMessage,
    { body: 'A' },
    { sender: '@alice:example.org', ts: 1 }
  );
  const editB = makeEvent(
    MessageEvent.RoomMessage,
    { body: 'B' },
    { sender: '@alice:example.org', ts: 2 }
  );
  const timelineSet: EventTimelineSetReading = {
    relations: {
      getChildEventsForEvent: (eventId, relationType, eventType) => {
        assert.equal(eventId, '$target');
        assert.equal(relationType, 'm.replace');
        assert.equal(eventType, MessageEvent.RoomMessage);
        return { getRelations: () => [editA, editB] };
      },
    },
  };
  const target = makeEvent(
    MessageEvent.RoomMessage,
    { body: 'orig' },
    { sender: '@alice:example.org' }
  );
  assert.equal(
    getEventEdits(timelineSet, '$target', MessageEvent.RoomMessage)?.getRelations().length,
    2
  );
  const edited = getEditedEvent('$target', target, timelineSet);
  assert.equal(edited?.getContent().body, 'B');
});

test('getEditedEvent also accepts native event content inputs', () => {
  const edit = makeEvent(
    MessageEvent.RoomMessage,
    { body: 'B' },
    { sender: '@alice:example.org', ts: 2 }
  );
  const timelineSet: EventTimelineSetReading = {
    relations: {
      getChildEventsForEvent: () => ({ getRelations: () => [edit] }),
    },
  };
  const nativeTarget = {
    eventId: '$target',
    sender: '@alice:example.org',
    type: MessageEvent.RoomMessage,
    originServerTs: 0,
    content: { body: 'orig' },
    redacted: false,
  };
  assert.equal(getEditedEvent('$target', nativeTarget, timelineSet)?.getContent().body, 'B');
});

test('getThreadRootEventId only accepts m.thread relations', () => {
  const thread = makeEvent(
    'm.room.message',
    {},
    {
      relation: { rel_type: 'm.thread', event_id: '$root' },
    }
  );
  const reaction = makeEvent(
    'm.room.message',
    {},
    {
      relation: { rel_type: 'm.annotation', event_id: '$root' },
    }
  );
  assert.equal(getThreadRootEventId(thread), '$root');
  assert.equal(getThreadRootEventId(reaction), undefined);
  assert.equal(getThreadRootEventId(undefined), undefined);
});

test('isNotificationEvent ignores redactions, replacement relations, and member events', () => {
  assert.equal(isNotificationEvent(makeEvent('m.room.message', {})), true);
  assert.equal(isNotificationEvent(makeEvent('m.room.message', {}, { redacted: true })), false);
  assert.equal(
    isNotificationEvent(
      makeEvent('m.room.message', {}, { relation: { rel_type: 'm.replace', event_id: '$x' } })
    ),
    false
  );
  assert.equal(isNotificationEvent(makeEvent('m.room.member', {})), false);
  assert.equal(isNotificationEvent(makeEvent('m.room.create', {})), true);
});

test('getNotificationType maps explicit, muted, and default rules', () => {
  assert.equal(
    getNotificationType(
      makeClient({ getRoomPushRule: () => ({ actions: ['notify'], rule_id: 'r' }) }),
      '!r:example.org'
    ),
    NotificationType.AllMessages
  );
  assert.equal(
    getNotificationType(
      makeClient({ getRoomPushRule: () => ({ actions: ['dont_notify'], rule_id: 'r' }) }),
      '!r:example.org'
    ),
    NotificationType.MentionsAndKeywords
  );
  const mutedOverride = () =>
    ({
      getContent: () => ({
        global: {
          override: [
            {
              actions: [],
              conditions: [{ kind: 'event_match' }],
              rule_id: '!r:example.org',
            },
          ],
        },
      }),
    } as MatrixEventReading);
  assert.equal(
    getNotificationType(
      makeClient({ getRoomPushRule: () => undefined, getAccountData: mutedOverride }),
      '!r:example.org'
    ),
    NotificationType.Mute
  );
  assert.equal(
    getNotificationType(makeClient({ getRoomPushRule: () => undefined }), '!r:example.org'),
    NotificationType.Default
  );
});

test('avatar helpers delegate mxc conversion to the client projection', () => {
  const room = makeRoom({}, { avatarUrl: 'mxc://room/avatar' });
  const client = makeClient({
    mxcUrlToHttp: (mxc, w, h, mode) => `http://img/${w}x${h}/${mode}/${mxc}`,
  });
  assert.equal(getRoomAvatarUrl(client, room, 32), 'http://img/32x32/crop/mxc://room/avatar');
  assert.equal(getDirectRoomAvatarUrl(client, room, 96), 'http://img/96x96/crop/mxc://room/avatar');
});

test('getMentionContent builds mentions content only for populated inputs', () => {
  assert.deepEqual(getMentionContent(['@bob:example.org'], false), {
    user_ids: ['@bob:example.org'],
  });
  assert.deepEqual(getMentionContent([], true), { room: true });
  assert.deepEqual(getMentionContent([], false), {});
});

test('getAllVersionsRoomCreator uses creators and additional_creators in legacy mode', () => {
  const originalWindow = globalThis.window;
  (globalThis as any).window = {};
  try {
    const create = makeEvent(
      'm.room.create',
      { room_version: '9', additional_creators: ['@co:example.org'] },
      { sender: '@owner:example.org' }
    );
    const room = makeRoom({ 'm.room.create': create }, { roomId: '!r:example.org' });
    assert.deepEqual(Array.from(getAllVersionsRoomCreator(room)).sort(), [
      '@co:example.org',
      '@owner:example.org',
    ]);
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('getAllVersionsRoomCreator consumes the native room state projection', () => {
  const originalWindow = globalThis.window;
  (globalThis as any).window = { __SYNARA_DESKTOP__: { platform: 'tauri' } };
  setSessionBootstrapResult({ source: 'native' });
  publishNativeRoomCreatorsProjection('!native:example.org', 5, ['@native:example.org']);
  try {
    const room = makeRoom({}, { roomId: '!native:example.org' });
    assert.deepEqual(Array.from(getAllVersionsRoomCreator(room)), ['@native:example.org']);
  } finally {
    clearNativeRoomStateProjections();
    resetSessionBootstrapForTests();
    (globalThis as any).window = originalWindow;
  }
});

test('guessPerfectParent favours the parent sharing the most special users', () => {
  const create = (sender: string) => (key: string) => {
    const room = makeRoom(
      { 'm.room.create': makeEvent('m.room.create', { room_version: '9' }, { sender }) },
      { roomId: key }
    );
    return room;
  };
  const makeClientWithRooms = (rooms: Record<string, RoomReading>) =>
    makeClient({ getRoom: (id) => rooms[id] ?? null });

  const roomA = create('@a:example.org')('!a:example.org');
  const roomB = create('@b:example.org')('!b:example.org');
  const target = makeRoom(
    {
      'm.room.create': makeEvent(
        'm.room.create',
        { room_version: '9', additional_creators: ['@a:example.org', '@b:example.org'] },
        { sender: '@a:example.org' }
      ),
    },
    { roomId: '!target:example.org' }
  );
  const client = makeClientWithRooms({
    '!target:example.org': target,
    '!a:example.org': roomA,
    '!b:example.org': roomB,
  });
  const originalWindow = globalThis.window;
  (globalThis as any).window = {};
  try {
    // room A shares one creator, room B shares one creator too -> ties diff by score; first max wins
    const perfect = guessPerfectParent(client, '!target:example.org', [
      '!a:example.org',
      '!b:example.org',
    ]);
    assert.equal(perfect, '!a:example.org');
  } finally {
    (globalThis as any).window = originalWindow;
  }
});

test('room utils source guard keeps the file free of the JS SDK import', () => {
  const source = readFileSync(join(process.cwd(), 'src/app/utils/room.ts'), 'utf8');
  assert.doesNotMatch(source, /from ["']matrix-js-sdk["']/);
});
