import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

import {
  FORBID_MEDIA_BYTES_OVER_JSON_IPC,
  FORBIDDEN_WIRE_FIELD_NAMES,
  MATRIX_DTO_MARKER,
  SESSION_LIFECYCLES,
  TIMELINE_ITEM_KINDS,
  parseMediaHandle,
  parseNotificationCandidate,
  parseReceipt,
  parseRelationRef,
  parseRoomMember,
  parseRoomSummary,
  parseSearchResult,
  parseSecurityStatus,
  parseSessionSnapshot,
  parseDirectoryPage,
  parseDirectoryProtocols,
  parseDirectorySearchResponse,
  parseSpaceSummary,
  parseThreadSummary,
  parseTimelineItem,
  parseTypingSnapshot,
  parseUploadJob,
} from '../index';

/**
 * Shared fixtures live at repo-root docs/ (authoritative for Rust + TS).
 * Tests run with cwd = synara/ package root.
 */
const fixtureDir = join(process.cwd(), '../docs/matrix-rust-sdk/dto/fixtures');

function loadFixture(name: string): unknown {
  const raw = readFileSync(join(fixtureDir, name), 'utf8');
  return JSON.parse(raw) as unknown;
}

function loadRaw(name: string): string {
  return readFileSync(join(fixtureDir, name), 'utf8');
}

test('markers and policy constants', () => {
  assert.equal(MATRIX_DTO_MARKER, 'matrix-domain-dtos-p1.4');
  assert.equal(FORBID_MEDIA_BYTES_OVER_JSON_IPC, true);
  assert.ok(FORBIDDEN_WIRE_FIELD_NAMES.length > 0);
  assert.equal(SESSION_LIFECYCLES.length, 10);
  assert.equal(TIMELINE_ITEM_KINDS.length, 9);
});

test('directory page parses bounded room and space projections', () => {
  const page = parseDirectoryPage({
    sessionGeneration: 4,
    requestId: 9,
    chunk: [
      {
        roomId: '!room:example.org',
        name: 'Room',
        topic: 'Topic',
        canonicalAlias: '#room:example.org',
        avatarUrl: 'mxc://example.org/avatar',
        memberCount: 12,
        worldReadable: true,
        guestCanJoin: false,
        roomType: 'room',
      },
      {
        roomId: '!space:example.org',
        memberCount: 2,
        worldReadable: true,
        guestCanJoin: true,
        roomType: 'space',
      },
    ],
    prevBatch: 'previous',
    nextBatch: 'next',
  });
  assert.ok(page);
  assert.equal(page.chunk[1]?.roomType, 'space');
  assert.equal(page.prevBatch, 'previous');
});

test('directory DTOs fail closed on unknown, secret, malformed, or unsupported fields', () => {
  const hit = {
    roomId: '!room:example.org',
    memberCount: 1,
    worldReadable: true,
    guestCanJoin: true,
    roomType: 'room',
  };
  assert.equal(
    parseDirectoryPage({ sessionGeneration: 1, requestId: 1, chunk: [hit], extra: true }),
    null
  );
  assert.equal(
    parseDirectoryPage({
      sessionGeneration: 1,
      requestId: 1,
      chunk: [{ ...hit, accessToken: 'secret' }],
    }),
    null
  );
  assert.equal(
    parseDirectoryPage({
      sessionGeneration: 1,
      requestId: 1,
      chunk: [{ ...hit, roomType: 'call' }],
    }),
    null
  );
  assert.equal(
    parseDirectoryPage({ sessionGeneration: 1, requestId: 1, chunk: [hit], nextBatch: '' }),
    null
  );
  assert.equal(
    parseDirectorySearchResponse({ sessionGeneration: 1, requestId: 1, status: 'ready' }),
    null
  );
});

test('directory protocol projection is strict and bounded', () => {
  const protocols = parseDirectoryProtocols({
    sessionGeneration: 4,
    instances: [{ protocolId: 'irc', instanceId: 'irc-example', description: 'IRC Example' }],
  });
  assert.ok(protocols);
  assert.equal(protocols.instances[0]?.instanceId, 'irc-example');
  assert.equal(
    parseDirectoryProtocols({
      sessionGeneration: 4,
      instances: [{ protocolId: 'irc', instanceId: 'irc-example', description: 'IRC', raw: {} }],
    }),
    null
  );
});

test('valid_session parses; no tokens', () => {
  const raw = loadRaw('valid_session.json');
  assert.equal(raw.includes('access_token'), false);
  assert.equal(raw.includes('accessToken'), false);
  assert.equal(raw.includes('refresh_token'), false);
  const s = parseSessionSnapshot(loadFixture('valid_session.json'));
  assert.ok(s);
  assert.equal(s.userId, '@alice:example.org');
  assert.equal(s.lifecycle, 'ready');
  assert.equal(s.cryptoReady, true);
  assert.equal(s.sessionGeneration, 3);
});

test('session rejects missing required fields', () => {
  assert.equal(parseSessionSnapshot({}), null);
  assert.equal(
    parseSessionSnapshot({
      sessionGeneration: 1,
      userId: '@a:b',
      deviceId: 'D',
      homeserverUrl: 'https://hs',
      lifecycle: 'ready',
      // missing cryptoReady
    }),
    null
  );
});

test('session rejects accessToken on wire object', () => {
  const withToken = {
    ...(loadFixture('valid_session.json') as object as Record<string, unknown>),
    accessToken: 's3cret',
  };
  assert.equal(parseSessionSnapshot(withToken), null);
});

