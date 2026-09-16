import React, { useEffect, useState } from 'react';
import { Box, Icons, Icon, Text, config } from 'folds';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { useRoom } from '../../hooks/useRoom';
import { useRoomMembers } from '../../hooks/useRoomMembers';
import { isNativeMatrixSession } from '../verification/nativeVerification';
import { normalizeRoomJoinRulePresentation } from '../matrix-dto/roomJoinRule';
import { getRoomIconSrc } from '../../utils/room';
import { RoomType } from '../../../types/matrix/room';
import { useNativeRoomListSnapshot } from '../../state/room-list/roomList';
import { snapshotRtcTransportsNative } from '../matrix-rtc/nativeRtcTransports';
import { voiceRoomLandingCopy } from '../matrix-rtc/liveCallChrome';

/**
 * Voice-room hero — first-class in-room surface for rooms created as a voice
 * room (`m.room.create` type `org.matrix.msc3417.call`). Provides an honest
 * landing for the live-conversation lane without inventing call controls the
 * native client does not yet expose. Live-call chrome is membership-driven;
 * this never offers Join/Leave.
 */
export function VoiceRoom() {
  const room = useRoom();
  const mx = useMatrixClient();
  const members = useRoomMembers(mx, room.roomId, isNativeMatrixSession());
  const nativeRooms = useNativeRoomListSnapshot();
  const nativeRoom = nativeRooms.rooms.find((summary) => summary.roomId === room.roomId);
  const [rtcStatus, setRtcStatus] = useState<'ready' | 'unsupported' | 'unavailable' | null>(null);
  const iconSrc = getRoomIconSrc(
    Icons,
    RoomType.Call,
    normalizeRoomJoinRulePresentation(room.getJoinRule())
  );

  useEffect(() => {
    let cancelled = false;
    snapshotRtcTransportsNative()
      .then((snapshot) => {
        if (!cancelled) setRtcStatus(snapshot?.status ?? null);
      })
      .catch(() => {
        if (!cancelled) setRtcStatus(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <Box
      style={{ padding: `${config.space.S200} ${config.space.S400}` }}
      gap="300"
      alignItems="Center"
    >
      <Icon size="500" src={iconSrc} />
      <Box direction="Column" grow="Yes">
        <Text size="T400" style={{ fontWeight: 600 }} truncate>
          Voice room
        </Text>
        <Text size="T200" priority="300">
          {voiceRoomLandingCopy({
            memberCount: members?.length ?? 0,
            hasActiveCall: nativeRoom?.hasActiveCall === true,
            liveParticipantCount: nativeRoom?.activeCallParticipantCount ?? 0,
            rtcStatus,
          })}
        </Text>
      </Box>
    </Box>
  );
}
