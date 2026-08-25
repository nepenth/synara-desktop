import React, { FormEventHandler, useMemo, useState } from 'react';
import {
  Box,
  Button,
  Checkbox,
  Chip,
  config,
  Header,
  Icon,
  IconButton,
  Icons,
  Line,
  Scroll,
  Text,
  TextArea,
  color,
  toRem,
} from 'folds';
import { useAtomValue } from 'jotai';
import { SynaraRoomNoteItem } from '../../../../types/matrix/accountData';
import {
  createManualRoomNoteItem,
  getRoomNoteItems,
  rankRoomNoteItem,
} from '../../../utils/roomNotes';
import { roomNotesContentAtom } from '../../../state/roomNotesList';
import {
  completeRoomTodoWithNativeOwner,
  deleteRoomNoteWithNativeOwner,
  moveRoomTodoWithNativeOwner,
  upsertRoomNoteWithNativeOwner,
} from '../nativeRoomNotesOwner';
import { useRoomNavigate } from '../../../hooks/useRoomNavigate';
import { getMxIdLocalPart } from '../../../utils/matrix';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { useRoomMembers, type RoomMemberListItem } from '../../../hooks/useRoomMembers';

type RoomIdentity = {
  roomId: string;
  name?: string;
};

const getProjectedMemberDisplayName = (
  member: RoomMemberListItem | undefined
): string | undefined => {
  if (!member) return undefined;
  let displayName: string | undefined;
  if ('displayName' in member && typeof member.displayName === 'string') {
    displayName = member.displayName;
  } else if ('rawDisplayName' in member) {
    displayName = member.rawDisplayName;
  }
  if (!displayName || displayName === member.userId) return undefined;
  return displayName;
};

type RoomNotesPanelProps = {
  room: RoomIdentity;
  requestClose: () => void;
  embedded?: boolean;
};

const formatNoteTime = (ts: number): string =>
  new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(ts));

type RoomNoteItemProps = {
  roomId: string;
  item: SynaraRoomNoteItem;
  memberDisplayNames: ReadonlyMap<string, string>;
  roomItems: SynaraRoomNoteItem[];
  canMoveUp: boolean;
  canMoveDown: boolean;
};

