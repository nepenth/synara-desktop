import React, { useState } from 'react';
import { useRoom } from '../../../hooks/useRoom';
import { usePowerLevels } from '../../../hooks/usePowerLevels';
import { StateEvent } from '../../../../types/matrix/room';
import { usePermissionGroups } from './usePermissionItems';
import { PermissionGroups, Powers, PowersEditor } from '../../common-settings/permissions';
import { useRoomPermissions } from '../../../hooks/useRoomPermissions';
import { SettingsPage } from '../../../components/settings-layout';

type PermissionsProps = {
  requestClose: () => void;
};
export function Permissions({ requestClose }: PermissionsProps) {
  const room = useRoom();
  const powerLevels = usePowerLevels(room);

  const permissions = useRoomPermissions(powerLevels);

  const canEditPowers = permissions.stateEvent(StateEvent.PowerLevelTags);
  const canEditPermissions = permissions.stateEvent(StateEvent.RoomPowerLevels);
  const permissionGroups = usePermissionGroups();

  const [powerEditor, setPowerEditor] = useState(false);

  const handleEditPowers = () => {
    setPowerEditor(true);
  };

  if (canEditPowers && powerEditor) {
    return <PowersEditor powerLevels={powerLevels} requestClose={() => setPowerEditor(false)} />;
  }

  return (
    <SettingsPage
      title="Permissions"
      description="Which roles can change settings, send messages and moderate."
      requestClose={requestClose}
    >
      <Powers
        powerLevels={powerLevels}
        onEdit={canEditPowers ? handleEditPowers : undefined}
        permissionGroups={permissionGroups}
      />
      <PermissionGroups
        canEdit={canEditPermissions}
        powerLevels={powerLevels}
        permissionGroups={permissionGroups}
      />
    </SettingsPage>
  );
}
