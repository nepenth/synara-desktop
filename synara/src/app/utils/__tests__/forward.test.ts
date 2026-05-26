import assert from 'node:assert/strict';
import test from 'node:test';
import {
  getForwardableEventContent,
  getForwardableEventContents,
  getRoomForwardTargets,
  makeForwardQuoteContent,
  makeForwardedContent,
} from '../forward';

test('makeForwardedContent adds attribution and strips relation fields', () => {
  const content = makeForwardedContent(
    {
      msgtype: 'm.text',
      body: '> <@bob:example.org> private reply\n\nhello <world>',
      formatted_body:
        '<mx-reply><blockquote>private reply</blockquote></mx-reply><p>hello <strong>world</strong></p>',
      'm.relates_to': { event_id: '$reply' },
      'm.mentions': { user_ids: ['@alice:example.org'] },
      'm.forwarded': { old: true },
    },
    { roomId: '!room', eventId: '$event', sender: '@alice:example.org', ts: 123 }
  );

  assert.equal(content.body, 'Forwarded from @alice:example.org\n\nhello <world>');
  assert.equal(
    content.formatted_body,
    '<p><strong>Forwarded from @alice:example.org</strong></p><p>hello <strong>world</strong></p>'
  );
  assert.equal('m.relates_to' in content, false);
  assert.equal('m.mentions' in content, false);
  assert.equal('m.forwarded' in content, false);
  assert.deepEqual(content['in.synara.forwarded'], {
    roomId: '!room',
    eventId: '$event',
    sender: '@alice:example.org',
    ts: 123,
  });
});

test('makeForwardedContent gives media messages visible attribution', () => {
  const content = makeForwardedContent(
    {
      msgtype: 'm.image',
      body: 'image.png',
      url: 'mxc://example.org/media',
    },
    { roomId: '!room', eventId: '$event', sender: '@alice:example.org', ts: 123 }
  );

  assert.equal(content.body, 'Forwarded from @alice:example.org\n\nimage.png');
  assert.deepEqual(content['in.synara.forwarded'], {
    roomId: '!room',
    eventId: '$event',
    sender: '@alice:example.org',
    ts: 123,
  });
});

test('makeForwardQuoteContent forwards text as quote without reply relation', () => {
  const content = makeForwardQuoteContent(
    {
      msgtype: 'm.text',
      body: '> <@bob:example.org> old reply\n\nquoted text',
      'm.relates_to': { event_id: '$reply' },
    },
    { roomId: '!room', eventId: '$event', sender: '@alice:example.org', ts: 123 }
  );

  assert.equal(content.msgtype, 'm.text');
  assert.equal(content.body, 'Forwarded quote from @alice:example.org\n\n> quoted text');
  assert.equal('m.relates_to' in content, false);
  assert.deepEqual(content['in.synara.forwarded'], {
    roomId: '!room',
    eventId: '$event',
    sender: '@alice:example.org',
    ts: 123,
    quote: true,
  });
});

test('getForwardableEventContent rejects redactions and annotations', () => {
  const reaction = {
    isRedacted: () => false,
    isRedaction: () => false,
    getRelation: () => ({ rel_type: 'm.annotation' }),
    getContent: () => ({ msgtype: 'm.text', body: 'reaction' }),
  } as any;
  const redacted = {
    isRedacted: () => true,
    isRedaction: () => false,
    getRelation: () => undefined,
  } as any;

  assert.equal(getForwardableEventContent(reaction), undefined);
  assert.equal(getForwardableEventContent(redacted), undefined);
});

test('getForwardableEventContents keeps same-room multi-message forwards', () => {
  const makeEvent = (eventId: string, roomId = '!room') =>
    ({
      isRedacted: () => false,
      isRedaction: () => false,
      getRelation: () => undefined,
      getContent: () => ({ msgtype: 'm.text', body: eventId }),
      getRoomId: () => roomId,
      getId: () => eventId,
      getSender: () => '@alice:example.org',
      getTs: () => 123,
    } as any);

  assert.equal(getForwardableEventContents([makeEvent('$a'), makeEvent('$b')]).length, 2);
  assert.equal(getForwardableEventContents([makeEvent('$a'), makeEvent('$b', '!other')]).length, 0);
});

test('getRoomForwardTargets filters source rooms and spaces', () => {
  const rooms = [
    {
      roomId: '!source',
      name: 'Source',
      getMyMembership: () => 'join',
      isSpaceRoom: () => false,
    },
    {
      roomId: '!space',
      name: 'Space',
      getMyMembership: () => 'join',
      isSpaceRoom: () => true,
    },
    {
      roomId: '!target',
      name: 'Target',
      getMyMembership: () => 'join',
      isSpaceRoom: () => false,
    },
  ] as any[];

  assert.deepEqual(
    getRoomForwardTargets(rooms, '!source').map((room) => room.roomId),
    ['!target']
  );
});

test('getRoomForwardTargets filters rooms where the user cannot send messages', () => {
  const roomWithPower = (roomId: string, userPower: number, sendPower: number) => ({
    roomId,
    name: roomId,
    getMyMembership: () => 'join',
    isSpaceRoom: () => false,
    getLiveTimeline: () => ({
      getState: () => ({
        getStateEvents: () => ({
          getContent: () => ({
            users: {
              '@me:example.org': userPower,
            },
            events: {
              'm.room.message': sendPower,
            },
          }),
        }),
      }),
    }),
  });

  assert.deepEqual(
    getRoomForwardTargets(
      [roomWithPower('!can-send', 50, 0), roomWithPower('!cannot-send', 0, 50)] as any[],
      '!source',
      '@me:example.org'
    ).map((room) => room.roomId),
    ['!can-send']
  );
});
