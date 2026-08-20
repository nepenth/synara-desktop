import React from 'react';
import { Avatar, Box, Icon, Icons, Text } from 'folds';
import { useAtomValue } from 'jotai';
import { NavCategory, NavItem, NavItemContent, NavLink } from '../../../components/nav';
import { getInboxInvitesPath, getInboxLaterPath, getInboxNotificationsPath } from '../../pathUtils';
import {
  useInboxInvitesSelected,
  useInboxLaterSelected,
  useInboxNotificationsSelected,
} from '../../../hooks/router/useInbox';
import { UnreadBadge } from '../../../components/unread-badge';
import { allInvitesAtom } from '../../../state/room-list/inviteList';
import { useNavToActivePathMapper } from '../../../hooks/useNavToActivePathMapper';
import { PageNav, PageNavContent, PageNavHeader } from '../../../components/page';
import { laterContentAtom } from '../../../state/laterList';
import { getLaterDueSummary } from '../../../utils/later';

function InvitesNavItem() {
  const invitesSelected = useInboxInvitesSelected();
  const allInvites = useAtomValue(allInvitesAtom);
  const inviteCount = allInvites.length;

  return (
    <NavItem
      variant="Surface"
      radii="400"
      highlight={inviteCount > 0}
      aria-selected={invitesSelected}
    >
      <NavLink to={getInboxInvitesPath()}>
        <NavItemContent>
          <Box as="span" grow="Yes" alignItems="Center" gap="200">
            <Avatar size="200" radii="400">
              <Icon src={Icons.Mail} size="100" filled={invitesSelected} />
            </Avatar>
            <Box as="span" grow="Yes">
              <Text as="span" size="Inherit" truncate>
                Invites
              </Text>
            </Box>
            {inviteCount > 0 && <UnreadBadge highlight count={inviteCount} />}
          </Box>
        </NavItemContent>
      </NavLink>
    </NavItem>
  );
}

function LaterNavItem() {
  const laterSelected = useInboxLaterSelected();
  const laterContent = useAtomValue(laterContentAtom);
  const laterSummary = getLaterDueSummary(laterContent);
  const laterCount = laterSummary.active;
  const dueLabel = [
    laterSummary.overdue > 0 ? `${laterSummary.overdue} overdue` : undefined,
    laterSummary.dueToday > 0 ? `${laterSummary.dueToday} today` : undefined,
  ]
    .filter(Boolean)
    .join(' / ');

  return (
    <NavItem variant="Surface" radii="400" highlight={laterCount > 0} aria-selected={laterSelected}>
      <NavLink to={getInboxLaterPath()}>
        <NavItemContent>
          <Box as="span" grow="Yes" alignItems="Center" gap="200">
            <Avatar size="200" radii="400">
              <Icon src={Icons.Bookmark} size="100" filled={laterSelected} />
            </Avatar>
            <Box as="span" grow="Yes">
              <Text as="span" size="Inherit" truncate>
                Later
              </Text>
            </Box>
            {dueLabel && (
              <Text as="span" size="L400" priority="300" truncate>
                {dueLabel}
              </Text>
            )}
            {laterCount > 0 && (
              <UnreadBadge count={laterCount} highlight={laterSummary.overdue > 0} />
            )}
          </Box>
        </NavItemContent>
      </NavLink>
    </NavItem>
  );
}

export function Inbox() {
  useNavToActivePathMapper('inbox');
  const notificationsSelected = useInboxNotificationsSelected();

  return (
    <PageNav>
      <PageNavHeader>
        <Box grow="Yes" gap="300">
          <Box grow="Yes">
            <Text size="H4" truncate>
              Inbox
            </Text>
          </Box>
        </Box>
      </PageNavHeader>

      <PageNavContent>
        <Box direction="Column" gap="300">
          <NavCategory>
            <NavItem variant="Surface" radii="400" aria-selected={notificationsSelected}>
              <NavLink to={getInboxNotificationsPath()}>
                <NavItemContent>
                  <Box as="span" grow="Yes" alignItems="Center" gap="200">
                    <Avatar size="200" radii="400">
                      <Icon src={Icons.MessageUnread} size="100" filled={notificationsSelected} />
                    </Avatar>
                    <Box as="span" grow="Yes">
                      <Text as="span" size="Inherit" truncate>
                        Notifications
                      </Text>
                    </Box>
                  </Box>
                </NavItemContent>
              </NavLink>
            </NavItem>
            <InvitesNavItem />
            <LaterNavItem />
          </NavCategory>
        </Box>
      </PageNavContent>
    </PageNav>
  );
}
