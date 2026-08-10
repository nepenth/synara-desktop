import React from 'react';
import { Box, Icons, Icon, Text, config } from 'folds';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { useRoom } from '../../hooks/useRoom';
import { useRoomMembers } from '../../hooks/useRoomMembers';
import { isNativeMatrixSession } from '../verification/nativeVerification';
import { normalizeRoomJoinRulePresentation } from '../matrix-dto/roomJoinRule';
import { getRoomIconSrc } from '../../utils/room';
import { RoomType } from '../../../types/matrix/room';

/**
 * Voice-room hero — first-class in-room surface for rooms created as a voice
 * room (`m.room.create` type `org.matrix.msc3417.call`). Provides an honest
 * landing for the live-conversation lane without inventing call controls the
 * native client does not yet expose.
 */
export function VoiceRoom() {
  const room = useRoom();
  const mx = useMatrixClient();
  const members = useRoomMembers(mx, room.roomId, isNativeMatrixSession());
  const iconSrc = getRoomIconSrc(
    Icons,
    RoomType.Call,
    normalizeRoomJoinRulePresentation(room.getJoinRule())
  );

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
          {members && members.length > 0
            ? `${members.length} ${
                members.length === 1 ? 'participant' : 'participants'
              } \u2014 built for live conversation. Messages you send here stay on the timeline.`
            : 'Built for live conversation. Messages you send here stay on the timeline.'}
        </Text>
      </Box>
    </Box>
  );
}
