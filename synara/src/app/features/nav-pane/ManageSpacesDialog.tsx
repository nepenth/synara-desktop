import React, { FormEvent, useId, useState } from 'react';
import {
  Avatar,
  Box,
  Button,
  Checkbox,
  Header,
  Icon,
  IconButton,
  Icons,
  Input,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Text,
  color,
  config,
  toRem,
} from 'folds';
import FocusTrap from '../../components/FocusTrap';
import { Dialog } from '../../components/modal-dialog';
import { RoomAvatar } from '../../components/room-avatar';
import { getNativeRoom } from '../../native/nativeSession';
import { getRoomAvatarUrl } from '../../utils/room';
import { nameInitials, randomStr } from '../../utils/common';
import { stopPropagation } from '../../utils/keyboard';
import type { ISidebarFolder, SidebarItems } from '../../hooks/useSidebarItems';
import type { SpaceOrder } from './useSpaceOrder';
import {
  addSpacesToFolder,
  folderLabel,
  groupSpacesIntoFolder,
  moveSpaceItem,
  removeSpaceFromFolder,
  renameFolder,
  ungroupFolder,
} from './spaceOrder';
import * as css from './NavPane.css';

const nameOf = (spaceId: string): string => getNativeRoom(spaceId)?.name ?? spaceId;
const NEW_FOLDER = '';

function SpaceAvatar({ spaceId }: { spaceId: string }) {
  const space = getNativeRoom(spaceId);
  if (!space) return null;
  return (
    <Avatar as="span" size="200" radii="300" className={css.SwitcherAvatar}>
      <RoomAvatar
        roomId={spaceId}
        src={getRoomAvatarUrl(space, 96) ?? undefined}
        alt=""
        renderFallback={() => (
          <Text as="span" size="H6">
            {nameInitials(space.name, 2)}
          </Text>
        )}
      />
    </Avatar>
  );
}

type MoveButtonsProps = {
  name: string;
  first: boolean;
  last: boolean;
  onMove: (direction: -1 | 1) => void;
};

function MoveButtons({ name, first, last, onMove }: MoveButtonsProps) {
  return (
    <>
      <IconButton
        size="300"
        radii="300"
        variant="Surface"
        fill="None"
        aria-label={`Move ${name} up`}
        disabled={first}
        onClick={() => onMove(-1)}
      >
        <Icon size="100" src={Icons.ArrowTop} />
      </IconButton>
      <IconButton
        size="300"
        radii="300"
        variant="Surface"
        fill="None"
        aria-label={`Move ${name} down`}
        disabled={last}
        onClick={() => onMove(1)}
      >
        <Icon size="100" src={Icons.ArrowBottom} />
      </IconButton>
    </>
  );
}

function FolderNameField({
  folder,
  onRename,
}: {
  folder: ISidebarFolder;
  onRename: (name: string) => void;
}) {
  const [value, setValue] = useState(folder.name ?? '');
  const commit = () => {
    if ((folder.name ?? '') !== value.trim()) onRename(value);
  };
  return (
    <Input
      size="300"
      radii="300"
      variant="Background"
      value={value}
      placeholder={folderLabel({ ...folder, name: undefined }, nameOf)}
      aria-label="Folder name"
      onChange={(evt) => setValue(evt.currentTarget.value)}
      onBlur={commit}
      onKeyDown={(evt) => {
        if (evt.key === 'Enter') {
          evt.preventDefault();
          commit();
        }
      }}
      style={{ minWidth: 0, width: '100%' }}
    />
  );
}

type ManageSpacesDialogProps = {
  order: SpaceOrder;
  requestClose: () => void;
};

/**
 * Manage spaces (D1): the keyboard and pointer home for what the rail did by
 * dragging. Reorder with the arrow buttons (or drag in the switcher), group
 * spaces into a folder, add them to one, rename, take a space out, ungroup.
 * Every change saves to `in.synara.spaces` at once and rolls back on failure.
 */
