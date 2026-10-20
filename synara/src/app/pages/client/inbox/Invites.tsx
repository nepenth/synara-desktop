import React, { useCallback, useMemo, useRef, useState } from 'react';
import {
  Avatar,
  Badge,
  Box,
  Button,
  Chip,
  Icon,
  IconButton,
  Icons,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Scroll,
  Spinner,
  Text,
  color,
  config,
} from 'folds';
import FocusTrap from '../../../components/FocusTrap';
import { useNavigate } from 'react-router-dom';
import {
  Page,
  PageContent,
  PageContentCenter,
  PageHeader,
  PageHero,
  PageHeroEmpty,
  PageHeroSection,
} from '../../../components/page';
import {
  NativeInvite,
  useNativeInviteCommand,
  useNativeInvites,
} from '../../../state/room-list/inviteList';
import { SequenceCard } from '../../../components/sequence-card';
import { nameInitials } from '../../../utils/common';
import { RoomAvatar } from '../../../components/room-avatar';
import { Time } from '../../../components/message';
import { useElementSizeObserver } from '../../../hooks/useElementSizeObserver';
import { onEnterOrSpace, stopPropagation } from '../../../utils/keyboard';
import { RoomTopicViewer } from '../../../components/room-topic-viewer';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { ScreenSize, useScreenSizeContext } from '../../../hooks/useScreenSize';
import { BackRouteHandler } from '../../../components/BackRouteHandler';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { convertDesktopFileSrc } from '../../../utils/desktop';
import { getDirectRoomPath, getHomeRoomPath, getSpacePath } from '../../pathUtils';
import {
  BulkActionError,
  bulkFailureMessage,
  runBulkAction,
  type BulkFailureCopy,
} from '../../../utils/bulkActions';

const COMPACT_CARD_WIDTH = 548;

/**
 * Run an invite command for each item in order. One failure does not stop the
 * rest; the thrown error says how many succeeded and which ones failed.
 */
const runInviteBulkAction = async <T,>(
  items: T[],
  label: (item: T) => string,
  run: (item: T) => Promise<unknown>,
  copy: BulkFailureCopy
): Promise<void> => {
  const failures = await runBulkAction(items, run);
  if (failures.length > 0) {
    throw new BulkActionError(
      bulkFailureMessage(
        items.length,
        failures.map(({ item }) => label(item)),
        copy
      ),
      failures
    );
  }
};

const inviteLabel = (invite: NativeInvite): string => invite.roomName || invite.roomId;

const DECLINE_COPY: BulkFailureCopy = { done: 'Declined', verb: 'decline', noun: 'invites' };

function BulkInviteError({ error }: { error: unknown }) {
  return (
    <Text size="T200" role="alert" style={{ color: color.Critical.Main }}>
      {error instanceof BulkActionError ? error.message : "Couldn't finish this action. Try again."}
    </Text>
  );
}

type NavigateHandler = (invite: NativeInvite) => void;

