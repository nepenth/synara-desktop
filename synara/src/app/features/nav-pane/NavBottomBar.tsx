import classNames from 'classnames';
import React, {
  KeyboardEventHandler,
  MouseEventHandler,
  ReactNode,
  RefCallback,
  forwardRef,
  useEffect,
  useRef,
  useState,
} from 'react';
import { useMatch } from 'react-router-dom';
import {
  Badge,
  Box,
  Icon,
  IconSrc,
  Icons,
  Line,
  Menu,
  MenuItem,
  PopOut,
  RectCords,
  Text,
  Tooltip,
  TooltipProvider,
  config,
  toRem,
} from 'folds';
import { useAtomValue } from 'jotai';
import FocusTrap from '../../components/FocusTrap';
import { UnreadBadge } from '../../components/unread-badge';
import { UserAvatar } from '../../components/user-avatar';
import { SettingsModal } from '../../components/settings-layout';
import { LogoutDialog } from '../../components/LogoutDialog';
import { usePageNavWidth } from '../../components/page';
import { Settings, SettingsPages } from '../settings';
import { useNativeNavigationScope } from '../../state/hooks/navigationUnread';
import { useHomeSelected } from '../../hooks/router/useHomeSelected';
import { useDirectSelected } from '../../hooks/router/useDirectSelected';
import { useInboxSelected } from '../../hooks/router/useInbox';
import { useMarkRoomsAsRead } from '../../hooks/useMarkRoomsAsRead';
import { allInvitesAtom } from '../../state/room-list/inviteList';
import { laterContentAtom } from '../../state/laterList';
import { getLaterDueSummary } from '../../utils/later';
import { useApprovalInboxSummary } from '../approvals/ApprovalInboxProvider';
import { APPROVALS_PATH } from '../../pages/paths';
import { useNavigateToApprovals } from '../../hooks/useNavigateToApprovals';
import { useCrossSigning } from '../../hooks/useCrossSigning';
import { canOfferNativeDeviceVerification } from '../cross-signing/nativeCrossSigning';
import { useDeviceList, useSplitCurrentDevice } from '../../hooks/useDeviceList';
import { useUserProfile } from '../../hooks/useUserProfile';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import { getSafeMyUserId } from '../../state/nativeIdentity';
import { getMxIdLocalPart } from '../../utils/matrix';
import { nameInitials } from '../../utils/common';
import { stopPropagation } from '../../utils/keyboard';
import {
  BarBadge,
  accountBarState,
  activityBarBadge,
  approvalsBarState,
  barShowsLabels,
  homeBarBadge,
} from './navBarBadges';
import { useNavPaneNavigation } from './useNavPaneNavigation';
import * as depthCss from '../../styles/Depth.css';
import * as css from './NavPane.css';

const menuTrapOptions = (close: () => void) => ({
  initialFocus: false as const,
  onDeactivate: close,
  clickOutsideDeactivates: true,
  isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
  isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
  escapeDeactivates: stopPropagation,
});

/** The badge on an item's icon; hidden from assistive tech, the item's name says it. */
function BarBadgeView({ badge, title }: { badge: BarBadge; title?: string }) {
  if (badge.kind === 'none') return null;
  return (
    <span className={css.BarBadge} aria-hidden="true" title={title || undefined}>
      {badge.kind === 'status' ? (
        <Badge variant={badge.tone} size="400" fill="Solid" radii="Pill" outlined={false}>
          <Text as="span" size="L400">
            {badge.text}
          </Text>
        </Badge>
      ) : (
        <UnreadBadge
          highlight={badge.kind === 'count' && badge.highlight}
          count={badge.kind === 'count' ? badge.count : 0}
        />
      )}
    </span>
  );
}

type BarItemProps = {
  /** Visible label (wide column) and tooltip. */
  label: string;
  /** Accessible name; defaults to `label`. Starts with the label (label in name). */
  name?: string;
  tooltip?: string;
  icon: ReactNode;
  badge: BarBadge;
  badgeTitle?: string;
  selected?: boolean;
  showLabel: boolean;
  onClick: MouseEventHandler<HTMLButtonElement>;
  onContextMenu?: MouseEventHandler<HTMLButtonElement>;
  onKeyDown?: KeyboardEventHandler<HTMLButtonElement>;
  'aria-keyshortcuts'?: string;
  'aria-haspopup'?: 'menu';
  'aria-expanded'?: boolean;
  testId: string;
};