export function ManageSpacesDialog({ order, requestClose }: ManageSpacesDialogProps) {
  const { items, save, saveError, clearSaveError } = order;
  const [selected, setSelected] = useState<string[]>([]);
  const [targetFolder, setTargetFolder] = useState(NEW_FOLDER);
  const [newFolderName, setNewFolderName] = useState('');
  const folderSelectId = useId();
  const folderNameId = useId();

  const folders = items.filter((item): item is ISidebarFolder => typeof item === 'object');
  const selectedTopLevel = selected.filter((id) => items.includes(id));

  const apply = (next: SidebarItems, failure: string) => {
    if (next === items) return;
    save(items, next, failure);
  };

  const toggleSelected = (spaceId: string) =>
    setSelected((current) =>
      current.includes(spaceId) ? current.filter((id) => id !== spaceId) : [...current, spaceId]
    );

  const handleGroup = (evt: FormEvent) => {
    evt.preventDefault();
    if (selectedTopLevel.length === 0) return;
    if (targetFolder === NEW_FOLDER) {
      apply(
        groupSpacesIntoFolder(items, selectedTopLevel, randomStr(), newFolderName),
        "Couldn't create the folder. Try again."
      );
      setNewFolderName('');
    } else {
      apply(
        addSpacesToFolder(items, selectedTopLevel, targetFolder),
        "Couldn't move the spaces into the folder. Try again."
      );
    }
    setSelected([]);
  };

  const orderFailure = "Couldn't save the new order of your spaces. Try again.";

  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            clickOutsideDeactivates: true,
            onDeactivate: requestClose,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Dialog style={{ width: toRem(440), maxWidth: 'calc(100vw - 32px)' }}>
            <Box grow="Yes" direction="Column" style={{ maxHeight: '80vh' }}>
              <Header
                size="500"
                style={{ padding: `0 ${config.space.S200} 0 ${config.space.S400}` }}
              >
                <Box grow="Yes">
                  <Text as="h2" size="H4" truncate>
                    Manage spaces
                  </Text>
                </Box>
                <Box shrink="No">
                  <IconButton size="300" radii="300" onClick={requestClose} aria-label="Close">
                    <Icon src={Icons.Cross} />
                  </IconButton>
                </Box>
              </Header>
              <Box
                direction="Column"
                gap="300"
                style={{ padding: config.space.S400, overflowY: 'auto', minHeight: 0 }}
              >
                <Text size="T300" style={{ color: 'var(--synara-content-secondary)' }}>
                  The order here is the order in the space switcher. You can also drag spaces there.
                </Text>
                {saveError && (
                  <div className={css.SwitcherError} role="alert" style={{ margin: 0 }}>
                    <Icon size="100" src={Icons.Warning} />
                    <span style={{ flexGrow: 1 }}>{saveError}</span>
                    <IconButton
                      size="300"
                      radii="300"
                      variant="Critical"
                      fill="None"
                      aria-label="Dismiss"
                      onClick={clearSaveError}
                    >
                      <Icon size="50" src={Icons.Cross} />
                    </IconButton>
                  </div>
                )}
                {items.length === 0 ? (
                  <Text size="T300">You have not joined any spaces yet.</Text>
                ) : (
                  <ul className={css.ManageList} aria-label="Spaces and folders">
                    {items.map((item, index) => {
                      const first = index === 0;
                      const last = index === items.length - 1;
                      if (typeof item === 'string') {
                        const name = nameOf(item);
                        return (
                          <li key={item} className={css.ManageRow}>
                            <Checkbox
                              size="300"
                              variant="Primary"
                              aria-label={`Select ${name}`}
                              checked={selected.includes(item)}
                              onClick={() => toggleSelected(item)}
                            />
                            <SpaceAvatar spaceId={item} />
                            <span className={css.SwitcherRowName}>{name}</span>
                            <MoveButtons
                              name={name}
                              first={first}
                              last={last}
                              onMove={(direction) =>
                                apply(moveSpaceItem(items, item, direction), orderFailure)
                              }
                            />
                          </li>
                        );
                      }
                      const label = folderLabel(item, nameOf);
                      return (
                        <li key={item.id}>
                          <div className={`${css.ManageRow} ${css.ManageFolder}`}>
                            <Icon size="100" src={Icons.Space} />
                            <Box grow="Yes" style={{ minWidth: 0 }}>
                              <FolderNameField
                                key={item.name ?? ''}
                                folder={item}
                                onRename={(name) =>
                                  apply(
                                    renameFolder(items, item.id, name),
                                    "Couldn't rename the folder. Try again."
                                  )
                                }
                              />
                            </Box>
                            <MoveButtons
                              name={label}
                              first={first}
                              last={last}
                              onMove={(direction) =>
                                apply(moveSpaceItem(items, item.id, direction), orderFailure)
                              }
                            />
                            <Button
                              size="300"
                              radii="300"
                              variant="Secondary"
                              fill="None"
                              onClick={() =>
                                apply(
                                  ungroupFolder(items, item.id),
                                  "Couldn't ungroup the folder. Try again."
                                )
                              }
                            >
                              <Text size="B300">Ungroup</Text>
                            </Button>
                          </div>
                          <ul className={css.ManageList} aria-label={`In ${label}`}>
                            {item.content.map((spaceId, childIndex) => {
                              const name = nameOf(spaceId);
                              return (
                                <li key={spaceId} className={css.ManageRow} data-depth="1">
                                  <SpaceAvatar spaceId={spaceId} />
                                  <span className={css.SwitcherRowName}>{name}</span>
                                  <MoveButtons
                                    name={name}
                                    first={childIndex === 0}
                                    last={childIndex === item.content.length - 1}
                                    onMove={(direction) =>
                                      apply(moveSpaceItem(items, spaceId, direction), orderFailure)
                                    }
                                  />
                                  <IconButton
                                    size="300"
                                    radii="300"
                                    variant="Surface"
                                    fill="None"
                                    aria-label={`Take ${name} out of ${label}`}
                                    onClick={() =>
                                      apply(removeSpaceFromFolder(items, spaceId), orderFailure)
                                    }
                                  >
                                    <Icon size="100" src={Icons.ArrowGoLeft} />
                                  </IconButton>
                                </li>
                              );
                            })}
                          </ul>
                        </li>
                      );
                    })}
                  </ul>
                )}
              </Box>
              <Box
                as="form"
                onSubmit={handleGroup}
                direction="Column"
                gap="200"
                shrink="No"
                style={{
                  padding: config.space.S400,
                  borderTop: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
                }}
              >
                <Text size="L400" as="p">
                  {selectedTopLevel.length === 0
                    ? 'Select spaces to put them in a folder.'
                    : `${selectedTopLevel.length} selected`}
                </Text>
                <Box gap="200" alignItems="Center" wrap="Wrap">
                  <label htmlFor={folderSelectId}>
                    <Text as="span" size="T300">
                      Folder
                    </Text>
                  </label>
                  <select
                    id={folderSelectId}
                    className={css.ManageSelect}
                    value={targetFolder}
                    onChange={(evt) => setTargetFolder(evt.currentTarget.value)}
                  >
                    <option value={NEW_FOLDER}>New folder</option>
                    {folders.map((folder) => (
                      <option key={folder.id} value={folder.id}>
                        {folderLabel(folder, nameOf)}
                      </option>
                    ))}
                  </select>
                  {targetFolder === NEW_FOLDER && (
                    <Box grow="Yes" style={{ minWidth: toRem(120) }}>
                      <Input
                        id={folderNameId}
                        size="300"
                        radii="300"
                        variant="Background"
                        value={newFolderName}
                        placeholder="Folder name (optional)"
                        aria-label="New folder name"
                        style={{ width: '100%' }}
                        onChange={(evt) => setNewFolderName(evt.currentTarget.value)}
                      />
                    </Box>
                  )}
                  <Button
                    type="submit"
                    size="300"
                    radii="300"
                    variant="Primary"
                    disabled={selectedTopLevel.length === 0}
                  >
                    <Text size="B300">
                      {targetFolder === NEW_FOLDER ? 'Create folder' : 'Move to folder'}
                    </Text>
                  </Button>
                </Box>
              </Box>
            </Box>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}
