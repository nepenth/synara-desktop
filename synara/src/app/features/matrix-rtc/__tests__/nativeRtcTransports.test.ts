import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  parseRtcTransport,
  parseRtcTransportsSnapshot,
  refreshRtcTransportsNative,
  snapshotRtcTransportsNative,
  type NativeRtcTransportsInvoke,
} from '../nativeRtcTransports';
import {
  RTC_CALL_NOT_IN_BUILD,
  RTC_TRANSPORT_ADVERTISED,
  RTC_TRANSPORT_MISSING,
  activeCallChromeCopy,
  incomingCallLabel,
  liveCallChipLabel,
  rtcCallAvailabilityCopy,
  rtcTransportsDiagnosticCopy,
  startCallDisabledReason,
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
  const refresh = await refreshRtcTransportsNative({
    desktopNativeSession: true,
    invoke,
  });
  assert.equal(refresh?.status, 'unsupported');
  assert.deepEqual(calls, ['matrix_rtc_transports_snapshot', 'matrix_rtc_transports_refresh']);
});

test('live-call chrome distinguishes voice-room type from a live call', () => {
  assert.equal(liveCallChipLabel(1), 'In a call');
  assert.equal(liveCallChipLabel(3), '3 live');
  const idle = voiceRoomLandingCopy({
    memberCount: 4,
    hasActiveCall: false,
    liveParticipantCount: 0,
  });
  assert.match(idle, /4 participants/);
  assert.match(idle, /shows live participants today/);
  assert.match(idle, /MatrixRTC on the homeserver/);
  assert.equal(idle.includes('join now'), false);
  assert.equal(
    voiceRoomLandingCopy({
      memberCount: 4,
      hasActiveCall: true,
      liveParticipantCount: 3,
    }).includes('A call is in progress — 3 live'),
    true
  );
  assert.equal(
    voiceRoomLandingCopy({
      memberCount: 2,
      hasActiveCall: false,
      liveParticipantCount: 0,
      rtcStatus: 'unsupported',
    }).includes(RTC_TRANSPORT_MISSING),
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

test('honest call availability copy does not offer join or start', () => {
  assert.equal(
    rtcCallAvailabilityCopy({
      status: 'ready',
      transports: [{ kind: 'livekit', serviceUrl: 'https://livekit.example.org' }],
    }),
    RTC_TRANSPORT_ADVERTISED
  );
  assert.equal(
    rtcCallAvailabilityCopy({ status: 'unsupported', transports: [] }),
    RTC_TRANSPORT_MISSING
  );
  assert.equal(
    rtcCallAvailabilityCopy({ status: 'unavailable', transports: [] }),
    RTC_TRANSPORT_MISSING
  );
  assert.equal(rtcCallAvailabilityCopy({ status: null }), 'Checking live-call transport…');
  assert.equal(
    startCallDisabledReason({ status: 'ready' }),
    'Starting calls is not in this app build yet.'
  );
  assert.equal(startCallDisabledReason({ status: 'unsupported' }), RTC_TRANSPORT_MISSING);
  assert.equal(activeCallChromeCopy({ hasActiveCall: false, liveParticipantCount: 0 }), null);
  assert.equal(
    activeCallChromeCopy({ hasActiveCall: true, liveParticipantCount: 2 }),
    'A call is in progress — 2 live.'
  );
  assert.match(RTC_CALL_NOT_IN_BUILD, /not in this app build yet/);
});

test('call chrome never writes call membership or set_call', () => {
  const header = readFileSync('src/app/features/room/RoomViewHeader.tsx', 'utf8');
  const voice = readFileSync('src/app/features/room/VoiceRoom.tsx', 'utf8');
  const general = readFileSync('src/app/features/settings/general/General.tsx', 'utf8');
  const chip = readFileSync('src/app/features/room-nav/LiveCallChip.tsx', 'utf8');
  for (const [name, source] of Object.entries({ header, voice, general, chip })) {
    assert.doesNotMatch(source, /set_call/, name);
    assert.doesNotMatch(source, /call\.member/, name);
    assert.doesNotMatch(source, /org\.matrix\.msc3401\.call\.member/, name);
  }
  assert.match(header, /aria-label="Call"/);
  assert.match(header, /Start call/);
  assert.match(header, /disabled/);
  assert.match(header, /rtcCallAvailabilityCopy/);
  assert.match(header, /Icons\.Phone/);
  assert.match(header, /Icons\.Code/);
  assert.match(header, /isSynaraDesktop\(\) && onToggleSidePanel/);
  assert.match(general, /rtcCallAvailabilityCopy/);
  assert.match(voice, /voiceRoomLandingCopy/);
});