export const BarItem = forwardRef<HTMLButtonElement, BarItemProps>(
  (
    {
      label,
      name,
      tooltip,
      icon,
      badge,
      badgeTitle,
      selected,
      showLabel,
      onClick,
      onContextMenu,
      testId,
      ...aria
    },
    ref
  ) => {
    const assignRef =
      (triggerRef: RefCallback<HTMLElement | SVGElement>) => (el: HTMLButtonElement | null) => {
        triggerRef(el);
        if (typeof ref === 'function') ref(el);
        else if (ref) ref.current = el;
      };
    const tip = tooltip ?? (showLabel ? undefined : label);
    const render = (triggerRef: RefCallback<HTMLElement | SVGElement>) => (
      <button
        ref={assignRef(triggerRef)}
        type="button"
        className={classNames(css.BarItem, depthCss.quietInteractiveSurface)}
        aria-label={name ?? label}
        aria-current={selected ? 'page' : undefined}
        data-testid={testId}
        onClick={onClick}
        onContextMenu={onContextMenu}
        {...aria}
      >
        <span className={css.BarIcon}>
          {icon}
          <BarBadgeView badge={badge} title={badgeTitle} />
        </span>
        {showLabel && <span className={css.BarLabel}>{label}</span>}
      </button>
    );
    if (!tip) return render(() => undefined);
    return (
      <TooltipProvider
        delay={400}
        position="Top"
        tooltip={
          <Tooltip style={{ maxWidth: toRem(280) }}>
            <Text size="T300">{tip}</Text>
          </Tooltip>
        }
      >
        {render}
      </TooltipProvider>
    );
  }
);

const barIcon = (src: IconSrc, filled: boolean) => <Icon size="200" src={src} filled={filled} />;

type HomeMenuProps = { requestClose: () => void };
const HomeMenu = forwardRef<HTMLDivElement, HomeMenuProps>(({ requestClose }, ref) => {
  // One Home list of rooms and DMs (NAV-12): mark both read.
  const { roomIds, unread } = useNativeNavigationScope('merged');
  const { markRead, markingRead, markReadError } = useMarkRoomsAsRead();

  const handleMarkAsRead = async () => {
    if (!unread || markingRead) return;
    if (await markRead(roomIds)) requestClose();
  };

  return (
    <Menu ref={ref} aria-label="Home options" style={{ maxWidth: toRem(200), width: '100vw' }}>
      <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
        <MenuItem
          variant="Surface"
          fill="None"
          onClick={handleMarkAsRead}
          size="300"
          after={<Icon size="100" src={Icons.CheckTwice} />}
          radii="300"
          aria-disabled={!unread}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            Mark as read
          </Text>
        </MenuItem>
        {markReadError && (
          <Text as="p" size="T200" role="alert" style={{ paddingInline: config.space.S200 }}>
            {markReadError}
          </Text>
        )}
      </Box>
    </Menu>
  );
});

/** Home: the one list of rooms and DMs. Right-click (or Shift+F10) marks it all read. */
export function HomeBarItem({ showLabel }: { showLabel: boolean }) {
  const { openHome } = useNavPaneNavigation();
  const { unread, attention } = useNativeNavigationScope('merged');
  // The Home list also hosts `/direct/` routes; Home stays current there.
  const homeSelected = useHomeSelected();
  const directSelected = useDirectSelected();
  const selected = homeSelected || directSelected;
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const badge = homeBarBadge(!!unread, attention);

  const openMenu = (target: HTMLElement) => {
    const rect = target.getBoundingClientRect();
    setMenuAnchor((current) => (current ? undefined : rect));
  };

  return (
    <>
      <BarItem
        label="Home"
        name={`Home${
          badge.kind === 'count'
            ? `, ${badge.count} ${badge.count === 1 ? 'mention or direct message' : 'mentions and direct messages'}`
            : badge.kind === 'dot'
              ? ', unread messages'
              : ''
        }`}
        icon={barIcon(Icons.Home, selected)}
        badge={badge}
        selected={selected}
        showLabel={showLabel}
        testId="nav-bar-home"
        onClick={openHome}
        onContextMenu={(evt) => {
          evt.preventDefault();
          openMenu(evt.currentTarget);
        }}
        aria-keyshortcuts="Shift+F10"
        onKeyDown={(evt) => {
          if (evt.key === 'ContextMenu' || (evt.key === 'F10' && evt.shiftKey)) {
            evt.preventDefault();
            openMenu(evt.currentTarget);
          }
        }}
      />
      {menuAnchor && (
        <PopOut
          anchor={menuAnchor}
          position="Top"
          align="Start"
          content={
            <FocusTrap focusTrapOptions={menuTrapOptions(() => setMenuAnchor(undefined))}>
              <HomeMenu requestClose={() => setMenuAnchor(undefined)} />
            </FocusTrap>
          }
        />
      )}
    </>
  );
}

