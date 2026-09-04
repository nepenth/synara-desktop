import React, {
  ChangeEventHandler,
  MouseEventHandler,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import {
  Avatar,
  Badge,
  Box,
  Chip,
  Header,
  Icon,
  IconButton,
  Icons,
  Input,
  MenuItem,
  PopOut,
  RectCords,
  Scroll,
  Spinner,
  Text,
  Tooltip,
  TooltipProvider,
  config,
} from 'folds';
import type { MatrixClientReading } from '../../utils/room';
import type { EventedRoomReading } from '../../utils/roomEvents';
import { useVirtualizer } from '@tanstack/react-virtual';
import classNames from 'classnames';

import * as css from './MembersDrawer.css';
import * as depthCss from '../../styles/Depth.css';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { UseStateProvider } from '../../components/UseStateProvider';
import {
  SearchItemStrGetter,
  UseAsyncSearchOptions,
  useAsyncSearch,
} from '../../hooks/useAsyncSearch';
import { useDebounce } from '../../hooks/useDebounce';
import { TypingIndicator } from '../../components/typing-indicator';
import { getMemberDisplayName, getMemberSearchStr } from '../../utils/room';
import { getMxIdLocalPart } from '../../utils/matrix';
import { useSetSetting, useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { millify } from '../../plugins/millify';
import { ScrollTopContainer } from '../../components/scroll-top-container';
import { UserAvatar } from '../../components/user-avatar';
import { useRoomTypingMember } from '../../hooks/useRoomTypingMembers';
import { useRoomMembers, type RoomMemberListItem } from '../../hooks/useRoomMembers';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { useMembershipFilter, useMembershipFilterMenu } from '../../hooks/useMemberFilter';
import { useMemberPowerSort, useMemberSort, useMemberSortMenu } from '../../hooks/useMemberSort';
import { useGetMemberPowerLevel, usePowerLevelsContext } from '../../hooks/usePowerLevels';
import { Membership } from '../../../types/matrix/room';
import { MembershipFilterMenu } from '../../components/MembershipFilterMenu';
import { MemberSortMenu } from '../../components/MemberSortMenu';
import { useOpenUserRoomProfile, useUserRoomProfileState } from '../../state/hooks/userRoomProfile';
import { useSpaceOptionally } from '../../hooks/useSpace';
import { ContainerColor } from '../../styles/ContainerColor.css';
import { useFlattenPowerTagMembers, useGetMemberPowerTag } from '../../hooks/useMemberPowerTag';
import { useRoomCreators } from '../../hooks/useRoomCreators';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import { getSessionBootstrapResult } from '../../state/sessionBootstrap';
import { isSynaraDesktop } from '../../utils/desktop';

type MemberDrawerHeaderProps = {
  joinedMemberCount?: number;
};
function MemberDrawerHeader({ joinedMemberCount }: MemberDrawerHeaderProps) {
  const setPeopleDrawer = useSetSetting(settingsAtom, 'isPeopleDrawer');
  const title =
    typeof joinedMemberCount === 'number' ? `${millify(joinedMemberCount)} Members` : 'Members';

  return (
    <Header
      className={css.MembersDrawerHeader}
      variant="Background"
      size="600"
      data-tauri-drag-region
    >
      <Box grow="Yes" alignItems="Center" gap="200">
        <Box grow="Yes" alignItems="Center" gap="200">
          <Text title={title} size="H5" truncate>
            {title}
          </Text>
        </Box>
        <Box shrink="No" alignItems="Center">
          <TooltipProvider
            position="Bottom"
            align="End"
            offset={4}
            tooltip={
              <Tooltip>
                <Text>Close</Text>
              </Tooltip>
            }
          >
            {(triggerRef) => (
              <IconButton
                ref={triggerRef}
                className={depthCss.quietInteractiveSurface}
                variant="Background"
                onClick={() => setPeopleDrawer(false)}
              >
                <Icon src={Icons.Cross} />
              </IconButton>
            )}
          </TooltipProvider>
        </Box>
      </Box>
    </Header>
  );
}

type MemberItemProps = {
  mx: MatrixClientReading;
  useAuthentication: boolean;
  room: EventedRoomReading;
  member: RoomMemberListItem;
  onClick: MouseEventHandler<HTMLButtonElement>;
  pressed?: boolean;
  typing?: boolean;
};
function MemberItem({
  mx,
  useAuthentication,
  room,
  member,
  onClick,
  pressed,
  typing,
}: MemberItemProps) {
  const name =
    (!('getMxcAvatarUrl' in member)
      ? member.displayName
      : getMemberDisplayName(room, member.userId)) ??
    getMxIdLocalPart(member.userId) ??
    member.userId;
  const avatarMxcUrl = !('getMxcAvatarUrl' in member) ? member.avatarUrl : member.getMxcAvatarUrl();
  const avatarUrl = avatarMxcUrl
    ? resolveMatrixThumbnailUrl(mx, avatarMxcUrl, 100, { useAuthentication })
    : undefined;

  return (
    <MenuItem
      className={depthCss.quietInteractiveSurface}
      style={{ padding: `0 ${config.space.S200}` }}
      aria-pressed={pressed}
      data-user-id={member.userId}
      variant="Background"
      radii="400"
      onClick={onClick}
      before={
        <Avatar size="200">
          <UserAvatar
            userId={member.userId}
            src={avatarUrl ?? undefined}
            alt={name}
            renderFallback={() => <Icon size="50" src={Icons.User} filled />}
          />
        </Avatar>
      }
      after={
        typing && (
          <Badge size="300" variant="Secondary" fill="Soft" radii="Pill" outlined>
            <TypingIndicator size="300" />
          </Badge>
        )
      }
    >
      <Box grow="Yes">
        <Text size="T400" truncate>
          {name}
        </Text>
      </Box>
    </MenuItem>
  );
}

const SEARCH_OPTIONS: UseAsyncSearchOptions = {
  limit: 1000,
  matchOptions: {
    contain: true,
  },
};

const mxIdToName = (mxId: string) => getMxIdLocalPart(mxId) ?? mxId;
const getRoomMemberStr: SearchItemStrGetter<RoomMemberListItem> = (m, query) =>
  getMemberSearchStr(m, query, mxIdToName);
const EMPTY_ROOM_MEMBERS: RoomMemberListItem[] = [];

type MembersDrawerProps = {
  room: EventedRoomReading;
};
export function MembersDrawer({ room }: MembersDrawerProps) {
  const mx = useMatrixClient();
  const useAuthentication = useMediaAuthentication();
  const nativeSession = isSynaraDesktop() && getSessionBootstrapResult().source === 'native';
  const memberSnapshot = useRoomMembers(mx, room.roomId, nativeSession);
  const members = memberSnapshot ?? EMPTY_ROOM_MEMBERS;
  const joinedMemberCount = nativeSession
    ? memberSnapshot?.filter((member) => member.membership === Membership.Join).length
    : room.getJoinedMemberCount();
  const scrollRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const scrollTopAnchorRef = useRef<HTMLDivElement>(null);
  const powerLevels = usePowerLevelsContext();
  const creators = useRoomCreators(room);
  const getPowerTag = useGetMemberPowerTag(room, creators, powerLevels);
  const getPowerLevel = useGetMemberPowerLevel(powerLevels);

  const fetchingMembers = nativeSession
    ? memberSnapshot === null
    : members.length < (joinedMemberCount ?? 0);
  const membersUnavailable = nativeSession && memberSnapshot === undefined;
  const openUserRoomProfile = useOpenUserRoomProfile();
  const space = useSpaceOptionally();
  const openProfileUserId = useUserRoomProfileState()?.userId;

  const membershipFilterMenu = useMembershipFilterMenu();
  const sortFilterMenu = useMemberSortMenu();
  const [sortFilterIndex, setSortFilterIndex] = useSetting(settingsAtom, 'memberSortFilterIndex');
  const [membershipFilterIndex, setMembershipFilterIndex] = useState(0);

  const membershipFilter = useMembershipFilter(membershipFilterIndex, membershipFilterMenu);
  const memberSort = useMemberSort(sortFilterIndex, sortFilterMenu);
  const memberPowerSort = useMemberPowerSort(creators, getPowerLevel);

  const typingMembers = useRoomTypingMember(room.roomId);

  const filteredMembers = useMemo(
    () => members.filter(membershipFilter.filterFn).sort(memberSort.sortFn).sort(memberPowerSort),
    [members, membershipFilter, memberSort, memberPowerSort]
  );

  const [result, search, resetSearch] = useAsyncSearch(
    filteredMembers,
    getRoomMemberStr,
    SEARCH_OPTIONS
  );
  // Re-run an existing input query only after commit; `search` can synchronously
  // publish a result for small member lists and must not run during render.
  useEffect(() => {
    if (!result && searchInputRef.current?.value) search(searchInputRef.current.value);
  }, [result, search]);

  const processMembers = result ? result.items : filteredMembers;

  const PLTagOrRoomMember = useFlattenPowerTagMembers(processMembers, getPowerTag);

  const virtualizer = useVirtualizer({
    count: PLTagOrRoomMember.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 40,
    overscan: 10,
  });

  const handleSearchChange: ChangeEventHandler<HTMLInputElement> = useDebounce(
    useCallback(
      (evt) => {
        if (evt.target.value) search(evt.target.value);
        else resetSearch();
      },
      [search, resetSearch]
    ),
    { wait: 200 }
  );

  const handleMemberClick: MouseEventHandler<HTMLButtonElement> = (evt) => {
    const btn = evt.currentTarget as HTMLButtonElement;
    const userId = btn.getAttribute('data-user-id');
    if (!userId) return;
    openUserRoomProfile(room.roomId, space?.roomId, userId, btn.getBoundingClientRect(), 'Left');
  };

  return (
    <Box
      className={classNames(css.MembersDrawer, ContainerColor({ variant: 'Background' }))}
      shrink="No"
      direction="Column"
    >
      <MemberDrawerHeader joinedMemberCount={joinedMemberCount} />
      <Box className={css.MemberDrawerContentBase} grow="Yes">
        <Scroll ref={scrollRef} variant="Background" size="300" visibility="Hover" hideTrack>
          <Box className={css.MemberDrawerContent} direction="Column" gap="200">
            <Box ref={scrollTopAnchorRef} className={css.DrawerGroup} direction="Column" gap="200">
              <Box alignItems="Center" justifyContent="SpaceBetween" gap="200">
                <UseStateProvider initial={undefined}>
                  {(anchor: RectCords | undefined, setAnchor) => (
                    <PopOut
                      anchor={anchor}
                      position="Bottom"
                      align="Start"
                      offset={4}
                      content={
                        <MembershipFilterMenu
                          selected={membershipFilterIndex}
                          onSelect={setMembershipFilterIndex}
                          requestClose={() => setAnchor(undefined)}
                        />
                      }
                    >
                      <Chip
                        className={depthCss.quietInteractiveSurface}
                        onClick={
                          ((evt) =>
                            setAnchor(
                              evt.currentTarget.getBoundingClientRect()
                            )) as MouseEventHandler<HTMLButtonElement>
                        }
                        variant="Background"
                        size="400"
                        radii="300"
                        before={<Icon src={Icons.Filter} size="50" />}
                      >
                        <Text size="T200">{membershipFilter.name}</Text>
                      </Chip>
                    </PopOut>
                  )}
                </UseStateProvider>
                <UseStateProvider initial={undefined}>
                  {(anchor: RectCords | undefined, setAnchor) => (
                    <PopOut
                      anchor={anchor}
                      position="Bottom"
                      align="End"
                      offset={4}
                      content={
                        <MemberSortMenu
                          selected={sortFilterIndex}
                          onSelect={setSortFilterIndex}
                          requestClose={() => setAnchor(undefined)}
                        />
                      }
                    >
                      <Chip
                        className={depthCss.quietInteractiveSurface}
                        onClick={
                          ((evt) =>
                            setAnchor(
                              evt.currentTarget.getBoundingClientRect()
                            )) as MouseEventHandler<HTMLButtonElement>
                        }
                        variant="Background"
                        size="400"
                        radii="300"
                        after={<Icon src={Icons.Sort} size="50" />}
                      >
                        <Text size="T200">{memberSort.name}</Text>
                      </Chip>
                    </PopOut>
                  )}
                </UseStateProvider>
              </Box>
              <Box direction="Column" gap="100">
                <Input
                  ref={searchInputRef}
                  onChange={handleSearchChange}
                  style={{ paddingRight: config.space.S200 }}
                  placeholder="Type name..."
                  variant="Surface"
                  size="400"
                  radii="400"
                  before={<Icon size="50" src={Icons.Search} />}
                  after={
                    result && (
                      <Chip
                        className={depthCss.quietInteractiveSurface}
                        variant={result.items.length > 0 ? 'Success' : 'Critical'}
                        size="400"
                        radii="Pill"
                        aria-pressed
                        onClick={() => {
                          if (searchInputRef.current) {
                            searchInputRef.current.value = '';
                            searchInputRef.current.focus();
                          }
                          resetSearch();
                        }}
                        after={<Icon size="50" src={Icons.Cross} />}
                      >
                        <Text size="B300">{`${result.items.length || 'No'} ${
                          result.items.length === 1 ? 'Result' : 'Results'
                        }`}</Text>
                      </Chip>
                    )
                  }
                />
              </Box>
            </Box>

            <ScrollTopContainer scrollRef={scrollRef} anchorRef={scrollTopAnchorRef}>
              <IconButton
                className={depthCss.quietInteractiveSurface}
                onClick={() => virtualizer.scrollToOffset(0)}
                variant="Surface"
                radii="Pill"
                outlined
                size="300"
                aria-label="Scroll to Top"
              >
                <Icon src={Icons.ChevronTop} size="300" />
              </IconButton>
            </ScrollTopContainer>

            {!fetchingMembers && !result && processMembers.length === 0 && (
              <Text style={{ padding: config.space.S300 }} align="Center">
                {membersUnavailable
                  ? 'Members are temporarily unavailable.'
                  : `No "${membershipFilter.name}" Members`}
              </Text>
            )}

            <Box className={css.MembersGroup} direction="Column" gap="100">
              <div
                style={{
                  position: 'relative',
                  height: virtualizer.getTotalSize(),
                }}
              >
                {virtualizer.getVirtualItems().map((vItem) => {
                  const tagOrMember = PLTagOrRoomMember[vItem.index];
                  if (!('userId' in tagOrMember)) {
                    return (
                      <Text
                        style={{
                          transform: `translateY(${vItem.start}px)`,
                        }}
                        data-index={vItem.index}
                        ref={virtualizer.measureElement}
                        key={`${room.roomId}-${vItem.index}`}
                        className={classNames(css.MembersGroupLabel, css.DrawerVirtualItem)}
                        size="L400"
                      >
                        {tagOrMember.name}
                      </Text>
                    );
                  }

                  return (
                    <div
                      style={{
                        transform: `translateY(${vItem.start}px)`,
                      }}
                      className={css.DrawerVirtualItem}
                      data-index={vItem.index}
                      key={`${room.roomId}-${tagOrMember.userId}`}
                      ref={virtualizer.measureElement}
                    >
                      <MemberItem
                        mx={mx}
                        useAuthentication={useAuthentication}
                        room={room}
                        member={tagOrMember}
                        onClick={handleMemberClick}
                        pressed={openProfileUserId === tagOrMember.userId}
                        typing={typingMembers.some(
                          (receipt) => receipt.userId === tagOrMember.userId
                        )}
                      />
                    </div>
                  );
                })}
              </div>
            </Box>

            {fetchingMembers && (
              <Box justifyContent="Center">
                <Spinner />
              </Box>
            )}
          </Box>
        </Scroll>
      </Box>
    </Box>
  );
}
