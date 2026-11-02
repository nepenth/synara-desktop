import React, { useCallback, useEffect, useState } from 'react';
import { Button, Text, config, Box } from 'folds';
import { ConfirmDialog } from '../../../components/confirm-dialog';
import type { MatrixError } from '../../../utils/matrix';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../../room-settings/styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useRoom } from '../../../hooks/useRoom';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { IRoomCreateContent, StateEvent } from '../../../../types/matrix/room';
import { useStateEvent } from '../../../hooks/useStateEvent';
import { useRoomNavigate } from '../../../hooks/useRoomNavigate';
import { useCapabilities } from '../../../hooks/useCapabilities';
import { RoomPermissionsAPI } from '../../../hooks/useRoomPermissions';
import {
  AdditionalCreatorInput,
  RoomVersionSelector,
  useAdditionalCreators,
} from '../../../components/create-room';
import { useAlive } from '../../../hooks/useAlive';
import { upgradeRoom } from '../../../native/nativeRoomExtras';
import { creatorsSupported } from '../../../utils/matrix';
import { useRoomCreators } from '../../../hooks/useRoomCreators';

type RoomTombstoneEventContent = {
  body: string;
  replacement_room: string;
};

function RoomUpgradeDialog({ requestClose }: { requestClose: () => void }) {
  const room = useRoom();
  const alive = useAlive();
  const creators = useRoomCreators(room);

  const capabilities = useCapabilities();
  const roomVersions = capabilities['m.room_versions'];
  const [selectedRoomVersion, selectRoomVersion] = useState(roomVersions?.default ?? '1');
  useEffect(() => {
    // capabilities load async
    selectRoomVersion(roomVersions?.default ?? '1');
  }, [roomVersions?.default]);

  const allowAdditionalCreators = creatorsSupported(selectedRoomVersion);
  const { additionalCreators, addAdditionalCreator, removeAdditionalCreator } =
    useAdditionalCreators(Array.from(creators));

  const { navigateRoom, navigateSpace } = useRoomNavigate();
  const [upgradeState, upgrade] = useAsyncCallback(
    useCallback<(version: string, additionalCreators?: string[]) => Promise<string>>(
      (version, extraCreators) => upgradeRoom(room.roomId, version, extraCreators),
      [room.roomId]
    )
  );

  const upgrading = upgradeState.status === AsyncStatus.Loading;

  const handleUpgradeRoom = () => {
    const version = selectedRoomVersion;

    upgrade(version, allowAdditionalCreators ? additionalCreators : undefined).then(
      (replacementRoomId) => {
        if (!alive()) return;
        requestClose();
        if (room.isSpaceRoom()) navigateSpace(replacementRoomId);
        else navigateRoom(replacementRoomId);
      }
    );
  };

  const kind = room.isSpaceRoom() ? 'space' : 'room';
  const name = room.name?.trim() || `this ${kind}`;

  return (
    <ConfirmDialog
      title={`Upgrade ${name}?`}
      description={`A new ${kind} replaces this one on the chosen version. Everyone is pointed to it, and the old ${kind} becomes read-only. This cannot be undone.`}
      confirmLabel="Upgrade"
      busyLabel="Upgrading…"
      tone="critical"
      busy={upgrading}
      error={
        upgradeState.status === AsyncStatus.Error
          ? (upgradeState.error as MatrixError).message
          : undefined
      }
      onConfirm={handleUpgradeRoom}
      onCancel={requestClose}
    >
      <Box direction="Column" gap="100">
        <Text size="L400" priority="300">
          Room version
        </Text>
        <RoomVersionSelector
          versions={roomVersions?.available ? Object.keys(roomVersions.available) : ['1']}
          value={selectedRoomVersion}
          onChange={selectRoomVersion}
          disabled={upgrading}
        />
        {allowAdditionalCreators && (
          <SequenceCard
            style={{ padding: config.space.S300 }}
            variant="SurfaceVariant"
            direction="Column"
            gap="500"
          >
            <AdditionalCreatorInput
              additionalCreators={additionalCreators}
              onSelect={addAdditionalCreator}
              onRemove={removeAdditionalCreator}
              disabled={upgrading}
            />
          </SequenceCard>
        )}
      </Box>
    </ConfirmDialog>
  );
}

type RoomUpgradeProps = {
  permissions: RoomPermissionsAPI;
  requestClose: () => void;
};
export function RoomUpgrade({ permissions, requestClose }: RoomUpgradeProps) {
  const room = useRoom();
  const { navigateRoom, navigateSpace } = useRoomNavigate();
  const createContent = useStateEvent(
    room,
    StateEvent.RoomCreate
  )?.getContent<IRoomCreateContent>();
  const roomVersion = createContent?.room_version ?? '1';
  const predecessorRoomId = createContent?.predecessor?.room_id;

  const tombstoneContent = useStateEvent(
    room,
    StateEvent.RoomTombstone
  )?.getContent<RoomTombstoneEventContent>();
  const replacementRoom = tombstoneContent?.replacement_room;

  const canUpgrade = permissions.stateEvent(StateEvent.RoomTombstone);

  const handleOpenRoom = () => {
    if (replacementRoom) {
      requestClose();
      if (room.isSpaceRoom()) {
        navigateSpace(replacementRoom);
      } else {
        navigateRoom(replacementRoom);
      }
    }
  };

  const handleOpenOldRoom = () => {
    if (predecessorRoomId) {
      requestClose();
      if (room.isSpaceRoom()) {
        navigateSpace(predecessorRoomId);
      } else {
        navigateRoom(predecessorRoomId, createContent.predecessor?.event_id);
      }
    }
  };

  const [prompt, setPrompt] = useState(false);

  return (
    <SequenceCard
      className={SequenceCardStyle}
      variant="SurfaceVariant"
      direction="Column"
      gap="400"
    >
      <SettingTile
        title={room.isSpaceRoom() ? 'Upgrade Space' : 'Upgrade Room'}
        description={
          replacementRoom
            ? tombstoneContent.body ||
              `This ${room.isSpaceRoom() ? 'space' : 'room'} has been replaced.`
            : `Room version ${roomVersion}.`
        }
        after={
          <Box alignItems="Center" gap="200">
            {predecessorRoomId && (
              <Button
                size="300"
                variant="Secondary"
                fill="Soft"
                outlined
                radii="300"
                onClick={handleOpenOldRoom}
              >
                <Text size="B300">{room.isSpaceRoom() ? 'Old space' : 'Old room'}</Text>
              </Button>
            )}
            {replacementRoom ? (
              <Button
                size="300"
                variant="Success"
                fill="Solid"
                radii="300"
                onClick={handleOpenRoom}
              >
                <Text size="B300">{room.isSpaceRoom() ? 'Open new space' : 'Open new room'}</Text>
              </Button>
            ) : (
              <Button
                size="300"
                variant="Secondary"
                fill="Solid"
                radii="300"
                disabled={!canUpgrade}
                onClick={() => setPrompt(true)}
              >
                <Text size="B300">Upgrade</Text>
              </Button>
            )}
          </Box>
        }
      >
        {prompt && <RoomUpgradeDialog requestClose={() => setPrompt(false)} />}
      </SettingTile>
    </SequenceCard>
  );
}
