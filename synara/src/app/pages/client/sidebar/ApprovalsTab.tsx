import React from 'react';
import { useMatch } from 'react-router-dom';
import { Badge, Icon, Icons, Text } from 'folds';
import {
  SidebarAvatar,
  SidebarItem,
  SidebarItemBadge,
  SidebarItemTooltip,
} from '../../../components/sidebar';
import { UnreadBadge } from '../../../components/unread-badge';
import { useApprovalInboxSummary } from '../../../features/approvals/ApprovalInboxProvider';
import { APPROVALS_PATH } from '../../paths';
import { useNavigateToApprovals } from '../../../hooks/useNavigateToApprovals';

export function ApprovalsTab() {
  const navigateToApprovals = useNavigateToApprovals();
  const selected = Boolean(useMatch({ path: APPROVALS_PATH, end: false }));
  const { pendingCount, loading, incomplete, coverage, error } = useApprovalInboxSummary();
  const needsAttention = incomplete || Boolean(error);
  const coverageDescription = error
    ? 'unavailable'
    : incomplete
    ? 'partial coverage'
    : loading
    ? 'checking rooms'
    : coverage === 'latest_event'
    ? 'recent activity'
    : '';
  const label = `Approvals${
    pendingCount ? ` · ${pendingCount}${needsAttention ? '+' : ''} pending` : ''
  }${coverageDescription ? ` · ${coverageDescription}` : ''}`;
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
            onClick={navigateToApprovals}
          >
            <Icon src={Icons.Shield} filled={selected} />
          </SidebarAvatar>
        )}
      </SidebarItemTooltip>
      {(pendingCount > 0 || needsAttention) && (
        <SidebarItemBadge hasCount>
          <span aria-hidden="true" title={coverageDescription || undefined}>
            {needsAttention ? (
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
