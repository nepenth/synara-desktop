import React, { useMemo, useState } from 'react';
import { useAtomValue } from 'jotai';
import { Avatar, Icon, IconButton, Icons, IconSrc, Text } from 'folds';
import { PageRoot } from '../../components/page';
import { SettingsNav, SettingsNavGroup } from '../../components/settings-layout';
import { commonSettingsSearchIndex } from '../common-settings/settingsSearchIndex';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { useRoomAvatar, useRoomJoinRule, useRoomName } from '../../hooks/useRoomMeta';
import { mDirectAtom } from '../../state/mDirectList';
import { RoomAvatar, RoomIcon } from '../../components/room-avatar';
import { General } from './general';
import { Members } from '../common-settings/members';
import { EmojisStickers } from '../common-settings/emojis-stickers';
import { Permissions } from './permissions';
import { RoomSettingsPage } from '../../state/roomSettings';
import { useRoom } from '../../hooks/useRoom';
import { DeveloperTools } from '../common-settings/developer-tools';
import { normalizeRoomJoinRulePresentation } from '../matrix-dto/roomJoinRule';
import * as depthCss from '../../styles/Depth.css';

type RoomSettingsMenuItem = {
  page: RoomSettingsPage;
  name: string;
  icon: IconSrc;
  keywords?: string[];
};

const useRoomSettingsMenuItems = (): RoomSettingsMenuItem[] =>
  useMemo(
    () => [
      {
        page: RoomSettingsPage.GeneralPage,
        name: 'General',
        icon: Icons.Setting,
      },
      {
        page: RoomSettingsPage.MembersPage,
        name: 'Members',
        icon: Icons.User,
      },
      {
        page: RoomSettingsPage.PermissionsPage,
        name: 'Permissions',
        icon: Icons.Lock,
      },
      {
        page: RoomSettingsPage.EmojisStickersPage,
        name: 'Custom Emoji',
        icon: Icons.Smile,
      },
      {
        page: RoomSettingsPage.DeveloperToolsPage,
        name: 'Developer Tools',
        icon: Icons.Terminal,
      },
    ],
    []
  );

type RoomSettingsProps = {
  initialPage?: RoomSettingsPage;
  requestClose: () => void;
};
export function RoomSettings({ initialPage, requestClose }: RoomSettingsProps) {
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
  const [activePage, setActivePage] = useState<RoomSettingsPage | undefined>(() => {
    if (initialPage) return initialPage;
    return screenSize === ScreenSize.Mobile ? undefined : RoomSettingsPage.GeneralPage;
  });
  const menuItems = useRoomSettingsMenuItems();

  const navGroups = useMemo<SettingsNavGroup<RoomSettingsPage>[]>(() => {
    const byPage = new Map(menuItems.map((item) => [item.page, item]));
    const pick = (...pages: RoomSettingsPage[]) =>
      pages.flatMap((page) => {
        const item = byPage.get(page);
        return item
          ? [{ id: item.page, name: item.name, icon: item.icon, keywords: item.keywords }]
          : [];
      });
    return [
      {
        label: 'Room',
        items: pick(
          RoomSettingsPage.GeneralPage,
          RoomSettingsPage.MembersPage,
          RoomSettingsPage.PermissionsPage
        ),
      },
      { label: 'Content', items: pick(RoomSettingsPage.EmojisStickersPage) },
      { label: 'Advanced', items: pick(RoomSettingsPage.DeveloperToolsPage) },
    ];
  }, [menuItems]);

  const searchIndex = useMemo(
    () =>
      commonSettingsSearchIndex('room', {
        general: RoomSettingsPage.GeneralPage,
        permissions: RoomSettingsPage.PermissionsPage,
        emojis: RoomSettingsPage.EmojisStickersPage,
        developer: RoomSettingsPage.DeveloperToolsPage,
      }),
    []
  );

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
                        size="50"
                        roomType={room.getType()}
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
            searchIndex={searchIndex}
            active={activePage}
            onSelect={setActivePage}
          />
        )
      }
    >
      {activePage === RoomSettingsPage.GeneralPage && (
        <General requestClose={handlePageRequestClose} />
      )}
      {activePage === RoomSettingsPage.MembersPage && (
        <Members requestClose={handlePageRequestClose} />
      )}
      {activePage === RoomSettingsPage.PermissionsPage && (
        <Permissions requestClose={handlePageRequestClose} />
      )}
      {activePage === RoomSettingsPage.EmojisStickersPage && (
        <EmojisStickers requestClose={handlePageRequestClose} />
      )}
      {activePage === RoomSettingsPage.DeveloperToolsPage && (
        <DeveloperTools requestClose={handlePageRequestClose} />
      )}
    </PageRoot>
  );
}
