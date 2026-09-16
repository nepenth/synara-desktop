import {
  Badge,
  Box,
  Button,
  color,
  config,
  Dialog,
  Header,
  Icon,
  IconButton,
  Icons,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Spinner,
  Text,
} from 'folds';
import React, { useCallback, useState } from 'react';
import type { MatrixError } from '../../../utils/matrix';
import FocusTrap from 'focus-trap-react';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../../room-settings/styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { StateEvent } from '../../../../types/matrix/room';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { useRoom } from '../../../hooks/useRoom';
import { useStateEvent } from '../../../hooks/useStateEvent';
import { stopPropagation } from '../../../utils/keyboard';
import { RoomPermissionsAPI } from '../../../hooks/useRoomPermissions';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { useNativeRoomListSnapshot } from '../../../state/room-list/roomList';
import {
  desktopUsesNativeStateEventOwner,
  enableRoomEncryptedStateWithNativeOwner,
} from '../../../components/nativeStateEventOwner';
import { pushEncryptedStateEventsSetting } from '../../settings/encryptedStateEvents';
import { getSharedSettings } from '../../../state/settings';

const ROOM_ENC_ALGO = 'm.megolm.v1.aes-sha2';

const encryptionContentRequestsState = (content?: {
  algorithm?: string;
  encrypt_state_events?: boolean;
  'io.element.msc4362.encrypt_state_events'?: boolean;
}): boolean =>
  content?.encrypt_state_events === true ||
  content?.['io.element.msc4362.encrypt_state_events'] === true;

