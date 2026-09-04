import React, { PointerEventHandler, useRef, useState } from 'react';
import { Box, Header, Icon, IconButton, Icons, Line, Scroll, Text, config, toRem } from 'folds';
import { ContainerColor } from '../../styles/ContainerColor.css';
import * as depthCss from '../../styles/Depth.css';
import type { EventedRoomReading } from '../../utils/roomEvents';
import { RoomNotesPanel } from './room-notes/RoomNotesPanel';
import { RoomPinMenu } from './room-pin-menu';
import { MessageSearch } from '../message-search';

export type RoomSidePanelType = 'notes' | 'pins' | 'search';

type RoomSearchPanelProps = {
  room: EventedRoomReading;
  requestClose: () => void;
};

function RoomSearchPanel({ room, requestClose }: RoomSearchPanelProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  return (
    <Box className={ContainerColor({ variant: 'Surface' })} direction="Column" grow="Yes">
      <Header
        size="600"
        data-tauri-drag-region
        style={{ padding: `0 ${config.space.S200} 0 ${config.space.S400}` }}
      >
        <Box grow="Yes" alignItems="Center" gap="200">
          <Icon src={Icons.Search} size="300" />
          <Text size="H4">Message Search</Text>
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
      <Scroll ref={scrollRef} size="300" hideTrack visibility="Hover">
        <Box direction="Column" style={{ padding: config.space.S400 }}>
          <MessageSearch
            defaultRoomsFilterName={room.name}
            rooms={[room.roomId]}
            scrollRef={scrollRef}
          />
        </Box>
      </Scroll>
    </Box>
  );
}

type RoomSidePanelProps = {
  room: EventedRoomReading;
  activePanel: RoomSidePanelType;
  requestClose: () => void;
};

const DEFAULT_PANEL_WIDTH: Record<RoomSidePanelType, number> = {
  notes: 380,
  pins: 420,
  search: 560,
};

const MIN_PANEL_WIDTH: Record<RoomSidePanelType, number> = {
  notes: 300,
  pins: 340,
  search: 420,
};

const MAX_PANEL_WIDTH = 760;

const clampWidth = (value: number, min: number): number =>
  Math.min(Math.max(value, min), Math.min(MAX_PANEL_WIDTH, Math.max(min, window.innerWidth - 360)));

export function RoomSidePanel({ room, activePanel, requestClose }: RoomSidePanelProps) {
  const [widths, setWidths] = useState(DEFAULT_PANEL_WIDTH);
  const width = widths[activePanel];
  const minWidth = MIN_PANEL_WIDTH[activePanel];

  const handleResizePointerDown: PointerEventHandler<HTMLDivElement> = (evt) => {
    evt.preventDefault();
    const startX = evt.clientX;
    const startWidth = width;
    const previousCursor = document.body.style.cursor;
    const previousSelect = document.body.style.userSelect;
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';

    const handlePointerMove = (moveEvt: PointerEvent) => {
      const nextWidth = clampWidth(startWidth + startX - moveEvt.clientX, minWidth);
      setWidths((current) => ({
        ...current,
        [activePanel]: nextWidth,
      }));
    };

    const handlePointerUp = () => {
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousSelect;
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
  };

  return (
    <Box
      className={ContainerColor({ variant: 'Background' })}
      shrink="No"
      style={{
        position: 'relative',
        width: toRem(width),
        minWidth: toRem(minWidth),
        maxWidth: `calc(100vw - ${toRem(360)})`,
      }}
    >
      <div
        aria-hidden
        onPointerDown={handleResizePointerDown}
        style={{
          position: 'absolute',
          top: 0,
          bottom: 0,
          left: toRem(-4),
          width: toRem(8),
          cursor: 'ew-resize',
          zIndex: 1,
        }}
      />
      {activePanel === 'notes' && (
        <RoomNotesPanel room={room} requestClose={requestClose} embedded />
      )}
      {activePanel === 'pins' && (
        <RoomPinMenu
          room={room as unknown as React.ComponentProps<typeof RoomPinMenu>['room']}
          requestClose={requestClose}
          mode="drawer"
        />
      )}
      {activePanel === 'search' && <RoomSearchPanel room={room} requestClose={requestClose} />}
    </Box>
  );
}
