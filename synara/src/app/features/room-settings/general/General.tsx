import React from 'react';
import { usePowerLevels } from '../../../hooks/usePowerLevels';
import { useRoom } from '../../../hooks/useRoom';
import {
  RoomProfile,
  RoomEncryption,
  RoomHistoryVisibility,
  RoomJoinRules,
  RoomLocalAddresses,
  RoomPublishedAddresses,
  RoomPublish,
  RoomRetention,
  RoomUpgrade,
} from '../../common-settings/general';
import { useRoomPermissions } from '../../../hooks/useRoomPermissions';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';

type GeneralProps = {
  requestClose: () => void;
};
export function General({ requestClose }: GeneralProps) {
  const room = useRoom();
  const powerLevels = usePowerLevels(room);
  const permissions = useRoomPermissions(powerLevels);

  return (
    <SettingsPage
      title="General"
      description="Name, access, history, encryption and addresses for this room."
      requestClose={requestClose}
    >
      <RoomProfile permissions={permissions} />
      <SettingsSection title="Options">
        <RoomJoinRules permissions={permissions} />
        <RoomHistoryVisibility permissions={permissions} />
        <RoomRetention />
        <RoomEncryption permissions={permissions} />
        <RoomPublish permissions={permissions} roomId={room.roomId} isSpace={room.isSpaceRoom()} />
      </SettingsSection>
      <SettingsSection title="Addresses">
        <RoomPublishedAddresses permissions={permissions} />
        <RoomLocalAddresses permissions={permissions} />
      </SettingsSection>
      <SettingsSection
        title="Danger Zone"
        description="Changes here cannot be undone."
        tone="critical"
      >
        <RoomUpgrade permissions={permissions} requestClose={requestClose} />
      </SettingsSection>
    </SettingsPage>
  );
}
