import React, { useId, useRef, useState } from 'react';
import { useAtom } from 'jotai';
import { Box, Input, Text } from 'folds';
import { ConfirmDialog } from '../../components/confirm-dialog';
import {
  ROOM_FOLDER_NAME_MAX,
  isValidRoomFolderName,
  normalizeRoomFolderName,
} from './nativeRoomFolderOwner';
import {
  roomFolderDialogAtom,
  useRoomFolderAction,
  useRoomFolderCommands,
  type RoomFolderDialogRequest,
} from './roomFolderState';

type NameDialogProps = {
  request: Extract<RoomFolderDialogRequest, { kind: 'create' | 'rename' }>;
  onDone: () => void;
};

/**
 * The one naming dialog for New folder and Rename: a single field, Enter
 * saves, Escape cancels, and a failed save says why under the field.
 */
function RoomFolderNameDialog({ request, onDone }: NameDialogProps) {
  const commands = useRoomFolderCommands();
  const { busy, error, run, clearError } = useRoomFolderAction();
  const [name, setName] = useState(request.kind === 'rename' ? request.name : '');
  const [invalid, setInvalid] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const fieldId = useId();
  const creating = request.kind === 'create';

  const handleConfirm = async () => {
    if (busy) return;
    if (!isValidRoomFolderName(name)) {
      setInvalid(true);
      inputRef.current?.focus();
      return;
    }
    if (request.kind === 'rename' && normalizeRoomFolderName(name) === request.name) {
      onDone();
      return;
    }
    const saved = await run(() =>
      request.kind === 'create'
        ? commands.create(name, request.roomId)
        : commands.rename(request.folderId, name)
    );
    if (saved) onDone();
    else inputRef.current?.focus();
  };

  const message = invalid ? `Folder names need 1 to ${ROOM_FOLDER_NAME_MAX} characters.` : error;

  return (
    <ConfirmDialog
      title={creating ? 'New folder' : 'Rename folder'}
      description={
        creating && request.roomId
          ? 'The room moves into the new folder. Folders sync to your other devices.'
          : creating
            ? 'Folders group rooms in the list and sync to your other devices.'
            : undefined
      }
      confirmLabel={creating ? 'Create' : 'Save'}
      busyLabel={creating ? 'Creating…' : 'Saving…'}
      tone="primary"
      busy={busy}
      error={message}
      initialFocusRef={inputRef}
      onConfirm={() => {
        void handleConfirm();
      }}
      onCancel={onDone}
    >
      <Box direction="Column" gap="100">
        <Text as="label" htmlFor={fieldId} size="L400">
          Name
        </Text>
        <Input
          ref={inputRef}
          id={fieldId}
          data-testid="room-folder-name"
          size="400"
          radii="300"
          variant="Background"
          value={name}
          maxLength={ROOM_FOLDER_NAME_MAX * 2}
          placeholder="Work"
          autoComplete="off"
          aria-invalid={message ? true : undefined}
          disabled={busy}
          // The shared trap leaves Escape to a focused field; here it cancels.
          onKeyDown={(evt) => {
            if (evt.key !== 'Escape' || busy) return;
            evt.preventDefault();
            evt.stopPropagation();
            onDone();
          }}
          onChange={(evt) => {
            setName(evt.currentTarget.value);
            setInvalid(false);
            clearError();
          }}
        />
      </Box>
    </ConfirmDialog>
  );
}

type DeleteDialogProps = {
  request: Extract<RoomFolderDialogRequest, { kind: 'delete' }>;
  onDone: () => void;
};

function RoomFolderDeleteDialog({ request, onDone }: DeleteDialogProps) {
  const commands = useRoomFolderCommands();
  const { busy, error, run } = useRoomFolderAction();
  return (
    <ConfirmDialog
      title={`Delete the ${request.name} folder?`}
      description="Its rooms go back to Rooms. You stay in every room, and your other devices update too."
      confirmLabel="Delete folder"
      busyLabel="Deleting…"
      tone="critical"
      busy={busy}
      error={error}
      onConfirm={() => {
        void run(() => commands.remove(request.folderId)).then((done) => {
          if (done) onDone();
        });
      }}
      onCancel={onDone}
    />
  );
}

/**
 * Hosts the folder dialogs for one room list. Menus and section headers
 * request a dialog through `roomFolderDialogAtom` and close themselves; the
 * dialog lives here, so neither a closing menu nor a virtual row scrolling
 * away unmounts it.
 */
export function RoomFolderDialogs() {
  const [request, setRequest] = useAtom(roomFolderDialogAtom);
  if (!request) return null;
  const close = () => setRequest(undefined);
  // A key per request, so a second request starts from a fresh field.
  const key =
    request.kind === 'create'
      ? `create:${request.roomId ?? ''}`
      : `${request.kind}:${request.folderId}`;
  if (request.kind === 'delete') {
    return <RoomFolderDeleteDialog key={key} request={request} onDone={close} />;
  }
  return <RoomFolderNameDialog key={key} request={request} onDone={close} />;
}
