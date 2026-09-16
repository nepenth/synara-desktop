import assert from 'node:assert/strict';
import test from 'node:test';
import {
  parseRtcTransport,
  parseRtcTransportsSnapshot,
  snapshotRtcTransportsNative,
  type NativeRtcTransportsInvoke,
} from '../nativeRtcTransports';
import {
  incomingCallLabel,
  liveCallChipLabel,
  rtcTransportsDiagnosticCopy,
  voiceRoomLandingCopy,
} from '../liveCallChrome';

test('rtc transport parser maps livekit urls and drops secrets', () => {
  const snapshot = parseRtcTransportsSnapshot({
    sessionGeneration: 4,
    status: 'ready',
    transports: [{ kind: 'livekit', serviceUrl: 'https://livekit.example.org/rtc' }],
  });
  assert.equal(snapshot?.status, 'ready');
  assert.equal(snapshot?.transports[0]?.serviceUrl, 'https://livekit.example.org/rtc');
  assert.equal(
    parseRtcTransportsSnapshot({
      sessionGeneration: 1,
      status: 'ready',
      transports: [{ kind: 'custom', jwt: 'secret.token' }],
    }),
    null
  );
  assert.equal(parseRtcTransport({ kind: 'livekit', serviceUrl: 'javascript:alert(1)' }), null);
  assert.equal(
    parseRtcTransport({
      kind: 'livekit',
      serviceUrl: 'https://user:token@livekit.example.org/',
    }),
    null
  );
});

test('rtc transport parser fail-closes on unknown keys and empty vs missing lists', () => {
  assert.equal(
    parseRtcTransportsSnapshot({ sessionGeneration: 1, status: 'unsupported', transports: [] })
      ?.status,
    'unsupported'
  );
  assert.equal(
    parseRtcTransportsSnapshot({ sessionGeneration: 1, status: 'unavailable', transports: [] })
      ?.status,
    'unavailable'
  );
  assert.equal(
    parseRtcTransportsSnapshot({
      sessionGeneration: 1,
      status: 'unsupported',
      transports: [{ kind: 'livekit' }],
    }),
    null
  );
  assert.equal(
    parseRtcTransportsSnapshot({
      sessionGeneration: 1,
      status: 'ready',
      transports: [],
      widget: true,
    }),
    null
  );
});

test('snapshot invoke stays on the native command and never widgets types', async () => {
  const calls: string[] = [];
  const invoke: NativeRtcTransportsInvoke = async (command) => {
    calls.push(command);
    return {
      available: true,
      value: { sessionGeneration: 2, status: 'unsupported', transports: [] },
    };
  };
  const snapshot = await snapshotRtcTransportsNative({
    desktopNativeSession: true,
    invoke,
  });
  assert.equal(snapshot?.status, 'unsupported');
  assert.deepEqual(calls, ['matrix_rtc_transports_snapshot']);
});

test('live-call chrome distinguishes voice-room type from a live call', () => {
  assert.equal(liveCallChipLabel(1), 'In a call');
  assert.equal(liveCallChipLabel(3), '3 live');
  assert.equal(
    voiceRoomLandingCopy({
      memberCount: 4,
      hasActiveCall: false,
      liveParticipantCount: 0,
    }).includes('built for live conversation'),
    true
  );
  assert.equal(
    voiceRoomLandingCopy({
      memberCount: 4,
      hasActiveCall: true,
      liveParticipantCount: 3,
    }).includes('Call in progress — 3 live'),
    true
  );
  assert.equal(
    voiceRoomLandingCopy({
      memberCount: 2,
      hasActiveCall: false,
      liveParticipantCount: 0,
      rtcStatus: 'unsupported',
    }).includes('Calls need a homeserver LiveKit transport'),
    true
  );
  assert.equal(
    rtcTransportsDiagnosticCopy({
      status: 'ready',
      transports: [{ kind: 'livekit', serviceUrl: 'https://livekit.example.org' }],
    }),
    'MatrixRTC transport: LiveKit at https://livekit.example.org'
  );
  assert.equal(
    rtcTransportsDiagnosticCopy({ status: 'unsupported', transports: [] }),
    'This homeserver does not advertise a call transport'
  );
  assert.equal(incomingCallLabel('notification'), 'Incoming call');
  assert.equal(incomingCallLabel('invite'), 'invite');
});
