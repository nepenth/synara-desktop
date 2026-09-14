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
import { compareRecentApprovals, historyDecisionLabel } from './approvalHistoryProjection';
import * as css from './Approvals.css';

type Filter = 'pending' | 'recent';

function remainingTime(expiresAt: number, now: number): string {
  const seconds = Math.max(0, Math.ceil((expiresAt - now) / 1000));
  return seconds >= 60 ? `${Math.ceil(seconds / 60)} min left` : `${seconds}s left`;
}

function formatApprovalInstant(ts: number): { iso: string; label: string } | undefined {
  if (!Number.isFinite(ts) || ts <= 0) return undefined;
  const date = new Date(ts);
  if (Number.isNaN(date.getTime())) return undefined;
  return { iso: date.toISOString(), label: date.toLocaleString() };
}

/** Presentation is shared with the browser fixture; reads and decisions remain native-owned. */
export function ApprovalsView({
  roomName,
  senderName,
  openMessage,
  roomAvailable,
}: {
  roomName: (id: string) => string;
  senderName: (item: ApprovalInboxItem) => string;
  openMessage: (item: ApprovalInboxItem) => void;
  roomAvailable?: (roomId: string) => boolean;
}) {
  const {
    sessionGeneration,
    items,
    recentItems,
    historyReady,
    pendingCount,
    loading,
    incomplete,
    coverage,
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
  const recentCount = recentItems.length;
  const visible = useMemo(() => {
    const query = search.trim().toLocaleLowerCase();
    const source =
      filter === 'pending' ? items.filter((item) => item.status === 'pending') : recentItems;
    return source
      .filter(
        (item) =>
          !query ||
          [
            roomName(item.roomId),
            senderName(item),
            item.sender,
            item.body,
            item.summary ?? '',
          ].some((value) => value.toLocaleLowerCase().includes(query))
      )
      .sort((a, b) =>
        filter === 'pending' ? a.expiresAt - b.expiresAt : compareRecentApprovals(a, b)
      );
  }, [items, recentItems, filter, search, roomName, senderName]);
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
            {filter === 'pending' && <Icon size="50" src={Icons.Check} aria-hidden />}
            Pending · {pendingCount}
            {(incomplete || error) && pendingCount > 0 ? '+' : ''}
          </button>
          <button
            type="button"
            className={css.Filter}
            aria-pressed={filter === 'recent'}
            onClick={() => setFilter('recent')}
          >
            {filter === 'recent' && <Icon size="50" src={Icons.Check} aria-hidden />}
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
      {(loading || (filter === 'recent' && historyReady === false)) && (
        <Box className={css.Notice} gap="200" alignItems="Center" role="status">
          <Spinner size="100" />
          <Text size="T300">
            {filter === 'recent' && historyReady === false && !loading
              ? 'Checking synced decisions…'
              : 'Checking recent requests across your rooms…'}
          </Text>
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
      {coverage === 'latest_event' && !loading && !incomplete && !error && (
        <Text size="T300" priority="300">
          Showing requests found in recent room activity.
        </Text>
      )}
      {filter === 'recent' && (
        <Text size="T300" priority="300">
          Decisions your account made, synced across devices. Inbox-only expired requests stay here
          until they age out of recent room activity.
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
              : loading || (filter === 'recent' && historyReady === false)
              ? 'Checking your rooms'
              : error || incomplete
              ? 'No requests loaded yet'
              : filter === 'pending'
              ? coverage === 'latest_event'
                ? 'No recent approvals found'
                : 'No pending approvals'
              : 'No recent requests'}
          </Text>
          <Text priority="300">
            {search
              ? 'Try a different agent, room, or command.'
              : filter === 'pending'
              ? 'New Hermes approval requests will appear here as they arrive.'
              : 'Decided approvals will appear here after you approve or deny a request.'}
          </Text>
        </div>
      ) : (
        <div role="list">
          {visible.map((item) => {
            const prompt = formatCoreAgentApprovalPrompt(item.body);
            const decidedInstant = formatApprovalInstant(item.decidedAt ?? Number.NaN);
            const receivedInstant = formatApprovalInstant(item.originServerTs);
            const summaryText = item.summary?.trim() || prompt.body || 'Command not recorded';
            const canOpenRoom = roomAvailable?.(item.roomId) !== false;
            return (
              <div role="listitem" key={`${sessionGeneration}:${approvalIdentity(item)}`}>
                <section
                  className={css.Card}
                  aria-label={`Approval from ${senderName(item)} in ${roomName(item.roomId)}`}
                >
                  <div className={css.CardHeader}>
                    <Box direction="Column" gap="100" style={{ minWidth: 0, flex: 1 }}>
                      <Text as="h2" size="H5" truncate>
                        {senderName(item)}
                      </Text>
                      <Text
                        size="T200"
                        priority="300"
                        style={{ overflowWrap: 'anywhere' }}
                        dir="auto"
                      >
                        {item.sender}
                      </Text>
                      <Text size="T300" truncate dir="auto">
                        {roomName(item.roomId)}
                      </Text>
                    </Box>
                    <Box gap="300" alignItems="Center" wrap="Wrap">
                      <Text className={css.Status} size="T200">
                        {item.status === 'pending'
                          ? remainingTime(item.expiresAt, now)
                          : historyDecisionLabel(item.decision, item.status)}
                      </Text>
                      <Button
                        size="300"
                        variant="Secondary"
                        fill="None"
                        onClick={() => openMessage(item)}
                        disabled={!canOpenRoom}
                        title={
                          canOpenRoom ? undefined : 'This room is no longer joined on this session.'
                        }
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
                      <pre
                        dir="auto"
                        style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere', margin: 0 }}
                      >
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
                      <Text
                        size="T300"
                        dir="auto"
                        style={{ overflowWrap: 'anywhere', fontFamily: 'monospace' }}
                      >
                        {summaryText}
                      </Text>
                      {prompt.commandPreview && !item.summary?.trim() && (
                        <Text
                          size="T200"
                          priority="300"
                          dir="auto"
                          style={{ overflowWrap: 'anywhere', fontFamily: 'monospace' }}
                        >
                          {prompt.commandPreview}
                        </Text>
                      )}
                      {decidedInstant ? (
                        <Text size="T200" priority="300">
                          Decided <time dateTime={decidedInstant.iso}>{decidedInstant.label}</time>
                        </Text>
                      ) : receivedInstant ? (
                        <Text size="T200" priority="300">
                          Received{' '}
                          <time dateTime={receivedInstant.iso}>{receivedInstant.label}</time>
                        </Text>
                      ) : null}
                      <Text size="T200" priority="300">
                        {item.status === 'expired'
                          ? 'This request can no longer be acted on. Open the conversation to ask Hermes for a fresh request.'
                          : canOpenRoom
                          ? 'Your account has already sent a decision for this request.'
                          : 'Your account has already sent a decision for this request. The room is no longer joined, so the original message cannot be opened.'}
                      </Text>
                    </Box>
                  )}
                </section>
              </div>
            );
          })}
        </div>
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
          roomAvailable={(id) => Boolean(mx.getRoom(id))}
        />
      </Scroll>
    </Page>
  );
}
