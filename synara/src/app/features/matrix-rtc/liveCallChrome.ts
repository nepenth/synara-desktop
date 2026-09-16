export function liveCallChipLabel(participantCount: number): string {
  return participantCount > 1 ? `${participantCount} live` : 'In a call';
}

export function voiceRoomLandingCopy(input: {
  memberCount: number;
  hasActiveCall: boolean;
  liveParticipantCount: number;
  rtcStatus?: 'ready' | 'unsupported' | 'unavailable' | null;
}): string {
  const memberWord = input.memberCount === 1 ? 'participant' : 'participants';
  let copy: string;
  if (input.hasActiveCall) {
    const live =
      input.liveParticipantCount > 1
        ? `Call in progress — ${input.liveParticipantCount} live.`
        : 'Call in progress.';
    copy =
      input.memberCount > 0
        ? `${live} ${input.memberCount} ${memberWord} in this room. Messages you send here stay on the timeline.`
        : `${live} Messages you send here stay on the timeline.`;
  } else {
    copy =
      input.memberCount > 0
        ? `${input.memberCount} ${memberWord} \u2014 built for live conversation. Messages you send here stay on the timeline.`
        : 'Built for live conversation. Messages you send here stay on the timeline.';
  }
  if (input.rtcStatus === 'unsupported' || input.rtcStatus === 'unavailable') {
    return `${copy} Calls need a homeserver LiveKit transport.`;
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