type RoomEncryptionProps = {
  permissions: RoomPermissionsAPI;
};
export function RoomEncryption({ permissions }: RoomEncryptionProps) {
  const mx = useMatrixClient();
  const room = useRoom();
  const [encryptedStateEventsSetting] = useSetting(settingsAtom, 'encryptedStateEvents');
  const nativeRooms = useNativeRoomListSnapshot();
  const nativeSummary = nativeRooms.rooms.find((item) => item.roomId === room.roomId);

  const canEnable = permissions.stateEvent(StateEvent.RoomEncryption, mx.getSafeUserId());
  const content = useStateEvent(room, StateEvent.RoomEncryption)?.getContent<{
    algorithm: string;
    encrypt_state_events?: boolean;
    'io.element.msc4362.encrypt_state_events'?: boolean;
  }>();
  const enabled =
    content?.algorithm === ROOM_ENC_ALGO || nativeSummary?.encryptionStatus === 'encrypted';
  const stateEncrypted =
    encryptionContentRequestsState(content) || nativeSummary?.stateEncrypted === true;
  const isCallRoom = nativeSummary?.isCall === true;
  const showStateOptIn =
    enabled && !stateEncrypted && encryptedStateEventsSetting && !isCallRoom;

  const [enableState, enable] = useAsyncCallback(
    useCallback(async () => {
      await pushEncryptedStateEventsSetting(getSharedSettings().encryptedStateEvents);
      if (desktopUsesNativeStateEventOwner()) {
        await enableRoomEncryptedStateWithNativeOwner(room.roomId, false);
        return;
      }
      await mx.sendStateEvent(room.roomId, StateEvent.RoomEncryption as any, {
        algorithm: ROOM_ENC_ALGO,
      });
    }, [mx, room.roomId])
  );

  const [optInState, optIn] = useAsyncCallback(
    useCallback(async () => {
      await pushEncryptedStateEventsSetting(getSharedSettings().encryptedStateEvents);
      if (desktopUsesNativeStateEventOwner()) {
        await enableRoomEncryptedStateWithNativeOwner(room.roomId, true);
        return;
      }
      await mx.sendStateEvent(room.roomId, StateEvent.RoomEncryption as any, {
        algorithm: ROOM_ENC_ALGO,
        encrypt_state_events: true,
        'io.element.msc4362.encrypt_state_events': true,
      });
    }, [mx, room.roomId])
  );

  const enabling = enableState.status === AsyncStatus.Loading;
  const optingIn = optInState.status === AsyncStatus.Loading;

  const [prompt, setPrompt] = useState(false);
  const [optInPrompt, setOptInPrompt] = useState(false);

  const handleEnable = () => {
    enable();
    setPrompt(false);
  };

  const handleOptIn = () => {
    optIn();
    setOptInPrompt(false);
  };

  return (
    <SequenceCard
      className={SequenceCardStyle}
      variant="SurfaceVariant"
      direction="Column"
      gap="400"
    >
      <SettingTile
        title="Room Encryption"
        description={
          enabled
            ? 'Messages in this room are protected by end-to-end encryption.'
            : 'Once enabled, encryption cannot be disabled!'
        }
        after={
          enabled ? (
            <Badge size="500" variant="Success" fill="Solid" radii="300">
              <Text size="L400">Enabled</Text>
            </Badge>
          ) : (
            <Button
              size="300"
              variant="Primary"
              fill="Solid"
              radii="300"
              disabled={!canEnable}
              onClick={() => setPrompt(true)}
              before={enabling && <Spinner size="100" variant="Primary" fill="Solid" />}
            >
              <Text size="B300">Enable</Text>
            </Button>
          )
        }
      >
        {enableState.status === AsyncStatus.Error && (
          <Text style={{ color: color.Critical.Main }} size="T200">
            {(enableState.error as MatrixError).message}
          </Text>
        )}
        {prompt && (
          <Overlay open backdrop={<OverlayBackdrop />}>
            <OverlayCenter>
              <FocusTrap
                focusTrapOptions={{
                  initialFocus: false,
                  onDeactivate: () => setPrompt(false),
                  clickOutsideDeactivates: true,
                  escapeDeactivates: stopPropagation,
                }}
              >
                <Dialog variant="Surface">
                  <Header
                    style={{
                      padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
                      borderBottomWidth: config.borderWidth.B300,
                    }}
                    variant="Surface"
                    size="500"
                  >
                    <Box grow="Yes">
                      <Text size="H4">Enable Encryption</Text>
                    </Box>
                    <IconButton size="300" onClick={() => setPrompt(false)} radii="300">
                      <Icon src={Icons.Cross} />
                    </IconButton>
                  </Header>
                  <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
                    <Text priority="400">
                      Are you sure? Once enabled, encryption cannot be disabled!
                    </Text>
                    <Button type="submit" variant="Primary" onClick={handleEnable}>
                      <Text size="B400">Enable E2E Encryption</Text>
                    </Button>
                  </Box>
                </Dialog>
              </FocusTrap>
            </OverlayCenter>
          </Overlay>
        )}
      </SettingTile>
      {stateEncrypted && (
        <SettingTile
          title="Encrypted state events"
          description="This room encrypts eligible state (name, topic, avatar, pins). Older clients will not see that chrome. This cannot be turned off."
          after={
            <Badge size="500" variant="Success" fill="Solid" radii="300">
              <Text size="L400">Enabled</Text>
            </Badge>
          }
        />
      )}
      {showStateOptIn && (
        <SettingTile
          title="Encrypt state events"
          description="Experimental MSC4362. Older clients will not see this room's name, topic, or avatar. Cannot be disabled after opt-in. This device will still decrypt rooms that already have the flag if you turn the account setting off."
          after={
            <Button
              size="300"
              variant="Secondary"
              fill="Soft"
              radii="300"
              disabled={!canEnable || optingIn}
              onClick={() => setOptInPrompt(true)}
              before={optingIn && <Spinner size="100" variant="Secondary" fill="Solid" />}
            >
              <Text size="B300">Encrypt state events</Text>
            </Button>
          }
        >
          {optInState.status === AsyncStatus.Error && (
            <Text style={{ color: color.Critical.Main }} size="T200">
              {(optInState.error as MatrixError).message}
            </Text>
          )}
          {optInPrompt && (
            <Overlay open backdrop={<OverlayBackdrop />}>
              <OverlayCenter>
                <FocusTrap
                  focusTrapOptions={{
                    initialFocus: false,
                    onDeactivate: () => setOptInPrompt(false),
                    clickOutsideDeactivates: true,
                    escapeDeactivates: stopPropagation,
                  }}
                >
                  <Dialog variant="Surface">
                    <Header
                      style={{
                        padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
                        borderBottomWidth: config.borderWidth.B300,
                      }}
                      variant="Surface"
                      size="500"
                    >
                      <Box grow="Yes">
                        <Text size="H4">Encrypt state events</Text>
                      </Box>
                      <IconButton size="300" onClick={() => setOptInPrompt(false)} radii="300">
                        <Icon src={Icons.Cross} />
                      </IconButton>
                    </Header>
                    <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
                      <Text priority="400">
                        Eligible state events will be encrypted. Older clients will lose room name,
                        topic, and avatar. This cannot be turned off.
                      </Text>
                      <Button type="submit" variant="Primary" onClick={handleOptIn}>
                        <Text size="B400">Encrypt state events</Text>
                      </Button>
                    </Box>
                  </Dialog>
                </FocusTrap>
              </OverlayCenter>
            </Overlay>
          )}
        </SettingTile>
      )}
    </SequenceCard>
  );
}