type InviteCardProps = {
  invite: NativeInvite;
  compact?: boolean;
  hour24Clock: boolean;
  dateFormatString: string;
  onNavigate: NavigateHandler;
  hideAvatar: boolean;
};
function InviteCard({
  invite,
  compact,
  hour24Clock,
  dateFormatString,
  onNavigate,
  hideAvatar,
}: InviteCardProps) {
  const nativeInviteCommand = useNativeInviteCommand();
  const avatarUrl = invite.avatarHandleId
    ? convertDesktopFileSrc(invite.avatarHandleId, 'synara-media')
    : undefined;

  const [viewTopic, setViewTopic] = useState(false);
  const closeTopic = () => setViewTopic(false);
  const openTopic = () => setViewTopic(true);

  const [joinState, join] = useAsyncCallback<void, Error, []>(
    useCallback(async () => {
      await nativeInviteCommand('matrix_invites_accept', invite.roomId);
      onNavigate(invite);
    }, [invite, nativeInviteCommand, onNavigate])
  );
  const [leaveState, leave] = useAsyncCallback<void, Error, []>(
    useCallback(
      () => nativeInviteCommand('matrix_invites_decline', invite.roomId).then(() => undefined),
      [invite.roomId, nativeInviteCommand]
    )
  );

  const joining =
    joinState.status === AsyncStatus.Loading || joinState.status === AsyncStatus.Success;
  const leaving =
    leaveState.status === AsyncStatus.Loading || leaveState.status === AsyncStatus.Success;

  return (
    <SequenceCard
      variant="SurfaceVariant"
      direction="Column"
      gap="300"
      style={{ padding: config.space.S400 }}
    >
      {(invite.isEncrypted || invite.isDirect || invite.isSpace) && (
        <Box gap="200" alignItems="Center">
          {invite.isEncrypted && (
            <Box shrink="No" alignItems="Center" justifyContent="Center">
              <Badge variant="Success" fill="Solid" size="400" radii="300">
                <Text size="L400">Encrypted</Text>
              </Badge>
            </Box>
          )}
          {invite.isDirect && (
            <Box shrink="No" alignItems="Center" justifyContent="Center">
              <Badge variant="Primary" fill="Solid" size="400" radii="300">
                <Text size="L400">Direct Message</Text>
              </Badge>
            </Box>
          )}
          {invite.isSpace && (
            <Box shrink="No" alignItems="Center" justifyContent="Center">
              <Badge variant="Secondary" fill="Soft" size="400" radii="300">
                <Text size="L400">Space</Text>
              </Badge>
            </Box>
          )}
        </Box>
      )}
      <Box gap="300">
        <Avatar size="300">
          <RoomAvatar
            roomId={invite.roomId}
            src={hideAvatar ? undefined : avatarUrl}
            alt={invite.roomName}
            renderFallback={() => (
              <Text as="span" size="H6">
                {nameInitials(hideAvatar && avatarUrl ? undefined : invite.roomName)}
              </Text>
            )}
          />
        </Avatar>
        <Box direction={compact ? 'Column' : 'Row'} grow="Yes" gap="200">
          <Box grow="Yes" direction="Column" gap="200">
            <Box direction="Column">
              <Text size="T300" truncate>
                <b>{invite.roomName}</b>
              </Text>
              {invite.roomTopic && (
                <Text
                  size="T200"
                  onClick={openTopic}
                  onKeyDown={onEnterOrSpace(openTopic)}
                  tabIndex={0}
                  truncate
                >
                  {invite.roomTopic}
                </Text>
              )}
              <Overlay open={viewTopic} backdrop={<OverlayBackdrop />}>
                <OverlayCenter>
                  <FocusTrap
                    focusTrapOptions={{
                      initialFocus: false,
                      clickOutsideDeactivates: true,
                      onDeactivate: closeTopic,
                      escapeDeactivates: stopPropagation,
                    }}
                  >
                    <RoomTopicViewer
                      name={invite.roomName}
                      topic={invite.roomTopic ?? ''}
                      requestClose={closeTopic}
                    />
                  </FocusTrap>
                </OverlayCenter>
              </Overlay>
            </Box>
            {joinState.status === AsyncStatus.Error && (
              <Text size="T200" style={{ color: color.Critical.Main }}>
                {joinState.error.message}
              </Text>
            )}
            {leaveState.status === AsyncStatus.Error && (
              <Text size="T200" style={{ color: color.Critical.Main }}>
                {leaveState.error.message}
              </Text>
            )}
          </Box>
          <Box gap="200" shrink="No" alignItems="Center">
            <Button
              onClick={leave}
              size="300"
              variant="Secondary"
              radii="300"
              fill="Soft"
              disabled={joining || leaving}
              before={leaving ? <Spinner variant="Secondary" size="100" /> : undefined}
            >
              <Text size="B300">Decline</Text>
            </Button>
            <Button
              onClick={join}
              size="300"
              variant="Success"
              fill="Soft"
              radii="300"
              outlined
              disabled={joining || leaving}
              before={joining ? <Spinner variant="Success" fill="Soft" size="100" /> : undefined}
            >
              <Text size="B300">Accept</Text>
            </Button>
          </Box>
        </Box>
      </Box>
      <Box direction="Column">
        <Box gap="200" alignItems="Baseline">
          <Box grow="Yes">
            <Text size="T200" priority="300">
              {/* Core sends an empty sender when the invite's own member event was unreadable. */}
              {invite.senderId ? (
                <>
                  From: <b>{invite.senderId}</b>
                </>
              ) : (
                'Sender unknown'
              )}
            </Text>
          </Box>
          {typeof invite.inviteTs === 'number' && invite.inviteTs !== 0 && (
            <Box shrink="No">
              <Time
                size="T200"
                ts={invite.inviteTs}
                hour24Clock={hour24Clock}
                dateFormatString={dateFormatString}
              />
            </Box>
          )}
        </Box>
        {invite.reason && (
          <Text size="T200" priority="300">
            Reason: {invite.reason}
          </Text>
        )}
      </Box>
    </SequenceCard>
  );
}

