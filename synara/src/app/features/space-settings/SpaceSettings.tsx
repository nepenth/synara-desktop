import React, { useMemo, useState } from 'react';
import { useAtomValue } from 'jotai';
import { Avatar, Icon, IconButton, Icons, IconSrc, Text } from 'folds';
import { PageRoot } from '../../components/page';
import { SettingsNav, SettingsNavGroup } from '../../components/settings-layout';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { useRoomAvatar, useRoomJoinRule, useRoomName } from '../../hooks/useRoomMeta';
import { mDirectAtom } from '../../state/mDirectList';
import { RoomAvatar, RoomIcon } from '../../components/room-avatar';
import { SpaceSettingsPage } from '../../state/spaceSettings';
import { useRoom } from '../../hooks/useRoom';
import { EmojisStickers } from '../common-settings/emojis-stickers';
import { Members } from '../common-settings/members';
import { DeveloperTools } from '../common-settings/developer-tools';
import { General } from './general';
import { Permissions } from './permissions';
import { normalizeRoomJoinRulePresentation } from '../matrix-dto/roomJoinRule';
import * as depthCss from '../../styles/Depth.css';

type SpaceSettingsMenuItem = {
  page: SpaceSettingsPage;
  name: string;
  icon: IconSrc;
  keywords?: string[];
};

const useSpaceSettingsMenuItems = (): SpaceSettingsMenuItem[] =>
  useMemo(
    () => [
      {
        page: SpaceSettingsPage.GeneralPage,
        name: 'General',
        icon: Icons.Setting,
      },
      {
        page: SpaceSettingsPage.MembersPage,
        name: 'Members',
        icon: Icons.User,
      },
      {
        page: SpaceSettingsPage.PermissionsPage,
        name: 'Permissions',
        icon: Icons.Lock,
      },
      {
        page: SpaceSettingsPage.EmojisStickersPage,
        name: 'Custom Emoji',
        icon: Icons.Smile,
      },
      {
        page: SpaceSettingsPage.DeveloperToolsPage,
        name: 'Developer Tools',
        icon: Icons.Terminal,
      },
    ],
    []
  );

type SpaceSettingsProps = {
  initialPage?: SpaceSettingsPage;
  requestClose: () => void;
};
export function SpaceSettings({ initialPage, requestClose }: SpaceSettingsProps) {
  const room = useRoom();
  const useAuthentication = useMediaAuthentication();
  const mDirects = useAtomValue(mDirectAtom);

  const roomAvatar = useRoomAvatar(room, mDirects.has(room.roomId));
  const roomName = useRoomName(room);
  const joinRuleContent = useRoomJoinRule(room);
  const rawJoinRule = joinRuleContent?.join_rule;

  const avatarUrl = roomAvatar
    ? resolveMatrixThumbnailUrl(roomAvatar, 96, { useAuthentication })
    : undefined;

  const screenSize = useScreenSizeContext();
  const [activePage, setActivePage] = useState<SpaceSettingsPage | undefined>(() => {
    if (initialPage) return initialPage;
    return screenSize === ScreenSize.Mobile ? undefined : SpaceSettingsPage.GeneralPage;
  });
  const menuItems = useSpaceSettingsMenuItems();

  const navGroups = useMemo<SettingsNavGroup<SpaceSettingsPage>[]>(() => {
    const byPage = new Map(menuItems.map((item) => [item.page, item]));
    const pick = (...pages: SpaceSettingsPage[]) =>
      pages.flatMap((page) => {
        const item = byPage.get(page);
        return item
          ? [{ id: item.page, name: item.name, icon: item.icon, keywords: item.keywords }]
          : [];
      });
    return [
      {
        label: 'Space',
        items: pick(
          SpaceSettingsPage.GeneralPage,
          SpaceSettingsPage.MembersPage,
          SpaceSettingsPage.PermissionsPage
        ),
      },
      { label: 'Content', items: pick(SpaceSettingsPage.EmojisStickersPage) },
      { label: 'Advanced', items: pick(SpaceSettingsPage.DeveloperToolsPage) },
    ];
  }, [menuItems]);

  const handlePageRequestClose = () => {
    if (screenSize === ScreenSize.Mobile) {
      setActivePage(undefined);
      return;
    }
    requestClose();
  };

  return (
    <PageRoot
      nav={
        screenSize === ScreenSize.Mobile && activePage !== undefined ? undefined : (
          <SettingsNav
            header={
              <>
                <Avatar size="200" radii="300">
                  <RoomAvatar
                    roomId={room.roomId}
                    src={avatarUrl}
                    alt={roomName}
                    renderFallback={() => (
                      <RoomIcon
                        roomType={room.getType()}
                        size="50"
                        joinRule={normalizeRoomJoinRulePresentation(rawJoinRule)}
                        filled
                      />
                    )}
                  />
                </Avatar>
                <Text size="H4" truncate>
                  {roomName}
                </Text>
              </>
            }
            headerAfter={
              screenSize === ScreenSize.Mobile ? (
                <IconButton
                  className={depthCss.quietInteractiveSurface}
                  onClick={requestClose}
                  variant="Surface"
                  fill="None"
                  aria-label="Close"
                >
                  <Icon src={Icons.Cross} />
                </IconButton>
              ) : undefined
            }
            groups={navGroups}
            active={activePage}
            onSelect={setActivePage}
          />
        )
      }
    >
      {activePage === SpaceSettingsPage.GeneralPage && (
        <General requestClose={handlePageRequestClose} />
      )}
      {activePage === SpaceSettingsPage.MembersPage && (
        <Members requestClose={handlePageRequestClose} />
      )}
      {activePage === SpaceSettingsPage.PermissionsPage && (
        <Permissions requestClose={handlePageRequestClose} />
      )}
      {activePage === SpaceSettingsPage.EmojisStickersPage && (
        <EmojisStickers requestClose={handlePageRequestClose} />
      )}
      {activePage === SpaceSettingsPage.DeveloperToolsPage && (
        <DeveloperTools requestClose={handlePageRequestClose} />
      )}
    </PageRoot>
  );
}