function RoomNoteItem({
  roomId,
  item,
  memberDisplayNames,
  roomItems,
  canMoveUp,
  canMoveDown,
}: RoomNoteItemProps) {
  const { navigateRoom } = useRoomNavigate();
  const senderName =
    item.sender &&
    (memberDisplayNames.get(item.sender) ?? getMxIdLocalPart(item.sender) ?? item.sender);

  const handleDelete = () => {
    void deleteRoomNoteWithNativeOwner(roomId, item.id).catch(() => undefined);
  };
  const handleToggleTodo = () => {
    void completeRoomTodoWithNativeOwner(roomId, item.id, !item.completedAt).catch(() => undefined);
  };
  const handleMoveUp = () => {
    if (item.kind === 'todo') {
      void moveRoomTodoWithNativeOwner(roomId, item.id, 'up').catch(() => undefined);
      return;
    }
    const ranked = rankRoomNoteItem(roomItems, item.id, 'up');
    if (ranked) void upsertRoomNoteWithNativeOwner(ranked).catch(() => undefined);
  };
  const handleMoveDown = () => {
    if (item.kind === 'todo') {
      void moveRoomTodoWithNativeOwner(roomId, item.id, 'down').catch(() => undefined);
      return;
    }
    const ranked = rankRoomNoteItem(roomItems, item.id, 'down');
    if (ranked) void upsertRoomNoteWithNativeOwner(ranked).catch(() => undefined);
  };
  const handleOpenMessage = () => {
    if (item.eventId) navigateRoom(roomId, item.eventId);
  };

  return (
    <Box
      direction="Column"
      gap="200"
      style={{
        padding: config.space.S300,
        borderRadius: config.radii.R400,
        backgroundColor: color.SurfaceVariant.Container,
      }}
    >
      <Box alignItems="Center" gap="200">
        {item.kind === 'todo' && (
          <Checkbox
            checked={!!item.completedAt}
            onClick={handleToggleTodo}
            size="300"
            variant="Primary"
          />
        )}
        <Chip
          size="400"
          radii="Pill"
          variant={item.kind === 'todo' && item.completedAt ? 'Success' : 'Surface'}
        >
          <Text size="L400">
            {item.kind === 'todo' ? 'ToDo' : item.kind === 'message' ? 'Message' : 'Note'}
          </Text>
        </Chip>
        <Box grow="Yes" justifyContent="End">
          <Text size="T200" priority="300">
            {formatNoteTime(item.updatedAt)}
          </Text>
        </Box>
      </Box>
      {item.body && (
        <Text
          size="T300"
          style={{
            whiteSpace: 'pre-wrap',
            textDecoration: item.completedAt ? 'line-through' : undefined,
            opacity: item.completedAt ? config.opacity.P300 : undefined,
          }}
        >
          {item.body}
        </Text>
      )}
      {item.kind === 'message' && (
        <Box alignItems="Center" gap="200">
          <Text size="T200" priority="300" truncate>
            {senderName ? `${senderName} · ` : ''}
            {item.eventTs ? formatNoteTime(item.eventTs) : item.eventId}
          </Text>
        </Box>
      )}
      <Box gap="200" justifyContent="SpaceBetween" alignItems="Center">
        <Box gap="100">
          {(item.kind === 'todo' || item.kind === 'note') && (
            <>
              <IconButton
                size="300"
                radii="300"
                disabled={!canMoveUp}
                onClick={handleMoveUp}
                aria-label={`Move ${item.kind === 'todo' ? 'ToDo' : 'note'} up`}
              >
                <Icon src={Icons.ChevronTop} size="200" />
              </IconButton>
              <IconButton
                size="300"
                radii="300"
                disabled={!canMoveDown}
                onClick={handleMoveDown}
                aria-label={`Move ${item.kind === 'todo' ? 'ToDo' : 'note'} down`}
              >
                <Icon src={Icons.ChevronBottom} size="200" />
              </IconButton>
            </>
          )}
        </Box>
        <Box gap="200" justifyContent="End">
          {item.eventId && (
            <Button size="300" radii="300" onClick={handleOpenMessage}>
              <Text size="B300">Open</Text>
            </Button>
          )}
          <Button size="300" radii="300" variant="Critical" fill="None" onClick={handleDelete}>
            <Text size="B300">Delete</Text>
          </Button>
        </Box>
      </Box>
    </Box>
  );
}

const getItemOrderState = (
  item: SynaraRoomNoteItem,
  roomItems: SynaraRoomNoteItem[]
): { canMoveUp: boolean; canMoveDown: boolean } => {
  if (item.kind !== 'todo' && item.kind !== 'note') {
    return { canMoveUp: false, canMoveDown: false };
  }
  const itemGroup = roomItems.filter(
    (candidate) =>
      candidate.kind === item.kind &&
      (item.kind !== 'todo' || !!candidate.completedAt === !!item.completedAt)
  );
  const index = itemGroup.findIndex((candidate) => candidate.id === item.id);
  return {
    canMoveUp: index > 0,
    canMoveDown: index >= 0 && index < itemGroup.length - 1,
  };
};

