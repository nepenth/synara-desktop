import React, { useCallback, useEffect, useState } from 'react';
import { Box, Button, Header, Icon, IconButton, Icons, Line, Scroll, Text, config } from 'folds';
import { ContainerColor } from '../../styles/ContainerColor.css';
import * as depthCss from '../../styles/Depth.css';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import {
  listExperimentalWidgets,
  openExperimentalWidget,
  type ListedWidget,
} from '../widgets/experimentalWidgets';

type RoomWidgetsPanelProps = {
  roomId: string;
  requestClose: () => void;
};

export function RoomWidgetsPanel({ roomId, requestClose }: RoomWidgetsPanelProps) {
  const [experimentalWidgetsEnabled] = useSetting(settingsAtom, 'experimentalWidgetsEnabled');
  const [agentWidgetEntries] = useSetting(settingsAtom, 'agentWidgetEntries');
  const [widgets, setWidgets] = useState<ListedWidget[]>([]);
  const [pending, setPending] = useState<ListedWidget>();
  const [allowSend, setAllowSend] = useState(false);

  const refresh = useCallback(async () => {
    const snapshot = await listExperimentalWidgets(
      roomId,
      agentWidgetEntries,
      experimentalWidgetsEnabled
    );
    setWidgets(snapshot?.widgets ?? []);
  }, [agentWidgetEntries, experimentalWidgetsEnabled, roomId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleConfirmOpen = async () => {
    if (!pending) return;
    await openExperimentalWidget({
      experimentalWidgetsEnabled,
      roomId,
      widget: pending,
      receiveRoom: true,
      sendRoomMessage: allowSend,
    });
    setPending(undefined);
    setAllowSend(false);
  };

  return (
    <Box className={ContainerColor({ variant: 'Surface' })} direction="Column" grow="Yes">
      <Header
        size="600"
        data-tauri-drag-region
        style={{ padding: `0 ${config.space.S200} 0 ${config.space.S400}` }}
      >
        <Box grow="Yes" alignItems="Center" gap="200">
          <Icon src={Icons.Code} size="300" />
          <Text size="H4">Widgets</Text>
        </Box>
        <IconButton
          className={depthCss.quietInteractiveSurface}
          size="300"
          onClick={requestClose}
          radii="300"
        >
          <Icon src={Icons.Cross} size="400" />
        </IconButton>
      </Header>
      <Line variant="Surface" size="300" />
      <Scroll size="300" hideTrack visibility="Hover">
        <Box direction="Column" gap="400" style={{ padding: config.space.S400 }}>
          <Text size="T200" priority="300">
            Widgets run third-party or local web code as the logged-in user for any granted
            capabilities. Room-state widgets never load loopback.
          </Text>
          {widgets.length === 0 && (
            <Text size="T200" priority="300">
              No room widgets or agent URLs are available in this room.
            </Text>
          )}
          {widgets.map((widget) => (
            <Box key={`${widget.kind}-${widget.widgetId}`} direction="Column" gap="200">
              <Text size="T300">{widget.name}</Text>
              <Text size="T200" priority="300">
                {widget.origin} · {widget.kind === 'agent' ? 'Agent' : 'Room'}
              </Text>
              <Button
                size="300"
                variant="Secondary"
                fill="Soft"
                onClick={() => {
                  setPending(widget);
                  setAllowSend(false);
                }}
              >
                <Text size="B300">Open</Text>
              </Button>
            </Box>
          ))}
          {pending && (
            <Box direction="Column" gap="200">
              <Text size="T300">Open {pending.name}?</Text>
              <Text size="T200" priority="300">
                Origin: {pending.origin}. Receive room events the user already sees. Send is limited
                to m.room.message after an explicit grant.
              </Text>
              <Button
                size="300"
                variant={allowSend ? 'Primary' : 'Secondary'}
                fill="Soft"
                onClick={() => setAllowSend((current) => !current)}
              >
                <Text size="B300">
                  {allowSend ? 'Sending messages allowed' : 'Allow sending m.room.message'}
                </Text>
              </Button>
              <Box gap="200">
                <Button size="300" variant="Primary" onClick={() => void handleConfirmOpen()}>
                  <Text size="B300">Open widget</Text>
                </Button>
                <Button
                  size="300"
                  variant="Secondary"
                  fill="Soft"
                  onClick={() => setPending(undefined)}
                >
                  <Text size="B300">Cancel</Text>
                </Button>
              </Box>
            </Box>
          )}
        </Box>
      </Scroll>
    </Box>
  );
}
