export const RTC_TRANSPORT_ADVERTISED =
  'This homeserver advertises a live-call transport. Joining and starting calls is not in this app build yet.';

export const RTC_TRANSPORT_MISSING =
  'This homeserver does not advertise a live-call transport (MatrixRTC / LiveKit).';

export const RTC_CALL_NOT_IN_BUILD = 'Joining and starting calls is not in this app build yet.';

export function liveCallChipLabel(participantCount: number): string {
  return participantCount > 1 ? `${participantCount} live` : 'In a call';
}

export function rtcCallAvailabilityCopy(input: {
  status: 'ready' | 'unsupported' | 'unavailable' | null;
  transports?: Array<{ kind: 'livekit' | 'custom'; serviceUrl?: string }>;
}): string {
  if (input.status === null) {
    return 'Checking live-call transport…';
  }
  if (input.status === 'ready') {
    return RTC_TRANSPORT_ADVERTISED;
  }
  return RTC_TRANSPORT_MISSING;
}

export function startCallDisabledReason(input: {
  status: 'ready' | 'unsupported' | 'unavailable' | null;
}): string {
  if (input.status === 'ready') {
    return 'Starting calls is not in this app build yet.';
  }
  if (input.status === null) {
    return 'Checking live-call transport…';
  }
  return RTC_TRANSPORT_MISSING;
}

export function activeCallChromeCopy(input: {
  hasActiveCall: boolean;
  liveParticipantCount: number;
}): string | null {
  if (!input.hasActiveCall) return null;
  if (input.liveParticipantCount > 0) {
    return `A call is in progress — ${input.liveParticipantCount} live.`;
  }
  return 'A call is in progress.';
}

export function voiceRoomLandingCopy(input: {
  memberCount: number;
  hasActiveCall: boolean;
  liveParticipantCount: number;
  rtcStatus?: 'ready' | 'unsupported' | 'unavailable' | null;
}): string {
  const memberWord = input.memberCount === 1 ? 'participant' : 'participants';
  const members = input.memberCount > 0 ? `${input.memberCount} ${memberWord}. ` : '';
  const live = activeCallChromeCopy({
    hasActiveCall: input.hasActiveCall,
    liveParticipantCount: input.liveParticipantCount,
  });
  const livePrefix = live ? `${live} ` : '';
  const how =
    'This client shows live participants today. Joining and starting calls uses MatrixRTC on the homeserver and is not in this app build yet.';
  const timeline = ' Messages you send here stay on the timeline.';
  const copy = `${members}${livePrefix}${how}${timeline}`;
  if (input.rtcStatus === 'unsupported' || input.rtcStatus === 'unavailable') {
    return `${copy} ${RTC_TRANSPORT_MISSING}`;
  }
  return copy;
}

export function rtcTransportsDiagnosticCopy(input: {
  status: 'ready' | 'unsupported' | 'unavailable';
  transports: Array<{ kind: 'livekit' | 'custom'; serviceUrl?: string }>;
}): string {
  if (input.status === 'unsupported' || input.status === 'unavailable') {
    return 'This homeserver does not advertise a call transport';
  }
  const livekit = input.transports.find((row) => row.kind === 'livekit');
  if (livekit?.serviceUrl) {
    return `MatrixRTC transport: LiveKit at ${livekit.serviceUrl}`;
  }
  if (livekit) {
    return 'MatrixRTC transport: LiveKit';
  }
  if (input.transports.some((row) => row.kind === 'custom')) {
    return 'MatrixRTC transport: custom';
  }
  return 'MatrixRTC transport: ready';
}

export function incomingCallLabel(callKind: string): string {
  return callKind === 'notification' ? 'Incoming call' : callKind;
}