enum InviteFilter {
  Known,
  Unknown,
  Spam,
}
type InviteFiltersProps = {
  filter: InviteFilter;
  onFilter: (filter: InviteFilter) => void;
  knownInvites: NativeInvite[];
  unknownInvites: NativeInvite[];
  spamInvites: NativeInvite[];
};
function InviteFilters({
  filter,
  onFilter,
  knownInvites,
  unknownInvites,
  spamInvites,
}: InviteFiltersProps) {
  const isKnown = filter === InviteFilter.Known;
  const isUnknown = filter === InviteFilter.Unknown;
  const isSpam = filter === InviteFilter.Spam;

  return (
    <Box gap="200">
      <Chip
        variant={isKnown ? 'Success' : 'Surface'}
        aria-selected={isKnown}
        outlined={!isKnown}
        onClick={() => onFilter(InviteFilter.Known)}
        before={isKnown && <Icon size="100" src={Icons.Check} />}
        after={
          knownInvites.length > 0 && (
            <Badge variant={isKnown ? 'Success' : 'Secondary'} fill="Solid" radii="Pill">
              <Text size="L400">{knownInvites.length}</Text>
            </Badge>
          )
        }
      >
        <Text size="T200">Primary</Text>
      </Chip>
      <Chip
        variant={isUnknown ? 'Warning' : 'Surface'}
        aria-selected={isUnknown}
        outlined={!isUnknown}
        onClick={() => onFilter(InviteFilter.Unknown)}
        before={isUnknown && <Icon size="100" src={Icons.Check} />}
        after={
          unknownInvites.length > 0 && (
            <Badge variant={isUnknown ? 'Warning' : 'Secondary'} fill="Solid" radii="Pill">
              <Text size="L400">{unknownInvites.length}</Text>
            </Badge>
          )
        }
      >
        <Text size="T200">Public</Text>
      </Chip>
      <Chip
        variant={isSpam ? 'Critical' : 'Surface'}
        aria-selected={isSpam}
        outlined={!isSpam}
        onClick={() => onFilter(InviteFilter.Spam)}
        before={isSpam && <Icon size="100" src={Icons.Check} />}
        after={
          spamInvites.length > 0 && (
            <Badge variant={isSpam ? 'Critical' : 'Secondary'} fill="Solid" radii="Pill">
              <Text size="L400">{spamInvites.length}</Text>
            </Badge>
          )
        }
      >
        <Text size="T200">Spam</Text>
      </Chip>
    </Box>
  );
}

type KnownInvitesProps = {
  invites: NativeInvite[];
  handleNavigate: NavigateHandler;
  compact: boolean;
  hour24Clock: boolean;
  dateFormatString: string;
};
function KnownInvites({
  invites,
  handleNavigate,
  compact,
  hour24Clock,
  dateFormatString,
}: KnownInvitesProps) {
  return (
    <Box direction="Column" gap="200">
      <Text size="H4">Primary</Text>
      {invites.length > 0 ? (
        <Box direction="Column" gap="100">
          {invites.map((invite) => (
            <InviteCard
              key={invite.roomId}
              invite={invite}
              compact={compact}
              hour24Clock={hour24Clock}
              dateFormatString={dateFormatString}
              onNavigate={handleNavigate}
              hideAvatar={false}
            />
          ))}
        </Box>
      ) : (
        <PageHeroEmpty>
          <PageHeroSection>
            <PageHero
              icon={<Icon size="600" src={Icons.Mail} />}
              title="No Invites"
              subTitle="When someone you share a room with sends you an invite, it’ll show up here."
            />
          </PageHeroSection>
        </PageHeroEmpty>
      )}
    </Box>
  );
}

