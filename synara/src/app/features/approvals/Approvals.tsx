import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Box, Button, Icon, Icons, Scroll, Spinner, Text, config } from 'folds';
import { Page, PageHeader } from '../../components/page';
import { AgentApprovalCard } from '../../components/agent-approval/AgentApprovalCard';
import { formatCoreAgentApprovalPrompt } from '../../utils/agentApprovals';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { BackRouteHandler } from '../../components/BackRouteHandler';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { useApprovalInbox } from './ApprovalInboxProvider';
import { approvalIdentity, type ApprovalInboxItem } from './nativeApprovalInbox';
import * as css from './Approvals.css';

type Filter = 'pending' | 'recent';

function remainingTime(expiresAt: number, now: number): string {
  const seconds = Math.max(0, Math.ceil((expiresAt - now) / 1000));
  return seconds >= 60 ? `${Math.ceil(seconds / 60)} min left` : `${seconds}s left`;
}

/** Presentation is shared with the browser fixture; reads and decisions remain native-owned. */
export function ApprovalsView({
  roomName,
  senderName,
  openMessage,
}: {
  roomName: (id: string) => string;
  senderName: (item: ApprovalInboxItem) => string;
  openMessage: (item: ApprovalInboxItem) => void;
}) {
  const {
    sessionGeneration,
    items,
    pendingCount,
    loading,
    incomplete,
    error,
    now,
    refresh,
    decided,
  } = useApprovalInbox();
  const mx = useMatrixClient();
  const [filter, setFilter] = useState<Filter>('pending');
  const [search, setSearch] = useState('');
  const [announcement, setAnnouncement] = useState('');
  useEffect(() => {
    setAnnouncement('');
    setSearch('');
    setFilter('pending');
  }, [mx, sessionGeneration]);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const recentCount = items.filter((item) => item.status !== 'pending').length;
  const visible = useMemo(() => {
    const query = search.trim().toLocaleLowerCase();
    return items
      .filter((item) =>
        filter === 'pending' ? item.status === 'pending' : item.status !== 'pending'
      )
      .filter(
        (item) =>
          !query ||
          [roomName(item.roomId), senderName(item), item.sender, item.body].some((value) =>
            value.toLocaleLowerCase().includes(query)
          )
      )
      .sort((a, b) =>
        filter === 'pending' ? a.expiresAt - b.expiresAt : b.originServerTs - a.originServerTs
      );
  }, [items, filter, search, roomName, senderName]);
  const handleDecision = (item: ApprovalInboxItem) => {
    if (!decided(item)) return;
    setAnnouncement(`Request in ${roomName(item.roomId)} resolved. Available in Recent.`);
    headingRef.current?.focus();
  };

  return (
    <div className={css.Content}>
      <Box direction="Column" gap="200">
        <Text as="h1" size="H3" tabIndex={-1} ref={headingRef}>
          Agent approvals
        </Text>
        <Text priority="300">
          Review Hermes requests across your rooms. Requests closest to expiry appear first.
        </Text>
      </Box>
      <div className={css.Toolbar}>
        <div className={css.Filters} role="group" aria-label="Approval request filter">
          <button
            type="button"
            className={css.Filter}
            aria-pressed={filter === 'pending'}
            onClick={() => setFilter('pending')}
          >
            Pending · {pendingCount}
            {(loading || incomplete || error) && pendingCount > 0 ? '+' : ''}
          </button>
          <button
            type="button"
            className={css.Filter}
            aria-pressed={filter === 'recent'}
            onClick={() => setFilter('recent')}
          >
            Recent · {recentCount}
          </button>
        </div>
        <input
          type="search"
          className={css.Search}
          aria-label="Search approval requests"
          placeholder="Search agent, room, or command"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <Button
          size="300"
          variant="Secondary"
          fill="None"
          onClick={refresh}
          aria-label="Refresh approval requests"
          before={<Icon src={Icons.Reload} size="100" />}
        >
          <Text size="B300">Refresh</Text>
        </Button>
      </div>
      {announcement && (
        <div role="status" aria-live="polite">
          <Text size="T300">{announcement}</Text>
        </div>
      )}
      {loading && (
        <Box className={css.Notice} gap="200" alignItems="Center" role="status">
          <Spinner size="100" />
          <Text size="T300">Checking recent requests across your rooms…</Text>
        </Box>
      )}
      {error && (
        <div className={css.Notice} role="alert">
          <Text size="T300">{error} Displayed requests may be out of date.</Text>
        </div>
      )}
      {incomplete && !loading && (
        <div className={css.Notice} role="status">
          <Text size="T300">
            Some rooms could not be fully checked. The pending count may be incomplete. Rooms are
            retried automatically.
          </Text>
        </div>
      )}
      {filter === 'recent' && (
        <Text size="T300" priority="300">
          Recently observed requests that were decided by your account or whose approval window has
          ended.
        </Text>
      )}
      {visible.length === 0 ? (
        <div className={css.Empty}>
          <Icon
            src={search ? Icons.Search : filter === 'pending' ? Icons.Shield : Icons.RecentClock}
            size="600"
          />
          <Text as="h2" size="H4">
            {search
              ? 'No matching requests'
              : loading
              ? 'Checking your rooms'
              : error || incomplete
              ? 'No requests loaded yet'
              : filter === 'pending'
              ? 'No pending approvals'
              : 'No recent requests'}
          </Text>
          <Text priority="300">
            {search
              ? 'Try a different agent, room, or command.'
              : filter === 'pending'
              ? 'New Hermes approval requests will appear here as they arrive.'
              : 'Decided and expired requests will appear here after they are observed.'}
          </Text>
        </div>
      ) : (
        visible.map((item) => {
          const prompt = formatCoreAgentApprovalPrompt(item.body);
          return (
            <section
              key={`${sessionGeneration}:${approvalIdentity(item)}`}
              className={css.Card}
              aria-label={`Approval from ${senderName(item)} in ${roomName(item.roomId)}`}
            >
              <div className={css.CardHeader}>
                <Box direction="Column" gap="100" style={{ minWidth: 0, flex: 1 }}>
                  <Text as="h2" size="H5" truncate>
                    {senderName(item)}
                  </Text>
                  <Text size="T200" priority="300" style={{ overflowWrap: 'anywhere' }}>
                    {item.sender}
                  </Text>
                  <Text size="T300" truncate>
                    {roomName(item.roomId)}
                  </Text>
                </Box>
                <Box gap="300" alignItems="Center" wrap="Wrap">
                  <Text
                    className={css.Status}
                    size="T200"
                    title={`Received ${new Date(item.originServerTs).toLocaleString()}`}
                  >
                    {item.status === 'pending'
                      ? remainingTime(item.expiresAt, now)
                      : item.status === 'decided'
                      ? 'Decided'
                      : 'Expired'}
                  </Text>
                  <Button
                    size="300"
                    variant="Secondary"
                    fill="None"
                    onClick={() => openMessage(item)}
                    before={<Icon src={Icons.ArrowGoRight} size="100" />}
                  >
                    <Text size="B300">Open message</Text>
                  </Button>
                </Box>
              </div>
              {item.status === 'pending' && item.bodyTruncated ? (
                <Box direction="Column" gap="200" style={{ padding: config.space.S400 }}>
                  <Text size="T300">
                    This is a shortened preview. Open the original message to review the full
                    request.
                  </Text>
                  <pre style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere', margin: 0 }}>
                    {item.body}
                  </pre>
                </Box>
              ) : item.status === 'pending' ? (
                <AgentApprovalCard
                  appearance="inbox"
                  prompt={prompt}
                  target={{
                    roomId: item.roomId,
                    eventId: item.eventId,
                    coreEligible: true,
                    canSendReaction: item.canSendReaction,
                  }}
                  onDecision={() => handleDecision(item)}
                />
              ) : (
                <Box direction="Column" gap="200" style={{ padding: config.space.S400 }}>
                  <Text size="T300">{prompt.body}</Text>
                  {prompt.commandPreview && (
                    <Text
                      size="T200"
                      priority="300"
                      style={{ overflowWrap: 'anywhere', fontFamily: 'monospace' }}
                    >
                      {prompt.commandPreview}
                    </Text>
                  )}
                  <Text size="T200" priority="300">
                    {item.status === 'expired'
                      ? 'This request can no longer be acted on. Open the conversation to ask Hermes for a fresh request.'
                      : 'Your account has already sent a decision for this request.'}
                  </Text>
                </Box>
              )}
            </section>
          );
        })
      )}
    </div>
  );
}

export function Approvals() {
  const mx = useMatrixClient();
  const { navigateRoom } = useRoomNavigate();
  const screenSize = useScreenSizeContext();
  return (
    <Page>
      <PageHeader>
        <Box alignItems="Center" gap="300" grow="Yes">
          {screenSize === ScreenSize.Mobile && (
            <BackRouteHandler>
              {(goBack) => (
                <Button
                  size="300"
                  variant="Secondary"
                  fill="None"
                  onClick={goBack}
                  aria-label="Back"
                >
                  <Icon src={Icons.ArrowLeft} size="100" />
                </Button>
              )}
            </BackRouteHandler>
          )}
          <Icon src={Icons.Shield} />
          <Text size="H4">Approvals</Text>
        </Box>
      </PageHeader>
      <Scroll variant="Background" size="300" hideTrack>
        <ApprovalsView
          roomName={(id) => mx.getRoom(id)?.name || id}
          senderName={(item) =>
            mx.getRoom(item.roomId)?.getMember(item.sender)?.name ||
            item.sender.split(':')[0].replace(/^@/, '')
          }
          openMessage={(item) => navigateRoom(item.roomId, item.eventId)}
        />
      </Scroll>
    </Page>
  );
}