export function RoomNotesPanel({ room, requestClose, embedded }: RoomNotesPanelProps) {
  const [kind, setKind] = useState<'note' | 'todo'>('note');
  const [body, setBody] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const mx = useMatrixClient();
  const nativeSession = isNativeMatrixSession();
  const memberSnapshot = useRoomMembers(mx, room.roomId, nativeSession);
  const memberDisplayNames = useMemo(() => {
    const names = new Map<string, string>();
    for (const member of memberSnapshot ?? []) {
      const displayName = getProjectedMemberDisplayName(member);
      if (displayName) names.set(member.userId, displayName);
    }
    return names;
  }, [memberSnapshot]);
  const notesContent = useAtomValue(roomNotesContentAtom);
  const roomItems = useMemo(
    () => getRoomNoteItems(notesContent, room.roomId),
    [notesContent, room.roomId]
  );

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    const item = createManualRoomNoteItem(room.roomId, kind, body);
    if (!item || saving) return;
    setSaving(true);
    setError(undefined);
    void upsertRoomNoteWithNativeOwner(item)
      .then(() => setBody(''))
      .catch(() => setError('Could not save this item.'))
      .finally(() => setSaving(false));
  };

  return (
    <Box
      direction="Column"
      style={{
        boxSizing: 'border-box',
        minWidth: 0,
        overflow: 'hidden',
        width: embedded ? '100%' : `min(${toRem(560)}, calc(100vw - ${config.space.S600}))`,
        height: embedded ? '100%' : undefined,
        maxHeight: embedded ? undefined : `min(${toRem(720)}, calc(100vh - ${config.space.S600}))`,
        borderRadius: embedded ? 0 : config.radii.R500,
        backgroundColor: color.Surface.Container,
        boxShadow: embedded ? undefined : '0 18px 60px rgba(0, 0, 0, 0.4)',
      }}
    >
      <Header size="600" style={{ padding: `0 ${config.space.S300}` }}>
        <Box grow="Yes" direction="Column">
          <Text size="H4">Personal Notes</Text>
          <Text size="T200" priority="300" truncate>
            {room.name ?? room.roomId}
          </Text>
        </Box>
        <IconButton size="300" onClick={requestClose} radii="300">
          <Icon src={Icons.Cross} />
        </IconButton>
      </Header>
      <Line variant="Surface" size="300" />
      <Box
        as="form"
        direction="Column"
        gap="300"
        onSubmit={handleSubmit}
        style={{ minWidth: 0, padding: config.space.S300 }}
      >
        <Box gap="200">
          <Chip
            type="button"
            radii="Pill"
            variant={kind === 'note' ? 'Primary' : 'SurfaceVariant'}
            onClick={() => setKind('note')}
          >
            <Text size="B300">Note</Text>
          </Chip>
          <Chip
            type="button"
            radii="Pill"
            variant={kind === 'todo' ? 'Primary' : 'SurfaceVariant'}
            onClick={() => setKind('todo')}
          >
            <Text size="B300">ToDo</Text>
          </Chip>
        </Box>
        <TextArea
          required
          value={body}
          onChange={(evt) => setBody(evt.currentTarget.value)}
          size="500"
          variant="SurfaceVariant"
          radii="400"
          placeholder={kind === 'todo' ? 'Add a ToDo item...' : 'Add a private note...'}
          style={{
            boxSizing: 'border-box',
            minHeight: toRem(96),
            resize: 'vertical',
            width: '100%',
          }}
          disabled={saving}
        />
        <Box alignItems="Center" gap="200" style={{ minWidth: 0 }}>
          <Box grow="Yes">
            {error && (
              <Text size="T200" style={{ color: color.Critical.Main }}>
                {error}
              </Text>
            )}
          </Box>
          <Button
            type="submit"
            variant="Primary"
            size="300"
            radii="300"
            disabled={saving || body.trim().length === 0}
            before={<Icon size="100" src={Icons.Plus} />}
          >
            <Text size="B300">Add</Text>
          </Button>
        </Box>
      </Box>
      <Line variant="Surface" size="300" />
      <Scroll style={{ flexGrow: 1, minHeight: 0 }}>
        <Box direction="Column" gap="200" style={{ minWidth: 0, padding: config.space.S300 }}>
          {roomItems.length > 0 ? (
            roomItems.map((item) => {
              const itemOrderState = getItemOrderState(item, roomItems);
              return (
                <RoomNoteItem
                  key={item.id}
                  roomId={room.roomId}
                  item={item}
                  memberDisplayNames={memberDisplayNames}
                  roomItems={roomItems}
                  canMoveUp={itemOrderState.canMoveUp}
                  canMoveDown={itemOrderState.canMoveDown}
                />
              );
            })
          ) : (
            <Box
              direction="Column"
              alignItems="Center"
              justifyContent="Center"
              gap="200"
              style={{ minHeight: toRem(180), padding: config.space.S300, textAlign: 'center' }}
            >
              <Icon size="600" src={Icons.Pencil} />
              <Text size="H5">No personal notes yet</Text>
              <Text size="T300" priority="300" style={{ maxWidth: toRem(280) }}>
                Add notes, ToDo items, or pin useful messages from the message menu.
              </Text>
            </Box>
          )}
        </Box>
      </Scroll>
    </Box>
  );
}