type UnknownInvitesProps = {
  invites: NativeInvite[];
  handleNavigate: NavigateHandler;
  compact: boolean;
  hour24Clock: boolean;
  dateFormatString: string;
};
function UnknownInvites({
  invites,
  handleNavigate,
  compact,
  hour24Clock,
  dateFormatString,
}: UnknownInvitesProps) {
  const nativeInviteCommand = useNativeInviteCommand();

  const [declineAllStatus, declineAll] = useAsyncCallback(
    useCallback(
      () =>
        runInviteBulkAction(
          invites,
          inviteLabel,
          (invite) => nativeInviteCommand('matrix_invites_decline', invite.roomId),
          DECLINE_COPY
        ),
      [invites, nativeInviteCommand]
    )
  );

  const declining = declineAllStatus.status === AsyncStatus.Loading;
  const handleDeclineAll = () => {
    // The failure is rendered from `declineAllStatus`.
    declineAll().catch(() => {});
  };

  return (
    <Box direction="Column" gap="200">
      <Box gap="200" justifyContent="SpaceBetween" alignItems="Center">
        <Text size="H4">Public</Text>
        <Box>
          {invites.length > 0 && (
            <Chip
              variant="SurfaceVariant"
              onClick={handleDeclineAll}
              before={declining && <Spinner size="50" variant="Secondary" fill="Soft" />}
              disabled={declining}
              radii="Pill"
            >
              <Text size="T200">Decline All</Text>
            </Chip>
          )}
        </Box>
      </Box>
      {declineAllStatus.status === AsyncStatus.Error && (
        <BulkInviteError error={declineAllStatus.error} />
      )}
      {invites.length > 0 ? (
        <Box direction="Column" gap="100">
          {invites.map((invite) => (
            <InviteCard
              key={invite.roomId}
              invite={invite}
              compact={compact}
              hour24Clock={hour24Clock}
              dateFormatString={dateFormatString}
              onNavigate={handleNavigate}
              hideAvatar
            />
          ))}
        </Box>
      ) : (
        <PageHeroEmpty>
          <PageHeroSection>
            <PageHero
              icon={<Icon size="600" src={Icons.Info} />}
              title="No Invites"
              subTitle="Invites from people outside your rooms will appear here."
            />
          </PageHeroSection>
        </PageHeroEmpty>
      )}
    </Box>
  );
}

