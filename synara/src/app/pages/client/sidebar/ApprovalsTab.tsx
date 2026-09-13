import React from 'react';
import { useMatch, useNavigate } from 'react-router-dom';
import { Badge, Icon, Icons, Text } from 'folds';
import {
  SidebarAvatar,
  SidebarItem,
  SidebarItemBadge,
  SidebarItemTooltip,
} from '../../../components/sidebar';
import { UnreadBadge } from '../../../components/unread-badge';
import { useApprovalInbox } from '../../../features/approvals/ApprovalInboxProvider';
import { APPROVALS_PATH } from '../../paths';

export function ApprovalsTab() {
  const navigate = useNavigate();
  const selected = Boolean(useMatch({ path: APPROVALS_PATH, end: false }));
  const { pendingCount, loading, incomplete, error } = useApprovalInbox();
  const uncertain = loading || incomplete || Boolean(error);
  const coverage = error
    ? 'unavailable'
    : loading
    ? 'checking rooms'
    : incomplete
    ? 'partial coverage'
    : '';
  const label = `Approvals${
    pendingCount ? ` · ${pendingCount}${uncertain ? '+' : ''} pending` : ''
  }${coverage ? ` · ${coverage}` : ''}`;
  return (
    <SidebarItem active={selected}>
      <SidebarItemTooltip tooltip={label}>
        {(triggerRef) => (
          <SidebarAvatar
            as="button"
            ref={triggerRef}
            outlined
            aria-label={label}
            aria-current={selected ? 'page' : undefined}
            onClick={() => navigate(APPROVALS_PATH)}
          >
            <Icon src={Icons.Shield} filled={selected} />
          </SidebarAvatar>
        )}
      </SidebarItemTooltip>
      {(pendingCount > 0 || uncertain) && (
        <SidebarItemBadge hasCount={pendingCount > 0 || (!loading && uncertain)}>
          <span aria-hidden="true" title={coverage || undefined}>
            {uncertain && (pendingCount > 0 || !loading) ? (
              <Badge
                variant={pendingCount > 0 ? 'Success' : 'Warning'}
                size="400"
                fill="Solid"
                radii="Pill"
                outlined={false}
              >
                <Text as="span" size="L400">
                  {pendingCount > 0 ? `${pendingCount}+` : '!'}
                </Text>
              </Badge>
            ) : (
              <UnreadBadge count={pendingCount} highlight={pendingCount > 0} />
            )}
          </span>
        </SidebarItemBadge>
      )}
    </SidebarItem>
  );
}
