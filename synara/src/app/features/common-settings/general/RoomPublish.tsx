import React from 'react';
import { Box, color, Spinner, Switch, Text } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../../room-settings/styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useRoomDirectoryVisibility } from '../../../hooks/useRoomDirectoryVisibility';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { StateEvent } from '../../../../types/matrix/room';
import { RoomPermissionsAPI } from '../../../hooks/useRoomPermissions';
import { useNativeRoomJoinRule } from '../../../hooks/useNativeRoomJoinRule';

type RoomPublishProps = {
  permissions: RoomPermissionsAPI;
  roomId: string;
  isSpace: boolean;
};

export function RoomPublish({ permissions, roomId, isSpace }: RoomPublishProps) {
  const joinRuleState = useNativeRoomJoinRule(roomId);
  const { visibilityState, setVisibility } = useRoomDirectoryVisibility(roomId);
  const [toggleState, toggleVisibility] = useAsyncCallback(setVisibility);

  const loading =
    joinRuleState.status === 'loading' ||
    visibilityState.status === AsyncStatus.Loading ||
    toggleState.status === AsyncStatus.Loading;
  const canEditCanonical =
    joinRuleState.status === 'ready' &&
    permissions.stateEvent(StateEvent.RoomCanonicalAlias, joinRuleState.userId);
  const validRule =
    joinRuleState.status === 'ready' &&
    (joinRuleState.snapshot.joinRule === 'public' ||
      joinRuleState.snapshot.joinRule === 'knock' ||
      joinRuleState.snapshot.joinRule === 'knock_restricted');
  const errorMessage =
    joinRuleState.status === 'error'
      ? joinRuleState.error.message
      : visibilityState.status === AsyncStatus.Error
      ? visibilityState.error instanceof Error
        ? visibilityState.error.message
        : 'Native Matrix directory visibility is unavailable.'
      : toggleState.status === AsyncStatus.Error
      ? toggleState.error instanceof Error
        ? toggleState.error.message
        : 'Native Matrix directory visibility is unavailable.'
      : undefined;

  return (
    <SequenceCard
      className={SequenceCardStyle}
      variant="SurfaceVariant"
      direction="Column"
      gap="400"
    >
      <SettingTile
        title="Publish to Directory"
        description={
          isSpace
            ? 'List the space in the public directory to make it discoverable by others.'
            : 'List the room in the public directory to make it discoverable by others.'
        }
        after={
          <Box gap="200" alignItems="Center">
            {loading && <Spinner variant="Secondary" />}
            {!loading && visibilityState.status === AsyncStatus.Success && (
              <Switch
                value={visibilityState.data}
                onChange={toggleVisibility}
                disabled={!canEditCanonical || !validRule}
              />
            )}
          </Box>
        }
      >
        {errorMessage && (
          <Text style={{ color: color.Critical.Main }} size="T200">
            {errorMessage}
          </Text>
        )}
      </SettingTile>
    </SequenceCard>
  );
}