type SpamInvitesProps = {
  invites: NativeInvite[];
  handleNavigate: NavigateHandler;
  compact: boolean;
  hour24Clock: boolean;
  dateFormatString: string;
};
function SpamInvites({
  invites,
  handleNavigate,
  compact,
  hour24Clock,
  dateFormatString,
}: SpamInvitesProps) {
  const nativeInviteCommand = useNativeInviteCommand();
  const [showInvites, setShowInvites] = useState(false);

  const [declineAllStatus, declineAll] = useAsyncCallback(
    useCallback(
      () =>
        runInviteBulkAction(
          invites,
          inviteLabel,
          (invite) => nativeInviteCommand('matrix_invites_decline', invite.roomId),
          DECLINE_COPY
        ),
      [invites, nativeInviteCommand]
    )
  );

  // Reported invites stay listed, so a retry must skip the ones already sent.
  const reportedRoomIds = useRef(new Set<string>());
  const [reportAllStatus, reportAll] = useAsyncCallback(
    useCallback(
      () =>
        runInviteBulkAction(
          invites.filter((invite) => !reportedRoomIds.current.has(invite.roomId)),
          inviteLabel,
          async (invite) => {
            await nativeInviteCommand('matrix_invites_report_spam', invite.roomId);
            reportedRoomIds.current.add(invite.roomId);
          },
          { done: 'Reported', verb: 'report', noun: 'invites' }
        ),
      [invites, nativeInviteCommand]
    )
  );

  const [blockAllStatus, blockAll] = useAsyncCallback(
    useCallback(async () => {
      const firstInviteForSender = new Map<string, NativeInvite>();
      invites.forEach((invite) => {
        // An empty sender (unreadable invite) cannot be blocked; skip it.
        if (
          invite.senderId &&
          !invite.senderIgnored &&
          !firstInviteForSender.has(invite.senderId)
        ) {
          firstInviteForSender.set(invite.senderId, invite);
        }
      });
      // `Account::ignore_user` is idempotent; one request per sender keeps
      // this native operation equivalent to the former bulk account-data write.
      await runInviteBulkAction(
        Array.from(firstInviteForSender.values()),
        (invite) => invite.senderId,
        (invite) => nativeInviteCommand('matrix_invites_block_sender', invite.roomId),
        { done: 'Blocked', verb: 'block', noun: 'senders' }
      );
    }, [invites, nativeInviteCommand])
  );

  const declining = declineAllStatus.status === AsyncStatus.Loading;
  const reporting = reportAllStatus.status === AsyncStatus.Loading;
  const blocking = blockAllStatus.status === AsyncStatus.Loading;
  const loading = blocking || reporting || declining;
  // Each failure is rendered from its status below.
  const handleDeclineAll = () => {
    declineAll().catch(() => {});
  };
  const handleReportAll = () => {
    reportAll().catch(() => {});
  };
  const handleBlockAll = () => {
    blockAll().catch(() => {});
  };

  return (
    <Box direction="Column" gap="200">
      <Text size="H4">Spam</Text>
      {invites.length > 0 ? (
        <Box direction="Column" gap="100">
          <SequenceCard
            variant="SurfaceVariant"
            direction="Column"
            gap="300"
            style={{ padding: `${config.space.S400} ${config.space.S400} 0` }}
          >
            <PageHeroSection>
              <PageHero
                icon={<Icon size="600" src={Icons.Warning} />}
                title={`${invites.length} Spam Invites`}
                subTitle="Some of the following invites may contain harmful content or have been sent by banned users."
              >
                <Box direction="Row" gap="200" justifyContent="Center" wrap="Wrap">
                  <Button
                    size="300"
                    variant="Critical"
                    fill="Solid"
                    radii="300"
                    onClick={handleDeclineAll}
                    before={declining && <Spinner size="100" variant="Critical" fill="Solid" />}
                    disabled={loading}
                  >
                    <Text size="B300" truncate>
                      Decline All
                    </Text>
                  </Button>
                  {reportAllStatus.status !== AsyncStatus.Success && (
                    <Button
                      size="300"
                      variant="Secondary"
                      fill="Solid"
                      radii="300"
                      onClick={handleReportAll}
                      before={reporting && <Spinner size="100" variant="Secondary" fill="Solid" />}
                      disabled={loading}
                    >
                      <Text size="B300" truncate>
                        Report All
                      </Text>
                    </Button>
                  )}
                  {invites.some((invite) => invite.senderId && !invite.senderIgnored) && (
                    <Button
                      size="300"
                      variant="Secondary"
                      fill="Solid"
                      radii="300"
                      disabled={loading}
                      onClick={handleBlockAll}
                      before={blocking && <Spinner size="100" variant="Secondary" fill="Solid" />}
                    >
                      <Text size="B300" truncate>
                        Block All
                      </Text>
                    </Button>
                  )}
                </Box>
                {declineAllStatus.status === AsyncStatus.Error && (
                  <BulkInviteError error={declineAllStatus.error} />
                )}
                {reportAllStatus.status === AsyncStatus.Error && (
                  <BulkInviteError error={reportAllStatus.error} />
                )}
                {blockAllStatus.status === AsyncStatus.Error && (
                  <BulkInviteError error={blockAllStatus.error} />
                )}

                <span data-spacing-node />

                <Button
                  size="300"
                  variant="Secondary"
                  fill="Soft"
                  radii="Pill"
                  before={
                    <Icon size="100" src={showInvites ? Icons.ChevronTop : Icons.ChevronBottom} />
                  }
                  onClick={() => setShowInvites(!showInvites)}
                >
                  <Text size="B300">{showInvites ? 'Hide All' : 'View All'}</Text>
                </Button>
              </PageHero>
            </PageHeroSection>
          </SequenceCard>
          {showInvites &&
            invites.map((invite) => (
              <InviteCard
                key={invite.roomId}
                invite={invite}
                compact={compact}
                hour24Clock={hour24Clock}
                dateFormatString={dateFormatString}
                onNavigate={handleNavigate}
                hideAvatar
              />
            ))}
        </Box>
      ) : (
        <PageHeroEmpty>
          <PageHeroSection>
            <PageHero
              icon={<Icon size="600" src={Icons.Warning} />}
              title="No Spam Invites"
              subTitle="Invites detected as spam appear here."
            />
          </PageHeroSection>
        </PageHeroEmpty>
      )}
    </Box>
  );
}