test('valid_room_summary parses', () => {
  const r = parseRoomSummary(loadFixture('valid_room_summary.json'));
  assert.ok(r);
  assert.equal(r.roomId, '!room:example.org');
  assert.equal(r.membership, 'join');
  assert.equal(r.notificationMode, 'mentions');
  assert.equal(r.lastMessagePreview, undefined);
  assert.equal(r.heroes?.length, 1);
  assert.equal(r.isFavorite, false);
});

test('room summary rejects invalid membership', () => {
  const bad = {
    ...(loadFixture('valid_room_summary.json') as object),
    membership: 'not-a-membership',
  };
  assert.equal(parseRoomSummary(bad), null);
});

test('valid_member parses', () => {
  const m = parseRoomMember(loadFixture('valid_member.json'));
  assert.ok(m);
  assert.equal(m.powerLevel, 50);
  assert.equal(m.isDirectTarget, true);
});

test('valid_timeline_item_message parses', () => {
  const t = parseTimelineItem(loadFixture('valid_timeline_item_message.json'));
  assert.ok(t);
  assert.equal(t.kind, 'message');
  if (t.kind === 'message') {
    assert.equal(t.body, 'hello world');
    assert.equal(t.itemId, '$msg1');
  }
});

test('valid_timeline_item_state parses', () => {
  const t = parseTimelineItem(loadFixture('valid_timeline_item_state.json'));
  assert.ok(t);
  assert.equal(t.kind, 'state');
  if (t.kind === 'state') {
    assert.equal(t.stateType, 'm.room.name');
  }
});

test('timeline rejects unknown kind', () => {
  assert.equal(parseTimelineItem({ kind: 'not_a_kind', itemId: 'x' }), null);
});

test('valid_relation_reaction parses', () => {
  const r = parseRelationRef(loadFixture('valid_relation_reaction.json'));
  assert.ok(r);
  assert.equal(r.relType, 'annotation');
  assert.equal(r.key, '👍');
});

test('valid_receipt parses', () => {
  const r = parseReceipt(loadFixture('valid_receipt.json'));
  assert.ok(r);
  assert.equal(r.receiptType, 'read');
});

test('valid_typing parses', () => {
  const t = parseTypingSnapshot(loadFixture('valid_typing.json'));
  assert.ok(t);
  assert.equal(t.userIds.length, 2);
});

test('valid_upload parses; no media bytes fields', () => {
  const raw = loadRaw('valid_upload.json');
  assert.equal(raw.includes('fileBytes'), false);
  assert.equal(raw.includes('mediaBytes'), false);
  const u = parseUploadJob(loadFixture('valid_upload.json'));
  assert.ok(u);
  assert.equal(u.state, 'uploading');
  assert.equal(u.progress01, 0.42);
});

test('upload rejects fileBytes key', () => {
  const bad = {
    ...(loadFixture('valid_upload.json') as object),
    fileBytes: 'AAAA',
  };
  assert.equal(parseUploadJob(bad), null);
});

test('valid_media_handle parses', () => {
  const m = parseMediaHandle(loadFixture('valid_media_handle.json'));
  assert.ok(m);
  assert.equal(m.source, 'mxc');
  assert.equal(m.handleId, 'media-handle-1');
});

test('valid_security_status parses', () => {
  const s = parseSecurityStatus(loadFixture('valid_security_status.json'));
  assert.ok(s);
  assert.equal(s.backupStatus, 'enabled');
  assert.equal(s.verificationState, 'verified');
});

test('valid_notification_candidate parses', () => {
  const n = parseNotificationCandidate(loadFixture('valid_notification_candidate.json'));
  assert.ok(n);
  assert.equal(n.kind, 'message');
  assert.equal(n.suppressIfFocusedRoom, true);
});

test('valid_search_result parses', () => {
  const s = parseSearchResult(loadFixture('valid_search_result.json'));
  assert.ok(s);
  assert.equal(s.query, 'hello');
  assert.equal(s.results.length, 1);
});

test('valid_space_summary parses', () => {
  const s = parseSpaceSummary(loadFixture('valid_space_summary.json'));
  assert.ok(s);
  assert.equal(s.children.length, 2);
});

test('valid_thread_summary parses', () => {
  const t = parseThreadSummary(loadFixture('valid_thread_summary.json'));
  assert.ok(t);
  assert.equal(t.replyCount, 4);
  assert.equal(t.participated, true);
});

test('all fixtures lack forbidden secret field names', () => {
  const names = [
    'valid_session.json',
    'valid_room_summary.json',
    'valid_member.json',
    'valid_timeline_item_message.json',
    'valid_timeline_item_state.json',
    'valid_relation_reaction.json',
    'valid_receipt.json',
    'valid_typing.json',
    'valid_upload.json',
    'valid_media_handle.json',
    'valid_security_status.json',
    'valid_notification_candidate.json',
    'valid_search_result.json',
    'valid_space_summary.json',
    'valid_thread_summary.json',
  ];
  for (const name of names) {
    const raw = loadRaw(name);
    for (const forbidden of [
      'access_token',
      'accessToken',
      'refresh_token',
      'refreshToken',
      'recovery_key',
      'password',
      'mediaBytes',
      'fileBytes',
    ]) {
      assert.equal(raw.includes(forbidden), false, `${name} must not contain ${forbidden}`);
    }
  }
});