/** Activity: invites and Later; opens the last Activity page, else what is waiting. */
export function ActivityBarItem({ showLabel }: { showLabel: boolean }) {
  const { openActivity } = useNavPaneNavigation();
  const selected = useInboxSelected();
  const inviteCount = useAtomValue(allInvitesAtom).length;
  const later = getLaterDueSummary(useAtomValue(laterContentAtom));
  const badge = activityBarBadge(inviteCount, later);
  const parts = [
    inviteCount > 0 ? `${inviteCount} ${inviteCount === 1 ? 'invite' : 'invites'}` : '',
    later.active > 0 ? `${later.active} in Later` : '',
  ].filter(Boolean);

  return (
    <BarItem
      label="Activity"
      name={`Activity${parts.length ? `, ${parts.join(', ')}` : ''}`}
      icon={barIcon(Icons.Inbox, selected)}
      badge={badge}
      selected={selected}
      showLabel={showLabel}
      testId="nav-bar-activity"
      onClick={() => openActivity(inviteCount, later.active)}
    />
  );
}

/** Approvals: pending agent approvals and how complete that count is. */
export function ApprovalsBarItem({ showLabel }: { showLabel: boolean }) {
  const navigateToApprovals = useNavigateToApprovals();
  const selected = Boolean(useMatch({ path: APPROVALS_PATH, end: false }));
  const { label, coverageDescription, badge } = approvalsBarState(useApprovalInboxSummary());
  return (
    <BarItem
      label="Approvals"
      name={label}
      tooltip={label}
      icon={barIcon(Icons.Shield, selected)}
      badge={badge}
      badgeTitle={coverageDescription}
      selected={selected}
      showLabel={showLabel}
      testId="nav-bar-approvals"
      onClick={navigateToApprovals}
    />
  );
}

type AccountMenuProps = {
  verify?: 'this-device' | 'other-devices';
  onSettings: () => void;
  onVerify: () => void;
  onLogout: () => void;
};
const AccountMenu = forwardRef<HTMLDivElement, AccountMenuProps>(
  ({ verify, onSettings, onVerify, onLogout }, ref) => (
    <Menu ref={ref} aria-label="Account" style={{ maxWidth: toRem(240), width: '100vw' }}>
      <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
        <MenuItem
          variant="Surface"
          fill="None"
          size="300"
          radii="300"
          after={<Icon size="100" src={Icons.Setting} />}
          onClick={onSettings}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            Settings
          </Text>
        </MenuItem>
        {verify && (
          <MenuItem
            variant={verify === 'this-device' ? 'Critical' : 'Warning'}
            fill="None"
            size="300"
            radii="300"
            after={<Icon size="100" src={Icons.ShieldUser} />}
            onClick={onVerify}
          >
            <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
              {verify === 'this-device' ? 'Verify this device' : 'Review unverified devices'}
            </Text>
          </MenuItem>
        )}
      </Box>
      <Line variant="Surface" size="300" />
      <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
        <MenuItem
          variant="Critical"
          fill="None"
          size="300"
          radii="300"
          after={<Icon size="100" src={Icons.Power} />}
          onClick={onLogout}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            Log out
          </Text>
        </MenuItem>
      </Box>
    </Menu>
  )
);