export function Invites() {
  const { invites: invitesData } = useNativeInvites();
  const navigate = useNavigate();

  const [filter, setFilter] = useState(InviteFilter.Known);

  const [knownInvites, unknownInvites, spamInvites] = useMemo(() => {
    const known: NativeInvite[] = [];
    const unknown: NativeInvite[] = [];
    const spam: NativeInvite[] = [];
    invitesData.forEach((invite) => {
      if (invite.triage === 'spam') spam.push(invite);
      else if (invite.triage === 'public') unknown.push(invite);
      else known.push(invite);
    });

    return [known, unknown, spam];
  }, [invitesData]);

  const containerRef = useRef<HTMLDivElement>(null);
  const [compact, setCompact] = useState(document.body.clientWidth <= COMPACT_CARD_WIDTH);
  useElementSizeObserver(
    useCallback(() => containerRef.current, []),
    useCallback((width) => setCompact(width <= COMPACT_CARD_WIDTH), [])
  );
  const screenSize = useScreenSizeContext();

  const [hour24Clock] = useSetting(settingsAtom, 'hour24Clock');
  const [dateFormatString] = useSetting(settingsAtom, 'dateFormatString');

  const handleNavigate = (invite: NativeInvite) => {
    const roomTarget = invite.roomAlias ?? invite.roomId;
    if (invite.isSpace) {
      navigate(getSpacePath(roomTarget));
    } else if (invite.isDirect) {
      navigate(getDirectRoomPath(roomTarget));
    } else {
      navigate(getHomeRoomPath(roomTarget));
    }
  };

  return (
    <Page>
      <PageHeader balance>
        <Box grow="Yes" gap="200">
          <Box grow="Yes" basis="No">
            {screenSize === ScreenSize.Mobile && (
              <BackRouteHandler>
                {(onBack) => (
                  <IconButton onClick={onBack} aria-label="Back">
                    <Icon src={Icons.ArrowLeft} />
                  </IconButton>
                )}
              </BackRouteHandler>
            )}
          </Box>
          <Box alignItems="Center" gap="200">
            {screenSize !== ScreenSize.Mobile && <Icon size="400" src={Icons.Mail} />}
            <Text size="H3" truncate>
              Invites
            </Text>
          </Box>
          <Box grow="Yes" basis="No" />
        </Box>
      </PageHeader>
      <Box grow="Yes">
        <Scroll hideTrack visibility="Hover">
          <PageContent>
            <PageContentCenter>
              <Box ref={containerRef} direction="Column" gap="600">
                <Box direction="Column" gap="100">
                  <span data-spacing-node />
                  <Text size="L400">Filter</Text>
                  <InviteFilters
                    filter={filter}
                    onFilter={setFilter}
                    knownInvites={knownInvites}
                    unknownInvites={unknownInvites}
                    spamInvites={spamInvites}
                  />
                </Box>
                {filter === InviteFilter.Known && (
                  <KnownInvites
                    invites={knownInvites}
                    compact={compact}
                    hour24Clock={hour24Clock}
                    dateFormatString={dateFormatString}
                    handleNavigate={handleNavigate}
                  />
                )}

                {filter === InviteFilter.Unknown && (
                  <UnknownInvites
                    invites={unknownInvites}
                    compact={compact}
                    hour24Clock={hour24Clock}
                    dateFormatString={dateFormatString}
                    handleNavigate={handleNavigate}
                  />
                )}

                {filter === InviteFilter.Spam && (
                  <SpamInvites
                    invites={spamInvites}
                    compact={compact}
                    hour24Clock={hour24Clock}
                    dateFormatString={dateFormatString}
                    handleNavigate={handleNavigate}
                  />
                )}
              </Box>
            </PageContentCenter>
          </PageContent>
        </Scroll>
      </Box>
    </Page>
  );
}
