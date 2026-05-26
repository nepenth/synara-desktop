import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Icon, Icons } from 'folds';
import { useAtomValue } from 'jotai';
import {
  SidebarAvatar,
  SidebarItem,
  SidebarItemBadge,
  SidebarItemTooltip,
} from '../../../components/sidebar';
import { allInvitesAtom } from '../../../state/room-list/inviteList';
import {
  getInboxInvitesPath,
  getInboxLaterPath,
  getInboxNotificationsPath,
  getInboxPath,
  joinPathComponent,
} from '../../pathUtils';
import { useInboxSelected } from '../../../hooks/router/useInbox';
import { UnreadBadge } from '../../../components/unread-badge';
import { ScreenSize, useScreenSizeContext } from '../../../hooks/useScreenSize';
import { useNavToActivePathAtom } from '../../../state/hooks/navToActivePath';
import { useAccountData } from '../../../hooks/useAccountData';
import { AccountDataEvent, SynaraLaterContent } from '../../../../types/matrix/accountData';
import { getLaterDueSummary } from '../../../utils/later';

export function InboxTab() {
  const screenSize = useScreenSizeContext();
  const navigate = useNavigate();
  const navToActivePath = useAtomValue(useNavToActivePathAtom());
  const inboxSelected = useInboxSelected();
  const allInvites = useAtomValue(allInvitesAtom);
  const inviteCount = allInvites.length;
  const laterContent = useAccountData(AccountDataEvent.SynaraLater)?.getContent() as
    | SynaraLaterContent
    | undefined;
  const laterSummary = getLaterDueSummary(laterContent);
  const laterCount = laterSummary.active;
  const badgeCount = inviteCount + laterCount;

  const handleInboxClick = () => {
    if (screenSize === ScreenSize.Mobile) {
      navigate(getInboxPath());
      return;
    }
    const activePath = navToActivePath.get('inbox');
    if (activePath) {
      navigate(joinPathComponent(activePath));
      return;
    }

    let path = getInboxNotificationsPath();
    if (laterCount > 0) {
      path = getInboxLaterPath();
    }
    if (inviteCount > 0) {
      path = getInboxInvitesPath();
    }
    navigate(path);
  };

  return (
    <SidebarItem active={inboxSelected}>
      <SidebarItemTooltip tooltip="Inbox">
        {(triggerRef) => (
          <SidebarAvatar as="button" ref={triggerRef} outlined onClick={handleInboxClick}>
            <Icon src={Icons.Inbox} filled={inboxSelected} />
          </SidebarAvatar>
        )}
      </SidebarItemTooltip>
      {badgeCount > 0 && (
        <SidebarItemBadge hasCount>
          <UnreadBadge highlight={inviteCount > 0 || laterSummary.overdue > 0} count={badgeCount} />
        </SidebarItemBadge>
      )}
    </SidebarItem>
  );
}
