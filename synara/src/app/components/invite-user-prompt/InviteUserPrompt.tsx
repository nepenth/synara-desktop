import type { EventedRoomReading } from '../../utils/roomEvents';
import React, {
  ChangeEventHandler,
  FormEventHandler,
  KeyboardEventHandler,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import {
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Box,
  Header,
  config,
  Text,
  IconButton,
  Icon,
  Icons,
  Input,
  Button,
  Spinner,
  color,
  TextArea,
  Dialog,
  Menu,
  toRem,
  Scroll,
  MenuItem,
} from 'folds';

import { isKeyHotkey } from 'is-hotkey';
import FocusTrap from 'focus-trap-react';
import { stopPropagation } from '../../utils/keyboard';
import { useDirectUsers } from '../../hooks/useDirectUsers';
import { getMxIdLocalPart, getMxIdServer, isUserId } from '../../utils/matrix';
import { Membership } from '../../../types/matrix/room';
import { useAsyncSearch, UseAsyncSearchOptions } from '../../hooks/useAsyncSearch';
import { highlightText, makeHighlightRegex } from '../../plugins/react-custom-html-parser';
import { AsyncStatus, useAsyncCallback } from '../../hooks/useAsyncCallback';
import { BreakWord } from '../../styles/Text.css';
import { useAlive } from '../../hooks/useAlive';
import { inviteUserWithNativeOwner } from '../nativeRoomModerationOwner';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { isNativeMatrixSession } from '../../features/verification/nativeVerification';
import { isObject, optString, reqString } from '../../features/matrix-dto/parseUtil';

const SEARCH_OPTIONS: UseAsyncSearchOptions = {
  limit: 1000,
  matchOptions: {
    contain: true,
  },
};
const DIRECTORY_SEARCH_LIMIT = 10;
const DIRECTORY_SEARCH_DEBOUNCE_MS = 300;
const getUserIdString = (userId: string) => getMxIdLocalPart(userId) ?? userId;

const parseDirectoryUserIds = (value: unknown): string[] => {
  if (!isObject(value) || !Array.isArray(value.results)) return [];
  const ids: string[] = [];
  for (const raw of value.results) {
    if (!isObject(raw)) continue;
    const userId = reqString(raw, 'userId') ?? optString(raw, 'user_id');
    if (typeof userId === 'string' && isUserId(userId)) ids.push(userId);
  }
  return ids;
};

type InviteUserProps = {
  room: EventedRoomReading;
  requestClose: () => void;
};
export function InviteUserPrompt({ room, requestClose }: InviteUserProps) {
  const alive = useAlive();

  const inputRef = useRef<HTMLInputElement>(null);
  const directoryTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const directoryGen = useRef(0);
  const directUsers = useDirectUsers();
  const [validUserId, setValidUserId] = useState<string>();
  const [directoryUsers, setDirectoryUsers] = useState<string[]>([]);
  const nativeDirectory = isNativeMatrixSession() || isSynaraDesktop();

  const filteredUsers = useMemo(
    () =>
      directUsers.filter((userId) => {
        const membership = (room.getMember(userId) as { membership?: string } | null)?.membership;
        return membership !== Membership.Join;
      }),
    [directUsers, room]
  );
  const [result, search, resetSearch] = useAsyncSearch(
    filteredUsers,
    getUserIdString,
    SEARCH_OPTIONS
  );
  const cancelDirectorySearch = useCallback(() => {
    directoryGen.current += 1;
    if (directoryTimer.current) clearTimeout(directoryTimer.current);
    setDirectoryUsers([]);
  }, []);
  const scheduleDirectorySearch = useCallback(
    (term: string) => {
      if (!nativeDirectory) return;
      directoryGen.current += 1;
      const gen = directoryGen.current;
      if (directoryTimer.current) clearTimeout(directoryTimer.current);
      directoryTimer.current = setTimeout(() => {
        void (async () => {
          try {
            const response = await invokeDesktopWithAvailability<unknown>(
              'matrix_user_directory_search',
              { term, limit: DIRECTORY_SEARCH_LIMIT }
            );
            if (gen !== directoryGen.current || !alive()) return;
            if (!response.available) {
              setDirectoryUsers([]);
              return;
            }
            setDirectoryUsers(parseDirectoryUserIds(response.value));
          } catch {
            if (gen !== directoryGen.current || !alive()) return;
            setDirectoryUsers([]);
          }
        })();
      }, DIRECTORY_SEARCH_DEBOUNCE_MS);
    },
    [alive, nativeDirectory]
  );
  useEffect(
    () => () => {
      if (directoryTimer.current) clearTimeout(directoryTimer.current);
    },
    []
  );
  const searchItems = useMemo(() => {
    const local = result?.items ?? [];
    if (directoryUsers.length === 0) return local;
    const seen = new Set(local);
    const merged = [...local];
    for (const userId of directoryUsers) {
      if (seen.has(userId)) continue;
      const membership = (room.getMember(userId) as { membership?: string } | null)?.membership;
      if (membership === Membership.Join) continue;
      seen.add(userId);
      merged.push(userId);
    }
    return merged;
  }, [directoryUsers, result?.items, room]);
  const queryHighlighRegex = result?.query
    ? makeHighlightRegex(result.query.split(' '))
    : undefined;

  const [inviteState, invite] = useAsyncCallback<void, Error, [string, string | undefined]>(
    useCallback(
      async (userId, reason) => {
        await inviteUserWithNativeOwner(
          room.roomId,
          userId,
          reason,
          isSynaraDesktop(),
          invokeDesktopWithAvailability
        );
      },
      [room]
    )
  );

  const inviting = inviteState.status === AsyncStatus.Loading;

  const handleReset = () => {
    if (inputRef.current) inputRef.current.value = '';
    setValidUserId(undefined);
    resetSearch();
    cancelDirectorySearch();
  };

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    const target = evt.target as HTMLFormElement | undefined;

    if (inviting || !validUserId) return;

    const reasonInput = target?.reasonInput as HTMLTextAreaElement | undefined;
    const reason = reasonInput?.value.trim();

    invite(validUserId, reason || undefined).then(() => {
      if (alive()) {
        handleReset();
        if (reasonInput) reasonInput.value = '';
      }
    });
  };

  const handleSearchChange: ChangeEventHandler<HTMLInputElement> = (evt) => {
    const value = evt.currentTarget.value.trim();
    if (isUserId(value)) {
      setValidUserId(value);
      resetSearch();
      cancelDirectorySearch();
    } else {
      setValidUserId(undefined);
      const term = getMxIdLocalPart(value) ?? (value.startsWith('@') ? value.slice(1) : value);
      if (term) {
        search(term);
        scheduleDirectorySearch(term);
      } else {
        resetSearch();
        cancelDirectorySearch();
      }
    }
  };

  const handleUserId = (userId: string) => {
    if (inputRef.current) {
      inputRef.current.value = userId;
      setValidUserId(userId);
      resetSearch();
      cancelDirectorySearch();
      inputRef.current.focus();
    }
  };

  const handleKeyDown: KeyboardEventHandler<HTMLInputElement> = (evt) => {
    if (isKeyHotkey('escape', evt)) {
      resetSearch();
      cancelDirectorySearch();
      return;
    }
    if (isKeyHotkey('tab', evt) && searchItems.length > 0) {
      evt.preventDefault();
      const userId = searchItems[0];
      handleUserId(userId);
    }
  };

  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: () => inputRef.current,
            clickOutsideDeactivates: true,
            onDeactivate: requestClose,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Dialog>
            <Box grow="Yes" direction="Column">
              <Header
                size="500"
                style={{ padding: `0 ${config.space.S200} 0 ${config.space.S400}` }}
              >
                <Box grow="Yes">
                  <Text size="H4" truncate>
                    Invite
                  </Text>
                </Box>
                <Box shrink="No">
                  <IconButton size="300" radii="300" onClick={requestClose}>
                    <Icon src={Icons.Cross} />
                  </IconButton>
                </Box>
              </Header>
              <Box
                as="form"
                onSubmit={handleSubmit}
                shrink="No"
                style={{ padding: config.space.S400 }}
                direction="Column"
                gap="400"
              >
                <Box direction="Column" gap="100">
                  <Text size="L400">User ID</Text>
                  <div>
                    <Input
                      size="500"
                      ref={inputRef}
                      onChange={handleSearchChange}
                      onKeyDown={handleKeyDown}
                      placeholder="@username:server"
                      name="userIdInput"
                      variant="Background"
                      disabled={inviting}
                      autoComplete="off"
                      required
                    />
                    {searchItems.length > 0 && (
                      <FocusTrap
                        focusTrapOptions={{
                          initialFocus: false,
                          onDeactivate: () => {
                            resetSearch();
                            cancelDirectorySearch();
                          },
                          returnFocusOnDeactivate: false,
                          clickOutsideDeactivates: true,
                          allowOutsideClick: true,
                          isKeyForward: (evt: KeyboardEvent) => isKeyHotkey('arrowdown', evt),
                          isKeyBackward: (evt: KeyboardEvent) => isKeyHotkey('arrowup', evt),
                          escapeDeactivates: stopPropagation,
                        }}
                      >
                        <Box style={{ position: 'relative' }}>
                          <Menu style={{ position: 'absolute', top: 0, zIndex: 1, width: '100%' }}>
                            <Scroll size="300" style={{ maxHeight: toRem(100) }}>
                              <div style={{ padding: config.space.S100 }}>
                                {searchItems.map((userId) => {
                                  const username = `${getMxIdLocalPart(userId)}`;
                                  const userServer = getMxIdServer(userId);

                                  return (
                                    <MenuItem
                                      key={userId}
                                      type="button"
                                      size="300"
                                      variant="Surface"
                                      radii="300"
                                      onClick={() => handleUserId(userId)}
                                      after={
                                        <Text size="T200" truncate>
                                          {userServer}
                                        </Text>
                                      }
                                      disabled={inviting}
                                    >
                                      <Box grow="Yes">
                                        <Text size="T300" truncate>
                                          <b>
                                            {queryHighlighRegex
                                              ? highlightText(queryHighlighRegex, [
                                                  username ?? userId,
                                                ])
                                              : username}
                                          </b>
                                        </Text>
                                      </Box>
                                    </MenuItem>
                                  );
                                })}
                              </div>
                            </Scroll>
                          </Menu>
                        </Box>
                      </FocusTrap>
                    )}
                  </div>
                </Box>
                <Box direction="Column" gap="100">
                  <Text size="L400">Reason (Optional)</Text>
                  <TextArea
                    size="500"
                    name="reasonInput"
                    variant="Background"
                    rows={4}
                    resize="None"
                  />
                </Box>
                {inviteState.status === AsyncStatus.Error && (
                  <Text size="T200" style={{ color: color.Critical.Main }} className={BreakWord}>
                    <b>{inviteState.error.message}</b>
                  </Text>
                )}
                <Button
                  type="submit"
                  disabled={!validUserId || inviting}
                  before={inviting && <Spinner size="200" variant="Primary" fill="Solid" />}
                >
                  <Text size="B400">Invite</Text>
                </Button>
              </Box>
            </Box>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}