function AccountBarItemView({
  showLabel,
  ownUnverified,
  otherUnverifiedCount,
  canOffer,
}: {
  showLabel: boolean;
  ownUnverified: boolean;
  otherUnverifiedCount: number;
  canOffer: boolean;
}) {
  const useAuthentication = useMediaAuthentication();
  const userId = getSafeMyUserId();
  const profile = useUserProfile(userId);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const [settingsPage, setSettingsPage] = useState<SettingsPages | 'default'>();
  const [logout, setLogout] = useState(false);
  const { label, badge, verify } = accountBarState(canOffer, ownUnverified, otherUnverifiedCount);

  const displayName = profile.displayName ?? getMxIdLocalPart(userId) ?? userId;
  const avatarUrl = profile.avatarUrl
    ? resolveMatrixThumbnailUrl(profile.avatarUrl, 96, { useAuthentication })
    : undefined;
  const closeMenu = () => setMenuAnchor(undefined);
  const closeSettings = () => setSettingsPage(undefined);

  return (
    <>
      <BarItem
        ref={triggerRef}
        label="Account"
        name={label}
        tooltip={displayName ? `${displayName} · ${userId}` : undefined}
        icon={
          <span className={css.BarAvatar}>
            <UserAvatar
              userId={userId}
              src={avatarUrl}
              renderFallback={() => (
                <Text as="span" size="T200">
                  {nameInitials(displayName)}
                </Text>
              )}
            />
          </span>
        }
        badge={badge}
        selected={false}
        showLabel={showLabel}
        testId="nav-bar-account"
        aria-haspopup="menu"
        aria-expanded={!!menuAnchor}
        onClick={(evt) => {
          const rect = evt.currentTarget.getBoundingClientRect();
          setMenuAnchor((current) => (current ? undefined : rect));
        }}
      />
      {menuAnchor && (
        <PopOut
          anchor={menuAnchor}
          position="Top"
          align="End"
          content={
            <FocusTrap focusTrapOptions={menuTrapOptions(closeMenu)}>
              <AccountMenu
                verify={verify}
                onSettings={() => {
                  closeMenu();
                  setSettingsPage('default');
                }}
                onVerify={() => {
                  closeMenu();
                  setSettingsPage(SettingsPages.DevicesPage);
                }}
                onLogout={() => {
                  closeMenu();
                  setLogout(true);
                }}
              />
            </FocusTrap>
          }
        />
      )}
      {settingsPage && (
        <SettingsModal requestClose={closeSettings}>
          <Settings
            initialPage={settingsPage === 'default' ? undefined : settingsPage}
            requestClose={closeSettings}
          />
        </SettingsModal>
      )}
      {logout && <LogoutDialog onClose={() => setLogout(false)} />}
    </>
  );
}

type DeviceVerificationState = { ownUnverified: boolean; otherUnverifiedCount: number };

/** Reads the device list (only mounted once verification can be offered, as on the rail). */
function DeviceVerificationProbe({
  onChange,
}: {
  onChange: (state: DeviceVerificationState) => void;
}) {
  const [deviceSnapshot] = useDeviceList();
  const [, otherDevices] = useSplitCurrentDevice(deviceSnapshot?.devices);
  const ownUnverified = deviceSnapshot?.ownVerification === 'unverified';
  const otherUnverifiedCount =
    otherDevices?.filter((device) => device.trust === 'unverified').length ?? 0;
  useEffect(() => {
    onChange({ ownUnverified, otherUnverifiedCount });
  }, [onChange, ownUnverified, otherUnverifiedCount]);
  return null;
}

/**
 * The account avatar: Settings, Verify this device (when needed) and Log out.
 * It carries the rail's unverified warning as a badge, and only reads the
 * device list once cross-signing can offer verification at all.
 */
export function AccountBarItem({ showLabel }: { showLabel: boolean }) {
  const crossSigning = useCrossSigning();
  const canOffer = canOfferNativeDeviceVerification(crossSigning.nativeStatus);
  const [devices, setDevices] = useState<DeviceVerificationState>({
    ownUnverified: false,
    otherUnverifiedCount: 0,
  });
  return (
    <>
      {canOffer && <DeviceVerificationProbe onChange={setDevices} />}
      <AccountBarItemView
        showLabel={showLabel}
        canOffer={canOffer}
        ownUnverified={canOffer && devices.ownUnverified}
        otherUnverifiedCount={canOffer ? devices.otherUnverifiedCount : 0}
      />
    </>
  );
}

/**
 * The list column's bottom bar (N2): Home, Activity, Approvals and the
 * account. Icons with tooltips; text labels once the column is wider than
 * 300px (and on Mobile, where it fills the window).
 */
export function NavBottomBar() {
  const showLabel = barShowsLabels(usePageNavWidth());
  return (
    <nav className={css.Bar} aria-label="Main" data-testid="nav-bottom-bar">
      <HomeBarItem showLabel={showLabel} />
      <ActivityBarItem showLabel={showLabel} />
      <ApprovalsBarItem showLabel={showLabel} />
      <AccountBarItem showLabel={showLabel} />
    </nav>
  );
}
